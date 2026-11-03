//! Design engine: non-regular templates (PDAs) and the not-regular report.

use rflap_core::design::{
    Design, DesignAutomaton, DesignError, DesignKind, DesignOptions, LanguageSource, LanguageSpec,
    PdaAcceptance, Relation, Template, design,
};
use rflap_core::pda::Pda;

fn template(alphabet: &str, t: Template) -> LanguageSpec {
    LanguageSpec::new(alphabet, LanguageSource::Template(t))
}

fn pda_of(d: &Design) -> &Pda {
    match &d.automaton {
        DesignAutomaton::Pda(p) => p,
        DesignAutomaton::Fa(_) => panic!("expected a pushdown automaton"),
    }
}

fn up_to(len: usize) -> DesignOptions {
    DesignOptions {
        accepted_max_len: len,
        accepted_limit: 10_000,
        ..DesignOptions::default()
    }
}

#[test]
fn an_bn_accepts_and_rejects_exact_strings() {
    let s = template(
        "ab",
        Template::AnBn {
            a: 'a',
            b: 'b',
            min: 0,
        },
    )
    .with_examples(
        &["", "ab", "aabb", "aaabbb"],
        &["a", "b", "ba", "aab", "abb", "abab"],
    );
    let d = design(&s, &up_to(6)).unwrap();
    assert_eq!(d.kind, DesignKind::Pda);
    assert_eq!(d.acceptance, Some(PdaAcceptance::FinalState));
    assert_eq!(d.accepted, vec!["", "ab", "aabb", "aaabbb"]);
    assert_eq!(d.verification.failed_examples, 0);
    let cc = &d.verification.cross_check;
    assert_eq!((cc.max_len, cc.strings_checked), (8, 511));
    assert_eq!((cc.mismatch_count, cc.inconclusive), (0, 0));
    assert!(cc.ok && d.verification.ok);
    assert!(d.regex.is_none());
    assert_eq!(d.stats.states, 3);
    let pda = pda_of(&d);
    assert_eq!(pda.initial(), Some(0));
    assert!(pda.state(0).unwrap().label.is_some());
}

#[test]
fn an_bn_with_min_one_rejects_the_empty_string() {
    let d = design(
        &template(
            "ab",
            Template::AnBn {
                a: 'a',
                b: 'b',
                min: 1,
            },
        ),
        &up_to(4),
    )
    .unwrap();
    assert_eq!(d.accepted, vec!["ab", "aabb"]);
    assert!(d.verification.ok);
}

#[test]
fn an_bm_relations() {
    let cases = [
        (
            Relation::Le,
            vec!["", "b", "ab", "abb", "aabb"],
            vec!["a", "aab", "ba"],
        ),
        (
            Relation::Ge,
            vec!["", "a", "ab", "aab", "aabb"],
            vec!["b", "abb", "ba"],
        ),
        (
            Relation::Ne,
            vec!["a", "b", "aab", "abb"],
            vec!["", "ab", "aabb", "ba"],
        ),
    ];
    for (relation, accept, reject) in cases {
        let s = template(
            "ab",
            Template::AnBm {
                a: 'a',
                b: 'b',
                relation,
            },
        )
        .with_examples(&accept, &reject);
        let d = design(&s, &DesignOptions::default()).unwrap();
        assert_eq!(
            d.verification.failed_examples, 0,
            "{relation:?}: {:?}",
            d.verification
        );
        assert!(d.verification.cross_check.ok, "{relation:?}");
    }
    let d = design(
        &template(
            "ab",
            Template::AnBm {
                a: 'a',
                b: 'b',
                relation: Relation::Ne,
            },
        ),
        &up_to(3),
    )
    .unwrap();
    assert_eq!(
        d.accepted,
        vec!["a", "b", "aa", "bb", "aaa", "aab", "abb", "bbb"]
    );
}

#[test]
fn equal_count_balanced_and_palindromes() {
    let cases: Vec<(&str, Template, Vec<&str>, Vec<&str>)> = vec![
        (
            "ab",
            Template::EqualCount { a: 'a', b: 'b' },
            vec!["", "ab", "ba", "abba", "baab", "aabbba"],
            vec!["a", "b", "aab", "abb", "aaabb"],
        ),
        (
            "()",
            Template::BalancedBrackets {
                pairs: vec![['(', ')']],
            },
            vec!["", "()", "(())", "()()", "(()())"],
            vec!["(", ")", ")(", "(()", "())("],
        ),
        (
            "()[]",
            Template::BalancedBrackets {
                pairs: vec![['(', ')'], ['[', ']']],
            },
            vec!["([])", "[()]()", "[][]"],
            vec!["([)]", "(]", "[(])"],
        ),
        (
            "abc",
            Template::WcWr { c: 'c' },
            vec!["c", "aca", "abcba", "bbcbb"],
            vec!["", "ab", "abcab", "abccba", "acb"],
        ),
        (
            "ab",
            Template::EvenPalindrome,
            vec!["", "aa", "abba", "baab", "aaaa"],
            vec!["a", "ab", "aba", "abab"],
        ),
        (
            "ab",
            Template::Palindrome,
            vec!["", "a", "b", "aa", "aba", "abba", "babab"],
            vec!["ab", "abb", "aab", "abab"],
        ),
    ];
    for (alphabet, t, accept, reject) in cases {
        let s = template(alphabet, t.clone()).with_examples(&accept, &reject);
        let d = design(&s, &DesignOptions::default()).unwrap_or_else(|e| panic!("{t:?}: {e}"));
        assert_eq!(
            d.verification.failed_examples, 0,
            "{t:?}: {:?}",
            d.verification.examples
        );
        let cc = &d.verification.cross_check;
        assert!(cc.ok, "{t:?}: {cc:?}");
        assert_eq!(cc.inconclusive, 0);
        assert!(!d.accepted.is_empty(), "{t:?}");
    }
}

#[test]
fn palindromes_listed_shortest_first() {
    let d = design(&template("ab", Template::Palindrome), &up_to(3)).unwrap();
    assert_eq!(
        d.accepted,
        vec!["", "a", "b", "aa", "bb", "aaa", "aba", "bab", "bbb"]
    );
    let d = design(&template("ab", Template::EvenPalindrome), &up_to(4)).unwrap();
    assert_eq!(
        d.accepted,
        vec!["", "aa", "bb", "aaaa", "abba", "baab", "bbbb"]
    );
}

#[test]
fn count_equality_as_a_constraint_is_reported_not_regular_with_the_template() {
    for text in [
        "count(a) == count(b)",
        "count(a) == count(b) and length <= 100",
        "not (count(a) == count(b))",
    ] {
        let spec = LanguageSpec::new("ab", LanguageSource::ConstraintText(text.to_string()));
        match design(&spec, &DesignOptions::default()).unwrap_err() {
            DesignError::NotRegular(report) => {
                assert_eq!(report.constraint, "count(a) == count(b)", "{text}");
                assert_eq!(report.template.as_deref(), Some("equal_count"), "{text}");
                assert!(report.reason.contains("not regular") || report.reason.contains("finite"));
                assert!(
                    report
                        .suggestion
                        .contains(r#"{"template":{"name":"equal_count","a":"a","b":"b"}}"#),
                    "{}",
                    report.suggestion
                );
            }
            other => panic!("{text}: {other:?}"),
        }
    }
}

#[test]
fn other_count_comparisons_are_not_regular_either() {
    let spec = LanguageSpec::new(
        "ab",
        LanguageSource::ConstraintText("count(a) < count(b)".to_string()),
    );
    match design(&spec, &DesignOptions::default()).unwrap_err() {
        DesignError::NotRegular(report) => {
            assert_eq!(report.template, None);
            assert!(report.suggestion.contains("an_bm"), "{}", report.suggestion);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn template_parameters_are_validated() {
    let bad = [
        (
            template(
                "ab",
                Template::AnBn {
                    a: 'a',
                    b: 'c',
                    min: 0,
                },
            ),
            "alphabet",
        ),
        (
            template(
                "ab",
                Template::AnBn {
                    a: 'a',
                    b: 'a',
                    min: 0,
                },
            ),
            "distinct",
        ),
        (
            template(
                "ab",
                Template::AnBn {
                    a: 'a',
                    b: 'b',
                    min: 2,
                },
            ),
            "`min` must be 0 or 1",
        ),
        (
            template(
                "Zz",
                Template::BalancedBrackets {
                    pairs: vec![['Z', 'z']],
                },
            ),
            "bottom-of-stack",
        ),
        (template("abZ", Template::Palindrome), "bottom-of-stack"),
        (template("ab", Template::WcWr { c: 'c' }), "alphabet"),
    ];
    for (spec, needle) in bad {
        match design(&spec, &DesignOptions::default()) {
            Err(e) => assert!(e.to_string().contains(needle), "{e}"),
            Ok(_) => panic!("{spec:?} should fail"),
        }
    }
}

#[test]
fn a_failed_example_of_a_template_is_reported_not_raised() {
    let s = template(
        "ab",
        Template::AnBn {
            a: 'a',
            b: 'b',
            min: 0,
        },
    )
    .with_examples(&["aab"], &["ab"]);
    let d = design(&s, &DesignOptions::default()).unwrap();
    assert_eq!(d.verification.failed_examples, 2);
    assert!(d.verification.cross_check.ok);
    assert!(!d.verification.ok);
    assert!(
        d.verification.examples[0]
            .detail
            .as_ref()
            .unwrap()
            .contains("a^n b^n")
    );
}

#[test]
fn pda_design_is_deterministic_down_to_the_json() {
    let s = template("ab", Template::Palindrome).with_examples(&["aba"], &["ab"]);
    let first = serde_json::to_string(&design(&s, &DesignOptions::default()).unwrap()).unwrap();
    let second = serde_json::to_string(&design(&s, &DesignOptions::default()).unwrap()).unwrap();
    assert_eq!(first, second);
}
