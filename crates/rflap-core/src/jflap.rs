//! Compatibility with classic JFLAP 7.1 (S8 of `odd/tasks/all-shrink.md`).
//!
//! Every machine Rflap designs must open in JFLAP 7.1 with the same states, initial state,
//! finals, labels and symbols, and run with the same verdicts. [`check_fa`], [`check_pda`] and
//! [`check_turing`] report what breaks that:
//!
//! * an **issue** is a construction JFLAP cannot represent or runs differently (the result is
//!   not `compatible`);
//! * a **note** is something the user must know to use the file in JFLAP, with the file still
//!   opening and running the same (an ALL, a setting JFLAP asks at run time, data the `.jff`
//!   does not store).
//!
//! What was compared against the jar (`JFLAP7_1.JAR`, the harness `HarnessSim.java`): FA, PDA,
//! Mealy, Moore and Turing machines written by Rflap load with the same graph and give the same
//! verdicts (and outputs) as JFLAP's own simulators; Mealy and Moore need no check. Everything
//! reported here is a difference found that way.

use serde::Serialize;

use crate::automaton::{Automaton, Label};
use crate::fa::Fa;
use crate::pda::Pda;
use crate::turing::TuringMachine;
use crate::turing::catedra::Model;

/// The most tapes JFLAP 7.1 opens (`TMTransducer`: "Valid # of tapes 1-5").
pub const MAX_TAPES: usize = 5;

/// One issue or note.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// A stable identifier such as `too_many_tapes` or `all_model`.
    pub code: &'static str,
    pub message: String,
    /// The transition involved as `[from, label, to]`, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<[String; 3]>,
}

impl Finding {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Finding {
            code,
            message: message.into(),
            transition: None,
        }
    }
}

/// The result of a check.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct JflapCheck {
    /// No issue: the file opens and runs the same in JFLAP 7.1 (notes may still apply).
    pub compatible: bool,
    pub issues: Vec<Finding>,
    pub notes: Vec<Finding>,
}

impl JflapCheck {
    /// The result for a document that has nothing JFLAP could object to.
    pub fn clean() -> Self {
        Self::from_parts(Vec::new(), Vec::new())
    }

    fn from_parts(issues: Vec<Finding>, notes: Vec<Finding>) -> Self {
        JflapCheck {
            compatible: issues.is_empty(),
            issues,
            notes,
        }
    }
}

fn name<L: Label>(a: &Automaton<L>, id: u32) -> String {
    a.state(id)
        .map_or_else(|| format!("q{id}"), |s| s.name.clone())
}

/// A finite automaton: a range label `[x-y]` into a state with a λ exit.
///
/// JFLAP's run (`SimulatorFactory` gives `FSAStepWithClosureSimulator`) does not take the λ
/// closure of the destination of a range, so it stops one state early; Rflap does take it.
pub fn check_fa(fa: &Fa) -> JflapCheck {
    let mut issues = Vec::new();
    for t in fa.transitions() {
        let has_lambda_exit = fa.transitions_from(t.to).any(|exit| exit.label.is_lambda());
        if t.label.range().is_some() && has_lambda_exit {
            let (from, to) = (name(fa, t.from), name(fa, t.to));
            let mut issue = Finding::new(
                "range_before_lambda",
                format!(
                    "the range {} from {from} to {to}: {to} has a λ transition out, and JFLAP's \
                     Run (closure simulator) does not follow it after a range, so it rejects \
                     words Rflap accepts; use a single-character label, or remove the λ exit",
                    t.label.0
                ),
            );
            issue.transition = Some([from, t.label.0.clone(), to]);
            issues.push(issue);
        }
    }
    JflapCheck::from_parts(issues, Vec::new())
}

/// A pushdown automaton: JFLAP asks "Accept by" when it runs one, the file does not say.
pub fn check_pda(pda: &Pda) -> JflapCheck {
    let mut notes = Vec::new();
    if pda.finals().is_empty() && pda.states().next().is_some() {
        notes.push(Finding::new(
            "accepts_by_empty_stack",
            "this PDA has no final state: in JFLAP choose \"Empty Stack\" in the \"Accept by\" \
             dialog when you run it (the .jff does not store how a PDA accepts)",
        ));
    }
    JflapCheck::from_parts(Vec::new(), notes)
}

/// A Turing machine: the number of tapes, the ALL model, and what the `.jff` does not store.
pub fn check_turing(machine: &TuringMachine) -> JflapCheck {
    let (mut issues, mut notes) = (Vec::new(), Vec::new());
    if machine.tapes() > MAX_TAPES {
        issues.push(Finding::new(
            "too_many_tapes",
            format!(
                "the machine has {} tapes and JFLAP 7.1 only opens machines with 1 to {MAX_TAPES} \
                 tapes (it refuses the file)",
                machine.tapes()
            ),
        ));
    }
    if Model::detect(machine) == Model::All {
        notes.push(Finding::new("all_model", ALL_NOTE));
    }
    if machine.input_alphabet().is_some() {
        notes.push(Finding::new(
            "input_alphabet_not_saved",
            "the declared input alphabet Σ_E is not stored in the .jff: JFLAP does not have it, \
             and Rflap infers it again when the file is reopened",
        ));
    }
    let automaton = machine.automaton();
    if automaton.finals().is_empty() && automaton.states().next().is_some() {
        notes.push(Finding::new(
            "accepts_by_halting",
            "this machine has no final state: JFLAP accepts by final state unless you tick \
             \"Accept by Halting\" in its run dialog, so tick it (the .jff does not store it)",
        ));
    }
    JflapCheck::from_parts(issues, notes)
}

/// How to run an ALL in JFLAP (JFLAP has no ALL model).
pub const ALL_NOTE: &str = "Este ALL abre igual en JFLAP 7.1 (mismos estados, inicial, finales \
y etiquetas), pero JFLAP no tiene el modelo ALL: ⟨ y ⟩ son símbolos comunes que no agrega, el \
cabezal arranca sobre el primer símbolo que escribas y la cinta no está acotada. Para correrlo \
en JFLAP: (1) agregá un estado inicial nuevo con una transición (⟨,⟨,D) \
(en JFLAP ⟨;⟨,R) hacia el estado inicial actual, (2) escribí la entrada con los delimitadores, \
⟨w⟩, y (3) corré con Input → Run o Step (acepta por estado final). La cinta ⟨w⟩ con el cabezal \
sobre el primer símbolo de w y la cinta que se achica solo valen en Rflap.";
