//! Integration tests for conversions between models (JFLAP 7.1: FSA <-> regular grammar,
//! PDA -> CFG, CFG -> PDA (LL and LR), Turing machine -> unrestricted grammar).

use std::collections::VecDeque;

use rflap_core::automaton::{Label, Point, StateId, Transition};
use rflap_core::convert::{
    ConvertError, FaToGrammarStep, cfg_to_pda_ll, cfg_to_pda_lr, check_pda_form, fa_to_grammar,
    grammar_to_fa, pda_to_cfg, tm_to_grammar,
};
use rflap_core::fa::{Fa, FaLabel, FaStepByState};
use rflap_core::fa_convert::equivalent;
use rflap_core::grammar::{Grammar, ParseOutcome, Production, brute_parse};
use rflap_core::pda::{Acceptance, Pda, PdaLabel, PdaStepByState};
use rflap_core::sim::{Machine, fast_run};
use rflap_core::turing::{AcceptanceFilters, TmSimulator, TuringMachine};

const BUDGET: usize = 200_000;

fn fa(n: u32, finals: &[StateId], edges: &[(StateId, StateId, &str)]) -> Fa {
    let mut fa = Fa::new();
    for i in 0..n {
        fa.create_state(Point {
            x: f64::from(i) * 100.0,
            y: 0.0,
        });
    }
    fa.set_initial(Some(0));
    for &f in finals {
        fa.set_final(f, true);
    }
    for &(from, to, label) in edges {
        assert!(fa.add_transition(Transition::new(from, to, FaLabel::from(label))));
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
    for &(from, to, input, pop, push) in edges {
        assert!(pda.add_transition(Transition::new(from, to, PdaLabel::new(input, pop, push))));
    }
    pda
}

/// Every string over `alphabet` of length at most `max`, shortest first.
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

fn grammar_accepts(g: &Grammar, s: &str) -> bool {
    let r = brute_parse(g, s, BUDGET).expect("input uses grammar terminals");
    match r.outcome {
        ParseOutcome::Accepted => true,
        ParseOutcome::Rejected => false,
        ParseOutcome::Undecided => panic!("brute parser undecided on {s:?}"),
    }
}

fn fa_accepts(fa: &Fa, s: &str) -> bool {
    fast_run(&FaStepByState::new(fa), s, BUDGET).is_accepted()
}

/// Accepting run of a PDA whose stack never exceeds `max_stack` symbols (the LR
/// construction can push λ-reductions forever, so the search must be bounded).
fn pda_accepts_bounded(pda: &Pda, s: &str, max_stack: usize) -> bool {
    let machine = PdaStepByState::new(pda, Acceptance::FinalState);
    let mut seen = std::collections::HashSet::new();
    let mut queue: VecDeque<_> = machine.initial_configs(s).into();
    while let Some(c) = queue.pop_front() {
        if c.stack.chars().count() > max_stack || !seen.insert(c.clone()) {
            continue;
        }
        if machine.is_accepting(&c) {
            return true;
        }
        queue.extend(machine.step(&c));
    }
    false
}

fn ends_in_a_dfa() -> Fa {
    // Strings over {a,b} ending in a.
    fa(
        2,
        &[1],
        &[(0, 1, "a"), (0, 0, "b"), (1, 1, "a"), (1, 0, "b")],
    )
}

// ---------------------------------------------------------------- FA -> regular grammar

#[test]
fn fa_to_grammar_exact_text_for_two_state_dfa() {
    let conv = fa_to_grammar(&ends_in_a_dfa()).unwrap();
    assert_eq!(
        conv.grammar.to_text(),
        "S -> aA\nS -> bS\nA -> aA\nA -> bS\nA -> λ\n"
    );
    assert_eq!(conv.grammar.start(), "S");
    let vars: Vec<(StateId, char)> = conv
        .variables
        .iter()
        .map(|v| (v.state, v.variable))
        .collect();
    assert_eq!(vars, vec![(0, 'S'), (1, 'A')]);
}

#[test]
fn fa_to_grammar_names_variables_by_state_id_skipping_s() {
    // The initial state is q2, so it is S and the others take A, B in id order.
    let mut f = fa(3, &[0], &[(2, 0, "a"), (0, 1, "b"), (1, 2, "c")]);
    f.set_initial(Some(2));
    let conv = fa_to_grammar(&f).unwrap();
    assert_eq!(
        conv.grammar.to_text(),
        "S -> aA\nA -> bB\nB -> cS\nA -> λ\n"
    );
}

#[test]
fn fa_to_grammar_steps_follow_transitions_then_final_states() {
    let conv = fa_to_grammar(&ends_in_a_dfa()).unwrap();
    assert_eq!(conv.steps.len(), 5);
    assert!(matches!(
        &conv.steps[0],
        FaToGrammarStep::Transition { from: 0, to: 1, productions, .. } if productions.len() == 1
    ));
    assert!(matches!(
        &conv.steps[4],
        FaToGrammarStep::FinalState { state: 1, production } if production.to_string() == "A → λ"
    ));
    serde_json::to_string(&conv.steps).expect("steps serialize");
}

#[test]
fn fa_to_grammar_rejects_what_jflap_rejects() {
    let mut no_initial = ends_in_a_dfa();
    no_initial.set_initial(None);
    let err = fa_to_grammar(&no_initial).unwrap_err();
    assert_eq!(err, ConvertError::NoInitialState);
    assert_eq!(err.to_string(), "There must be an initial state!");

    let capital = fa(2, &[1], &[(0, 1, "aB")]);
    let err = fa_to_grammar(&capital).unwrap_err();
    assert_eq!(
        err,
        ConvertError::CapitalInTransition {
            from: 0,
            to: 1,
            label: "aB".to_string()
        }
    );
    assert_eq!(
        err.to_string(),
        "Capital letters are reserved for grammar variables."
    );

    let mut big = Fa::new();
    for i in 0..27 {
        big.create_state(Point {
            x: f64::from(i),
            y: 0.0,
        });
    }
    big.set_initial(Some(0));
    let err = fa_to_grammar(&big).unwrap_err();
    assert_eq!(err, ConvertError::TooManyStates { count: 27 });
    assert_eq!(
        err.to_string(),
        "There may be at most 26 states for conversion."
    );
}

#[test]
fn fa_to_grammar_expands_ranges() {
    let f = fa(2, &[1], &[(0, 1, "[a-c]")]);
    let conv = fa_to_grammar(&f).unwrap();
    assert_eq!(
        conv.grammar.to_text(),
        "S -> aA\nS -> bA\nS -> cA\nA -> λ\n"
    );
}

#[test]
fn fa_grammar_fa_is_language_equivalent() {
    // 1. ends in a (DFA).
    // 2. a*b with a lambda transition in the middle.
    // 3. NFA for (ab)*|a, with a multi-character label and a lambda loop.
    let automata = vec![
        ends_in_a_dfa(),
        fa(3, &[2], &[(0, 0, "a"), (0, 1, ""), (1, 2, "b")]),
        fa(
            4,
            &[0, 3],
            &[
                (0, 1, "ab"),
                (1, 0, ""),
                (0, 3, "a"),
                (1, 2, "b"),
                (2, 1, ""),
            ],
        ),
    ];
    for original in &automata {
        let grammar = fa_to_grammar(original).unwrap().grammar;
        let back = grammar_to_fa(&grammar).unwrap().fa;
        let eq = equivalent(original, &back);
        assert!(
            eq.equivalent,
            "not equivalent, counterexample {:?}\n{}",
            eq.counterexample,
            grammar.to_text()
        );
    }
}

// ---------------------------------------------------------------- regular grammar -> FA

#[test]
fn right_linear_grammar_to_fa_accepts_exactly_the_grammar_language() {
    let g = Grammar::parse("S -> aS | bA\nA -> b | λ").unwrap();
    let conv = grammar_to_fa(&g).unwrap();
    for s in strings_up_to(&['a', 'b'], 5) {
        assert_eq!(
            fa_accepts(&conv.fa, &s),
            grammar_accepts(&g, &s),
            "disagree on {s:?}"
        );
    }
}

#[test]
fn grammar_to_fa_has_one_state_per_variable_plus_final() {
    let g = Grammar::parse("S -> aS | bA\nA -> b | λ").unwrap();
    let conv = grammar_to_fa(&g).unwrap();
    assert_eq!(conv.fa.states().count(), 3);
    let by_var: Vec<(&str, StateId)> = conv
        .variables
        .iter()
        .map(|v| (v.variable.as_str(), v.state))
        .collect();
    assert_eq!(by_var.len(), 2);
    let s_state = by_var.iter().find(|(v, _)| *v == "S").unwrap().1;
    assert_eq!(conv.fa.initial(), Some(s_state));
    assert_eq!(conv.fa.state(s_state).unwrap().label.as_deref(), Some("S"));
    assert_eq!(
        conv.fa.finals().iter().copied().collect::<Vec<_>>(),
        vec![conv.final_state]
    );
    // One transition per production, in production order: aS, bA, b, λ.
    let labels: Vec<&str> = conv
        .fa
        .transitions()
        .iter()
        .map(|t| t.label.0.as_str())
        .collect();
    assert_eq!(labels, vec!["a", "b", "b", ""]);
    assert_eq!(conv.steps.len(), 4);
    serde_json::to_string(&conv.steps).expect("steps serialize");
}

#[test]
fn grammar_to_fa_rejects_what_jflap_rejects() {
    let err = grammar_to_fa(&Grammar::empty()).unwrap_err();
    assert_eq!(err, ConvertError::EmptyGrammar);
    assert_eq!(err.to_string(), "The grammar should exist.");

    let left = Grammar::parse("S -> Ab\nA -> a").unwrap();
    let err = grammar_to_fa(&left).unwrap_err();
    assert!(
        matches!(&err, ConvertError::NotRightLinear { production } if production.to_string() == "S → Ab")
    );
    assert_eq!(err.to_string(), "The production is not right linear.");
}

// ---------------------------------------------------------------- CFG -> PDA

fn anbn_grammar() -> Grammar {
    Grammar::parse("S -> aSb | λ").unwrap()
}

fn desc(p: &Pda) -> Vec<String> {
    p.transitions()
        .iter()
        .map(|t| format!("{}>{} {}", t.from, t.to, t.label.description()))
        .collect()
}

#[test]
fn cfg_to_pda_ll_has_jflap_shape() {
    let conv = cfg_to_pda_ll(&anbn_grammar()).unwrap();
    let p = &conv.pda;
    assert_eq!(p.states().count(), 3);
    assert_eq!(p.initial(), Some(0));
    assert_eq!(p.finals().iter().copied().collect::<Vec<_>>(), vec![2]);
    assert_eq!(
        desc(p),
        vec![
            "0>1 λ , Z ; SZ",
            "1>2 λ , Z ; λ",
            "1>1 a , a ; λ",
            "1>1 b , b ; λ",
            "1>1 λ , S ; aSb",
            "1>1 λ , S ; λ",
        ]
    );
    serde_json::to_string(&conv.steps).expect("steps serialize");
}

#[test]
fn cfg_to_pda_lr_has_jflap_shape() {
    let conv = cfg_to_pda_lr(&anbn_grammar()).unwrap();
    let p = &conv.pda;
    assert_eq!(p.states().count(), 3);
    assert_eq!(p.initial(), Some(0));
    assert_eq!(p.finals().iter().copied().collect::<Vec<_>>(), vec![2]);
    assert_eq!(
        desc(p),
        vec![
            "0>1 λ , S ; λ",
            "1>2 λ , Z ; λ",
            "0>0 a , λ ; a",
            "0>0 b , λ ; b",
            "0>0 λ , bSa ; S",
            "0>0 λ , λ ; S",
        ]
    );
}

#[test]
fn cfg_to_pda_ll_agrees_with_the_grammar_up_to_length_8() {
    let g = anbn_grammar();
    let pda = cfg_to_pda_ll(&g).unwrap().pda;
    let machine = PdaStepByState::new(&pda, Acceptance::FinalState);
    for s in strings_up_to(&['a', 'b'], 8) {
        let run = fast_run(&machine, &s, BUDGET);
        assert!(
            matches!(run, rflap_core::sim::RunOutcome::Accepted { .. })
                || matches!(run, rflap_core::sim::RunOutcome::Rejected),
            "undecided on {s:?}"
        );
        assert_eq!(
            run.is_accepted(),
            grammar_accepts(&g, &s),
            "disagree on {s:?}"
        );
    }
}

#[test]
fn cfg_to_pda_lr_agrees_with_the_grammar_up_to_length_8() {
    let g = anbn_grammar();
    let pda = cfg_to_pda_lr(&g).unwrap().pda;
    for s in strings_up_to(&['a', 'b'], 8) {
        let bound = s.len() / 2 + 3;
        assert_eq!(
            pda_accepts_bounded(&pda, &s, bound),
            grammar_accepts(&g, &s),
            "disagree on {s:?}"
        );
    }
}

#[test]
fn cfg_to_pda_works_on_a_grammar_with_several_variables() {
    let g = Grammar::parse("S -> AB\nA -> aA | a\nB -> bB | b").unwrap();
    let ll = cfg_to_pda_ll(&g).unwrap().pda;
    let lr = cfg_to_pda_lr(&g).unwrap().pda;
    for s in strings_up_to(&['a', 'b'], 6) {
        let expected = grammar_accepts(&g, &s);
        assert_eq!(
            fast_run(
                &PdaStepByState::new(&ll, Acceptance::FinalState),
                &s,
                BUDGET
            )
            .is_accepted(),
            expected,
            "LL disagrees on {s:?}"
        );
        assert_eq!(
            pda_accepts_bounded(&lr, &s, s.len() + 3),
            expected,
            "LR disagrees on {s:?}"
        );
    }
}

#[test]
fn cfg_to_pda_rejects_empty_and_non_context_free_grammars() {
    for convert in [cfg_to_pda_ll, cfg_to_pda_lr] {
        let err = convert(&Grammar::empty()).unwrap_err();
        assert_eq!(err, ConvertError::EmptyGrammar);
        let g = Grammar::parse("S -> aBc\naB -> d").unwrap();
        let err = convert(&g).unwrap_err();
        assert!(
            matches!(&err, ConvertError::NotContextFree { production } if production.lhs == "aB")
        );
    }
}

// ---------------------------------------------------------------- PDA -> CFG

/// a^n b^n (n >= 1) in JFLAP's conversion form: one final state, every transition pops one
/// symbol and pushes zero or two, transitions to the final state pop Z.
fn anbn_jflap_form() -> Pda {
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
    )
}

#[test]
fn pda_to_cfg_agrees_with_the_pda_up_to_length_6() {
    let p = anbn_jflap_form();
    let conv = pda_to_cfg(&p).unwrap();
    assert_eq!(conv.grammar.start(), "S");
    let machine = PdaStepByState::new(&p, Acceptance::FinalState);
    for s in strings_up_to(&['a', 'b'], 6) {
        assert_eq!(
            grammar_accepts(&conv.grammar, &s),
            fast_run(&machine, &s, BUDGET).is_accepted(),
            "disagree on {s:?}\n{}",
            conv.grammar.to_text()
        );
    }
    serde_json::to_string(&conv.steps).expect("steps serialize");
}

#[test]
fn pda_to_cfg_trims_to_reachable_generating_variables() {
    let conv = pda_to_cfg(&anbn_jflap_form()).unwrap();
    // Every variable on a right side has a production of its own and nothing is left over.
    let lhs: std::collections::BTreeSet<char> = conv
        .grammar
        .productions()
        .iter()
        .map(|p| p.lhs.chars().next().unwrap())
        .collect();
    assert_eq!(lhs, conv.grammar.variables());
    assert!(conv.grammar.variables().len() <= 26);
}

#[test]
fn pda_to_cfg_reports_jflap_form_errors_in_jflap_order() {
    // No initial state.
    let mut p = anbn_jflap_form();
    p.set_initial(None);
    let err = check_pda_form(&p).unwrap_err();
    assert_eq!(err, ConvertError::NoInitialState);
    assert_eq!(err.to_string(), "There must be an initial state!");

    // Two final states.
    let mut p = anbn_jflap_form();
    p.set_final(1, true);
    let err = pda_to_cfg(&p).unwrap_err();
    assert_eq!(err, ConvertError::NotExactlyOneFinalState { count: 2 });
    assert_eq!(err.to_string(), "There must be exactly one final state!");

    // A transition to the final state that does not pop exactly Z.
    let p = pda(2, &[1], &[(0, 0, "a", "Z", "AZ"), (0, 1, "b", "A", "")]);
    let err = pda_to_cfg(&p).unwrap_err();
    assert!(
        matches!(&err, ConvertError::FinalTransitionsMustPopZ { transitions } if transitions.len() == 1 && transitions[0].label == PdaLabel::new("b", "A", ""))
    );
    assert_eq!(err.to_string(), "Transitions to final must pop only 'Z'.");

    // A transition that pushes one symbol.
    let p = pda(2, &[1], &[(0, 0, "a", "Z", "A"), (0, 1, "", "Z", "")]);
    let err = pda_to_cfg(&p).unwrap_err();
    assert!(
        matches!(&err, ConvertError::TransitionsMustPopOnePushZeroOrTwo { transitions } if transitions.len() == 1 && transitions[0].label == PdaLabel::new("a", "Z", "A"))
    );
    assert_eq!(err.to_string(), "Transitions must pop 1 and push 0 or 2.");

    // A transition that pops nothing.
    let p = pda(2, &[1], &[(0, 0, "a", "", "AZ"), (0, 1, "", "Z", "")]);
    assert!(matches!(
        pda_to_cfg(&p).unwrap_err(),
        ConvertError::TransitionsMustPopOnePushZeroOrTwo { .. }
    ));
}

// ---------------------------------------------------------------- Turing machine -> grammar

/// q0 -a/b,R-> q1 -a/a,L-> q2 (final): accepts a^n for n >= 2.
fn tiny_tm() -> TuringMachine {
    let mut tm = TuringMachine::new(1);
    for i in 0..3 {
        tm.create_state(Point {
            x: f64::from(i) * 100.0,
            y: 0.0,
        });
    }
    tm.set_initial(Some(0));
    tm.set_final(2, true);
    tm.add_transition(
        0,
        1,
        rflap_core::turing::TmLabel::single("a", "b", "R").unwrap(),
    )
    .unwrap();
    tm.add_transition(
        1,
        2,
        rflap_core::turing::TmLabel::single("a", "a", "L").unwrap(),
    )
    .unwrap();
    tm
}

#[test]
fn tm_to_grammar_starts_with_jflap_template() {
    let conv = tm_to_grammar(&tiny_tm()).unwrap();
    let text = conv.grammar.to_text();
    let head: Vec<&str> = text.lines().take(6).collect();
    assert_eq!(
        head,
        vec![
            "S -> V(==)S",
            "S -> SV(==)",
            "S -> T",
            "T -> TV(aa)",
            "T -> V(a0a)",
            "= -> λ",
        ]
    );
    assert_eq!(conv.grammar.start(), "S");
    assert!(conv.grammar.is_turing_converted());
    serde_json::to_string(&conv.steps).expect("steps serialize");
}

#[test]
fn tm_to_grammar_derives_exactly_the_accepted_inputs() {
    let tm = tiny_tm();
    let conv = tm_to_grammar(&tm).unwrap();
    // The blank-padding productions (S -> V(==)S, S -> SV(==)) only add cells nobody reads for
    // these inputs, and they make the breadth-first search explode; derive without them.
    let mut productions = conv.grammar.productions().to_vec();
    productions.drain(0..3);
    productions.insert(0, Production::new("S", "T"));
    let g = Grammar::from_productions(productions).unwrap();

    let machine = TmSimulator::new(&tm, AcceptanceFilters::by_final_state());
    for (input, accepted) in [("aa", true), ("aaa", true), ("a", false)] {
        assert_eq!(
            fast_run(&machine, input, BUDGET).is_accepted(),
            accepted,
            "machine on {input:?}"
        );
        let outcome = brute_parse(&g, input, 3_000).unwrap().outcome;
        if accepted {
            assert_eq!(outcome, ParseOutcome::Accepted, "grammar on {input:?}");
        } else {
            assert_ne!(outcome, ParseOutcome::Accepted, "grammar on {input:?}");
        }
    }
}

#[test]
fn tm_to_grammar_rejects_unsupported_machines() {
    let err = tm_to_grammar(&TuringMachine::new(2)).unwrap_err();
    assert_eq!(err, ConvertError::NotSingleTape { tapes: 2 });

    let mut no_initial = tiny_tm();
    no_initial.set_initial(None);
    assert_eq!(
        tm_to_grammar(&no_initial).unwrap_err(),
        ConvertError::NoInitialState
    );
}

#[test]
fn tm_to_grammar_warns_about_stay_moves_jflap_ignores() {
    let mut tm = tiny_tm();
    tm.add_transition(
        0,
        2,
        rflap_core::turing::TmLabel::single("a", "a", "S").unwrap(),
    )
    .unwrap();
    let conv = tm_to_grammar(&tm).unwrap();
    assert_eq!(
        conv.warnings.iter().filter(|w| w.contains("stays")).count(),
        1
    );
}
