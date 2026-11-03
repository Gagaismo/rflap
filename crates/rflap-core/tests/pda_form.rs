//! `pda_to_jflap_form` (Rflap addition): any PDA that accepts by final state becomes one in the
//! form `pda_to_cfg` requires, with the same language.

use rflap_core::automaton::{Point, Transition};
use rflap_core::convert::{
    ConvertError, PdaToJflapFormStep, check_pda_form, pda_to_cfg, pda_to_jflap_form,
};
use rflap_core::design::{
    DesignAutomaton, DesignOptions, LanguageSource, LanguageSpec, Relation, Template, design,
};
use rflap_core::grammar::{Grammar, ParseOutcome, brute_parse};
use rflap_core::pda::{Acceptance, Pda, PdaLabel, PdaStepByState};
use rflap_core::sim::{RunOutcome, fast_run};

const BUDGET: usize = 500_000;

fn designed(alphabet: &str, template: Template) -> Pda {
    let spec = LanguageSpec::new(alphabet, LanguageSource::Template(template));
    match design(&spec, &DesignOptions::default()).unwrap().automaton {
        DesignAutomaton::Pda(p) => p,
        DesignAutomaton::Fa(_) => panic!("expected a pushdown automaton"),
    }
}

fn strings_up_to(alphabet: &[char], max: usize) -> Vec<String> {
    let mut all = vec![String::new()];
    let mut frontier = vec![String::new()];
    for _ in 0..max {
        let mut next = Vec::new();
        for s in &frontier {
            for c in alphabet {
                next.push(format!("{s}{c}"));
            }
        }
        all.extend(next.iter().cloned());
        frontier = next;
    }
    all
}

fn pda_accepts(pda: &Pda, s: &str) -> bool {
    fast_run(&PdaStepByState::new(pda, Acceptance::FinalState), s, BUDGET).is_accepted()
}

fn grammar_accepts(g: &Grammar, s: &str) -> bool {
    match brute_parse(g, s, BUDGET)
        .expect("terminals of the grammar")
        .outcome
    {
        ParseOutcome::Accepted => true,
        ParseOutcome::Rejected => false,
        ParseOutcome::Undecided => panic!("undecided on {s:?}"),
    }
}

fn pda(n: u32, finals: &[u32], edges: &[(u32, u32, &str, &str, &str)]) -> Pda {
    let mut pda = Pda::new();
    for i in 0..n {
        pda.create_state(Point {
            x: f64::from(i) * 100.0,
            y: 0.0,
        });
    }
    pda.set_initial(Some(0));
    for &f in finals {
        pda.set_final(f, true);
    }
    for &(from, to, input, pop, push) in edges {
        assert!(pda.add_transition(Transition::new(from, to, PdaLabel::new(input, pop, push))));
    }
    pda
}

/// The converted machine has the same language as the original (up to `max` symbols).
fn assert_same_language(original: &Pda, converted: &Pda, alphabet: &[char], max: usize) {
    for s in strings_up_to(alphabet, max) {
        assert_eq!(
            pda_accepts(original, &s),
            pda_accepts_empty_stack_at_final(converted, &s),
            "disagree on {s:?}"
        );
    }
}

/// The form is read as "reach the single final state with an empty stack".
fn pda_accepts_empty_stack_at_final(p: &Pda, s: &str) -> bool {
    let machine = PdaStepByState::new(p, Acceptance::EmptyStack);
    match fast_run(&machine, s, BUDGET) {
        RunOutcome::Accepted { trace } => p.is_final(trace.last().expect("trace").state),
        _ => false,
    }
}

#[test]
fn a_designed_an_bn_becomes_a_jflap_form_pda_and_converts_to_a_cfg() {
    let original = designed(
        "ab",
        Template::AnBn {
            a: 'a',
            b: 'b',
            min: 0,
        },
    );
    assert!(check_pda_form(&original).is_err());
    let form = pda_to_jflap_form(&original).unwrap();
    check_pda_form(&form.pda).expect("the result is in JFLAP form");
    assert_eq!(form.pda.finals().len(), 1);
    assert!(!form.steps.is_empty());
    serde_json::to_string(&form.steps).expect("steps serialize");

    let cfg = pda_to_cfg(&form.pda).unwrap();
    for s in strings_up_to(&['a', 'b'], 6) {
        assert_eq!(
            grammar_accepts(&cfg.grammar, &s),
            pda_accepts(&original, &s),
            "disagree on {s:?}\n{}",
            cfg.grammar.to_text()
        );
    }
}

#[test]
fn the_language_is_kept_for_several_templates() {
    let cases: Vec<(&str, Template, Vec<char>)> = vec![
        (
            "ab",
            Template::AnBm {
                a: 'a',
                b: 'b',
                relation: Relation::Le,
            },
            vec!['a', 'b'],
        ),
        (
            "ab",
            Template::EqualCount { a: 'a', b: 'b' },
            vec!['a', 'b'],
        ),
        (
            "()",
            Template::BalancedBrackets {
                pairs: vec![['(', ')']],
            },
            vec!['(', ')'],
        ),
        ("ab", Template::EvenPalindrome, vec!['a', 'b']),
    ];
    for (alphabet, template, symbols) in cases {
        let original = designed(alphabet, template);
        let form = pda_to_jflap_form(&original).unwrap();
        check_pda_form(&form.pda).expect("the result is in JFLAP form");
        assert_same_language(&original, &form.pda, &symbols, 6);
    }
}

#[test]
fn multi_symbol_pops_and_pushes_are_split() {
    // Pops two symbols, pushes three, reads a λ-pop: all outside the form.
    let original = pda(
        3,
        &[2],
        &[
            (0, 0, "a", "", "XY"),
            (0, 0, "a", "", "X"),
            (0, 1, "b", "XY", "ABC"),
            (1, 1, "b", "A", ""),
            (1, 1, "", "B", ""),
            (1, 1, "", "C", ""),
            (1, 2, "", "Z", "Z"),
        ],
    );
    let form = pda_to_jflap_form(&original).unwrap();
    check_pda_form(&form.pda).unwrap();
    assert_same_language(&original, &form.pda, &['a', 'b'], 6);
}

#[test]
fn a_pda_already_in_form_is_returned_unchanged() {
    let p = pda(
        3,
        &[2],
        &[
            (0, 0, "a", "Z", "AZ"),
            (0, 0, "a", "A", "AA"),
            (0, 1, "b", "A", ""),
            (1, 1, "b", "A", ""),
            (1, 2, "", "Z", ""),
        ],
    );
    let form = pda_to_jflap_form(&p).unwrap();
    assert_eq!(form.pda, p);
    assert_eq!(form.steps, vec![PdaToJflapFormStep::AlreadyInForm]);
}

#[test]
fn a_pda_without_an_initial_state_is_refused() {
    let mut p = Pda::new();
    p.create_state(Point::default());
    assert_eq!(
        pda_to_jflap_form(&p).unwrap_err(),
        ConvertError::NoInitialState
    );
}
