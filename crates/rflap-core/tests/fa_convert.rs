//! Integration tests for FA conversions (JFLAP 7.1 NFA→DFA, minimizer, FA↔RE, equality).

use rflap_core::automaton::{Point, StateId, Transition};
use rflap_core::fa::{Fa, FaLabel, FaWithClosure};
use rflap_core::fa_convert::{
    FaToRegexStep, MinimizeStep, NfaToDfaStep, ReAction, equivalent, fa_to_regex,
    fa_to_regex_steps, minimize, nfa_to_dfa, regex_to_nfa, regex_to_nfa_steps,
};
use rflap_core::regex::Regex;
use rflap_core::sim::{RunOutcome, fast_run};

type Edge = (StateId, StateId, String);

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
        assert!(fa.add_transition(Transition::new(from, to, FaLabel::from(label))));
    }
    fa
}

fn accepts(fa: &Fa, input: &str) -> bool {
    matches!(
        fast_run(&FaWithClosure::new(fa), input, 100_000),
        RunOutcome::Accepted { .. }
    )
}

fn words(alphabet: &[char], max_len: usize) -> Vec<String> {
    let mut all = vec![String::new()];
    let mut layer = vec![String::new()];
    for _ in 0..max_len {
        layer = layer
            .iter()
            .flat_map(|w| alphabet.iter().map(move |c| format!("{w}{c}")))
            .collect();
        all.extend(layer.iter().cloned());
    }
    all
}

fn same_language_up_to_6(a: &Fa, b: &Fa) {
    for w in words(&['a', 'b'], 6) {
        assert_eq!(accepts(a, &w), accepts(b, &w), "word {w:?}");
    }
}

fn labels(fa: &Fa) -> Vec<String> {
    fa.states()
        .map(|s| s.label.clone().unwrap_or_default())
        .collect()
}

fn edges(fa: &Fa) -> Vec<Edge> {
    fa.transitions()
        .iter()
        .map(|t| (t.from, t.to, t.label.0.clone()))
        .collect()
}

fn edge_list(list: &[(StateId, StateId, &str)]) -> Vec<Edge> {
    list.iter()
        .map(|&(f, t, l)| (f, t, l.to_string()))
        .collect()
}

fn finals(fa: &Fa) -> Vec<StateId> {
    fa.finals().iter().copied().collect()
}

/// Dragon-book NFA for (a+b)*abb.
fn abb_nfa() -> Fa {
    build(
        4,
        0,
        &[3],
        &[
            (0, 0, "a"),
            (0, 0, "b"),
            (0, 1, "a"),
            (1, 2, "b"),
            (2, 3, "b"),
        ],
    )
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
fn nfa_to_dfa_builds_the_textbook_subsets() {
    let conv = nfa_to_dfa(&abb_nfa());
    assert_eq!(labels(&conv.dfa), ["0", "0,1", "0,2", "0,3"]);
    let sets: Vec<Vec<StateId>> = conv
        .dfa_states
        .iter()
        .map(|d| d.nfa_states.clone())
        .collect();
    assert_eq!(sets, [vec![0], vec![0, 1], vec![0, 2], vec![0, 3]]);
    assert_eq!(conv.dfa.initial(), Some(0));
    assert_eq!(finals(&conv.dfa), [3]);
    assert_eq!(
        edges(&conv.dfa),
        edge_list(&[
            (0, 1, "a"),
            (0, 0, "b"),
            (1, 1, "a"),
            (1, 2, "b"),
            (2, 1, "a"),
            (2, 3, "b"),
            (3, 1, "a"),
            (3, 0, "b"),
        ])
    );
    assert_eq!(conv.dfa.state(2).unwrap().name, "q2");
    // 4 states + 8 transitions
    assert_eq!(conv.steps.len(), 12);
    assert_eq!(
        conv.steps[0],
        NfaToDfaStep::NewState {
            state: 0,
            nfa_states: vec![0]
        }
    );
    assert_eq!(
        conv.steps[1],
        NfaToDfaStep::NewState {
            state: 1,
            nfa_states: vec![0, 1]
        }
    );
    assert_eq!(
        conv.steps[2],
        NfaToDfaStep::AddTransition {
            from: 0,
            to: 1,
            symbol: "a".into()
        }
    );
    same_language_up_to_6(&abb_nfa(), &conv.dfa);
}

#[test]
fn nfa_to_dfa_takes_lambda_closures() {
    let nfa = build(
        3,
        0,
        &[2],
        &[(0, 1, ""), (1, 1, "a"), (1, 2, "b"), (2, 0, "")],
    );
    let conv = nfa_to_dfa(&nfa);
    let sets: Vec<Vec<StateId>> = conv
        .dfa_states
        .iter()
        .map(|d| d.nfa_states.clone())
        .collect();
    assert_eq!(sets, [vec![0, 1], vec![1], vec![0, 1, 2]]);
    assert_eq!(labels(&conv.dfa), ["0,1", "1", "0,1,2"]);
    assert_eq!(finals(&conv.dfa), [2]);
    assert_eq!(conv.dfa.transitions().len(), 6);
    assert_eq!(conv.steps.len(), 9);
    same_language_up_to_6(&nfa, &conv.dfa);
}

#[test]
fn nfa_to_dfa_leaves_a_dfa_alone() {
    let conv = nfa_to_dfa(&even_a());
    assert_eq!(conv.dfa, even_a());
    assert!(conv.steps.is_empty());
}

#[test]
fn nfa_to_dfa_expands_ranges_and_multi_character_labels() {
    let nfa = build(3, 0, &[2], &[(0, 1, "[a-b]"), (1, 2, "ab"), (0, 2, "b")]);
    let conv = nfa_to_dfa(&nfa);
    for w in ["aab", "bab", "b", "", "ab", "aa"] {
        assert_eq!(accepts(&nfa, w), accepts(&conv.dfa, w), "{w}");
    }
}

fn six_state_mod3() -> Fa {
    let mut e = Vec::new();
    for i in 0..6u32 {
        e.push((i, (i + 1) % 6, "a"));
        e.push((i, i, "b"));
    }
    build(6, 0, &[0, 3], &e)
}

#[test]
fn minimize_collapses_six_states_to_three() {
    let min = minimize(&six_state_mod3());
    assert_eq!(min.dfa.states().count(), 3);
    assert_eq!(labels(&min.dfa), ["1,4", "2,5", "0,3"]);
    assert_eq!(min.dfa.initial(), Some(2));
    assert_eq!(finals(&min.dfa), [2]);
    let mut got = edges(&min.dfa);
    got.sort();
    let mut want = edge_list(&[
        (0, 1, "a"),
        (0, 0, "b"),
        (1, 2, "a"),
        (1, 1, "b"),
        (2, 0, "a"),
        (2, 2, "b"),
    ]);
    want.sort();
    assert_eq!(got, want);
    // initial partition + one split + three merged groups
    assert_eq!(
        min.steps,
        vec![
            MinimizeStep::InitialPartition {
                groups: vec![vec![1, 2, 4, 5], vec![0, 3]]
            },
            MinimizeStep::Split {
                group: vec![1, 2, 4, 5],
                symbol: "a".into(),
                into: vec![vec![1, 4], vec![2, 5]],
            },
            MinimizeStep::MergeGroup {
                group: vec![1, 4],
                state: 0
            },
            MinimizeStep::MergeGroup {
                group: vec![2, 5],
                state: 1
            },
            MinimizeStep::MergeGroup {
                group: vec![0, 3],
                state: 2
            },
        ]
    );
    assert_eq!(min.tree.states, [0, 1, 2, 3, 4, 5]);
    assert_eq!(min.tree.symbol, None);
    assert_eq!(min.tree.children.len(), 2);
    assert_eq!(min.tree.children[0].symbol.as_deref(), Some("a"));
    assert_eq!(min.tree.children[0].children.len(), 2);
    same_language_up_to_6(&six_state_mod3(), &min.dfa);
}

#[test]
fn minimize_adds_and_then_drops_the_trap_state() {
    let dfa = build(2, 0, &[1], &[(0, 1, "a")]);
    let min = minimize(&dfa);
    assert_eq!(min.dfa.states().count(), 2);
    assert_eq!(edges(&min.dfa), edge_list(&[(0, 1, "a")]));
    assert_eq!(min.steps.len(), 5);
    assert_eq!(min.steps[0], MinimizeStep::AddTrapState { state: 2 });
    assert_eq!(
        min.steps[2],
        MinimizeStep::Split {
            group: vec![0, 2],
            symbol: "a".into(),
            into: vec![vec![2], vec![0]],
        }
    );
}

#[test]
fn minimize_removes_unreachable_states() {
    let dfa = build(3, 0, &[0], &[(0, 0, "a"), (1, 2, "a"), (2, 1, "a")]);
    let min = minimize(&dfa);
    assert_eq!(
        min.steps[0],
        MinimizeStep::RemoveUnreachable { states: vec![1, 2] }
    );
    assert_eq!(min.dfa.states().count(), 1);
    assert_eq!(edges(&min.dfa), edge_list(&[(0, 0, "a")]));
}

#[test]
fn regex_to_nfa_to_dfa_to_min_has_four_states() {
    let nfa = regex_to_nfa(&Regex::parse("(a+b)*abb").unwrap());
    let min = minimize(&nfa_to_dfa(&nfa).dfa);
    assert_eq!(min.dfa.states().count(), 4);
    same_language_up_to_6(&nfa, &min.dfa);
    same_language_up_to_6(&nfa, &abb_nfa());
}

#[test]
fn regex_to_nfa_steps_follow_the_deexpressionify_order() {
    let built = regex_to_nfa_steps(&Regex::parse("(a+b)*abb").unwrap());
    let actions: Vec<ReAction> = built.steps.iter().map(|s| s.action).collect();
    assert_eq!(
        actions,
        [
            ReAction::Decat,
            ReAction::Destar,
            ReAction::Deparens,
            ReAction::Deor
        ]
    );
    assert_eq!(built.steps[0].transition.label, "(a+b)*abb");
    assert_eq!(built.steps[0].new_states, [2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(built.nfa.states().count(), 16);
    assert_eq!(built.nfa.initial(), Some(0));
    assert_eq!(finals(&built.nfa), [1]);
    assert!(
        built
            .nfa
            .transitions()
            .iter()
            .all(|t| t.label.0.chars().count() <= 1)
    );
}

#[test]
fn regex_to_nfa_handles_lambda_and_the_empty_set() {
    let opt = regex_to_nfa(&Regex::parse("a*+λ").unwrap());
    assert!(accepts(&opt, "") && accepts(&opt, "aaa") && !accepts(&opt, "b"));
    let none = regex_to_nfa(&Regex::Empty);
    assert!(!accepts(&none, "") && !accepts(&none, "a"));
    let lam = regex_to_nfa(&Regex::Lambda);
    assert!(accepts(&lam, "") && !accepts(&lam, "a"));
}

#[test]
fn fa_to_regex_single_symbol() {
    let conv = fa_to_regex_steps(&build(2, 0, &[1], &[(0, 1, "a")]));
    assert_eq!(conv.regex, Regex::Symbol('a'));
    assert_eq!(conv.steps.len(), 2);
    assert_eq!(
        conv.steps[0],
        FaToRegexStep::AddEmptyTransitions {
            pairs: vec![(0, 0), (1, 0), (1, 1)]
        }
    );
    assert_eq!(
        conv.steps[1],
        FaToRegexStep::Result {
            expression: "a".into()
        }
    );
}

#[test]
fn fa_to_regex_eliminates_states_in_id_order() {
    let conv = fa_to_regex_steps(&build(3, 0, &[2], &[(0, 1, "a"), (1, 2, "b")]));
    assert_eq!(conv.regex.to_string(), "ab");
    assert_eq!(conv.steps.len(), 3);
    match &conv.steps[1] {
        FaToRegexStep::EliminateState { state, edges } => {
            assert_eq!(*state, 1);
            assert_eq!(edges.len(), 4);
            let ab = edges.iter().find(|e| (e.from, e.to) == (0, 2)).unwrap();
            assert_eq!(ab.label, "ab");
        }
        other => panic!("unexpected step {other:?}"),
    }
}

#[test]
fn fa_to_regex_adds_a_final_state_when_initial_is_final() {
    let conv = fa_to_regex_steps(&even_a());
    assert_eq!(conv.steps.len(), 4);
    assert_eq!(
        conv.steps[0],
        FaToRegexStep::AddFinalState {
            state: 2,
            old_finals: vec![0]
        }
    );
    assert!(matches!(
        conv.steps[1],
        FaToRegexStep::AddEmptyTransitions { .. }
    ));
}

#[test]
fn fa_to_regex_collapses_parallel_transitions() {
    let conv = fa_to_regex_steps(&build(2, 0, &[1], &[(0, 1, "a"), (0, 1, "b")]));
    assert_eq!(conv.regex.to_string(), "a+b");
    assert_eq!(
        conv.steps[0],
        FaToRegexStep::CollapseParallel {
            from: 0,
            to: 1,
            labels: vec!["a".into(), "b".into()],
            label: "a+b".into()
        }
    );
}

#[test]
fn fa_to_regex_without_finals_is_the_empty_language() {
    assert_eq!(fa_to_regex(&build(2, 0, &[], &[(0, 1, "a")])), Regex::Empty);
}

#[test]
fn fa_to_regex_then_regex_to_nfa_keeps_the_language() {
    let machines = [
        even_a(),
        abb_nfa(),
        six_state_mod3(),
        build(
            3,
            0,
            &[2],
            &[(0, 1, ""), (1, 1, "a"), (1, 2, "b"), (2, 0, "")],
        ),
        build(2, 0, &[1], &[(0, 1, "a"), (0, 1, "b"), (1, 0, "a")]),
        build(3, 0, &[0, 2], &[(0, 1, "a"), (1, 2, "b"), (2, 0, "a")]),
    ];
    for fa in &machines {
        let re = fa_to_regex(fa);
        let back = regex_to_nfa(&re);
        let result = equivalent(fa, &back);
        assert!(result.equivalent, "{re}: {:?}", result.counterexample);
        same_language_up_to_6(fa, &back);
        for w in words(&['a', 'b'], 6) {
            assert_eq!(re.matches(&w), accepts(fa, &w), "{re} on {w:?}");
        }
    }
}

#[test]
fn equivalent_accepts_the_same_language() {
    let min = minimize(&nfa_to_dfa(&abb_nfa()).dfa).dfa;
    let result = equivalent(&abb_nfa(), &min);
    assert!(result.equivalent);
    assert_eq!(result.counterexample, None);
}

#[test]
fn equivalent_returns_the_shortest_counterexample() {
    let ends_ab = build(
        3,
        0,
        &[2],
        &[(0, 0, "a"), (0, 0, "b"), (0, 1, "a"), (1, 2, "b")],
    );
    let r = equivalent(&ends_ab, &abb_nfa());
    assert!(!r.equivalent);
    assert_eq!(r.counterexample.as_deref(), Some("ab"));

    let even = regex_to_nfa(&Regex::parse("(aa)*").unwrap());
    let any = regex_to_nfa(&Regex::parse("a*").unwrap());
    assert_eq!(equivalent(&even, &any).counterexample.as_deref(), Some("a"));

    let just_a = build(2, 0, &[1], &[(0, 1, "a")]);
    let a_or_b = build(2, 0, &[1], &[(0, 1, "a"), (0, 1, "b")]);
    assert_eq!(
        equivalent(&just_a, &a_or_b).counterexample.as_deref(),
        Some("b")
    );

    let star = build(1, 0, &[0], &[(0, 0, "a")]);
    let plus = build(2, 0, &[1], &[(0, 1, "a"), (1, 1, "a")]);
    assert_eq!(equivalent(&star, &plus).counterexample.as_deref(), Some(""));
}

#[test]
fn automata_without_initial_state_accept_nothing() {
    let mut none = build(1, 0, &[0], &[]);
    none.set_initial(None);
    assert!(equivalent(&none, &Fa::new()).equivalent);
    assert_eq!(
        equivalent(&none, &build(1, 0, &[0], &[]))
            .counterexample
            .as_deref(),
        Some("")
    );
    assert_eq!(nfa_to_dfa(&none).dfa.states().count(), 0);
}

#[test]
fn steps_serialize_with_a_kind_tag() {
    let conv = nfa_to_dfa(&abb_nfa());
    let json = serde_json::to_value(&conv.steps[0]).unwrap();
    assert_eq!(json["kind"], "new_state");
    let back: NfaToDfaStep = serde_json::from_value(json).unwrap();
    assert_eq!(back, conv.steps[0]);
}
