//! Integration tests for the FA core (JFLAP 7.1 `automata`, `automata.fsa`).

use std::collections::BTreeSet;

use rflap_core::analysis::{
    NoInitialState, clean_useless, lambda_transitions, nondeterministic_states, unreachable_states,
    useless_states,
};
use rflap_core::automaton::{Automaton, Point, StateId, Transition};
use rflap_core::fa::{
    Fa, FaConfig, FaLabel, FaStepByState, FaWithClosure, add_trap_state, alphabet, combine,
    has_multi_char_labels, lambda_closure, split_multi_char_labels,
};
use rflap_core::sim::{Machine, RunOutcome, StepRun, fast_run, multiple_run};

const BUDGET: usize = 10_000;

fn tr(from: StateId, to: StateId, label: &str) -> Transition<FaLabel> {
    Transition::new(from, to, FaLabel::from(label))
}

/// Builds an FA with states `0..n` laid out on a row.
fn build(n: u32, initial: StateId, finals: &[StateId], edges: &[(StateId, StateId, &str)]) -> Fa {
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
    for &(from, to, label) in edges {
        assert!(fa.add_transition(tr(from, to, label)));
    }
    fa
}

fn accepts<M: Machine>(machine: &M, input: &str) -> bool {
    matches!(
        fast_run(machine, input, BUDGET),
        RunOutcome::Accepted { .. }
    )
}

fn states_of(trace: &[FaConfig]) -> Vec<StateId> {
    trace.iter().map(|c| c.state).collect()
}

fn even_a() -> Fa {
    build(
        2,
        0,
        &[0],
        &[(0, 1, "a"), (1, 0, "a"), (0, 0, "b"), (1, 1, "b")],
    )
}

#[test]
fn dfa_even_number_of_as() {
    let fa = even_a();
    let by_state = FaStepByState::new(&fa);
    let closure = FaWithClosure::new(&fa);
    for input in ["", "aa", "abab", "bb", "aba"] {
        assert!(accepts(&by_state, input), "{input:?}");
        assert!(accepts(&closure, input), "{input:?}");
    }
    for input in ["a", "bab", "aaa"] {
        assert!(!accepts(&by_state, input), "{input:?}");
        assert!(!accepts(&closure, input), "{input:?}");
    }
}

#[test]
fn nfa_with_lambda_both_simulators() {
    let fa = build(3, 0, &[2], &[(0, 1, ""), (1, 2, "a")]);
    let by_state = FaStepByState::new(&fa);
    let closure = FaWithClosure::new(&fa);
    assert!(accepts(&by_state, "a") && accepts(&closure, "a"));
    for input in ["", "aa"] {
        assert!(!accepts(&by_state, input), "{input:?}");
        assert!(!accepts(&closure, input), "{input:?}");
    }
    let initial: Vec<StateId> = closure
        .initial_configs("a")
        .iter()
        .map(|c| c.state)
        .collect();
    assert_eq!(initial, vec![0, 1]);
    let initial: Vec<StateId> = by_state
        .initial_configs("a")
        .iter()
        .map(|c| c.state)
        .collect();
    assert_eq!(initial, vec![0]);
    assert_eq!(lambda_closure(&fa, 0), vec![0, 1]);
    assert_eq!(lambda_closure(&fa, 2), vec![2]);
}

#[test]
fn range_labels() {
    let fa = build(2, 0, &[1], &[(0, 1, "[0-9]"), (1, 1, "[0-9]")]);
    let by_state = FaStepByState::new(&fa);
    let closure = FaWithClosure::new(&fa);
    for input in ["123", "7"] {
        assert!(accepts(&by_state, input), "{input:?}");
        assert!(accepts(&closure, input), "{input:?}");
    }
    for input in ["", "1a"] {
        assert!(!accepts(&by_state, input), "{input:?}");
        assert!(!accepts(&closure, input), "{input:?}");
    }
    assert!(
        !has_multi_char_labels(&fa),
        "a range is not a multi-char label"
    );
    assert_eq!(split_multi_char_labels(&fa), fa);
}

#[test]
fn range_transition_takes_lambda_closure_with_closure_simulator() {
    // L5: the destination of a range transition is λ-closed.
    let fa = build(3, 0, &[2], &[(0, 1, "[a-c]"), (1, 2, "")]);
    assert!(accepts(&FaWithClosure::new(&fa), "b"));
    assert!(accepts(&FaStepByState::new(&fa), "b"));
    let closure = FaWithClosure::new(&fa);
    let next = closure.step(&closure.initial_configs("b")[0]);
    assert_eq!(next.iter().map(|c| c.state).collect::<Vec<_>>(), vec![1, 2]);
    // A bracketed label that is not exactly `[x-y]` is a literal.
    let literal = build(2, 0, &[1], &[(0, 1, "[ab]")]);
    assert!(accepts(&FaStepByState::new(&literal), "[ab]"));
    assert!(!accepts(&FaStepByState::new(&literal), "a"));
}

#[test]
fn multi_char_label() {
    let fa = build(2, 0, &[1], &[(0, 1, "ab")]);
    assert!(has_multi_char_labels(&fa));
    let machine = FaStepByState::new(&fa);
    assert!(accepts(&machine, "ab"));
    assert!(!accepts(&machine, "a"));
    let split = split_multi_char_labels(&fa);
    assert_eq!(split.states().count(), 3);
    assert_eq!(split.transitions().len(), 2);
    assert_eq!(split.transitions()[0], tr(0, 2, "a"));
    assert_eq!(split.transitions()[1], tr(2, 1, "b"));
    assert!(!has_multi_char_labels(&split));
    assert!(accepts(&FaStepByState::new(&split), "ab"));
    assert!(!accepts(&FaStepByState::new(&split), "a"));
    // Interpolated position: halfway between (0,0) and (100,0).
    assert_eq!(split.state(2).unwrap().point, Point { x: 50.0, y: 0.0 });
}

#[test]
fn create_state_reuses_smallest_free_id() {
    let mut fa = Fa::new();
    for _ in 0..3 {
        fa.create_state(Point::default());
    }
    fa.set_initial(Some(1));
    fa.set_final(1, true);
    fa.remove_state(1);
    assert_eq!(fa.initial(), None);
    assert!(!fa.is_final(1));
    let id = fa.create_state(Point::default());
    assert_eq!(id, 1);
    assert_eq!(fa.state(1).unwrap().name, "q1");
    assert_eq!(fa.states().map(|s| s.id).collect::<Vec<_>>(), vec![0, 1, 2]);
}

#[test]
fn remove_state_removes_its_transitions() {
    let mut fa = build(3, 0, &[2], &[(0, 1, "a"), (1, 2, "b"), (0, 2, "c")]);
    fa.remove_state(1);
    assert_eq!(fa.transitions().to_vec(), vec![tr(0, 2, "c")]);
}

#[test]
fn add_transition_rejects_duplicates_and_dangling() {
    let mut fa = build(2, 0, &[1], &[(0, 1, "a")]);
    assert!(!fa.add_transition(tr(0, 1, "a")));
    assert_eq!(fa.transitions().len(), 1);
    assert!(!fa.add_transition(tr(0, 5, "a")));
    assert!(!fa.add_transition(tr(7, 1, "a")));
    assert_eq!(fa.transitions().len(), 1);
    assert!(fa.add_transition(tr(0, 1, "b")));
    assert!(fa.remove_transition(&tr(0, 1, "a")));
    assert!(!fa.remove_transition(&tr(0, 1, "a")));
    assert_eq!(fa.transitions_from(0).count(), 1);
    assert_eq!(fa.transitions_to(1).count(), 1);
}

#[test]
fn nondeterminism_detection() {
    let two_a = build(2, 0, &[1], &[(0, 1, "a"), (0, 0, "a")]);
    assert_eq!(nondeterministic_states(&two_a), vec![0]);
    let lambda = build(3, 0, &[2], &[(0, 1, "a"), (1, 2, ""), (2, 2, "b")]);
    assert_eq!(nondeterministic_states(&lambda), vec![1]);
    assert_eq!(lambda_transitions(&lambda), vec![&tr(1, 2, "")]);
    let prefix = build(3, 0, &[2], &[(0, 1, "a"), (0, 2, "ab")]);
    assert_eq!(nondeterministic_states(&prefix), vec![0]);
    assert!(nondeterministic_states(&even_a()).is_empty());
    assert!(lambda_transitions(&even_a()).is_empty());
    let range = build(3, 0, &[2], &[(0, 1, "[0-9]"), (0, 2, "5")]);
    assert_eq!(nondeterministic_states(&range), vec![0]);
    let disjoint = build(3, 0, &[2], &[(0, 1, "[0-4]"), (0, 2, "5"), (0, 0, "[6-9]")]);
    assert!(nondeterministic_states(&disjoint).is_empty());
}

#[test]
fn unreachable_and_useless_states() {
    // q2 is a dead end, q3 is unreachable.
    let fa = build(
        4,
        0,
        &[1],
        &[(0, 1, "a"), (1, 1, "a"), (0, 2, "b"), (3, 1, "a")],
    );
    assert_eq!(unreachable_states(&fa), BTreeSet::from([3]));
    assert_eq!(useless_states(&fa), Ok(BTreeSet::from([2, 3])));
    let clean = clean_useless(&fa).unwrap();
    assert_eq!(clean.states().map(|s| s.id).collect::<Vec<_>>(), vec![0, 1]);
    assert_eq!(
        clean.transitions().to_vec(),
        vec![tr(0, 1, "a"), tr(1, 1, "a")]
    );

    let mut no_initial = fa.clone();
    no_initial.set_initial(None);
    assert_eq!(useless_states(&no_initial), Err(NoInitialState));
    assert!(clean_useless(&no_initial).is_err());
}

#[test]
fn clean_keeps_a_useless_initial_state_but_drops_its_transitions() {
    let fa = build(2, 0, &[], &[(0, 1, "a"), (0, 0, "b")]);
    let clean = clean_useless(&fa).unwrap();
    assert_eq!(clean.states().map(|s| s.id).collect::<Vec<_>>(), vec![0]);
    assert!(clean.transitions().is_empty());
}

#[test]
fn alphabet_lists_distinct_non_lambda_labels_in_order() {
    let fa = build(
        2,
        0,
        &[1],
        &[(0, 1, "b"), (0, 0, ""), (1, 1, "a"), (1, 0, "b")],
    );
    assert_eq!(alphabet(&fa), vec!["b".to_string(), "a".to_string()]);
}

#[test]
fn trap_state_completes_a_partial_dfa() {
    let mut fa = build(2, 0, &[1], &[(0, 1, "a"), (1, 1, "a"), (1, 0, "b")]);
    let before = fa.clone();
    let trap = add_trap_state(&mut fa);
    assert_eq!(trap, 2);
    assert_eq!(fa.state(trap).unwrap().label.as_deref(), Some("Trap State"));
    assert_eq!(
        fa.transitions()[before.transitions().len()..].to_vec(),
        vec![tr(0, 2, "b"), tr(2, 2, "a"), tr(2, 2, "b")]
    );
    assert_eq!(fa.transitions().len(), 6);
    let (old, new) = (FaStepByState::new(&before), FaStepByState::new(&fa));
    for input in ["", "a", "b", "ab", "aab", "ba", "aba", "abab", "bb"] {
        assert_eq!(accepts(&old, input), accepts(&new, input), "{input:?}");
    }
    // Completeness: every state has a transition for every symbol.
    for s in fa.states() {
        for sym in ["a", "b"] {
            assert_eq!(
                fa.transitions_from(s.id)
                    .filter(|t| t.label.0 == sym)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn combine_appends_b_below_a() {
    let mut a = build(2, 0, &[1], &[(0, 1, "a")]);
    a.state_mut(0).unwrap().point = Point { x: 0.0, y: 0.0 };
    a.state_mut(1).unwrap().point = Point { x: 100.0, y: 50.0 };
    let mut b = Fa::new();
    let b0 = b.create_state(Point { x: 10.0, y: 200.0 });
    let b1 = b.create_state(Point { x: 30.0, y: 260.0 });
    b.set_initial(Some(b0));
    b.set_final(b1, true);
    b.state_mut(b1).unwrap().label = Some("end".into());
    b.add_transition(tr(b0, b1, "L"));

    let c = combine(&a, &b);
    assert_eq!(c.states().count(), 4);
    // a bottom = 50 + 20, b top = 200 - 20, gap 20: shift = 70 - 180 + 20 = -90.
    assert_eq!(c.state(2).unwrap().point, Point { x: 10.0, y: 110.0 });
    assert_eq!(c.state(3).unwrap().point, Point { x: 30.0, y: 170.0 });
    assert_eq!(c.state(3).unwrap().label.as_deref(), Some("end"));
    assert_eq!(c.initial(), Some(0));
    assert_eq!(c.finals().iter().copied().collect::<Vec<_>>(), vec![1, 3]);
    assert_eq!(c.transitions().to_vec(), vec![tr(0, 1, "a"), tr(2, 3, "L")]);
    // Inputs are untouched.
    assert_eq!(a.states().count(), 2);
}

#[test]
fn fast_run_trace_lists_the_state_path() {
    let fa = even_a();
    let m = FaStepByState::new(&fa);
    match fast_run(&m, "abab", BUDGET) {
        RunOutcome::Accepted { trace } => {
            assert_eq!(states_of(&trace), vec![0, 1, 1, 0, 0]);
            assert_eq!(trace[0].remaining, "abab");
            assert_eq!(trace[4].remaining, "");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn lambda_self_loop_terminates_as_rejected() {
    let fa = build(1, 0, &[], &[(0, 0, "")]);
    let m = FaStepByState::new(&fa);
    assert_eq!(fast_run(&m, "a", BUDGET), RunOutcome::Rejected);
    assert_eq!(fast_run(&m, "", BUDGET), RunOutcome::Rejected);
}

#[test]
fn budget_exhaustion_is_undecided() {
    let fa = build(3, 0, &[2], &[(0, 1, "a"), (0, 2, "a"), (1, 1, "a")]);
    let m = FaStepByState::new(&fa);
    assert_eq!(fast_run(&m, "aa", 1), RunOutcome::Undecided);
    assert!(fast_run(&m, "a", BUDGET).is_accepted());
    assert_eq!(fast_run(&m, "aa", BUDGET), RunOutcome::Rejected);
}

#[test]
fn no_initial_state_rejects() {
    let mut fa = even_a();
    fa.set_initial(None);
    assert_eq!(
        fast_run(&FaStepByState::new(&fa), "", BUDGET),
        RunOutcome::Rejected
    );
}

#[test]
fn multiple_run_reports_each_input() {
    let fa = even_a();
    let m = FaWithClosure::new(&fa);
    let results = multiple_run(&m, ["aa", "a", ""], BUDGET);
    let verdicts: Vec<(&str, bool)> = results
        .iter()
        .map(|(i, o)| (i.as_str(), o.is_accepted()))
        .collect();
    assert_eq!(verdicts, vec![("aa", true), ("a", false), ("", true)]);
}

#[test]
fn step_run_frontier_and_rejected_sets() {
    let fa = build(
        4,
        0,
        &[3],
        &[(0, 1, "a"), (0, 2, "a"), (1, 3, "b"), (2, 3, "c")],
    );
    let m = FaStepByState::new(&fa);
    let mut run = StepRun::new(&m, "ab");
    let frontier_states = |run: &StepRun<'_, FaStepByState<'_>>| -> Vec<StateId> {
        run.frontier()
            .iter()
            .map(|&n| run.config(n).state)
            .collect()
    };
    assert_eq!(frontier_states(&run), vec![0]);
    assert!(run.step());
    assert_eq!(frontier_states(&run), vec![1, 2]);
    assert!(run.rejected().is_empty());
    assert!(run.step());
    assert_eq!(frontier_states(&run), vec![3]);
    let rejected: Vec<StateId> = run
        .rejected()
        .iter()
        .map(|&n| run.config(n).state)
        .collect();
    assert_eq!(rejected, vec![2]);
    assert_eq!(run.accepted().len(), 1);
    let node = run.accepted()[0];
    assert_eq!(states_of(&run.trace(node)), vec![0, 1, 3]);
    assert!(!run.step());
    assert!(run.frontier().is_empty());
    // An accepting configuration with no successors is not rejected.
    assert_eq!(run.rejected().len(), 1);
}

#[test]
fn serde_round_trip() {
    let fa = build(2, 0, &[1], &[(0, 1, "[0-9]")]);
    let json = serde_json::to_string(&fa).unwrap();
    let back: Automaton<FaLabel> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, fa);
}
