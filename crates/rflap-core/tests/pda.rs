//! Integration tests for the PDA model and simulation (JFLAP 7.1 `automata.pda`).

use rflap_core::analysis::{lambda_transitions, nondeterministic_states};
use rflap_core::automaton::{Label, Point, StateId, Transition};
use rflap_core::pda::{
    Acceptance, INITIAL_STACK, Pda, PdaConfig, PdaLabel, PdaStepByState, PdaWithClosure,
    lambda_closure,
};
use rflap_core::sim::{Machine, RunOutcome, StepRun, fast_run, multiple_run};

const BUDGET: usize = 10_000;

type Edge<'a> = (StateId, StateId, &'a str, &'a str, &'a str);

fn build(n: u32, finals: &[StateId], edges: &[Edge]) -> Pda {
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

fn cfg(state: StateId, remaining: &str, stack: &str) -> PdaConfig {
    PdaConfig {
        state,
        remaining: remaining.to_string(),
        stack: stack.to_string(),
    }
}

fn accepts<M: Machine>(machine: &M, input: &str) -> bool {
    fast_run(machine, input, BUDGET).is_accepted()
}

/// a^n b^n, accepting by final state.
fn anbn_final() -> Pda {
    build(
        3,
        &[2],
        &[
            (0, 0, "a", "", "A"),
            (0, 1, "", "", ""),
            (1, 1, "b", "A", ""),
            (1, 2, "", "Z", "Z"),
        ],
    )
}

/// a^n b^n, accepting by empty stack.
fn anbn_empty() -> Pda {
    build(
        2,
        &[],
        &[
            (0, 0, "a", "", "A"),
            (0, 1, "", "", ""),
            (1, 1, "b", "A", ""),
            (1, 1, "", "Z", ""),
        ],
    )
}

/// Even palindromes over {a, b}, accepting by final state.
fn even_palindromes() -> Pda {
    build(
        3,
        &[2],
        &[
            (0, 0, "a", "", "a"),
            (0, 0, "b", "", "b"),
            (0, 1, "", "", ""),
            (1, 1, "a", "a", ""),
            (1, 1, "b", "b", ""),
            (1, 2, "", "Z", "Z"),
        ],
    )
}

#[test]
fn initial_stack_symbol_is_z() {
    assert_eq!(INITIAL_STACK, "Z");
    let pda = anbn_final();
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    assert_eq!(m.initial_configs("ab"), vec![cfg(0, "ab", "Z")]);
}

#[test]
fn no_initial_state_has_no_configs() {
    let mut pda = anbn_final();
    pda.set_initial(None);
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    assert!(m.initial_configs("a").is_empty());
    assert_eq!(fast_run(&m, "a", BUDGET), RunOutcome::Rejected);
}

#[test]
fn label_description_follows_jflap() {
    assert_eq!(PdaLabel::new("a", "Z", "aZ").description(), "a , Z ; aZ");
    assert_eq!(PdaLabel::new("", "", "").description(), "λ , λ ; λ");
    assert_eq!(PdaLabel::new("b", "", "XY").description(), "b , λ ; XY");
}

#[test]
fn label_lambda_means_all_three_empty() {
    assert!(PdaLabel::lambda().is_lambda());
    assert!(PdaLabel::new("", "", "").is_lambda());
    assert!(!PdaLabel::new("", "Z", "Z").is_lambda());
    assert!(!PdaLabel::new("", "", "A").is_lambda());
    assert!(!PdaLabel::new("a", "", "").is_lambda());
}

#[test]
fn label_conflicts_need_prefix_related_input_and_pop() {
    let l = |i, p, u| PdaLabel::new(i, p, u);
    // same read and pop, different push
    assert!(l("a", "Z", "AZ").conflicts(&l("a", "Z", "Z")));
    // λ is a prefix of everything
    assert!(l("", "Z", "Z").conflicts(&l("b", "Z", "")));
    assert!(l("a", "", "A").conflicts(&l("a", "Z", "")));
    // prefixes of each other in both fields
    assert!(l("ab", "XY", "").conflicts(&l("a", "X", "")));
    // disjoint input or disjoint pop
    assert!(!l("a", "Z", "").conflicts(&l("b", "Z", "")));
    assert!(!l("a", "A", "").conflicts(&l("a", "B", "")));
    assert!(!l("ab", "Z", "").conflicts(&l("ac", "Z", "")));
}

#[test]
fn anbn_final_state_mode() {
    let pda = anbn_final();
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    for yes in ["", "ab", "aabb", "aaabbb"] {
        assert!(accepts(&m, yes), "{yes:?} should be accepted");
    }
    for no in ["aab", "ba", "abb", "abab", "a", "b"] {
        assert_eq!(fast_run(&m, no, BUDGET), RunOutcome::Rejected, "{no:?}");
    }
}

#[test]
fn anbn_empty_stack_mode() {
    let pda = anbn_empty();
    let m = PdaStepByState::new(&pda, Acceptance::EmptyStack);
    for yes in ["", "ab", "aabb", "aaabbb"] {
        assert!(accepts(&m, yes), "{yes:?} should be accepted");
    }
    for no in ["aab", "ba", "abb", "abab", "a"] {
        assert_eq!(fast_run(&m, no, BUDGET), RunOutcome::Rejected, "{no:?}");
    }
}

#[test]
fn acceptance_mode_changes_the_verdict() {
    // anbn_empty has no final state, anbn_final never empties the stack
    let empty = anbn_empty();
    assert!(!accepts(
        &PdaStepByState::new(&empty, Acceptance::FinalState),
        "ab"
    ));
    let fin = anbn_final();
    assert!(!accepts(
        &PdaStepByState::new(&fin, Acceptance::EmptyStack),
        "ab"
    ));
}

#[test]
fn final_state_does_not_require_empty_stack() {
    let pda = anbn_final();
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    let trace = match fast_run(&m, "", BUDGET) {
        RunOutcome::Accepted { trace } => trace,
        other => panic!("expected accept, got {other:?}"),
    };
    assert_eq!(trace.last(), Some(&cfg(2, "", "Z")));
}

#[test]
fn even_palindromes_nondeterministic() {
    let pda = even_palindromes();
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    for yes in ["", "aa", "abba", "baab", "aabbaa", "abbbba"] {
        assert!(accepts(&m, yes), "{yes:?} should be accepted");
    }
    for no in ["abab", "a", "ab", "aba", "abb"] {
        assert_eq!(fast_run(&m, no, BUDGET), RunOutcome::Rejected, "{no:?}");
    }
}

#[test]
fn step_run_stack_contents_after_k_steps() {
    let pda = anbn_final();
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    let mut run = StepRun::new(&m, "aab");
    let frontier = |run: &StepRun<PdaStepByState>| -> Vec<PdaConfig> {
        run.frontier()
            .iter()
            .map(|&n| run.config(n).clone())
            .collect()
    };
    assert_eq!(frontier(&run), vec![cfg(0, "aab", "Z")]);

    assert!(run.step());
    assert_eq!(frontier(&run), vec![cfg(0, "ab", "AZ"), cfg(1, "aab", "Z")]);

    assert!(run.step());
    assert_eq!(
        frontier(&run),
        vec![cfg(0, "b", "AAZ"), cfg(1, "ab", "AZ"), cfg(2, "aab", "Z")]
    );

    assert!(run.step());
    assert_eq!(frontier(&run), vec![cfg(1, "b", "AAZ")]);
    assert_eq!(run.rejected().len(), 2);

    assert!(run.step());
    assert_eq!(frontier(&run), vec![cfg(1, "", "AZ")]);

    assert!(!run.step());
    assert_eq!(run.rejected().len(), 3);
    assert!(run.accepted().is_empty());
}

#[test]
fn step_run_marks_accepted_configuration() {
    let pda = anbn_final();
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    let mut run = StepRun::new(&m, "ab");
    while run.step() {}
    let accepted: Vec<_> = run
        .accepted()
        .iter()
        .map(|&n| run.config(n).clone())
        .collect();
    assert_eq!(accepted, vec![cfg(2, "", "Z")]);
}

#[test]
fn trace_of_accepted_run_lists_exact_configurations() {
    let pda = anbn_final();
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    assert_eq!(
        fast_run(&m, "ab", BUDGET),
        RunOutcome::Accepted {
            trace: vec![
                cfg(0, "ab", "Z"),
                cfg(0, "b", "AZ"),
                cfg(1, "b", "AZ"),
                cfg(1, "", "Z"),
                cfg(2, "", "Z"),
            ]
        }
    );
}

#[test]
fn empty_stack_trace_ends_with_empty_stack() {
    let pda = anbn_empty();
    let m = PdaStepByState::new(&pda, Acceptance::EmptyStack);
    assert_eq!(
        fast_run(&m, "ab", BUDGET),
        RunOutcome::Accepted {
            trace: vec![
                cfg(0, "ab", "Z"),
                cfg(0, "b", "AZ"),
                cfg(1, "b", "AZ"),
                cfg(1, "", "Z"),
                cfg(1, "", ""),
            ]
        }
    );
}

/// Pushes `XYZ` (X on top), then pops `XY` and pushes `W`, then pops `WZ`.
fn multi_symbol() -> Pda {
    build(
        4,
        &[3],
        &[
            (0, 1, "a", "Z", "XYZ"),
            (1, 2, "b", "XY", "W"),
            (2, 3, "", "WZ", ""),
        ],
    )
}

#[test]
fn multi_symbol_push_and_pop() {
    let pda = multi_symbol();
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    assert_eq!(
        fast_run(&m, "ab", BUDGET),
        RunOutcome::Accepted {
            trace: vec![
                cfg(0, "ab", "Z"),
                cfg(1, "b", "XYZ"),
                cfg(2, "", "WZ"),
                cfg(3, "", ""),
            ]
        }
    );
    assert!(!accepts(&m, "a"));
    assert!(!accepts(&m, "abb"));
    assert!(!accepts(&m, ""));
    let e = PdaStepByState::new(&pda, Acceptance::EmptyStack);
    assert!(accepts(&e, "ab"));
}

#[test]
fn pop_longer_than_stack_does_not_apply() {
    let pda = build(2, &[1], &[(0, 1, "", "ZZ", "")]);
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    assert_eq!(fast_run(&m, "", BUDGET), RunOutcome::Rejected);
}

#[test]
fn pop_must_match_the_top_in_order() {
    // stack is "XYZ" after the first step; popping "YX" must not match
    let pda = build(3, &[2], &[(0, 1, "", "Z", "XYZ"), (1, 2, "", "YX", "")]);
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    assert!(!accepts(&m, ""));
}

#[test]
fn nondeterministic_states_detection() {
    // q0 has a λ transition, q1 is deterministic (read b/pop A vs read λ/pop Z)
    assert_eq!(nondeterministic_states(&anbn_final()), vec![0]);
    // reading a while popping Z conflicts with reading a while popping λ
    let pda = build(
        3,
        &[],
        &[
            (0, 1, "a", "Z", "AZ"),
            (0, 2, "a", "", "A"),
            (1, 2, "a", "A", ""),
            (1, 2, "b", "A", ""),
        ],
    );
    assert_eq!(nondeterministic_states(&pda), vec![0]);
}

#[test]
fn deterministic_pda_has_no_nondeterministic_states() {
    let pda = build(
        2,
        &[1],
        &[
            (0, 0, "a", "Z", "AZ"),
            (0, 0, "a", "A", "AA"),
            (0, 1, "b", "A", ""),
        ],
    );
    // a/Z vs a/A: pops are not prefixes of each other
    assert!(nondeterministic_states(&pda).is_empty());
}

#[test]
fn lambda_transitions_are_the_all_empty_labels() {
    let pda = anbn_final();
    let found: Vec<(StateId, StateId)> = lambda_transitions(&pda)
        .iter()
        .map(|t| (t.from, t.to))
        .collect();
    // (q1, q2, λ, Z; Z) pops and pushes, so it is not a λ transition
    assert_eq!(found, vec![(0, 1)]);
}

#[test]
fn lambda_push_loop_is_undecided_under_a_small_budget() {
    let pda = build(2, &[1], &[(0, 0, "", "", "A")]);
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    assert_eq!(fast_run(&m, "a", 50), RunOutcome::Undecided);
    assert_eq!(fast_run(&m, "", 50), RunOutcome::Undecided);
}

#[test]
fn pure_lambda_cycle_terminates_as_rejected() {
    let pda = build(3, &[2], &[(0, 1, "", "", ""), (1, 0, "", "", "")]);
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    assert_eq!(fast_run(&m, "", BUDGET), RunOutcome::Rejected);
}

#[test]
fn multiple_run_reports_each_input() {
    let pda = anbn_final();
    let m = PdaStepByState::new(&pda, Acceptance::FinalState);
    let results = multiple_run(&m, ["ab", "aab", ""], BUDGET);
    let verdicts: Vec<(String, bool)> = results
        .into_iter()
        .map(|(i, o)| (i, o.is_accepted()))
        .collect();
    assert_eq!(
        verdicts,
        vec![
            ("ab".to_string(), true),
            ("aab".to_string(), false),
            ("".to_string(), true)
        ]
    );
}

#[test]
fn lambda_closure_follows_only_pure_lambda_transitions() {
    let pda = anbn_final();
    assert_eq!(lambda_closure(&pda, 0), vec![0, 1]);
    assert_eq!(lambda_closure(&pda, 1), vec![1]);
    assert_eq!(lambda_closure(&pda, 2), vec![2]);
}

#[test]
fn with_closure_initial_configs_are_closed() {
    let pda = anbn_final();
    let m = PdaWithClosure::new(&pda, Acceptance::FinalState);
    assert_eq!(
        m.initial_configs("ab"),
        vec![cfg(0, "ab", "Z"), cfg(1, "ab", "Z")]
    );
}

#[test]
fn with_closure_accepts_same_language() {
    for (pda, mode) in [
        (anbn_final(), Acceptance::FinalState),
        (anbn_empty(), Acceptance::EmptyStack),
        (even_palindromes(), Acceptance::FinalState),
    ] {
        let by_state = PdaStepByState::new(&pda, mode);
        let closure = PdaWithClosure::new(&pda, mode);
        for input in [
            "", "a", "b", "ab", "ba", "aa", "abba", "abab", "aabb", "aab",
        ] {
            assert_eq!(
                accepts(&by_state, input),
                accepts(&closure, input),
                "{input:?}"
            );
        }
    }
}

#[test]
fn with_closure_step_closes_each_destination_with_its_own_stack_copy() {
    let pda = anbn_final();
    let m = PdaWithClosure::new(&pda, Acceptance::FinalState);
    assert_eq!(
        m.step(&cfg(0, "ab", "Z")),
        // the a-loop closes q0 into {q0, q1}; the λ transition re-enters q1 unchanged
        vec![cfg(0, "b", "AZ"), cfg(1, "b", "AZ"), cfg(1, "ab", "Z")]
    );
}

#[test]
fn with_closure_trace_skips_lambda_steps() {
    let pda = anbn_final();
    let m = PdaWithClosure::new(&pda, Acceptance::FinalState);
    assert_eq!(
        fast_run(&m, "ab", BUDGET),
        RunOutcome::Accepted {
            trace: vec![
                cfg(0, "ab", "Z"),
                cfg(1, "b", "AZ"),
                cfg(1, "", "Z"),
                cfg(2, "", "Z"),
            ]
        }
    );
}
