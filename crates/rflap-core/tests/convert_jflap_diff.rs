//! Differential check against `fixtures/t15a/expected.txt`, produced by the real JFLAP 7.1 jar
//! (`fixtures/t15a/Harness.java`). Each section rebuilds the harness input and compares.
//!
//! JFLAP keeps transitions and symbol sets in hash sets, so wherever the order of its output
//! is an accident of hashing the comparison is between sorted lists; the variable letters of
//! the PDA conversion depend on the order transitions are visited in, which the harness fixes
//! to insertion order.

use std::collections::BTreeSet;

use rflap_core::automaton::{Point, StateId, Transition};
use rflap_core::convert::{
    TmToGrammarStep, cfg_to_pda_ll, cfg_to_pda_lr, fa_to_grammar, grammar_to_fa, pda_to_cfg,
    tm_to_grammar,
};
use rflap_core::fa::{Fa, FaLabel};
use rflap_core::grammar::{Grammar, Production};
use rflap_core::pda::{Pda, PdaLabel};
use rflap_core::turing::{TmLabel, TuringMachine};

fn expected(prefix: &str) -> Vec<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/t15a/expected.txt"
    );
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter_map(|l| l.strip_prefix(prefix))
        .map(|r| r.trim().to_string())
        .collect()
}

fn one(prefix: &str) -> String {
    let v = expected(prefix);
    assert_eq!(v.len(), 1, "{prefix}");
    v.into_iter().next().unwrap()
}

/// `a->b ; c->d ;` as a sorted set of `lhs->rhs`.
fn set_of_line(line: &str) -> BTreeSet<String> {
    line.trim_end_matches(';')
        .trim()
        .split(" ; ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn set_of(ps: &[Production]) -> BTreeSet<String> {
    ps.iter().map(|p| format!("{}->{}", p.lhs, p.rhs)).collect()
}

fn fa(n: u32, initial: StateId, finals: &[StateId], edges: &[(StateId, StateId, &str)]) -> Fa {
    let mut fa = Fa::new();
    for i in 0..n {
        fa.create_state(Point {
            x: f64::from(i) * 100.0,
            y: 0.0,
        });
    }
    fa.set_initial(Some(initial));
    for &f in finals {
        fa.set_final(f, true);
    }
    for &(a, b, l) in edges {
        fa.add_transition(Transition::new(a, b, FaLabel::from(l)));
    }
    fa
}

fn pda(n: u32, finals: &[StateId], edges: &[(StateId, StateId, &str, &str, &str)]) -> Pda {
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
    for &(a, b, i, p, q) in edges {
        pda.add_transition(Transition::new(a, b, PdaLabel::new(i, p, q)));
    }
    pda
}

#[test]
fn fa_to_grammar_matches_jflap() {
    let cases = [
        (
            "dfa",
            fa(
                2,
                0,
                &[1],
                &[(0, 1, "a"), (0, 0, "b"), (1, 1, "a"), (1, 0, "b")],
            ),
        ),
        (
            "ids",
            fa(3, 2, &[0], &[(2, 0, "a"), (0, 1, "b"), (1, 2, "c")]),
        ),
        (
            "lambda",
            fa(3, 0, &[2], &[(0, 0, "a"), (0, 1, ""), (1, 2, "ab")]),
        ),
    ];
    for (tag, automaton) in cases {
        let conv = fa_to_grammar(&automaton).unwrap();
        assert_eq!(
            set_of(conv.grammar.productions()),
            set_of_line(&one(&format!("FA2G {tag} "))),
            "{tag}"
        );
    }
}

#[test]
fn grammar_to_fa_matches_jflap() {
    let g = Grammar::from_pairs(&[("S", "aS"), ("S", "bA"), ("A", "b"), ("A", "")]).unwrap();
    let conv = grammar_to_fa(&g).unwrap();
    let states: Vec<String> = conv
        .fa
        .states()
        .map(|s| {
            format!(
                "{}={}{}{}",
                s.id,
                s.label.as_deref().unwrap_or("null"),
                if conv.fa.is_final(s.id) {
                    "(final)"
                } else {
                    ""
                },
                if conv.fa.initial() == Some(s.id) {
                    "(initial)"
                } else {
                    ""
                }
            )
        })
        .collect();
    assert_eq!(states.join(" "), one("RL2FA states "));
    let transitions: Vec<String> = conv
        .steps
        .iter()
        .map(|t| format!("{}>{}[{}]", t.from, t.to, t.label))
        .collect();
    assert_eq!(transitions.join(" ; ") + " ;", one("RL2FA transitions "));
}

fn pda_transitions(p: &Pda) -> BTreeSet<String> {
    p.transitions()
        .iter()
        .map(|t| {
            format!(
                "{}>{} [{},{};{}]",
                t.from, t.to, t.label.input, t.label.pop, t.label.push
            )
        })
        .collect()
}

#[test]
fn cfg_to_pda_matches_jflap() {
    let g = Grammar::from_pairs(&[("S", "aSb"), ("S", "")]).unwrap();
    assert_eq!(
        pda_transitions(&cfg_to_pda_ll(&g).unwrap().pda),
        set_of_line(&one("LL "))
    );
    assert_eq!(
        pda_transitions(&cfg_to_pda_lr(&g).unwrap().pda),
        set_of_line(&one("LR "))
    );
}

#[test]
fn pda_to_cfg_matches_jflap_names_included() {
    let cases = [
        (
            "PDA1",
            pda(
                3,
                &[2],
                &[
                    (0, 0, "a", "Z", "AZ"),
                    (0, 0, "a", "A", "AA"),
                    (0, 1, "b", "A", ""),
                    (1, 1, "b", "A", ""),
                    (1, 2, "", "Z", ""),
                ],
            ),
        ),
        (
            "PDA2",
            pda(
                2,
                &[1],
                &[
                    (0, 0, "a", "Z", "AZ"),
                    (0, 0, "a", "A", "AA"),
                    (0, 0, "b", "A", ""),
                    (0, 1, "", "Z", ""),
                ],
            ),
        ),
        (
            "PDA3",
            pda(
                4,
                &[3],
                &[
                    (0, 1, "a", "Z", "AZ"),
                    (1, 1, "a", "A", "AA"),
                    (1, 2, "b", "A", ""),
                    (2, 2, "b", "A", ""),
                    (2, 3, "", "Z", ""),
                    (0, 3, "c", "Z", ""),
                ],
            ),
        ),
    ];
    for (tag, automaton) in cases {
        let conv = pda_to_cfg(&automaton).unwrap();
        assert_eq!(
            set_of(conv.grammar.productions()),
            set_of_line(&one(&format!("{tag} out "))),
            "{tag}"
        );
        let raw: usize = conv
            .steps
            .iter()
            .map(|s| match s {
                rflap_core::convert::PdaToCfgStep::Transition { productions, .. } => {
                    productions.len()
                }
                _ => 0,
            })
            .sum();
        assert_eq!(
            format!("{raw} kept {}", conv.grammar.productions().len()),
            one(&format!("{tag} raw ")),
            "{tag}"
        );
    }
}

fn tm(
    states: u32,
    finals: &[StateId],
    edges: &[(StateId, StateId, &str, &str, &str)],
) -> TuringMachine {
    let mut tm = TuringMachine::new(1);
    for i in 0..states {
        tm.create_state(Point {
            x: f64::from(i) * 100.0,
            y: 0.0,
        });
    }
    tm.set_initial(Some(0));
    for &f in finals {
        tm.set_final(f, true);
    }
    for &(a, b, r, w, d) in edges {
        tm.add_transition(a, b, TmLabel::single(r, w, d).unwrap())
            .unwrap();
    }
    tm
}

#[test]
fn tm_to_grammar_matches_jflap() {
    let cases = [
        (
            "TM1",
            tm(3, &[2], &[(0, 1, "a", "b", "R"), (1, 2, "a", "a", "L")]),
        ),
        (
            "TM2",
            tm(2, &[1], &[(0, 0, "a", "a", "R"), (0, 1, "b", "b", "R")]),
        ),
        (
            "TM3",
            tm(3, &[2], &[(0, 1, "a", "", "R"), (1, 2, "", "a", "L")]),
        ),
    ];
    for (tag, machine) in cases {
        let conv = tm_to_grammar(&machine).unwrap();
        let TmToGrammarStep::Initial { productions, .. } = &conv.steps[0] else {
            panic!("first step is the initial state");
        };
        assert_eq!(
            set_of(productions),
            set_of_line(&one(&format!("{tag} init "))),
            "{tag} init"
        );
        // The initial productions come out of JFLAP in the same order.
        let init_line = one(&format!("{tag} init "));
        let ordered: Vec<String> = productions
            .iter()
            .map(|p| format!("{}->{}", p.lhs, p.rhs))
            .collect();
        assert_eq!(ordered.join(" ; ") + " ;", init_line, "{tag} init order");

        for step in &conv.steps[1..] {
            let TmToGrammarStep::Transition {
                from,
                to,
                productions,
                ..
            } = step
            else {
                panic!("transition step");
            };
            let line = expected(&format!("{tag} trans {from}>{to} "));
            assert_eq!(line.len(), 1, "{tag} {from}>{to}");
            assert_eq!(
                set_of(productions),
                set_of_line(&line[0]),
                "{tag} {from}>{to}"
            );
        }
    }
}
