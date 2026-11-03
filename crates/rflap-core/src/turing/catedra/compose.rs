//! Composition of Turing machines (CLASE 41) and multipista machines.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::CatedraError;
use crate::automaton::{Point, StateId, Transition};
use crate::turing::TuringMachine;

/// Gap between the two machines of a composition.
const GAP: f64 = 120.0;

/// `MT3 = ⟨Q1∪Q2, Σ_E, Σ_A, q01, F2, f3⟩`: `f3` follows `f1` in `Q1`, except that a transition
/// of `f1` that goes to a state of `F1` goes to `q02` instead, and follows `f2` in `Q2`. A state
/// of `MT2` whose name clashes with one already used gets primes (`q0'`). `MT2` is laid out to
/// the right of `MT1`.
pub fn compose(a: &TuringMachine, b: &TuringMachine) -> Result<TuringMachine, CatedraError> {
    if a.tapes() != b.tapes() {
        return Err(CatedraError::TapeMismatch {
            a: a.tapes(),
            b: b.tapes(),
        });
    }
    if !a.blocks().is_empty() || !b.blocks().is_empty() {
        return Err(CatedraError::HasBlocks);
    }
    let (aa, ba) = (a.automaton(), b.automaton());
    let q01 = aa.initial().ok_or(CatedraError::NoInitialState)?;
    let q02 = ba.initial().ok_or(CatedraError::NoInitialState)?;

    let mut out = TuringMachine::new(a.tapes());
    let mut taken: BTreeSet<String> = BTreeSet::new();
    for s in aa.states() {
        out.create_state_with_id(s.id, s.point);
        if let Some(n) = out.state_mut(s.id) {
            *n = s.clone();
        }
        taken.insert(s.name.clone());
    }
    let right_edge = aa.states().map(|s| s.point.x).fold(f64::MIN, f64::max);
    let left_edge = ba.states().map(|s| s.point.x).fold(f64::MAX, f64::min);
    let dx = if aa.states().next().is_some() && ba.states().next().is_some() {
        right_edge + GAP - left_edge
    } else {
        0.0
    };
    let shift = |p: Point| Point {
        x: p.x + dx,
        y: p.y,
    };
    let mut renamed: BTreeMap<StateId, StateId> = BTreeMap::new();
    for s in ba.states() {
        let id = out.create_state(shift(s.point));
        let mut name = s.name.clone();
        while taken.contains(&name) {
            name.push('\'');
        }
        taken.insert(name.clone());
        if let Some(n) = out.state_mut(id) {
            n.name = name;
            n.label = s.label.clone();
            n.output = s.output.clone();
        }
        renamed.insert(s.id, id);
    }
    for t in aa.transitions() {
        let to = if aa.is_final(t.to) {
            renamed[&q02]
        } else {
            t.to
        };
        let mut copy = Transition::new(t.from, to, t.label.clone());
        copy.control = t.control;
        out.automaton_mut().add_transition(copy);
    }
    for t in ba.transitions() {
        let mut copy = Transition::new(renamed[&t.from], renamed[&t.to], t.label.clone());
        copy.control = t.control.map(shift);
        out.automaton_mut().add_transition(copy);
    }
    out.set_initial(Some(q01));
    for f in ba.finals() {
        out.set_final(renamed[f], true);
    }
    if a.input_alphabet().is_some() || b.input_alphabet().is_some() {
        let union: BTreeSet<char> = a
            .input_alphabet()
            .into_iter()
            .chain(b.input_alphabet())
            .flatten()
            .copied()
            .collect();
        out.set_input_alphabet(Some(union));
    }
    Ok(out)
}

/// Whether a multi-tape machine is multipista: "los cabezales están restringidos a realizar
/// todos juntos el mismo movimiento".
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Multipista {
    pub tapes: usize,
    /// The question makes sense: the machine has more than one tape.
    pub applicable: bool,
    /// Every transition moves all the heads the same way (true for one tape).
    pub is_multipista: bool,
    /// Indices of the transitions where the heads move differently.
    pub offending: Vec<usize>,
}

/// Reports whether every transition moves all the heads the same way.
pub fn multipista(machine: &TuringMachine) -> Multipista {
    let offending: Vec<usize> = machine
        .automaton()
        .transitions()
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            t.label
                .actions()
                .windows(2)
                .any(|w| w[0].direction() != w[1].direction())
        })
        .map(|(i, _)| i)
        .collect();
    Multipista {
        tapes: machine.tapes(),
        applicable: machine.tapes() > 1,
        is_multipista: offending.is_empty(),
        offending,
    }
}
