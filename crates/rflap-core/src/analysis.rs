//! Structural analyses shared by all machines (JFLAP `NondeterminismDetector`,
//! `UnreachableStatesDetector`, `UselessStatesDetector`).

use std::collections::BTreeSet;
use std::fmt;

use crate::automaton::{Automaton, Label, StateId, Transition};

/// The analysis needs an initial state and the automaton has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoInitialState;

impl fmt::Display for NoInitialState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("automaton does not have an initial state")
    }
}

impl std::error::Error for NoInitialState {}

/// States sorted by id that are nondeterministic: they have a λ transition or two
/// transitions whose labels conflict.
pub fn nondeterministic_states<L: Label>(a: &Automaton<L>) -> Vec<StateId> {
    a.states()
        .map(|s| s.id)
        .filter(|&id| {
            let out: Vec<_> = a.transitions_from(id).collect();
            out.iter().enumerate().any(|(j, t)| {
                t.label.is_lambda() || out[j + 1..].iter().any(|u| t.label.conflicts(&u.label))
            })
        })
        .collect()
}

/// The λ transitions in insertion order.
pub fn lambda_transitions<L: Label>(a: &Automaton<L>) -> Vec<&Transition<L>> {
    a.transitions()
        .iter()
        .filter(|t| t.label.is_lambda())
        .collect()
}

/// States not reachable from the initial state (all states when there is none).
pub fn unreachable_states<L: Label>(a: &Automaton<L>) -> BTreeSet<StateId> {
    let reached = a
        .initial()
        .map(|i| reachable_from(a, i))
        .unwrap_or_default();
    a.states()
        .map(|s| s.id)
        .filter(|id| !reached.contains(id))
        .collect()
}

/// States that are unreachable or from which no final state can be reached.
pub fn useless_states<L: Label>(a: &Automaton<L>) -> Result<BTreeSet<StateId>, NoInitialState> {
    let initial = a.initial().ok_or(NoInitialState)?;
    let from_initial = reachable_from(a, initial);
    let mut to_final: BTreeSet<StateId> = a.finals().clone();
    loop {
        let before = to_final.len();
        for t in a.transitions() {
            if to_final.contains(&t.to) {
                to_final.insert(t.from);
            }
        }
        if to_final.len() == before {
            break;
        }
    }
    Ok(a.states()
        .map(|s| s.id)
        .filter(|id| !(from_initial.contains(id) && to_final.contains(id)))
        .collect())
}

/// Copy without useless states. The initial state is always kept; if it is useless
/// the language is empty and every transition is dropped.
pub fn clean_useless<L: Label>(a: &Automaton<L>) -> Result<Automaton<L>, NoInitialState> {
    let useless = useless_states(a)?;
    let mut clean = a.clone();
    for &id in &useless {
        if Some(id) != clean.initial() {
            clean.remove_state(id);
        }
    }
    if clean.initial().is_some_and(|i| useless.contains(&i)) {
        for t in clean.transitions().to_vec() {
            clean.remove_transition(&t);
        }
    }
    Ok(clean)
}

fn reachable_from<L: Label>(a: &Automaton<L>, start: StateId) -> BTreeSet<StateId> {
    let mut seen = BTreeSet::from([start]);
    let mut stack = vec![start];
    while let Some(id) = stack.pop() {
        for t in a.transitions_from(id) {
            if seen.insert(t.to) {
                stack.push(t.to);
            }
        }
    }
    seen
}
