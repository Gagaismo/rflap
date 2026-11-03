//! Pushdown automata (JFLAP `automata.pda`).

use serde::{Deserialize, Serialize};

use crate::automaton::{Automaton, Label, StateId};
use crate::sim::Machine;

/// Pushdown automaton over `PdaLabel`.
pub type Pda = Automaton<PdaLabel>;

/// Symbol on the stack at the start of every run (JFLAP pushes `"Z"`).
pub const INITIAL_STACK: &str = "Z";

/// Transition label `input, pop; push`; an empty string is λ. Pop and push may hold
/// several symbols; the first symbol of each is the top of the stack.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PdaLabel {
    pub input: String,
    pub pop: String,
    pub push: String,
}

impl PdaLabel {
    pub fn new(input: &str, pop: &str, push: &str) -> Self {
        PdaLabel {
            input: input.to_string(),
            pop: pop.to_string(),
            push: push.to_string(),
        }
    }

    /// The label that reads, pops and pushes nothing.
    pub fn lambda() -> Self {
        PdaLabel::new("", "", "")
    }
}

fn or_lambda(s: &str) -> &str {
    if s.is_empty() { "λ" } else { s }
}

fn prefixes_of_each_other(a: &str, b: &str) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

impl Label for PdaLabel {
    /// Only a label with all three parts empty (JFLAP `PDALambdaTransitionChecker`).
    fn is_lambda(&self) -> bool {
        self.input.is_empty() && self.pop.is_empty() && self.push.is_empty()
    }

    /// `input , pop ; push` with λ for empty parts, as JFLAP prints it.
    fn description(&self) -> String {
        format!(
            "{} , {} ; {}",
            or_lambda(&self.input),
            or_lambda(&self.pop),
            or_lambda(&self.push)
        )
    }

    /// Input and pop are prefixes of each other (JFLAP `PDANondeterminismDetector`).
    fn conflicts(&self, other: &Self) -> bool {
        prefixes_of_each_other(&self.input, &other.input)
            && prefixes_of_each_other(&self.pop, &other.pop)
    }
}

/// What makes a configuration accept (JFLAP asks the user: "Final State" / "Empty Stack").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acceptance {
    /// All input read and the state is final.
    FinalState,
    /// All input read and the stack is empty.
    EmptyStack,
}

/// A simulation configuration: state, input not yet read and the stack, top first.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PdaConfig {
    pub state: StateId,
    pub remaining: String,
    pub stack: String,
}

/// States reachable from `state` through pure λ transitions (itself first, breadth-first).
pub fn lambda_closure(pda: &Pda, state: StateId) -> Vec<StateId> {
    let mut closure = vec![state];
    let mut i = 0;
    while i < closure.len() {
        for t in pda.transitions_from(closure[i]) {
            if t.label.is_lambda() && !closure.contains(&t.to) {
                closure.push(t.to);
            }
        }
        i += 1;
    }
    closure
}

fn initial_stack_config(state: StateId, input: &str) -> PdaConfig {
    PdaConfig {
        state,
        remaining: input.to_string(),
        stack: INITIAL_STACK.to_string(),
    }
}

/// Successors of `c` through every applicable transition, before any closure.
fn apply(pda: &Pda, c: &PdaConfig) -> Vec<(StateId, PdaConfig)> {
    pda.transitions_from(c.state)
        .filter_map(|t| {
            let remaining = c.remaining.strip_prefix(t.label.input.as_str())?;
            let below = c.stack.strip_prefix(t.label.pop.as_str())?;
            Some((
                t.to,
                PdaConfig {
                    state: t.to,
                    remaining: remaining.to_string(),
                    stack: format!("{}{}", t.label.push, below),
                },
            ))
        })
        .collect()
}

fn accepting(pda: &Pda, mode: Acceptance, c: &PdaConfig) -> bool {
    c.remaining.is_empty()
        && match mode {
            Acceptance::FinalState => pda.is_final(c.state),
            Acceptance::EmptyStack => c.stack.is_empty(),
        }
}

/// Simulator where λ transitions are explicit steps (JFLAP `PDAStepByStateSimulator`).
pub struct PdaStepByState<'a> {
    pda: &'a Pda,
    acceptance: Acceptance,
}

impl<'a> PdaStepByState<'a> {
    pub fn new(pda: &'a Pda, acceptance: Acceptance) -> Self {
        PdaStepByState { pda, acceptance }
    }
}

impl Machine for PdaStepByState<'_> {
    type Config = PdaConfig;

    fn initial_configs(&self, input: &str) -> Vec<PdaConfig> {
        self.pda
            .initial()
            .map(|state| initial_stack_config(state, input))
            .into_iter()
            .collect()
    }

    fn step(&self, c: &PdaConfig) -> Vec<PdaConfig> {
        apply(self.pda, c).into_iter().map(|(_, c)| c).collect()
    }

    fn is_accepting(&self, c: &PdaConfig) -> bool {
        accepting(self.pda, self.acceptance, c)
    }
}

/// Simulator whose configurations are λ-closed (JFLAP `PDAStepWithClosureSimulator`).
/// The closure ignores the stack: it follows only transitions that read, pop and push
/// nothing.
pub struct PdaWithClosure<'a> {
    pda: &'a Pda,
    acceptance: Acceptance,
}

impl<'a> PdaWithClosure<'a> {
    pub fn new(pda: &'a Pda, acceptance: Acceptance) -> Self {
        PdaWithClosure { pda, acceptance }
    }
}

impl Machine for PdaWithClosure<'_> {
    type Config = PdaConfig;

    fn initial_configs(&self, input: &str) -> Vec<PdaConfig> {
        let Some(initial) = self.pda.initial() else {
            return Vec::new();
        };
        lambda_closure(self.pda, initial)
            .into_iter()
            .map(|state| initial_stack_config(state, input))
            .collect()
    }

    fn step(&self, c: &PdaConfig) -> Vec<PdaConfig> {
        let mut out = Vec::new();
        for (to, next) in apply(self.pda, c) {
            out.extend(
                lambda_closure(self.pda, to)
                    .into_iter()
                    .map(|state| PdaConfig {
                        state,
                        ..next.clone()
                    }),
            );
        }
        out
    }

    fn is_accepting(&self, c: &PdaConfig) -> bool {
        accepting(self.pda, self.acceptance, c)
    }
}
