//! Turing machines and linear bounded automata in the notation of the course "Sintaxis y
//! Semántica de los Lenguajes" (Ing. Jorge Buabud, UTN FRT, CLASE 39 and CLASE 41).
//!
//! The course writes a one-tape machine as `⟨Q, Σ_E, Σ_A, q0, F, f⟩` with `Δ` for the blank,
//! moves `I` (left), `N` (stay) and `D` (right) and transition labels `(e,s,m)`. Everything
//! here is a view over [`TuringMachine`]: labels are still [`TmLabel`]s with the JFLAP blank
//! `□` and moves `L`, `S`, `R`, and a text in either notation reads back to the same label.
//! Multi-tape machines keep the JFLAP form (the course shows them that way), with `Δ`.

mod compose;
mod definition;
mod run;

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{BLANK, Direction, TapeAction, TmError, TmLabel, TuringMachine};

pub use compose::{Multipista, compose, multipista};
pub use definition::{Definition, Issue, Table, definition, transition_table};
pub use run::{AllReport, AllViolation, RunReport, Verdict, all_check, run};

/// The blank of the course.
pub const DELTA: char = 'Δ';
/// Left delimiter of a linear bounded automaton.
pub const LEFT_DELIMITER: char = '⟨';
/// Right delimiter of a linear bounded automaton.
pub const RIGHT_DELIMITER: char = '⟩';

/// How labels are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Notation {
    /// `(e,s,m)` with `Δ`, `I`, `N`, `D` (a tape of a multi-tape label is `e;s,m` with `Δ`).
    Catedra,
    /// `e;s,m` with `□`, `L`, `S`, `R`.
    Jflap,
}

/// What a machine is: a Turing machine or a linear bounded automaton.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    /// Turing machine: the tape is blank (`Δ`) around the input.
    Mt,
    /// Linear bounded automaton: the tape is `⟨w⟩` and the head never leaves it.
    All,
}

impl Model {
    /// `MT` or `ALL`, as the course writes it.
    pub fn name(self) -> &'static str {
        match self {
            Model::Mt => "MT",
            Model::All => "ALL",
        }
    }

    /// `mt` or `all` (any case).
    pub fn parse(s: &str) -> Option<Model> {
        match s.to_lowercase().as_str() {
            "mt" => Some(Model::Mt),
            "all" => Some(Model::All),
            _ => None,
        }
    }

    /// `All` when some transition reads or writes `⟨` or `⟩`, otherwise `Mt`.
    pub fn detect(machine: &TuringMachine) -> Model {
        let is_delimiter = |c: char| c == LEFT_DELIMITER || c == RIGHT_DELIMITER;
        let uses = machine.automaton().transitions().iter().any(|t| {
            t.label
                .actions()
                .iter()
                .any(|a| a.read().chars().any(is_delimiter) || is_delimiter(a.write()))
        });
        if uses { Model::All } else { Model::Mt }
    }
}

/// Why a cátedra operation could not be done.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatedraError {
    /// The notation of the course is for one tape.
    MultiTape { tapes: usize },
    /// The machine has no initial state.
    NoInitialState,
    /// An input the model cannot start from.
    InvalidInput { input: String, reason: String },
    /// Two machines of a different number of tapes cannot be composed.
    TapeMismatch { a: usize, b: usize },
    /// Building blocks (machines inside states) are a JFLAP feature, not the course's.
    HasBlocks,
}

impl fmt::Display for CatedraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatedraError::MultiTape { tapes } => write!(
                f,
                "La notación de la cátedra es para máquinas de una cinta; esta tiene {tapes} cintas."
            ),
            CatedraError::NoInitialState => f.write_str("La máquina no tiene estado inicial."),
            CatedraError::InvalidInput { input, reason } => {
                write!(f, "Entrada \"{input}\" inválida: {reason}.")
            }
            CatedraError::TapeMismatch { a, b } => write!(
                f,
                "No se pueden componer máquinas de {a} y de {b} cintas: deben tener la misma cantidad."
            ),
            CatedraError::HasBlocks => f.write_str(
                "La notación de la cátedra no admite bloques (máquinas dentro de estados).",
            ),
        }
    }
}

impl std::error::Error for CatedraError {}

/// The machine is one-tape and has no building blocks.
pub(crate) fn check_supported(machine: &TuringMachine) -> Result<(), CatedraError> {
    if machine.tapes() != 1 {
        return Err(CatedraError::MultiTape {
            tapes: machine.tapes(),
        });
    }
    if !machine.blocks().is_empty() {
        return Err(CatedraError::HasBlocks);
    }
    Ok(())
}

/// A tape symbol as the course shows it: the blank is `Δ`.
pub(crate) fn show(c: char) -> char {
    if c == BLANK { DELTA } else { c }
}

/// A read pattern as text: the blank is `Δ` in the course notation, `□` in JFLAP's.
fn show_read(read: &str, notation: Notation) -> String {
    match notation {
        Notation::Catedra => read.replace(BLANK, &DELTA.to_string()),
        Notation::Jflap => read.to_string(),
    }
}

fn show_write(c: char, notation: Notation) -> char {
    match notation {
        Notation::Catedra => show(c),
        Notation::Jflap => c,
    }
}

fn move_letter(d: Direction, notation: Notation, single: bool) -> char {
    match (notation, single, d) {
        (Notation::Catedra, true, Direction::Left) => 'I',
        (Notation::Catedra, true, Direction::Stay) => 'N',
        (Notation::Catedra, true, Direction::Right) => 'D',
        _ => d.symbol(),
    }
}

/// One label as text: `(e,s,m)` for a one-tape label in the course notation, otherwise
/// `e;s,m` per tape joined by `|`.
pub fn format_label(label: &TmLabel, notation: Notation) -> String {
    let actions = label.actions();
    let single_cat = notation == Notation::Catedra && actions.len() == 1;
    let one = |a: &TapeAction| {
        let (r, w, m) = (
            show_read(a.read(), notation),
            show_write(a.write(), notation),
            move_letter(a.direction(), notation, single_cat),
        );
        if single_cat {
            format!("({r},{w},{m})")
        } else {
            format!("{r};{w},{m}")
        }
    };
    actions.iter().map(one).collect::<Vec<_>>().join("|")
}

/// The labels of one edge, joined with ` / ` as in `(a,a,D) / (b,b,D)`.
pub fn format_edge(labels: &[&TmLabel], notation: Notation) -> String {
    labels
        .iter()
        .map(|l| format_label(l, notation))
        .collect::<Vec<_>>()
        .join(" / ")
}

/// Splits the text of an edge into its labels (` / `).
pub fn split_labels(text: &str) -> Vec<&str> {
    text.split(" / ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

fn direction_letter(s: &str) -> Option<&'static str> {
    match s.trim() {
        "I" | "L" => Some("L"),
        "N" | "S" => Some("S"),
        "D" | "R" => Some("R"),
        _ => None,
    }
}

/// Reads a symbol token: `Δ` and `_` are the blank (`""` for [`TapeAction::new`]).
fn read_token(t: &str) -> String {
    let t = t.trim();
    if t == "Δ" || t == "_" {
        String::new()
    } else {
        t.replace(DELTA, &BLANK.to_string())
    }
}

fn write_token(t: &str) -> String {
    let t = t.trim();
    if t == "Δ" || t == "_" {
        String::new()
    } else {
        t.to_string()
    }
}

fn action(text: &str, read: &str, write: &str, direction: &str) -> Result<TapeAction, TmError> {
    let direction = direction_letter(direction).ok_or_else(|| TmError::BadLabel(text.into()))?;
    TapeAction::new(&read_token(read), &write_token(write), direction)
        .map_err(|_| TmError::BadLabel(text.into()))
}

/// Reads a label in any of its forms: the course's `(a,X,D)` or `a,X,D`, or JFLAP's `a;X,R`
/// (one per tape joined by `|`). `Δ`, `□` and `_` are the blank; the moves are `I`/`N`/`D`
/// or `L`/`S`/`R`.
pub fn parse_label(text: &str) -> Result<TmLabel, TmError> {
    let bad = || TmError::BadLabel(text.to_string());
    let t = text.trim();
    if t.is_empty() {
        return Err(bad());
    }
    let mut actions = Vec::new();
    if t.contains(';') {
        for part in t.split('|') {
            let (read, rest) = part.split_once(';').ok_or_else(bad)?;
            let (write, direction) = rest.rsplit_once(',').ok_or_else(bad)?;
            actions.push(action(text, read, write, direction)?);
        }
    } else {
        let inner = match (t.strip_prefix('('), t.ends_with(')')) {
            (Some(rest), true) => &rest[..rest.len() - 1],
            (None, false) => t,
            _ => return Err(bad()),
        };
        let parts: Vec<&str> = inner.split(',').collect();
        let [read, write, direction] = parts.as_slice() else {
            return Err(bad());
        };
        actions.push(action(text, read, write, direction)?);
    }
    TmLabel::new(actions)
}
