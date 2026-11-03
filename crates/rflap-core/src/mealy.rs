//! Mealy and Moore machines (JFLAP `automata.mealy`).

use serde::{Deserialize, Serialize};

use crate::automaton::{Automaton, Label, StateId};
use crate::sim::{Machine, RunOutcome, fast_run};

/// Mealy machine: the output is on the transitions.
pub type Mealy = Automaton<MealyLabel>;

/// Moore machine: the output is on the states (`State::output`).
pub type Moore = Automaton<MooreLabel>;

fn describe(s: &str) -> String {
    if s.is_empty() {
        "λ".to_string()
    } else {
        s.to_string()
    }
}

/// JFLAP `MealyNondeterminismDetector`: equal labels or one a prefix of the other.
fn labels_conflict(a: &str, b: &str) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

/// Mealy transition label: the string read and the string written; an empty input is λ.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MealyLabel {
    pub input: String,
    pub output: String,
}

impl MealyLabel {
    pub fn new(input: &str, output: &str) -> Self {
        MealyLabel {
            input: input.to_string(),
            output: output.to_string(),
        }
    }
}

impl Label for MealyLabel {
    fn is_lambda(&self) -> bool {
        self.input.is_empty()
    }

    fn description(&self) -> String {
        format!("{} ; {}", describe(&self.input), describe(&self.output))
    }

    fn conflicts(&self, other: &Self) -> bool {
        labels_conflict(&self.input, &other.input)
    }
}

/// Moore transition label: the string read; the output is the destination's.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MooreLabel(pub String);

impl From<&str> for MooreLabel {
    fn from(s: &str) -> Self {
        MooreLabel(s.to_string())
    }
}

impl Label for MooreLabel {
    fn is_lambda(&self) -> bool {
        self.0.is_empty()
    }

    fn description(&self) -> String {
        describe(&self.0)
    }

    fn conflicts(&self, other: &Self) -> bool {
        labels_conflict(&self.0, &other.0)
    }
}

/// Output of a Moore state; empty when it has none (JFLAP `MooreMachine.getOutput`).
pub fn moore_output(m: &Moore, state: StateId) -> &str {
    m.state(state)
        .and_then(|s| s.output.as_deref())
        .unwrap_or("")
}

/// Sets the output of a Moore state; `None` clears it. Ignored for unknown states.
pub fn set_moore_output(m: &mut Moore, state: StateId, output: Option<&str>) {
    if let Some(s) = m.state_mut(state) {
        s.output = output.map(str::to_string);
    }
}

/// A simulation configuration: current state, input not yet read and output so far.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MealyConfig {
    pub state: StateId,
    pub remaining: String,
    pub output: String,
}

/// Accepting means the whole input was consumed; final states play no role.
fn accepting(c: &MealyConfig) -> bool {
    c.remaining.is_empty()
}

fn step<L, F>(a: &Automaton<L>, c: &MealyConfig, project: F) -> Vec<MealyConfig>
where
    L: Label,
    F: Fn(&crate::automaton::Transition<L>) -> (&str, String),
{
    a.transitions_from(c.state)
        .filter_map(|t| {
            let (read, out) = project(t);
            let rest = c.remaining.strip_prefix(read)?;
            Some(MealyConfig {
                state: t.to,
                remaining: rest.to_string(),
                output: format!("{}{}", c.output, out),
            })
        })
        .collect()
}

/// Step-by-state simulator for Mealy machines (JFLAP `MealyStepByStateSimulator`).
pub struct MealyStepByState<'a> {
    machine: &'a Mealy,
}

impl<'a> MealyStepByState<'a> {
    pub fn new(machine: &'a Mealy) -> Self {
        MealyStepByState { machine }
    }
}

impl Machine for MealyStepByState<'_> {
    type Config = MealyConfig;

    fn initial_configs(&self, input: &str) -> Vec<MealyConfig> {
        self.machine
            .initial()
            .map(|state| MealyConfig {
                state,
                remaining: input.to_string(),
                output: String::new(),
            })
            .into_iter()
            .collect()
    }

    fn step(&self, c: &MealyConfig) -> Vec<MealyConfig> {
        step(self.machine, c, |t| {
            (t.label.input.as_str(), t.label.output.clone())
        })
    }

    fn is_accepting(&self, c: &MealyConfig) -> bool {
        accepting(c)
    }
}

/// Step-by-state simulator for Moore machines (JFLAP `MooreStepByStateSimulator`): the
/// output starts with the initial state's and every transition adds its destination's.
pub struct MooreStepByState<'a> {
    machine: &'a Moore,
}

impl<'a> MooreStepByState<'a> {
    pub fn new(machine: &'a Moore) -> Self {
        MooreStepByState { machine }
    }
}

impl Machine for MooreStepByState<'_> {
    type Config = MealyConfig;

    fn initial_configs(&self, input: &str) -> Vec<MealyConfig> {
        self.machine
            .initial()
            .map(|state| MealyConfig {
                state,
                remaining: input.to_string(),
                output: moore_output(self.machine, state).to_string(),
            })
            .into_iter()
            .collect()
    }

    fn step(&self, c: &MealyConfig) -> Vec<MealyConfig> {
        step(self.machine, c, |t| {
            (
                t.label.0.as_str(),
                moore_output(self.machine, t.to).to_string(),
            )
        })
    }

    fn is_accepting(&self, c: &MealyConfig) -> bool {
        accepting(c)
    }
}

/// Result of one input in a multiple-output run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputRun {
    pub input: String,
    pub outcome: RunOutcome<MealyConfig>,
    /// The output of the accepting configuration found, if any.
    pub output: Option<String>,
}

/// `fast_run` over several inputs keeping the output of each accepted one (JFLAP
/// `MultipleOutputSimulateAction`, "Multiple Run (Transducer)").
pub fn multiple_output_run<M, I, S>(machine: &M, inputs: I, max_configs: usize) -> Vec<OutputRun>
where
    M: Machine<Config = MealyConfig>,
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    inputs
        .into_iter()
        .map(|input| {
            let input = input.as_ref();
            let outcome = fast_run(machine, input, max_configs);
            let output = match &outcome {
                RunOutcome::Accepted { trace } => trace.last().map(|c| c.output.clone()),
                _ => None,
            };
            OutputRun {
                input: input.to_string(),
                outcome,
                output,
            }
        })
        .collect()
}
