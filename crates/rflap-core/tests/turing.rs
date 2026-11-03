//! Integration tests for Turing machines (JFLAP 7.1 `automata.turing`).

use rflap_core::analysis::{lambda_transitions, nondeterministic_states};
use rflap_core::automaton::{Label, Point, StateId};
use rflap_core::sim::{Machine, RunOutcome, StepRun, fast_run, multiple_run};
use rflap_core::turing::{
    AcceptanceFilters, BLANK, Direction, Tape, TapeAction, TmConfig, TmError, TmLabel, TmSimulator,
    TuringMachine, tape_outputs,
};

const BUDGET: usize = 10_000;

fn lbl(read: &str, write: &str, dir: &str) -> TmLabel {
    TmLabel::single(read, write, dir).unwrap()
}

type Edge<'a> = (StateId, StateId, &'a str, &'a str, &'a str);

/// One-tape machine with states `0..n` on a row.
fn build(n: u32, initial: StateId, finals: &[StateId], edges: &[Edge]) -> TuringMachine {
    let mut tm = TuringMachine::new(1);
    for i in 0..n {
        tm.create_state(Point {
            x: f64::from(i) * 100.0,
            y: 0.0,
        });
    }
    tm.set_initial(Some(initial));
    for &f in finals {
        tm.set_final(f, true);
    }
    for &(from, to, r, w, d) in edges {
        assert!(tm.add_transition(from, to, lbl(r, w, d)).unwrap());
    }
    tm
}

fn run(tm: &TuringMachine, filters: AcceptanceFilters, input: &str) -> RunOutcome<TmConfig> {
    fast_run(&TmSimulator::new(tm, filters), input, BUDGET)
}

fn accepts(tm: &TuringMachine, input: &str) -> bool {
    run(tm, AcceptanceFilters::by_final_state(), input).is_accepted()
}

fn last_tape(outcome: &RunOutcome<TmConfig>, tape: usize) -> Tape {
    match outcome {
        RunOutcome::Accepted { trace } => trace.last().unwrap().tapes[tape].clone(),
        other => panic!("expected an accepted run, got {other:?}"),
    }
}

fn abc_machine() -> TuringMachine {
    build(
        6,
        0,
        &[5],
        &[
            (0, 1, "a", "X", "R"),
            (0, 4, "Y", "Y", "R"),
            (1, 1, "a", "a", "R"),
            (1, 1, "Y", "Y", "R"),
            (1, 2, "b", "Y", "R"),
            (2, 2, "b", "b", "R"),
            (2, 2, "Z", "Z", "R"),
            (2, 3, "c", "Z", "L"),
            (3, 3, "Z", "Z", "L"),
            (3, 3, "b", "b", "L"),
            (3, 3, "Y", "Y", "L"),
            (3, 3, "a", "a", "L"),
            (3, 0, "X", "X", "R"),
            (4, 4, "Y", "Y", "R"),
            (4, 4, "Z", "Z", "R"),
            (4, 5, "", "", "S"),
        ],
    )
}

#[test]
fn anbncn_accepts_and_rejects() {
    let tm = abc_machine();
    assert!(accepts(&tm, "abc"));
    assert!(accepts(&tm, "aabbcc"));
    assert!(accepts(&tm, "aaabbbccc"));
    assert!(!accepts(&tm, "aabbc"));
    assert!(!accepts(&tm, "abcc"));
    assert!(!accepts(&tm, "acb"));
    assert!(!accepts(&tm, ""));
}

#[test]
fn anbncn_final_tape_is_exact() {
    let outcome = run(&abc_machine(), AcceptanceFilters::by_final_state(), "abc");
    let tape = last_tape(&outcome, 0);
    assert_eq!(tape.contents(), format!("XYZ{BLANK}"));
    assert_eq!(tape.head(), 3);
}

#[test]
fn unary_addition_leaves_exact_tape() {
    let tm = build(
        4,
        0,
        &[3],
        &[
            (0, 0, "1", "1", "R"),
            (0, 1, "+", "1", "R"),
            (1, 1, "1", "1", "R"),
            (1, 2, "", "", "L"),
            (2, 3, "1", "", "S"),
        ],
    );
    let outcome = run(&tm, AcceptanceFilters::by_final_state(), "11+111");
    let tape = last_tape(&outcome, 0);
    assert_eq!(tape.contents(), format!("11111{BLANK}{BLANK}"));
    assert_eq!(tape.trimmed(), "11111");
    assert_eq!(tape.head(), 5);
    assert_eq!(tape_outputs(&outcome), Some(vec![String::new()]));
}

#[test]
fn output_runs_from_head_to_next_blank() {
    let mut tape = Tape::new("ab");
    assert_eq!(tape.output(), "ab");
    tape.move_head(Direction::Right);
    assert_eq!(tape.output(), "b");
    tape.move_head(Direction::Right);
    assert_eq!(tape.output(), "");
}

#[test]
fn two_tape_machine_copies_the_input() {
    let mut tm = TuringMachine::new(2);
    for _ in 0..2 {
        tm.create_state(Point::default());
    }
    tm.set_initial(Some(0));
    tm.set_final(1, true);
    for s in ["a", "b", "c"] {
        let label = TmLabel::new(vec![
            TapeAction::new(s, s, "R").unwrap(),
            TapeAction::new("", s, "R").unwrap(),
        ])
        .unwrap();
        tm.add_transition(0, 0, label).unwrap();
    }
    let done = TmLabel::new(vec![
        TapeAction::new("", "", "S").unwrap(),
        TapeAction::new("", "", "S").unwrap(),
    ])
    .unwrap();
    tm.add_transition(0, 1, done).unwrap();

    let sim = TmSimulator::with_inputs(
        &tm,
        AcceptanceFilters::by_final_state(),
        vec!["abc".into(), String::new()],
    )
    .unwrap();
    let outcome = fast_run(&sim, "ignored", BUDGET);
    assert_eq!(last_tape(&outcome, 0).contents(), format!("abc{BLANK}"));
    assert_eq!(last_tape(&outcome, 1).contents(), format!("abc{BLANK}"));
    assert_eq!(last_tape(&outcome, 0).head(), 3);
    assert_eq!(last_tape(&outcome, 1).head(), 3);
    assert_eq!(
        tape_outputs(&outcome),
        Some(vec![String::new(), String::new()])
    );

    // A single input string goes on every tape (JFLAP `getInitialConfigurations(String)`).
    let sim = TmSimulator::new(&tm, AcceptanceFilters::default());
    let configs = sim.initial_configs("ab");
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].tapes[0].contents(), "ab");
    assert_eq!(configs[0].tapes[1].contents(), "ab");
}

#[test]
fn input_count_must_match_tape_count() {
    let tm = TuringMachine::new(2);
    let err = TmSimulator::with_inputs(&tm, AcceptanceFilters::default(), vec!["a".into()]);
    assert_eq!(
        err.err(),
        Some(TmError::TapeCount {
            expected: 2,
            found: 1
        })
    );
}

#[test]
fn halting_and_final_state_acceptance_differ() {
    // q0 -a-> q1 (final); q1 -b-> q2 (dead end). Input "b" halts in q0, a non-final state.
    let tm = build(3, 0, &[1], &[(0, 1, "a", "a", "R"), (1, 2, "b", "b", "R")]);
    let by_final = AcceptanceFilters::by_final_state();
    let by_halt = AcceptanceFilters::by_halting();
    let both = AcceptanceFilters {
        final_state: true,
        halting: true,
    };
    assert_eq!(run(&tm, by_final, "b"), RunOutcome::Rejected);
    assert!(run(&tm, by_halt, "b").is_accepted());
    assert!(run(&tm, both, "b").is_accepted());
    assert!(run(&tm, by_final, "a").is_accepted());
    assert!(run(&tm, by_halt, "a").is_accepted());

    // A final state that never halts: accepted by final state, not by halting.
    let looping = build(1, 0, &[0], &[(0, 0, "a", "a", "S")]);
    assert!(run(&looping, by_final, "a").is_accepted());
    assert!(!run(&looping, by_halt, "a").is_accepted());

    // Defaults follow JFLAP: final state on, halting off.
    assert_eq!(AcceptanceFilters::default(), by_final);
}

/// Inner machine rewriting every `a` as `x` while moving right.
fn rewrite_block() -> TuringMachine {
    build(1, 0, &[], &[(0, 0, "a", "x", "R")])
}

#[test]
fn building_block_reused_twice() {
    let mut tm = TuringMachine::new(1);
    let b1 = tm.add_block(Point::default(), rewrite_block()).unwrap();
    let b2 = tm.add_block(Point::default(), rewrite_block()).unwrap();
    let end = tm.create_state(Point::default());
    tm.set_initial(Some(b1));
    tm.set_final(end, true);
    tm.add_transition(b1, b2, lbl("b", "b", "R")).unwrap();
    tm.add_transition(b2, end, lbl("c", "c", "S")).unwrap();

    let outcome = run(&tm, AcceptanceFilters::by_final_state(), "aabaac");
    let tape = last_tape(&outcome, 0);
    assert_eq!(tape.contents(), "xxbxxc");
    assert_eq!(tape.head(), 5);
    assert!(accepts(&tm, "bc"));
    assert!(accepts(&tm, "abaac"));
    assert!(!accepts(&tm, "aabaa"));
    assert!(!accepts(&tm, "aabbc"));

    // States inside a block never accept by final state.
    let mut inner_final = rewrite_block();
    inner_final.set_final(0, true);
    let mut only_block = TuringMachine::new(1);
    let b = only_block.add_block(Point::default(), inner_final).unwrap();
    only_block.set_initial(Some(b));
    assert!(!accepts(&only_block, "aa"));
    assert!(run(&only_block, AcceptanceFilters::by_halting(), "aa").is_accepted());
}

#[test]
fn nested_blocks_run_inside_out() {
    let mut mid = TuringMachine::new(1);
    let inner = mid.add_block(Point::default(), rewrite_block()).unwrap();
    let after = mid.create_state(Point::default());
    mid.set_initial(Some(inner));
    mid.add_transition(inner, after, lbl("b", "y", "R"))
        .unwrap();
    let mut top = TuringMachine::new(1);
    let block = top.add_block(Point::default(), mid).unwrap();
    let end = top.create_state(Point::default());
    top.set_initial(Some(block));
    top.set_final(end, true);
    // No transition of `after` matches, so control climbs back to the top block's exit.
    top.add_transition(block, end, lbl("c", "c", "S")).unwrap();
    let outcome = run(&top, AcceptanceFilters::by_final_state(), "aabc");
    assert_eq!(last_tape(&outcome, 0).contents(), "xxyc");
}

#[test]
fn block_tape_count_must_match() {
    let mut tm = TuringMachine::new(2);
    assert_eq!(
        tm.add_block(Point::default(), TuringMachine::new(1)),
        Err(TmError::TapeCount {
            expected: 2,
            found: 1
        })
    );
    tm.create_state(Point::default());
    assert_eq!(
        tm.add_transition(0, 0, lbl("a", "a", "R")),
        Err(TmError::TapeCount {
            expected: 2,
            found: 1
        })
    );
}

#[test]
fn non_halting_machine_is_undecided() {
    let tm = build(1, 0, &[], &[(0, 0, "~", "~", "R")]);
    let sim = TmSimulator::new(&tm, AcceptanceFilters::by_halting());
    assert_eq!(fast_run(&sim, "", 50), RunOutcome::Undecided);
    let results = multiple_run(&sim, ["", "a"], 50);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|(_, o)| *o == RunOutcome::Undecided));
}

#[test]
fn step_run_shows_halted_configuration() {
    let tm = build(2, 0, &[], &[(0, 1, "a", "b", "R")]);
    let sim = TmSimulator::new(&tm, AcceptanceFilters::by_halting());
    let mut step = StepRun::new(&sim, "a");
    assert!(step.step());
    assert!(!step.config(step.frontier()[0]).halted);
    assert!(step.step());
    let halted = step.config(step.frontier()[0]);
    assert!(halted.halted);
    assert_eq!(halted.state(), 1);
    assert_eq!(step.accepted().len(), 1);
    assert!(!step.step());
}

#[test]
fn tape_extends_in_both_directions() {
    let mut t = Tape::new("");
    assert_eq!(t.contents(), BLANK.to_string());
    t.write('a');
    t.move_head(Direction::Left);
    assert_eq!(t.contents(), format!("{BLANK}a"));
    assert_eq!(t.head(), 0);
    t.move_head(Direction::Right);
    t.move_head(Direction::Right);
    assert_eq!(t.contents(), format!("{BLANK}a{BLANK}"));
    assert_eq!(t.head(), 2);
    // Padding blanks do not make tapes different, the head position relative to the data does.
    assert_eq!(Tape::new("ab"), Tape::new("ab"));
    let mut padded = Tape::new("ab");
    padded.move_head(Direction::Right);
    assert_ne!(padded, Tape::new("ab"));
}

#[test]
fn read_symbols_any_not_and_variables() {
    // ~ matches anything and ~ as write keeps the symbol.
    let tm = build(2, 0, &[1], &[(0, 1, "~", "~", "R")]);
    assert!(accepts(&tm, "q"));
    let outcome = run(&tm, AcceptanceFilters::default(), "q");
    assert_eq!(last_tape(&outcome, 0).contents(), format!("q{BLANK}"));

    // !a matches everything but a.
    let tm = build(2, 0, &[1], &[(0, 1, "!a", "~", "S")]);
    assert!(accepts(&tm, "b"));
    assert!(!accepts(&tm, "a"));
    // A lone "!" means "not blank".
    let tm = build(2, 0, &[1], &[(0, 1, "!", "x", "S")]);
    assert!(accepts(&tm, "a"));
    assert!(!accepts(&tm, ""));

    // a,b}v reads a or b into variable v; writing v writes that symbol back.
    let tm = build(
        3,
        0,
        &[2],
        &[(0, 1, "a,b}v", "v", "R"), (1, 2, "c", "v", "S")],
    );
    let outcome = run(&tm, AcceptanceFilters::default(), "bc");
    assert_eq!(last_tape(&outcome, 0).contents(), "bb");
    assert!(!accepts(&tm, "xc"));
}

#[test]
fn label_validation_follows_jflap() {
    assert_eq!(TmLabel::new(vec![]).err(), Some(TmError::NoTapes));
    assert!(matches!(
        TapeAction::new("ab", "a", "R"),
        Err(TmError::BadRead(_))
    ));
    assert!(matches!(
        TapeAction::new("!a}v", "a", "R"),
        Err(TmError::BadRead(_))
    ));
    assert!(matches!(
        TapeAction::new("a", "ab", "R"),
        Err(TmError::BadWrite(_))
    ));
    assert!(matches!(
        TapeAction::new("a", "a", "X"),
        Err(TmError::BadDirection(_))
    ));
    // Empty read and write are the blank; "!" alone is "not blank".
    let blank = TapeAction::new("", "", "S").unwrap();
    assert_eq!(blank.read(), BLANK.to_string());
    assert_eq!(blank.write(), BLANK);
    assert_eq!(blank.direction(), Direction::Stay);
    assert_eq!(
        TapeAction::new("!", "a", "L").unwrap().read(),
        format!("!{BLANK}")
    );
    assert_eq!(lbl("a", "b", "R").description(), "a ; b , R");
    let two = TmLabel::new(vec![
        TapeAction::new("a", "b", "R").unwrap(),
        TapeAction::new("c", "d", "L").unwrap(),
    ])
    .unwrap();
    assert_eq!(two.description(), "a ; b , R | c ; d , L");
    assert_eq!(two.tapes(), 2);
}

fn nd(a: (&str, &str, &str), b: (&str, &str, &str)) -> bool {
    let tm = build(3, 0, &[], &[(0, 1, a.0, a.1, a.2), (0, 2, b.0, b.1, b.2)]);
    nondeterministic_states(tm.automaton()) == vec![0]
}

#[test]
fn nondeterminism_detection_follows_jflap() {
    assert!(nd(("a", "a", "R"), ("a", "b", "L")));
    assert!(!nd(("a", "a", "R"), ("b", "a", "R")));
    assert!(nd(("~", "a", "R"), ("b", "a", "R")));
    assert!(nd(("!b", "a", "R"), ("a", "a", "R")));
    assert!(!nd(("!b", "a", "R"), ("b", "a", "R")));
    assert!(nd(("!a", "a", "R"), ("!b", "a", "R")));

    let tm = build(2, 0, &[1], &[(0, 1, "a", "a", "R"), (1, 0, "a", "a", "R")]);
    assert!(nondeterministic_states(tm.automaton()).is_empty());
    assert!(lambda_transitions(tm.automaton()).is_empty());
    assert!(!lbl("", "", "S").is_lambda());
}

#[test]
fn simulation_takes_first_match_with_not_forms_last() {
    // JFLAP tries transitions in order, `!` forms last, and follows only the first match.
    let tm = build(3, 0, &[2], &[(0, 1, "!a", "z", "S"), (0, 2, "b", "b", "S")]);
    let outcome = run(&tm, AcceptanceFilters::default(), "b");
    assert_eq!(last_tape(&outcome, 0).contents(), "b");
}
