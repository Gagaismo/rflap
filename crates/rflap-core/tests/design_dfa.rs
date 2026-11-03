//! Design engine: regular languages (constraints, regular expressions, regular grammars).

use rflap_core::design::{
    Cmp, Constraint, Design, DesignAutomaton, DesignError, DesignKind, DesignOptions,
    LanguageSource, LanguageSpec, design, explain,
};
use rflap_core::fa::Fa;
use rflap_core::fa_convert::{equivalent, regex_to_nfa};
use rflap_core::regex::Regex;

fn fa_of(d: &Design) -> &Fa {
    match &d.automaton {
        DesignAutomaton::Fa(fa) => fa,
        DesignAutomaton::Pda(_) => panic!("expected a finite automaton"),
    }
}

fn spec(alphabet: &str, source: LanguageSource) -> LanguageSpec {
    LanguageSpec::new(alphabet, source)
}

fn text(alphabet: &str, constraint: &str) -> LanguageSpec {
    spec(
        alphabet,
        LanguageSource::ConstraintText(constraint.to_string()),
    )
}

fn up_to(len: usize) -> DesignOptions {
    DesignOptions {
        accepted_max_len: len,
        accepted_limit: 10_000,
        ..DesignOptions::default()
    }
}

type Edge = (String, String, String);

/// `(from, symbol, to)` triples by state name.
fn edges(fa: &Fa) -> Vec<Edge> {
    let name = |id| fa.state(id).unwrap().name.clone();
    fa.transitions()
        .iter()
        .map(|t| (name(t.from), t.label.0.clone(), name(t.to)))
        .collect()
}

fn e(from: &str, sym: &str, to: &str) -> Edge {
    (from.to_string(), sym.to_string(), to.to_string())
}

#[test]
fn ends_with_ab_is_a_three_state_minimal_dfa() {
    let d = design(&text("ab", r#"ends_with("ab")"#), &up_to(3)).unwrap();
    let fa = fa_of(&d);
    assert_eq!(d.kind, DesignKind::Dfa);
    assert_eq!(d.stats.states, 3);
    assert_eq!(d.stats.final_states, 1);
    assert_eq!(fa.initial(), Some(0));
    assert_eq!(fa.finals().iter().copied().collect::<Vec<_>>(), vec![2]);
    assert_eq!(
        edges(fa),
        vec![
            e("q0", "a", "q1"),
            e("q0", "b", "q0"),
            e("q1", "a", "q1"),
            e("q1", "b", "q2"),
            e("q2", "a", "q1"),
            e("q2", "b", "q0"),
        ]
    );
    assert_eq!(d.accepted, vec!["ab", "aab", "bab"]);
    assert!(d.verification.ok);
    assert!(d.verification.examples.is_empty());
}

#[test]
fn states_are_named_in_bfs_order_and_have_a_meaning() {
    let d = design(&text("ab", r#"ends_with("ab")"#), &DesignOptions::default()).unwrap();
    let fa = fa_of(&d);
    let names: Vec<_> = fa.states().map(|s| s.name.clone()).collect();
    assert_eq!(names, vec!["q0", "q1", "q2"]);
    let labels: Vec<_> = fa.states().map(|s| s.label.clone().unwrap()).collect();
    assert_eq!(
        labels,
        vec![
            r#"ends with no start of "ab""#,
            r#"ends with "a", the start of "ab""#,
            r#"ends with "ab""#,
        ]
    );
    let infos: Vec<_> = d.states.iter().map(|s| s.meaning.clone()).collect();
    assert_eq!(infos, labels);
    let access: Vec<_> = d
        .states
        .iter()
        .map(|s| s.access_word.clone().unwrap())
        .collect();
    assert_eq!(access, vec!["", "a", "ab"]);
}

#[test]
fn even_a_and_ends_with_b_has_the_three_state_minimal_product() {
    let d = design(
        &text("ab", r#"count(a) % 2 == 0 and ends_with("b")"#),
        &up_to(3),
    )
    .unwrap();
    let fa = fa_of(&d);
    assert_eq!(d.stats.states, 3);
    assert!(d.stats.states_before_minimization.unwrap() >= 4);
    let labels: Vec<_> = fa.states().map(|s| s.label.clone().unwrap()).collect();
    assert_eq!(
        labels,
        vec![
            r#"count(a) is even, does not end with "b""#,
            "count(a) is odd",
            r#"count(a) is even, ends with "b""#,
        ]
    );
    assert_eq!(d.accepted, vec!["b", "bb", "aab", "bbb"]);
    assert!(d.verification.ok);
}

#[test]
fn not_contains_aa_is_exact() {
    let d = design(
        &spec(
            "ab",
            LanguageSource::Constraint(Constraint::Not {
                constraint: Box::new(Constraint::Contains {
                    word: "aa".to_string(),
                }),
            }),
        ),
        &up_to(3),
    )
    .unwrap();
    let fa = fa_of(&d);
    assert_eq!(d.stats.states, 3);
    assert!(d.stats.has_trap_state);
    assert_eq!(
        edges(fa),
        vec![
            e("q0", "a", "q1"),
            e("q0", "b", "q0"),
            e("q1", "a", "q2"),
            e("q1", "b", "q0"),
            e("q2", "a", "q2"),
            e("q2", "b", "q2"),
        ]
    );
    assert_eq!(fa.finals().iter().copied().collect::<Vec<_>>(), vec![0, 1]);
    assert_eq!(
        d.accepted,
        vec![
            "", "a", "b", "ab", "ba", "bb", "aba", "abb", "bab", "bba", "bbb"
        ]
    );
    assert!(
        fa.state(2)
            .unwrap()
            .label
            .as_ref()
            .unwrap()
            .contains("trap state")
    );
}

#[test]
fn the_trap_state_can_be_left_out() {
    let options = DesignOptions {
        include_trap_state: false,
        ..DesignOptions::default()
    };
    let d = design(&text("ab", r#"not contains("aa")"#), &options).unwrap();
    let fa = fa_of(&d);
    assert_eq!(d.stats.states, 2);
    assert!(!d.stats.has_trap_state);
    assert_eq!(
        edges(fa),
        vec![e("q0", "a", "q1"), e("q0", "b", "q0"), e("q1", "b", "q0")]
    );
    assert!(d.verification.ok);
}

#[test]
fn regex_a_or_b_star_abb_has_four_states() {
    let d = design(
        &spec("ab", LanguageSource::Regex("(a+b)*abb".to_string())),
        &up_to(4),
    )
    .unwrap();
    assert_eq!(d.stats.states, 4);
    assert_eq!(d.accepted, vec!["abb", "aabb", "babb"]);
    assert!(d.verification.ok);
    // The derived regular expression denotes the same language.
    let regex = Regex::parse(d.regex.as_ref().unwrap()).unwrap();
    assert!(equivalent(&regex_to_nfa(&regex), fa_of(&d)).equivalent);
}

#[test]
fn regex_symbols_outside_the_alphabet_are_reported() {
    let err = design(
        &spec("ab", LanguageSource::Regex("(a+c)*".to_string())),
        &DesignOptions::default(),
    )
    .unwrap_err();
    assert!(
        matches!(err, DesignError::AlphabetViolation { symbol: 'c', .. }),
        "{err:?}"
    );
    let err = design(
        &spec("ab", LanguageSource::Regex("(a+b".to_string())),
        &DesignOptions::default(),
    )
    .unwrap_err();
    assert!(matches!(err, DesignError::RegexSyntax { .. }), "{err:?}");
}

#[test]
fn length_and_count_primitives() {
    let cases: [(&str, &[&str]); 5] = [
        ("length == 2", &["aa", "ab", "ba", "bb"]),
        ("length % 3 == 1", &["a", "b", "aaaa", "aaab"]),
        (
            "count(a) >= 2 and length <= 3",
            &["aa", "aaa", "aab", "aba", "baa"],
        ),
        ("count(a,b) != 1", &["", "aa", "ab", "ba", "bb"]),
        ("count(b) < 1 xor length == 1", &["", "b", "aa"]),
    ];
    for (constraint, expected) in cases {
        let d = design(&text("ab", constraint), &up_to(4)).unwrap();
        assert!(d.verification.ok, "{constraint}: {:?}", d.verification);
        for w in expected {
            assert!(d.accepted.contains(&w.to_string()), "{constraint}: {w}");
        }
    }
    let d = design(&text("ab", "length == 2"), &up_to(5)).unwrap();
    assert_eq!(d.accepted, vec!["aa", "ab", "ba", "bb"]);
    assert_eq!(d.stats.states, 4);
    let d = design(&text("ab", "count(a) % 3 == 1"), &DesignOptions::default()).unwrap();
    assert_eq!(d.stats.states, 3);
}

#[test]
fn starts_with_and_or_combinations() {
    let d = design(
        &text("ab", r#"starts_with("ab") or starts_with("ba")"#),
        &up_to(3),
    )
    .unwrap();
    assert_eq!(d.accepted, vec!["ab", "ba", "aba", "abb", "baa", "bab"]);
    assert!(d.verification.ok);
}

#[test]
fn regex_constraint_composes_with_the_others() {
    let d = design(&text("ab", r#"regex("a*b") and length > 1"#), &up_to(4)).unwrap();
    assert_eq!(d.accepted, vec!["ab", "aab", "aaab"]);
    assert!(d.verification.ok);
}

#[test]
fn examples_report_exactly_which_ones_fail() {
    let s = text("ab", r#"ends_with("ab")"#).with_examples(&["ab", "b"], &["ab", "a"]);
    let d = design(&s, &DesignOptions::default()).unwrap();
    let v = &d.verification;
    assert_eq!(v.failed_examples, 2);
    assert!(!v.ok);
    assert!(v.cross_check.ok, "a bad example is not a bad automaton");
    let summary: Vec<_> = v
        .examples
        .iter()
        .map(|x| (x.word.as_str(), x.expected_accept, x.actual, x.passed))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("ab", true, Some(true), true),
            ("b", true, Some(false), false),
            ("ab", false, Some(true), false),
            ("a", false, Some(false), true),
        ]
    );
    assert_eq!(
        v.examples[1].detail.as_deref(),
        Some("the constraint rejects this string")
    );
    assert!(v.examples[0].detail.is_none());
}

#[test]
fn failed_examples_of_a_conjunction_name_the_false_part() {
    let s = text("ab", r#"ends_with("b") and count(a) % 2 == 0"#).with_examples(&["ab"], &[]);
    let d = design(&s, &DesignOptions::default()).unwrap();
    let detail = d.verification.examples[0].detail.clone().unwrap();
    assert!(detail.contains(r#"`ends_with("b")` is true"#), "{detail}");
    assert!(detail.contains("`count(a) % 2 == 0` is false"), "{detail}");
}

#[test]
fn alphabet_violations_name_the_symbol_and_where() {
    let err = design(
        &text("ab", r#"ends_with("abc")"#),
        &DesignOptions::default(),
    )
    .unwrap_err();
    match err {
        DesignError::AlphabetViolation {
            symbol,
            position,
            context,
            message,
        } => {
            assert_eq!((symbol, position), ('c', 2));
            assert_eq!(context, r#"ends_with("abc")"#);
            assert!(message.contains("{a, b}"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    let s = text("ab", "length == 1").with_examples(&["a", "abx"], &[]);
    match design(&s, &DesignOptions::default()).unwrap_err() {
        DesignError::AlphabetViolation {
            symbol,
            position,
            context,
            ..
        } => {
            assert_eq!((symbol, position), ('x', 2));
            assert!(context.starts_with("examples.accept[1]"), "{context}");
        }
        other => panic!("{other:?}"),
    }
    let s = text("ab", "length == 1").with_examples(&[], &["ε"]);
    match design(&s, &DesignOptions::default()).unwrap_err() {
        DesignError::AlphabetViolation { message, .. } => {
            assert!(message.contains("empty string as \"\""), "{message}")
        }
        other => panic!("{other:?}"),
    }
    let err = design(&text("ab", "count(c) == 1"), &DesignOptions::default()).unwrap_err();
    assert!(
        matches!(err, DesignError::AlphabetViolation { symbol: 'c', .. }),
        "{err:?}"
    );
}

#[test]
fn regex_syntax_characters_cannot_be_symbols_of_a_regex_spec() {
    let s = spec("a+", LanguageSource::Regex("a".to_string()));
    match design(&s, &DesignOptions::default()).unwrap_err() {
        DesignError::InvalidAlphabet { message } => assert!(message.contains("'+'"), "{message}"),
        other => panic!("{other:?}"),
    }
    // Constraints without regular expressions have no such restriction.
    let d = design(&text("a+", r#"ends_with("+")"#), &DesignOptions::default()).unwrap();
    assert_eq!(d.stats.states, 2);
    assert!(d.regex.is_none());
}

#[test]
fn bad_alphabets_and_parameters_are_precise() {
    let bad_alphabets: [(&[&str], &str); 4] = [
        (&[], "empty"),
        (&["ab"], "exactly one character"),
        (&["a", "a"], "twice"),
        (&["a", "λ"], "reserved"),
    ];
    for (alphabet, needle) in bad_alphabets {
        let s = LanguageSpec {
            alphabet: alphabet.iter().map(|s| s.to_string()).collect(),
            language: LanguageSource::ConstraintText("length == 1".to_string()),
            examples: Default::default(),
        };
        match design(&s, &DesignOptions::default()).unwrap_err() {
            DesignError::InvalidAlphabet { message } => {
                assert!(message.contains(needle), "{message}")
            }
            other => panic!("{other:?}"),
        }
    }
    for (constraint, needle) in [
        ("length % 0 == 0", "modulus must be at least 1"),
        (
            "count(a) % 2 == 2",
            "remainder 2 must be less than the modulus 2",
        ),
        ("length == 9999999", "above the supported maximum"),
    ] {
        let err = design(&text("ab", constraint), &DesignOptions::default()).unwrap_err();
        assert!(err.to_string().contains(needle), "{constraint}: {err}");
    }
}

#[test]
fn right_linear_grammar_builds_the_same_dfa_as_the_constraint() {
    let g = spec(
        "ab",
        LanguageSource::RegularGrammar("S -> aS | bS | aA\nA -> b".to_string()),
    );
    let d = design(&g, &up_to(3)).unwrap();
    assert_eq!(d.stats.states, 3);
    assert_eq!(d.accepted, vec!["ab", "aab", "bab"]);
    assert!(d.verification.ok);
    let c = design(&text("ab", r#"ends_with("ab")"#), &up_to(3)).unwrap();
    assert!(equivalent(fa_of(&d), fa_of(&c)).equivalent);
}

#[test]
fn left_linear_grammar_is_supported() {
    let g = spec(
        "ab",
        LanguageSource::RegularGrammar("S -> Sa | Sb | ab".to_string()),
    );
    let d = design(&g, &up_to(3)).unwrap();
    assert_eq!(d.stats.states, 4);
    assert_eq!(d.accepted, vec!["ab", "aba", "abb"]);
    assert!(d.verification.ok);
}

#[test]
fn grammars_with_lambda_and_unit_productions() {
    let g = spec(
        "ab",
        LanguageSource::RegularGrammar("S -> aS | B\nB -> bB | λ".to_string()),
    );
    let d = design(&g, &up_to(3)).unwrap();
    assert_eq!(
        d.accepted,
        vec!["", "a", "b", "aa", "ab", "bb", "aaa", "aab", "abb", "bbb"]
    );
    assert!(d.verification.ok);
}

#[test]
fn non_regular_grammars_are_rejected_with_the_reason() {
    let g = spec(
        "ab",
        LanguageSource::RegularGrammar("S -> aSb | λ".to_string()),
    );
    match design(&g, &DesignOptions::default()).unwrap_err() {
        DesignError::NotRegularGrammar { message } => {
            assert!(message.contains("production `S → aSb`"), "{message}")
        }
        other => panic!("{other:?}"),
    }
    let g = spec("ab", LanguageSource::RegularGrammar("S -> cS".to_string()));
    assert!(matches!(
        design(&g, &DesignOptions::default()).unwrap_err(),
        DesignError::AlphabetViolation { symbol: 'c', .. }
    ));
}

#[test]
fn the_empty_and_universal_languages() {
    let d = design(
        &text("ab", r#"contains("a") and not contains("a")"#),
        &up_to(3),
    )
    .unwrap();
    assert_eq!(d.stats.states, 1);
    assert!(d.accepted.is_empty());
    assert!(d.verification.ok);
    let d = design(&text("ab", "length >= 0"), &up_to(1)).unwrap();
    assert_eq!(d.stats.states, 1);
    assert_eq!(d.accepted, vec!["", "a", "b"]);
}

#[test]
fn design_is_deterministic_down_to_the_json() {
    let s = text(
        "abc",
        r#"count(a) % 2 == 0 and ends_with("b") and not contains("cc")"#,
    )
    .with_examples(&["b"], &["a"]);
    let first = serde_json::to_string(&design(&s, &DesignOptions::default()).unwrap()).unwrap();
    let second = serde_json::to_string(&design(&s, &DesignOptions::default()).unwrap()).unwrap();
    assert_eq!(first, second);
}

#[test]
fn states_get_distinct_laid_out_positions() {
    let d = design(
        &text("ab", r#"ends_with("abb")"#),
        &DesignOptions::default(),
    )
    .unwrap();
    let fa = fa_of(&d);
    let points: Vec<_> = fa
        .states()
        .map(|s| (s.point.x as i64, s.point.y as i64))
        .collect();
    for (i, p) in points.iter().enumerate() {
        assert!(!points[..i].contains(p), "{points:?}");
    }
}

#[test]
fn cross_check_covers_every_string_up_to_the_configured_length() {
    let d = design(&text("ab", "length >= 0"), &DesignOptions::default()).unwrap();
    assert_eq!(d.verification.cross_check.max_len, 8);
    assert_eq!(d.verification.cross_check.strings_checked, 511);
    let small = DesignOptions {
        check_len: 3,
        ..DesignOptions::default()
    };
    let d = design(&text("ab", "length >= 0"), &small).unwrap();
    assert_eq!(d.verification.cross_check.strings_checked, 15);
    let capped = DesignOptions {
        max_check_strings: 100,
        ..DesignOptions::default()
    };
    let d = design(&text("ab", "length >= 0"), &capped).unwrap();
    assert_eq!(d.verification.cross_check.max_len, 5);
    assert_eq!(d.verification.cross_check.requested_len, 8);
    assert_eq!(d.verification.cross_check.strings_checked, 63);
}

#[test]
fn explain_summarizes_the_automaton_for_a_chat_answer() {
    let d = design(&text("ab", r#"ends_with("ab")"#), &DesignOptions::default()).unwrap();
    let out = explain(fa_of(&d));
    for needle in [
        "Finite automaton with 3 states and 6 transitions.",
        "Start state: q0.",
        "Accepting states: q2.",
        "a complete DFA",
        r#"- q2: ends with "ab""#,
        "| state | a | b |",
        "| →q0 | q1 | q0 |",
        "| q1 | q1 | q2 |",
        "| *q2 | q1 | q0 |",
    ] {
        assert!(out.contains(needle), "missing {needle:?} in:\n{out}");
    }
}

#[test]
fn constraint_values_round_trip_through_text_and_json() {
    let c = Constraint::And {
        constraints: vec![
            Constraint::CountMod {
                symbols: "a".to_string(),
                m: 2,
                r: 0,
            },
            Constraint::Length { cmp: Cmp::Ge, n: 2 },
        ],
    };
    let json = serde_json::to_string(&c).unwrap();
    assert_eq!(serde_json::from_str::<Constraint>(&json).unwrap(), c);
    assert_eq!(Constraint::parse(&c.to_string()).unwrap(), c);
}

#[test]
fn products_too_big_to_build_are_refused_not_attempted() {
    let s = text("ab", "count(a) % 1000 == 3 and count(b) % 1000 == 4");
    match design(&s, &DesignOptions::default()).unwrap_err() {
        DesignError::TooLarge { message } => assert!(message.contains("20000"), "{message}"),
        other => panic!("{other:?}"),
    }
}
