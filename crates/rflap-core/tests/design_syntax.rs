//! Design engine: the constraint text syntax and the JSON form of a spec.

use rflap_core::design::{
    Cmp, Constraint, DesignAutomaton, DesignError, DesignOptions, LanguageSpec, design,
};

fn parse_err(input: &str) -> rflap_core::design::ConstraintSyntaxError {
    Constraint::parse(input).unwrap_err()
}

#[test]
fn parses_the_documented_example_into_a_flat_conjunction() {
    let c = Constraint::parse(r#"ends_with("ab") and count(a) % 2 == 0 and not contains("bb")"#)
        .unwrap();
    assert_eq!(
        c,
        Constraint::And {
            constraints: vec![
                Constraint::EndsWith {
                    word: "ab".to_string()
                },
                Constraint::CountMod {
                    symbols: "a".to_string(),
                    m: 2,
                    r: 0
                },
                Constraint::Not {
                    constraint: Box::new(Constraint::Contains {
                        word: "bb".to_string()
                    })
                },
            ]
        }
    );
}

#[test]
fn precedence_is_not_then_and_then_xor_then_or() {
    let c =
        Constraint::parse("length == 1 or length == 2 and length == 3 xor length == 4").unwrap();
    let Constraint::Or { constraints } = c else {
        panic!("top level must be `or`")
    };
    assert_eq!(constraints.len(), 2);
    assert!(matches!(constraints[1], Constraint::Xor { .. }));
    let c = Constraint::parse("!(length == 1) && length != 2 || length < 3").unwrap();
    assert!(matches!(c, Constraint::Or { .. }));
}

#[test]
fn every_primitive_has_a_text_form_that_round_trips() {
    for text in [
        r#"starts_with("ab")"#,
        r#"ends_with("")"#,
        r#"contains("a\"b")"#,
        "length == 3",
        "length != 3",
        "length < 3",
        "length <= 3",
        "length > 3",
        "length >= 3",
        "length % 4 == 1",
        "count(a) == 2",
        "count(a,b) >= 2",
        "count(a) % 3 == 2",
        "count(a) == count(b)",
        r#"regex("(a+b)*abb")"#,
        "not length == 1",
        "(length == 1 or length == 2) and count(a) > 0",
        "length == 1 xor length == 2",
    ] {
        let c = Constraint::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"));
        let shown = c.to_string();
        assert_eq!(Constraint::parse(&shown).unwrap(), c, "{text} -> {shown}");
    }
}

#[test]
fn lenient_spellings_are_accepted() {
    let canonical = Constraint::parse("count(a) % 2 == 0 and length >= 2").unwrap();
    for text in [
        "count(a) % 2 = 0 && len >= 2",
        "count( a ) % 2 == 0 and length >= 2",
        "count(\"a\") % 2 == 0 and length() >= 2",
    ] {
        assert_eq!(Constraint::parse(text).unwrap(), canonical, "{text}");
    }
    // `% m != r` is the negation of `% m == r`.
    assert_eq!(
        Constraint::parse("count(a) % 2 != 0").unwrap(),
        Constraint::Not {
            constraint: Box::new(Constraint::CountMod {
                symbols: "a".to_string(),
                m: 2,
                r: 0
            })
        }
    );
    assert_eq!(
        Constraint::parse("count(ab) >= 1").unwrap(),
        Constraint::Count {
            symbols: "ab".to_string(),
            cmp: Cmp::Ge,
            n: 1
        }
    );
}

#[test]
fn syntax_errors_carry_the_position_and_what_was_expected() {
    // (input, position, found, expected fragment)
    let cases = [
        (r#"ends_with("ab") and"#, 19, "end of input", "a condition"),
        ("contains(ab)", 9, "`ab`", "a quoted string"),
        ("length >= x", 10, "`x`", "a non-negative integer"),
        (r#"ends_with("ab") foo"#, 16, "`foo`", "`and`, `or`, `xor`"),
        (r#"ends_with("ab""#, 14, "end of input", "`)`"),
        (r#"(ends_with("ab")"#, 16, "end of input", "`)`"),
        ("ends_with(\"ab)", 10, "`\"`", "a closing quote"),
        ("count(a) % 2", 12, "end of input", "`==` or `!=`"),
        ("count(a) ~ 2", 9, "`~`", "a comparison"),
        ("colour(a) == 1", 0, "`colour`", "a condition"),
        ("count() == 1", 6, "`)`", "at least one symbol"),
        ("", 0, "end of input", "a condition"),
        ("length == 1 )", 12, "`)`", "`and`, `or`, `xor`"),
    ];
    for (input, position, found, expected) in cases {
        let err = parse_err(input);
        assert_eq!(err.position, position, "{input}: {err}");
        assert_eq!(err.found, found, "{input}: {err}");
        assert!(err.expected.contains(expected), "{input}: {err}");
        assert!(
            err.message
                .starts_with(&format!("at position {position}: expected ")),
            "{err}"
        );
    }
}

#[test]
fn syntax_errors_surface_through_design_with_their_position() {
    let spec = LanguageSpec::new(
        "ab",
        rflap_core::design::LanguageSource::ConstraintText("length >= ".to_string()),
    );
    match design(&spec, &DesignOptions::default()).unwrap_err() {
        DesignError::ConstraintSyntax(e) => {
            assert_eq!(e.position, 10);
            assert_eq!(e.found, "end of input");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_spec_arrives_as_json_from_an_ai() {
    let json = r#"{
        "alphabet": "ab",
        "language": {"constraint_text": "ends_with(\"ab\") and count(a) % 2 == 0"},
        "examples": {"accept": ["aab"], "reject": ["ab"]}
    }"#;
    let spec: LanguageSpec = serde_json::from_str(json).unwrap();
    let d = design(&spec, &DesignOptions::default()).unwrap();
    assert!(d.verification.ok, "{:?}", d.verification);
    assert!(matches!(d.automaton, DesignAutomaton::Fa(_)));

    let json = r#"{
        "alphabet": ["a", "b"],
        "language": {"constraint": {"type": "and", "constraints": [
            {"type": "count_mod", "symbols": "a", "m": 2, "r": 0},
            {"type": "length", "cmp": ">=", "n": 2}
        ]}}
    }"#;
    let spec: LanguageSpec = serde_json::from_str(json).unwrap();
    let d = design(&spec, &DesignOptions::default()).unwrap();
    assert!(d.verification.ok);
    assert_eq!(d.accepted[..3].to_vec(), vec!["aa", "bb", "aab"]);
}

#[test]
fn json_regex_grammar_and_template_forms_deserialize() {
    for json in [
        r#"{"alphabet":"ab","language":{"regex":"(a+b)*abb"}}"#,
        r#"{"alphabet":"ab","language":{"regular_grammar":"S -> aS | b"}}"#,
        r#"{"alphabet":"ab","language":{"template":{"name":"an_bn","a":"a","b":"b"}}}"#,
        r#"{"alphabet":"ab","language":{"template":{"name":"an_bm","a":"a","b":"b","relation":"ne"}}}"#,
        r#"{"alphabet":"()","language":{"template":{"name":"balanced_brackets"}}}"#,
        r#"{"alphabet":"ab","language":{"template":{"name":"even_palindrome"}}}"#,
    ] {
        let spec: LanguageSpec =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("{json}: {e}"));
        let d = design(&spec, &DesignOptions::default()).unwrap_or_else(|e| panic!("{json}: {e}"));
        assert!(d.verification.ok, "{json}: {:?}", d.verification);
    }
}

#[test]
fn a_multi_character_symbol_in_a_json_alphabet_is_reported() {
    let spec: LanguageSpec = serde_json::from_str(
        r#"{"alphabet":["a","ab"],"language":{"constraint_text":"length == 1"}}"#,
    )
    .unwrap();
    assert!(matches!(
        design(&spec, &DesignOptions::default()).unwrap_err(),
        DesignError::InvalidAlphabet { .. }
    ));
}
