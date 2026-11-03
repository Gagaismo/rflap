//! Integration tests for Mealy and Moore machines (JFLAP 7.1 `automata.mealy`).

use rflap_core::analysis::{lambda_transitions, nondeterministic_states};
use rflap_core::automaton::{Automaton, Label, Point, StateId, Transition};
use rflap_core::mealy::{
    Mealy, MealyConfig, MealyLabel, MealyStepByState, Moore, MooreLabel, MooreStepByState,
    moore_output, multiple_output_run, set_moore_output,
};
use rflap_core::sim::{RunOutcome, StepRun, fast_run};

const BUDGET: usize = 10_000;

fn mealy_edge(from: StateId, to: StateId, input: &str, output: &str) -> Transition<MealyLabel> {
    Transition::new(from, to, MealyLabel::new(input, output))
}

fn mealy(n: u32, edges: &[(StateId, StateId, &str, &str)]) -> Mealy {
    let mut m = Mealy::new();
    for i in 0..n {
        m.create_state(Point {
            x: f64::from(i) * 100.0,
            y: 0.0,
        });
    }
    m.set_initial(Some(0));
    for &(f, t, i, o) in edges {
        assert!(m.add_transition(mealy_edge(f, t, i, o)));
    }
    m
}

fn output_of(outcome: &RunOutcome<MealyConfig>) -> Option<String> {
    match outcome {
        RunOutcome::Accepted { trace } => Some(trace.last().unwrap().output.clone()),
        _ => None,
    }
}

fn complement() -> Mealy {
    mealy(1, &[(0, 0, "0", "1"), (0, 0, "1", "0")])
}

#[test]
fn mealy_outputs_the_complement() {
    let m = complement();
    let sim = MealyStepByState::new(&m);
    assert_eq!(
        output_of(&fast_run(&sim, "0110", BUDGET)).as_deref(),
        Some("1001")
    );
    assert_eq!(output_of(&fast_run(&sim, "", BUDGET)).as_deref(), Some(""));
    // A symbol with no transition leaves input unread: rejected, no output.
    assert_eq!(fast_run(&sim, "012", BUDGET), RunOutcome::Rejected);
}

#[test]
fn mealy_step_by_state_trace() {
    let m = complement();
    let sim = MealyStepByState::new(&m);
    let mut run = StepRun::new(&sim, "01");
    assert!(run.step());
    let c = run.config(run.frontier()[0]).clone();
    assert_eq!(
        c,
        MealyConfig {
            state: 0,
            remaining: "1".into(),
            output: "1".into()
        }
    );
    assert!(run.step());
    assert_eq!(run.accepted().len(), 1);
    assert_eq!(run.config(run.accepted()[0]).output, "10");
}

#[test]
fn mealy_accepts_by_consuming_input_not_by_final_state() {
    let m = mealy(2, &[(0, 1, "a", "x")]);
    let sim = MealyStepByState::new(&m);
    assert_eq!(
        output_of(&fast_run(&sim, "a", BUDGET)).as_deref(),
        Some("x")
    );
    assert_eq!(fast_run(&sim, "aa", BUDGET), RunOutcome::Rejected);
}

#[test]
fn mealy_multi_character_labels_and_outputs() {
    let m = mealy(2, &[(0, 1, "ab", "XYZ"), (1, 0, "c", "")]);
    let sim = MealyStepByState::new(&m);
    assert_eq!(
        output_of(&fast_run(&sim, "abcab", BUDGET)).as_deref(),
        Some("XYZXYZ")
    );
}

#[test]
fn mealy_without_initial_state_rejects() {
    let mut m = Mealy::new();
    m.create_state(Point::default());
    let sim = MealyStepByState::new(&m);
    assert_eq!(fast_run(&sim, "", BUDGET), RunOutcome::Rejected);
}

#[test]
fn mealy_lambda_loop_is_undecided() {
    let m = mealy(1, &[(0, 0, "", "1")]);
    let sim = MealyStepByState::new(&m);
    assert_eq!(fast_run(&sim, "a", 50), RunOutcome::Undecided);
}

#[test]
fn multiple_output_run_reports_each_output() {
    let m = complement();
    let sim = MealyStepByState::new(&m);
    let runs = multiple_output_run(&sim, ["01", "", "2", "111"], BUDGET);
    let summary: Vec<(&str, Option<&str>)> = runs
        .iter()
        .map(|r| (r.input.as_str(), r.output.as_deref()))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("01", Some("10")),
            ("", Some("")),
            ("2", None),
            ("111", Some("000"))
        ]
    );
    assert_eq!(runs[2].outcome, RunOutcome::Rejected);
}

fn mod3() -> Moore {
    let mut m = Moore::new();
    for i in 0..3 {
        let id = m.create_state(Point {
            x: f64::from(i) * 100.0,
            y: 0.0,
        });
        set_moore_output(&mut m, id, Some(&i.to_string()));
    }
    m.set_initial(Some(0));
    for (f, t) in [(0, 1), (1, 2), (2, 0)] {
        m.add_transition(Transition::new(f, t, MooreLabel::from("a")));
    }
    m
}

#[test]
fn moore_counts_a_mod_3() {
    let m = mod3();
    let sim = MooreStepByState::new(&m);
    let out = |input: &str| output_of(&fast_run(&sim, input, BUDGET));
    // The initial state's output comes first, then the output of every state entered.
    assert_eq!(out("").as_deref(), Some("0"));
    assert_eq!(out("a").as_deref(), Some("01"));
    assert_eq!(out("aaaa").as_deref(), Some("01201"));
    assert_eq!(out("aaaaaa").as_deref(), Some("0120120"));
    assert_eq!(out("b"), None);
}

#[test]
fn moore_output_defaults_to_empty() {
    let mut m = mod3();
    assert_eq!(moore_output(&m, 1), "1");
    set_moore_output(&mut m, 1, None);
    assert_eq!(moore_output(&m, 1), "");
    assert_eq!(m.state(1).unwrap().output, None);
    let sim = MooreStepByState::new(&m);
    assert_eq!(
        output_of(&fast_run(&sim, "aa", BUDGET)).as_deref(),
        Some("02")
    );
    assert_eq!(moore_output(&m, 99), "");
}

#[test]
fn moore_multiple_output_run() {
    let m = mod3();
    let sim = MooreStepByState::new(&m);
    let runs = multiple_output_run(&sim, ["aa", "ab"], BUDGET);
    assert_eq!(runs[0].output.as_deref(), Some("012"));
    assert_eq!(runs[1].output, None);
}

#[test]
fn mealy_nondeterminism_and_lambda_detection() {
    // Equal labels, or one a prefix of the other, are nondeterministic (outputs are ignored).
    let m = mealy(3, &[(0, 1, "a", "x"), (0, 2, "a", "y")]);
    assert_eq!(nondeterministic_states(&m), vec![0]);
    let m = mealy(3, &[(0, 1, "a", "x"), (0, 2, "ab", "y")]);
    assert_eq!(nondeterministic_states(&m), vec![0]);
    let m = mealy(3, &[(0, 1, "a", "x"), (0, 2, "b", "y")]);
    assert!(nondeterministic_states(&m).is_empty());
    assert!(lambda_transitions(&m).is_empty());

    let m = mealy(2, &[(0, 1, "", "x")]);
    assert_eq!(lambda_transitions(&m).len(), 1);
    assert_eq!(nondeterministic_states(&m), vec![0]);
    assert!(MealyLabel::new("", "q").is_lambda());
    assert!(!MealyLabel::new("a", "").is_lambda());
}

#[test]
fn mealy_nondeterministic_machine_finds_an_accepting_branch() {
    let m = mealy(3, &[(0, 1, "a", "x"), (0, 2, "a", "y"), (2, 2, "b", "z")]);
    let sim = MealyStepByState::new(&m);
    assert_eq!(
        output_of(&fast_run(&sim, "ab", BUDGET)).as_deref(),
        Some("yz")
    );
}

#[test]
fn labels_describe_themselves_like_jflap() {
    assert_eq!(MealyLabel::new("a", "b").description(), "a ; b");
    assert_eq!(MealyLabel::new("", "").description(), "λ ; λ");
    assert_eq!(MooreLabel::from("a").description(), "a");
    assert_eq!(MooreLabel::from("").description(), "λ");
    assert!(MooreLabel::from("").is_lambda());
    assert!(MooreLabel::from("a").conflicts(&MooreLabel::from("ab")));
}

#[test]
fn machine_types_share_the_generic_automaton() {
    let _: Automaton<MealyLabel> = Mealy::new();
    let _: Automaton<MooreLabel> = Moore::new();
}
