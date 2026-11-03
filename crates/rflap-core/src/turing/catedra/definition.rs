//! The formal definition `⟨Q, Σ_E, Σ_A, q0, F, f⟩` of a machine and its transition table.

use std::collections::BTreeSet;

use serde::Serialize;

use super::{
    CatedraError, DELTA, LEFT_DELIMITER, Model, Notation, RIGHT_DELIMITER, check_supported,
    format_label, show,
};
use crate::automaton::Label;
use crate::turing::{BLANK, ReadSymbol, TuringMachine};

/// A problem of the definition: a transition leaving a final state, nondeterminism, ...
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// `sin_estado_inicial`, `alfabeto_entrada_invalido`, `no_determinista` or
    /// `dominio_incluye_final`.
    pub kind: String,
    pub message: String,
    /// Indices (in the transition list of the document) of the transitions involved.
    pub transitions: Vec<usize>,
}

/// `⟨Q, Σ_E, Σ_A, q0, F, f⟩` of a machine, with the sets spelled out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Definition {
    pub model: Model,
    /// Q, in state order (the order the states were created).
    pub states: Vec<String>,
    /// Σ_E.
    pub input_alphabet: Vec<String>,
    /// Σ_A: `⟨`, `⟩` first (when present), then the other auxiliary symbols, `Δ` last.
    pub auxiliary_alphabet: Vec<String>,
    pub initial: Option<String>,
    pub finals: Vec<String>,
    /// Σ_E was not declared and was inferred from the symbols the machine uses.
    pub input_alphabet_inferred: bool,
    /// No two transitions apply to the same state and symbol.
    pub deterministic: bool,
    pub issues: Vec<Issue>,
    /// The definition as the course writes it, one set per line.
    pub text: String,
}

/// The symbols a transition reads (concrete ones) and writes.
fn symbols(machine: &TuringMachine) -> (BTreeSet<char>, BTreeSet<char>) {
    let mut read = BTreeSet::new();
    let mut written = BTreeSet::new();
    for t in machine.automaton().transitions() {
        for a in t.label.actions() {
            let parsed = ReadSymbol::parse(a.read());
            match &parsed {
                Some(ReadSymbol::Char(c)) | Some(ReadSymbol::Not(c)) => {
                    read.insert(*c);
                }
                Some(ReadSymbol::OneOf { chars, .. }) => read.extend(chars.iter().copied()),
                _ => {}
            }
            let is_variable =
                matches!(&parsed, Some(ReadSymbol::OneOf { var, .. }) if *var == a.write());
            if a.write() != '~' && !is_variable {
                written.insert(a.write());
            }
        }
    }
    (read, written)
}

/// Σ_E and Σ_A (without the implied delimiters or blank) of a machine.
struct Alphabets {
    input: BTreeSet<char>,
    auxiliary: BTreeSet<char>,
    inferred: bool,
}

fn alphabets(machine: &TuringMachine) -> Alphabets {
    let (read, written) = symbols(machine);
    let used: BTreeSet<char> = read.union(&written).copied().collect();
    let special = |c: char| c == BLANK || c == LEFT_DELIMITER || c == RIGHT_DELIMITER;
    match machine.input_alphabet() {
        Some(declared) => {
            let input: BTreeSet<char> = declared.iter().copied().filter(|c| !special(*c)).collect();
            let auxiliary = used
                .iter()
                .copied()
                .filter(|c| !input.contains(c))
                .collect();
            Alphabets {
                input,
                auxiliary,
                inferred: false,
            }
        }
        None => {
            let is_input = |c: char| {
                if special(c) || c.is_uppercase() {
                    false
                } else if c.is_alphanumeric() {
                    true
                } else {
                    // a symbol that is not a letter or digit is auxiliary when the machine writes it
                    !written.contains(&c)
                }
            };
            Alphabets {
                input: used.iter().copied().filter(|c| is_input(*c)).collect(),
                auxiliary: used.iter().copied().filter(|c| !is_input(*c)).collect(),
                inferred: true,
            }
        }
    }
}

/// `⟨`, `⟩`, the other auxiliary symbols, `Δ` last.
fn auxiliary_order(mut aux: BTreeSet<char>, model: Model) -> Vec<char> {
    match model {
        Model::Mt => {
            aux.insert(BLANK);
        }
        Model::All => {
            aux.insert(LEFT_DELIMITER);
            aux.insert(RIGHT_DELIMITER);
        }
    }
    let rank = |c: &char| match *c {
        LEFT_DELIMITER => 0,
        RIGHT_DELIMITER => 1,
        BLANK => 3,
        _ => 2,
    };
    let mut v: Vec<char> = aux.into_iter().collect();
    v.sort_by_key(|c| (rank(c), *c));
    v
}

fn set_text(items: &[String]) -> String {
    format!("{{{}}}", items.join(", "))
}

fn name_of(machine: &TuringMachine, id: u32) -> String {
    machine
        .automaton()
        .state(id)
        .map_or_else(|| format!("q{id}"), |s| s.name.clone())
}

/// `from --(e,s,m)--> to`.
pub(crate) fn transition_text(machine: &TuringMachine, index: usize) -> String {
    let t = &machine.automaton().transitions()[index];
    format!(
        "{} --{}--> {}",
        name_of(machine, t.from),
        format_label(&t.label, Notation::Catedra),
        name_of(machine, t.to)
    )
}

/// The definition of a one-tape machine; `model` defaults to the one the transitions suggest
/// (an ALL when they mention `⟨` or `⟩`).
pub fn definition(
    machine: &TuringMachine,
    model: Option<Model>,
) -> Result<Definition, CatedraError> {
    check_supported(machine)?;
    let model = model.unwrap_or_else(|| Model::detect(machine));
    let a = machine.automaton();
    let alpha = alphabets(machine);
    let states: Vec<String> = a.states().map(|s| s.name.clone()).collect();
    let input: Vec<String> = alpha.input.iter().map(|c| c.to_string()).collect();
    let auxiliary: Vec<String> = auxiliary_order(alpha.auxiliary, model)
        .into_iter()
        .map(|c| show(c).to_string())
        .collect();
    let initial = a.initial().map(|i| name_of(machine, i));
    let finals: Vec<String> = a.finals().iter().map(|f| name_of(machine, *f)).collect();

    let mut issues = Vec::new();
    if initial.is_none() {
        issues.push(Issue {
            kind: "sin_estado_inicial".into(),
            message: "La máquina no tiene estado inicial.".into(),
            transitions: vec![],
        });
    }
    if let Some(declared) = machine.input_alphabet() {
        let bad: Vec<String> = declared
            .iter()
            .filter(|c| **c == BLANK || **c == LEFT_DELIMITER || **c == RIGHT_DELIMITER)
            .map(|c| show(*c).to_string())
            .collect();
        if !bad.is_empty() {
            issues.push(Issue {
                kind: "alfabeto_entrada_invalido".into(),
                message: format!(
                    "Σ_E no puede contener Δ, ⟨ ni ⟩ (contiene {}).",
                    bad.join(", ")
                ),
                transitions: vec![],
            });
        }
    }
    let transitions = a.transitions();
    for (i, t) in transitions.iter().enumerate() {
        for (j, u) in transitions.iter().enumerate().skip(i + 1) {
            if t.from == u.from && t.label.conflicts(&u.label) {
                let (r1, r2) = (
                    super::show_read(t.label.actions()[0].read(), Notation::Catedra),
                    super::show_read(u.label.actions()[0].read(), Notation::Catedra),
                );
                let symbols = if r1 == r2 {
                    format!("el símbolo {r1}")
                } else {
                    format!("los símbolos {r1} y {r2}")
                };
                issues.push(Issue {
                    kind: "no_determinista".into(),
                    message: format!(
                        "Hay dos transiciones para el estado {} y {symbols}: {} y {}.",
                        name_of(machine, t.from),
                        transition_text(machine, i),
                        transition_text(machine, j)
                    ),
                    transitions: vec![i, j],
                });
            }
        }
    }
    for (i, t) in transitions.iter().enumerate() {
        if a.is_final(t.from) {
            issues.push(Issue {
                kind: "dominio_incluye_final".into(),
                message: format!(
                    "El dominio no incluye a F: el estado final {} tiene la transición {}.",
                    name_of(machine, t.from),
                    transition_text(machine, i)
                ),
                transitions: vec![i],
            });
        }
    }
    let deterministic = !issues.iter().any(|i| i.kind == "no_determinista");

    let text = format!(
        "{} = ⟨Q, Σ_E, Σ_A, q0, F, f⟩\nQ = {}\nΣ_E = {}{}\nΣ_A = {}\nq0 = {}\nF = {}\n",
        model.name(),
        set_text(&states),
        set_text(&input),
        if alpha.inferred { " (inferido)" } else { "" },
        set_text(&auxiliary),
        initial.as_deref().unwrap_or("-"),
        set_text(&finals),
    );
    Ok(Definition {
        model,
        states,
        input_alphabet: input,
        auxiliary_alphabet: auxiliary,
        initial,
        finals,
        input_alphabet_inferred: alpha.inferred,
        deterministic,
        issues,
        text,
    })
}

/// The transition table of the course: `Estado Actual | Símbolo Entrada | Estado Nuevo |
/// Símbolo Salida | Movimiento Cabezal`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Table {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

const HEADER: [&str; 5] = [
    "Estado Actual",
    "Símbolo Entrada",
    "Estado Nuevo",
    "Símbolo Salida",
    "Movimiento Cabezal",
];

impl Table {
    /// A GitHub-flavoured Markdown table.
    pub fn to_markdown(&self) -> String {
        let line = |cells: &[String]| format!("| {} |\n", cells.join(" | "));
        let mut out = line(&self.header);
        out.push_str("|---|---|---|---|---|\n");
        for row in &self.rows {
            out.push_str(&line(row));
        }
        out
    }

    /// Aligned plain text, columns separated by two spaces.
    pub fn to_text(&self) -> String {
        let width = |i: usize| {
            std::iter::once(&self.header)
                .chain(&self.rows)
                .map(|r| r[i].chars().count())
                .max()
                .unwrap_or(0)
        };
        let widths: Vec<usize> = (0..5).map(width).collect();
        let line = |cells: &[String]| {
            let padded: Vec<String> = cells
                .iter()
                .enumerate()
                .map(|(i, c)| format!("{c:<w$}", w = widths[i]))
                .collect();
            format!("{}\n", padded.join("  ").trim_end())
        };
        let mut out = line(&self.header);
        for row in &self.rows {
            out.push_str(&line(row));
        }
        out
    }
}

/// The transition table of a one-tape machine: rows grouped by state in state order, symbols
/// ordered input, auxiliary, `⟨`, `⟩`, `Δ`; `→` marks the initial state, `*` the finals, and a
/// final state has one row of dashes (its transitions are not part of `f`).
pub fn transition_table(machine: &TuringMachine) -> Result<Table, CatedraError> {
    check_supported(machine)?;
    let a = machine.automaton();
    let alpha = alphabets(machine);
    let rank = |read: &str| {
        let mut chars = read.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => match c {
                LEFT_DELIMITER => (2, c),
                RIGHT_DELIMITER => (3, c),
                BLANK => (4, c),
                c if alpha.input.contains(&c) => (0, c),
                c => (1, c),
            },
            _ => (1, read.chars().next().unwrap_or('\u{10ffff}')),
        }
    };
    let mut rows = Vec::new();
    for state in a.states() {
        let marker = format!(
            "{}{}{}",
            if a.initial() == Some(state.id) {
                "→"
            } else {
                ""
            },
            if a.is_final(state.id) { "*" } else { "" },
            state.name
        );
        if a.is_final(state.id) {
            rows.push(vec![marker, "-".into(), "-".into(), "-".into(), "-".into()]);
            continue;
        }
        let mut own: Vec<_> = a.transitions_from(state.id).collect();
        own.sort_by_key(|t| rank(t.label.actions()[0].read()));
        for t in own {
            let act = &t.label.actions()[0];
            let label = format_label(&t.label, Notation::Catedra);
            let movement = label
                .trim_end_matches(')')
                .rsplit(',')
                .next()
                .unwrap_or("")
                .to_string();
            rows.push(vec![
                marker.clone(),
                act.read().replace(BLANK, &DELTA.to_string()),
                name_of(machine, t.to),
                show(act.write()).to_string(),
                movement,
            ]);
        }
    }
    Ok(Table {
        header: HEADER.iter().map(|s| s.to_string()).collect(),
        rows,
    })
}
