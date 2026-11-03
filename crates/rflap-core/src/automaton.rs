//! Generic automaton model (JFLAP `automata.Automaton`, `State`, `Transition`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

/// Identifier of a state inside one automaton.
pub type StateId = u32;

/// Canvas position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A state; `output` is the Moore output and is unused by finite automata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub id: StateId,
    pub name: String,
    pub label: Option<String>,
    pub point: Point,
    pub output: Option<String>,
}

impl State {
    fn new(id: StateId, point: Point) -> Self {
        State {
            id,
            name: format!("q{id}"),
            label: None,
            point,
            output: None,
        }
    }
}

/// A free-text annotation on the canvas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub point: Point,
    pub text: String,
}

/// An edge between two states carrying a machine-specific label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transition<L> {
    pub from: StateId,
    pub to: StateId,
    pub label: L,
    /// Bezier control point used only for drawing.
    pub control: Option<Point>,
}

impl<L> Transition<L> {
    pub fn new(from: StateId, to: StateId, label: L) -> Self {
        Transition {
            from,
            to,
            label,
            control: None,
        }
    }
}

/// Transition label of one kind of machine.
pub trait Label: Clone + Eq + Hash + Debug {
    /// Whether the transition consumes nothing (JFLAP lambda checker).
    fn is_lambda(&self) -> bool;
    /// Text shown to the user (λ for the empty label).
    fn description(&self) -> String;
    /// Whether two transitions leaving the same state with these labels are nondeterministic.
    fn conflicts(&self, other: &Self) -> bool;
}

/// A finite set of states, labelled transitions, one initial state and final states.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Automaton<L: Label> {
    states: BTreeMap<StateId, State>,
    transitions: Vec<Transition<L>>,
    initial: Option<StateId>,
    finals: BTreeSet<StateId>,
    notes: Vec<Note>,
}

impl<L: Label> Default for Automaton<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: Label> Automaton<L> {
    pub fn new() -> Self {
        Automaton {
            states: BTreeMap::new(),
            transitions: Vec::new(),
            initial: None,
            finals: BTreeSet::new(),
            notes: Vec::new(),
        }
    }

    /// Creates a state with the smallest free id.
    pub fn create_state(&mut self, point: Point) -> StateId {
        let id = (0..)
            .find(|id| !self.states.contains_key(id))
            .expect("state ids exhausted");
        self.states.insert(id, State::new(id, point));
        id
    }

    /// Creates a state with a given id; `None` if the id is taken.
    pub fn create_state_with_id(&mut self, id: StateId, point: Point) -> Option<StateId> {
        if self.states.contains_key(&id) {
            return None;
        }
        self.states.insert(id, State::new(id, point));
        Some(id)
    }

    /// Removes a state with its transitions, final mark and initial mark.
    pub fn remove_state(&mut self, id: StateId) {
        if self.states.remove(&id).is_none() {
            return;
        }
        self.transitions.retain(|t| t.from != id && t.to != id);
        self.finals.remove(&id);
        if self.initial == Some(id) {
            self.initial = None;
        }
    }

    pub fn state(&self, id: StateId) -> Option<&State> {
        self.states.get(&id)
    }

    pub fn state_mut(&mut self, id: StateId) -> Option<&mut State> {
        self.states.get_mut(&id)
    }

    /// States sorted by id.
    pub fn states(&self) -> impl Iterator<Item = &State> + '_ {
        self.states.values()
    }

    /// Adds a transition; ignores duplicates (same states and label) and dangling ones.
    pub fn add_transition(&mut self, t: Transition<L>) -> bool {
        if !self.states.contains_key(&t.from)
            || !self.states.contains_key(&t.to)
            || self.transitions.iter().any(|o| same_edge(o, &t))
        {
            return false;
        }
        self.transitions.push(t);
        true
    }

    /// Removes the transition with the same states and label.
    pub fn remove_transition(&mut self, t: &Transition<L>) -> bool {
        match self.transitions.iter().position(|o| same_edge(o, t)) {
            Some(i) => {
                self.transitions.remove(i);
                true
            }
            None => false,
        }
    }

    /// All transitions in insertion order.
    pub fn transitions(&self) -> &[Transition<L>] {
        &self.transitions
    }

    pub fn transitions_from(&self, id: StateId) -> impl Iterator<Item = &Transition<L>> + '_ {
        self.transitions.iter().filter(move |t| t.from == id)
    }

    pub fn transitions_to(&self, id: StateId) -> impl Iterator<Item = &Transition<L>> + '_ {
        self.transitions.iter().filter(move |t| t.to == id)
    }

    /// Sets the initial state; ignored if the state does not exist.
    pub fn set_initial(&mut self, id: Option<StateId>) {
        if id.is_none_or(|id| self.states.contains_key(&id)) {
            self.initial = id;
        }
    }

    pub fn initial(&self) -> Option<StateId> {
        self.initial
    }

    /// Marks or unmarks a state as final; ignored if the state does not exist.
    pub fn set_final(&mut self, id: StateId, is_final: bool) {
        if !self.states.contains_key(&id) {
            return;
        }
        if is_final {
            self.finals.insert(id);
        } else {
            self.finals.remove(&id);
        }
    }

    pub fn is_final(&self, id: StateId) -> bool {
        self.finals.contains(&id)
    }

    pub fn finals(&self) -> &BTreeSet<StateId> {
        &self.finals
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
    }
}

fn same_edge<L: Label>(a: &Transition<L>, b: &Transition<L>) -> bool {
    a.from == b.from && a.to == b.to && a.label == b.label
}
