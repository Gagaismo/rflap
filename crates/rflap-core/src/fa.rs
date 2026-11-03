//! Finite automata (JFLAP `automata.fsa`).

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::automaton::{Automaton, Label, Point, StateId, Transition};
use crate::sim::Machine;

/// Finite automaton over `FaLabel`.
pub type Fa = Automaton<FaLabel>;

/// Transition label: a string read from the input; empty is λ. A label of the exact
/// form `[x-y]` reads any single character from `x` to `y`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FaLabel(pub String);

impl From<&str> for FaLabel {
    fn from(s: &str) -> Self {
        FaLabel(s.to_string())
    }
}

impl FaLabel {
    /// The λ label.
    pub fn lambda() -> Self {
        FaLabel(String::new())
    }

    /// The bounds if the label has the exact form `[x-y]`.
    pub fn range(&self) -> Option<(char, char)> {
        let mut chars = self.0.chars();
        match (
            chars.next(),
            chars.next(),
            chars.next(),
            chars.next(),
            chars.next(),
            chars.next(),
        ) {
            (Some('['), Some(lo), Some('-'), Some(hi), Some(']'), None) => Some((lo, hi)),
            _ => None,
        }
    }

    /// The input left after reading this label from `rest`, if it matches.
    fn consume<'a>(&self, rest: &'a str) -> Option<&'a str> {
        match self.range() {
            Some((lo, hi)) => {
                let c = rest.chars().next()?;
                (lo..=hi).contains(&c).then(|| &rest[c.len_utf8()..])
            }
            None => rest.strip_prefix(self.0.as_str()),
        }
    }
}

impl Label for FaLabel {
    fn is_lambda(&self) -> bool {
        self.0.is_empty()
    }

    fn description(&self) -> String {
        if self.0.is_empty() {
            "λ".to_string()
        } else {
            self.0.clone()
        }
    }

    fn conflicts(&self, other: &Self) -> bool {
        if self == other || self.0.is_empty() || other.0.is_empty() {
            return true;
        }
        let first_in = |lit: &FaLabel, (lo, hi): (char, char)| {
            lit.0.chars().next().is_some_and(|c| (lo..=hi).contains(&c))
        };
        match (self.range(), other.range()) {
            (Some((a, b)), Some((c, d))) => a.max(c) <= b.min(d),
            (Some(r), None) => first_in(other, r),
            (None, Some(r)) => first_in(self, r),
            (None, None) => self.0.starts_with(&other.0) || other.0.starts_with(&self.0),
        }
    }
}

/// A simulation configuration: current state and the input not yet read.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FaConfig {
    pub state: StateId,
    pub remaining: String,
}

/// States reachable from `state` through λ transitions (itself first, breadth-first).
pub fn lambda_closure(fa: &Fa, state: StateId) -> Vec<StateId> {
    let mut closure = vec![state];
    let mut i = 0;
    while i < closure.len() {
        for t in fa.transitions_from(closure[i]) {
            if t.label.is_lambda() && !closure.contains(&t.to) {
                closure.push(t.to);
            }
        }
        i += 1;
    }
    closure
}

fn accepting(fa: &Fa, c: &FaConfig) -> bool {
    c.remaining.is_empty() && fa.is_final(c.state)
}

/// Simulator where λ transitions are explicit steps (JFLAP `FSAStepByStateSimulator`).
pub struct FaStepByState<'a> {
    fa: &'a Fa,
}

impl<'a> FaStepByState<'a> {
    pub fn new(fa: &'a Fa) -> Self {
        FaStepByState { fa }
    }
}

impl Machine for FaStepByState<'_> {
    type Config = FaConfig;

    fn initial_configs(&self, input: &str) -> Vec<FaConfig> {
        self.fa
            .initial()
            .map(|state| FaConfig {
                state,
                remaining: input.to_string(),
            })
            .into_iter()
            .collect()
    }

    fn step(&self, c: &FaConfig) -> Vec<FaConfig> {
        self.fa
            .transitions_from(c.state)
            .filter_map(|t| {
                let rest = t.label.consume(&c.remaining)?;
                Some(FaConfig {
                    state: t.to,
                    remaining: rest.to_string(),
                })
            })
            .collect()
    }

    fn is_accepting(&self, c: &FaConfig) -> bool {
        accepting(self.fa, c)
    }
}

/// Simulator whose configurations are λ-closed (JFLAP `FSAStepWithClosureSimulator`).
/// Unlike JFLAP, range transitions also take the closure of their destination.
pub struct FaWithClosure<'a> {
    fa: &'a Fa,
}

impl<'a> FaWithClosure<'a> {
    pub fn new(fa: &'a Fa) -> Self {
        FaWithClosure { fa }
    }
}

impl Machine for FaWithClosure<'_> {
    type Config = FaConfig;

    fn initial_configs(&self, input: &str) -> Vec<FaConfig> {
        let Some(initial) = self.fa.initial() else {
            return Vec::new();
        };
        lambda_closure(self.fa, initial)
            .into_iter()
            .map(|state| FaConfig {
                state,
                remaining: input.to_string(),
            })
            .collect()
    }

    fn step(&self, c: &FaConfig) -> Vec<FaConfig> {
        let mut out = Vec::new();
        for t in self
            .fa
            .transitions_from(c.state)
            .filter(|t| !t.label.is_lambda())
        {
            if let Some(rest) = t.label.consume(&c.remaining) {
                out.extend(
                    lambda_closure(self.fa, t.to)
                        .into_iter()
                        .map(|state| FaConfig {
                            state,
                            remaining: rest.to_string(),
                        }),
                );
            }
        }
        out
    }

    fn is_accepting(&self, c: &FaConfig) -> bool {
        accepting(self.fa, c)
    }
}

fn is_multi_char(label: &FaLabel) -> bool {
    label.range().is_none() && label.0.chars().count() > 1
}

/// Whether some transition reads more than one character (ranges do not count).
pub fn has_multi_char_labels(fa: &Fa) -> bool {
    fa.transitions().iter().any(|t| is_multi_char(&t.label))
}

/// Replaces every multi-character transition by a chain of single-character ones,
/// with intermediate states at interpolated points (JFLAP `FSALabelHandler.handleLabel`).
pub fn split_multi_char_labels(fa: &Fa) -> Fa {
    let mut out = fa.clone();
    for t in fa.transitions().iter().filter(|t| is_multi_char(&t.label)) {
        out.remove_transition(t);
        let (p, q) = (
            fa.state(t.from).map(|s| s.point),
            fa.state(t.to).map(|s| s.point),
        );
        let (Some(p), Some(q)) = (p, q) else { continue };
        let symbols: Vec<char> = t.label.0.chars().collect();
        let n = symbols.len();
        let mut from = t.from;
        for (i, sym) in symbols.iter().enumerate() {
            let to = if i == n - 1 {
                t.to
            } else {
                let (k, n) = (i as f64, n as f64);
                out.create_state(Point {
                    x: (p.x * (n - k - 1.0) + q.x * (k + 1.0)) / n,
                    y: (p.y * (n - k - 1.0) + q.y * (k + 1.0)) / n,
                })
            };
            out.add_transition(Transition::new(from, to, FaLabel(sym.to_string())));
            from = to;
        }
    }
    out
}

/// Distinct non-λ labels in order of first use (JFLAP `FSAAlphabetRetriever`).
pub fn alphabet(fa: &Fa) -> Vec<String> {
    let mut symbols: Vec<String> = Vec::new();
    for t in fa.transitions().iter().filter(|t| !t.label.is_lambda()) {
        if !symbols.contains(&t.label.0) {
            symbols.push(t.label.0.clone());
        }
    }
    symbols
}

/// Adds a state labeled "Trap State" and, from every state (the trap included), a
/// transition to it for each symbol of the read set the state lacks (JFLAP
/// `AddTrapStateController`). Returns the trap state.
pub fn add_trap_state(fa: &mut Fa) -> StateId {
    let points: Vec<Point> = fa.states().map(|s| s.point).collect();
    let count = points.len().max(1) as f64;
    let at = Point {
        x: points.iter().map(|p| p.x).sum::<f64>() / count,
        y: points.iter().map(|p| p.y).fold(0.0, f64::max) + 100.0,
    };
    let trap = fa.create_state(at);
    if let Some(s) = fa.state_mut(trap) {
        s.label = Some("Trap State".to_string());
    }
    let read_set: BTreeSet<String> = alphabet(fa).into_iter().collect();
    let ids: Vec<StateId> = fa.states().map(|s| s.id).collect();
    for id in ids {
        for symbol in &read_set {
            if !fa.transitions_from(id).any(|t| &t.label.0 == symbol) {
                fa.add_transition(Transition::new(id, trap, FaLabel(symbol.clone())));
            }
        }
    }
    trap
}

/// Radius of a drawn state and gap kept between combined automata (JFLAP `CombineAutomaton`).
const STATE_RADIUS: f64 = 20.0;
const GAP: f64 = 20.0;

/// Copy of `a` with the states and transitions of `b` appended below it. Finals, labels
/// and Moore outputs of `b` are copied; the initial state of `b` is not.
pub fn combine<L: Label>(a: &Automaton<L>, b: &Automaton<L>) -> Automaton<L> {
    let mut out = a.clone();
    let bottom = a
        .states()
        .map(|s| s.point.y)
        .fold(None, |m: Option<f64>, y| Some(m.map_or(y, |m| m.max(y))));
    let top = b
        .states()
        .map(|s| s.point.y)
        .fold(None, |m: Option<f64>, y| Some(m.map_or(y, |m| m.min(y))));
    let shift = (bottom.map_or(0.0, |y| y + STATE_RADIUS) - top.map_or(0.0, |y| y - STATE_RADIUS)
        + GAP)
        .trunc();
    let mut ids: HashMap<StateId, StateId> = HashMap::new();
    for s in b.states() {
        let id = out.create_state(Point {
            x: s.point.x,
            y: s.point.y + shift,
        });
        if b.is_final(s.id) {
            out.set_final(id, true);
        }
        if let Some(new) = out.state_mut(id) {
            new.label = s.label.clone();
            new.output = s.output.clone();
        }
        ids.insert(s.id, id);
    }
    for t in b.transitions() {
        let control = t.control.map(|p| Point {
            x: p.x,
            y: p.y + shift,
        });
        out.add_transition(Transition {
            from: ids[&t.from],
            to: ids[&t.to],
            label: t.label.clone(),
            control,
        });
    }
    out
}
