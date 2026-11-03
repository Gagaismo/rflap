//! Runs in the course's configuration notation, and the ALL check.

use std::collections::BTreeMap;

use serde::Serialize;

use super::definition::transition_text;
use super::{
    CatedraError, DELTA, LEFT_DELIMITER, Model, RIGHT_DELIMITER, check_supported, show,
    with_delimiter_aliases,
};
use crate::automaton::StateId;
use crate::turing::{BLANK, Direction, ReadSymbol, TuringMachine, label_matches};

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// A final state was reached: "final/acepta".
    Accepted,
    /// No transition applies: "bloqueo/rechazo".
    Rejected,
    /// The step budget ran out: "no se detiene".
    NoHalt,
    /// An ALL left its tape or overwrote a delimiter.
    Violation,
}

/// A rule of the ALL the machine breaks: statically (a transition) or dynamically (a step of a
/// run).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AllViolation {
    /// Static: `delimitador_izquierdo`, `delimitador_derecho`, `usa_blanco`,
    /// `escribe_delimitador`. Dynamic: `sobrescribe_delimitador`, `escribe_delimitador`,
    /// `sale_por_izquierda`, `sale_por_derecha`.
    pub kind: String,
    /// Dynamic only: the step (1-based) that breaks the rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<usize>,
    /// Dynamic only: the input of the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    /// Index of the transition in the transition list of the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<usize>,
    /// `from --(e,s,m)--> to`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_text: Option<String>,
    pub message: String,
}

/// A run: the configuration sequence `c0 ├─ c1 ├─ ... verdict`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RunReport {
    pub input: String,
    pub model: Model,
    pub verdict: Verdict,
    /// Transitions applied.
    pub steps: usize,
    pub configurations: Vec<String>,
    /// The configurations joined with " ├─ " followed by the verdict.
    pub sequence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violation: Option<AllViolation>,
    /// Informational remarks, never violations: the tape shrinks, a final state halts, the
    /// input brought its own delimiters.
    pub notes: Vec<String>,
}

/// A sparse tape: every cell not written is blank.
struct Cells {
    cells: BTreeMap<i64, char>,
    head: i64,
}

impl Cells {
    fn at(&self, pos: i64) -> char {
        self.cells.get(&pos).copied().unwrap_or(BLANK)
    }
}

fn is_delimiter(c: char) -> bool {
    c == LEFT_DELIMITER || c == RIGHT_DELIMITER
}

/// The configuration `c1…ci-1 qk ci…cn` over the window `lo..=hi`.
fn render(state: &str, tape: &Cells, lo: i64, hi: i64) -> String {
    let mut out = String::new();
    for pos in lo..=hi {
        if pos == tape.head {
            out.push_str(state);
        }
        out.push(show(tape.at(pos)));
    }
    out
}

/// The first transition from `state` that reads `symbol`: those with a `!` read go last, like
/// the JFLAP simulator does.
fn applicable(machine: &TuringMachine, state: StateId, symbol: char) -> Option<usize> {
    let transitions = machine.automaton().transitions();
    let mut candidates: Vec<usize> = (0..transitions.len())
        .filter(|&i| transitions[i].from == state)
        .collect();
    candidates.sort_by_key(|&i| transitions[i].label.actions()[0].read().starts_with('!'));
    candidates
        .into_iter()
        .find(|&i| label_matches(&transitions[i].label, &[symbol]))
}

fn name_of(machine: &TuringMachine, id: StateId) -> String {
    machine
        .automaton()
        .state(id)
        .map_or_else(|| format!("q{id}"), |s| s.name.clone())
}

fn check_input(input: &str, model: Model) -> Result<Vec<char>, CatedraError> {
    let chars: Vec<char> = input
        .chars()
        .map(|c| if c == DELTA { BLANK } else { c })
        .collect();
    if model == Model::All && chars.iter().any(|&c| is_delimiter(c) || c == BLANK) {
        return Err(CatedraError::InvalidInput {
            input: input.to_string(),
            reason: "la palabra de entrada de un ALL no puede contener ⟨, ⟩ ni Δ".into(),
        });
    }
    Ok(chars)
}

/// Runs a one-tape machine deterministically (the first applicable transition) and returns the
/// configuration sequence. Entering a final state halts ("final/acepta"); no transition for the
/// symbol read is "bloqueo/rechazo"; `max_steps` transitions without halting is "no se
/// detiene". The `Mt` model starts on a tape of `Δ` with the head on the first symbol and
/// shows, in each configuration, the cells from one before the word to one after it plus every
/// cell the head has visited. The `All` model puts `⟨w⟩` on the tape, starts on the first symbol
/// (on `⟩` for λ), shows the whole tape and stops with a violation when the head leaves `⟨…⟩`
/// or a delimiter is overwritten; there `<` and `>` count as `⟨` and `⟩` when the machine uses
/// both and neither `⟨` nor `⟩`. Writing `⟨` over another symbol moving right (or `⟩` moving left)
/// shrinks the tape and is valid; see [`run_with`] for the strict rule.
pub fn run(
    machine: &TuringMachine,
    input: &str,
    model: Model,
    max_steps: usize,
) -> Result<RunReport, CatedraError> {
    run_with(machine, input, model, max_steps, true)
}

/// Writing `⟨` over another symbol moving right, or `⟩` moving left, consumes a cell of the
/// tape: "la cinta se achica".
fn is_shrink(written: char, direction: Direction) -> bool {
    (written == LEFT_DELIMITER && direction == Direction::Right)
        || (written == RIGHT_DELIMITER && direction == Direction::Left)
}

/// Reading a delimiter, rewriting it and staying in place, into a final state: the machine
/// halts there, so it never has to move.
fn is_halting_stay(read: char, written: char, direction: Direction, to_final: bool) -> bool {
    to_final && direction == Direction::Stay && is_delimiter(read) && written == read
}

/// The word of an input that already brings its delimiters (`⟨w⟩` or `<w>`).
fn trim_delimiters(input: &str) -> Option<String> {
    let chars: Vec<char> = input.chars().collect();
    let (first, last) = (*chars.first()?, *chars.last()?);
    if chars.len() >= 2
        && (first == LEFT_DELIMITER || first == '<')
        && (last == RIGHT_DELIMITER || last == '>')
    {
        Some(chars[1..chars.len() - 1].iter().collect())
    } else {
        None
    }
}

fn shrink_note(machine: &TuringMachine, from: StateId, written: char, read: &str) -> String {
    format!(
        "{} escribe {written} sobre {read}: la cinta se achica",
        name_of(machine, from)
    )
}

fn halting_note(machine: &TuringMachine, from: StateId, read: char, to: StateId) -> String {
    format!(
        "{} lee {read} y escribe {read} sin moverse (N) hacia {}: los estados finales son de parada",
        name_of(machine, from),
        name_of(machine, to)
    )
}

fn add_note(notes: &mut Vec<String>, note: String) {
    if !notes.contains(&note) {
        notes.push(note);
    }
}

/// [`run`] with `allow_shrink` chosen: in an `All` run, writing `⟨` over another symbol moving
/// right (or `⟩` moving left) is valid and reported as a note when it is `true`, and a violation
/// when it is `false`. An `All` input that already brings its delimiters is run without them
/// and says so in a note.
pub fn run_with(
    machine: &TuringMachine,
    input: &str,
    model: Model,
    max_steps: usize,
    allow_shrink: bool,
) -> Result<RunReport, CatedraError> {
    check_supported(machine)?;
    let aliased;
    let machine = if model == Model::All {
        aliased = with_delimiter_aliases(machine);
        &aliased
    } else {
        machine
    };
    let initial = machine
        .automaton()
        .initial()
        .ok_or(CatedraError::NoInitialState)?;
    let mut notes = Vec::new();
    let trimmed = if model == Model::All {
        trim_delimiters(input)
    } else {
        None
    };
    if let Some(inner) = &trimmed {
        notes.push(format!(
            "La entrada «{input}» ya traía los delimitadores; se quitaron y se corre ⟨{inner}⟩."
        ));
    }
    let word = check_input(trimmed.as_deref().unwrap_or(input), model)?;
    let n = word.len() as i64;
    let mut tape = Cells {
        cells: BTreeMap::new(),
        head: 0,
    };
    let (mut lo, mut hi);
    match model {
        Model::Mt => {
            for (i, c) in word.iter().enumerate() {
                tape.cells.insert(i as i64, *c);
            }
            (lo, hi) = (-1, n);
        }
        Model::All => {
            tape.cells.insert(0, LEFT_DELIMITER);
            for (i, c) in word.iter().enumerate() {
                tape.cells.insert(i as i64 + 1, *c);
            }
            tape.cells.insert(n + 1, RIGHT_DELIMITER);
            tape.head = 1;
            (lo, hi) = (0, n + 1);
        }
    }
    lo = lo.min(tape.head);
    hi = hi.max(tape.head);

    let mut state = initial;
    let mut variables: BTreeMap<char, char> = BTreeMap::new();
    let mut configurations = vec![render(&name_of(machine, state), &tape, lo, hi)];
    let mut steps = 0;
    let mut violation = None;
    let verdict = loop {
        if machine.automaton().is_final(state) {
            break Verdict::Accepted;
        }
        if steps >= max_steps {
            break Verdict::NoHalt;
        }
        let read = tape.at(tape.head);
        let Some(index) = applicable(machine, state, read) else {
            break Verdict::Rejected;
        };
        let t = &machine.automaton().transitions()[index];
        let act = &t.label.actions()[0];
        if let Some(ReadSymbol::OneOf { var, .. }) = ReadSymbol::parse(act.read()) {
            variables.insert(var, read);
        }
        let written = if act.write() == '~' {
            read
        } else {
            variables.get(&act.write()).copied().unwrap_or(act.write())
        };
        let next_head = tape.head
            + match act.direction() {
                Direction::Left => -1,
                Direction::Stay => 0,
                Direction::Right => 1,
            };
        if model == Model::All {
            let step = steps + 1;
            let found = if is_delimiter(read) && written != read {
                Some((
                    "sobrescribe_delimitador",
                    format!("escribe {} sobre el delimitador {read}", show(written)),
                ))
            } else if !is_delimiter(read) && is_delimiter(written) {
                if allow_shrink && is_shrink(written, act.direction()) {
                    let note = shrink_note(machine, state, written, &show(read).to_string());
                    add_note(&mut notes, note);
                    None
                } else if allow_shrink {
                    Some((
                        "escribe_delimitador",
                        format!(
                            "escribe {written} sobre {} moviéndose a {}: {}",
                            show(read),
                            direction_name(act.direction()),
                            shrink_rule(written)
                        ),
                    ))
                } else {
                    Some((
                        "escribe_delimitador",
                        format!(
                            "escribe {written} sobre {}: ⟨ y ⟩ solo se escriben sobre sí mismos",
                            show(read)
                        ),
                    ))
                }
            } else if next_head < 0
                || (read == LEFT_DELIMITER && act.direction() == Direction::Left)
            {
                Some((
                    "sale_por_izquierda",
                    "mueve el cabezal a la izquierda de ⟨".to_string(),
                ))
            } else if next_head > n + 1
                || (read == RIGHT_DELIMITER && act.direction() == Direction::Right)
            {
                Some((
                    "sale_por_derecha",
                    "mueve el cabezal a la derecha de ⟩".to_string(),
                ))
            } else {
                None
            };
            if found.is_none()
                && is_halting_stay(
                    read,
                    written,
                    act.direction(),
                    machine.automaton().is_final(t.to),
                )
            {
                add_note(&mut notes, halting_note(machine, state, read, t.to));
            }
            if let Some((kind, detail)) = found {
                let text = transition_text(machine, index);
                violation = Some(AllViolation {
                    kind: kind.into(),
                    step: Some(step),
                    input: Some(input.to_string()),
                    transition: Some(index),
                    message: format!(
                        "Paso {step} con la entrada \"{input}\": la transición {text} {detail}."
                    ),
                    transition_text: Some(text),
                });
                break Verdict::Violation;
            }
        }
        tape.cells.insert(tape.head, written);
        tape.head = next_head;
        state = t.to;
        steps += 1;
        lo = lo.min(tape.head);
        hi = hi.max(tape.head);
        configurations.push(render(&name_of(machine, state), &tape, lo, hi));
    };
    let ending = match verdict {
        Verdict::Accepted => " final/acepta".to_string(),
        Verdict::Rejected => match model {
            Model::Mt => " bloqueo/rechazo".to_string(),
            Model::All => " bloqueo/rechaza".to_string(),
        },
        Verdict::NoHalt => format!(" no se detiene (límite de {max_steps} pasos)"),
        Verdict::Violation => format!(
            " violación del ALL: {}",
            violation.as_ref().map_or("", |v| v.message.as_str())
        ),
    };
    Ok(RunReport {
        input: input.to_string(),
        model,
        verdict,
        steps,
        sequence: format!("{}{ending}", configurations.join(" ├─ ")),
        configurations,
        violation,
        notes,
    })
}

fn direction_name(d: Direction) -> &'static str {
    match d {
        Direction::Left => "la izquierda (I)",
        Direction::Stay => "N",
        Direction::Right => "la derecha (D)",
    }
}

fn shrink_rule(written: char) -> &'static str {
    if written == LEFT_DELIMITER {
        "⟨ solo se escribe sobre otro símbolo moviéndose a la derecha (D)"
    } else {
        "⟩ solo se escribe sobre otro símbolo moviéndose a la izquierda (I)"
    }
}

/// The result of checking that a machine is an ALL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AllReport {
    /// No violation, static or dynamic.
    pub ok: bool,
    /// Static remarks, one per shrinking transition or halting delimiter, without repeats.
    pub notes: Vec<String>,
    /// The static violations (in transition order), then the dynamic ones (in input order).
    pub violations: Vec<AllViolation>,
    /// One run per input, as the `All` model.
    pub runs: Vec<RunReport>,
}

fn static_violation(machine: &TuringMachine, index: usize, kind: &str, rule: &str) -> AllViolation {
    let text = transition_text(machine, index);
    AllViolation {
        kind: kind.into(),
        step: None,
        input: None,
        transition: Some(index),
        message: format!("La transición {text} {rule}"),
        transition_text: Some(text),
    }
}

/// [`all_check_with`] with the tape allowed to shrink.
pub fn all_check(
    machine: &TuringMachine,
    inputs: &[String],
    max_steps: usize,
) -> Result<AllReport, CatedraError> {
    all_check_with(machine, inputs, max_steps, true)
}

/// Checks that a machine follows the rules of an ALL. Statically, every transition that reads
/// `⟨` writes `⟨` and moves right (D) (or stays, into a final state, which halts), every one
/// that reads `⟩` writes `⟩` and moves left (I) (or stays into a final state), `Δ` is neither
/// read nor written and `⟨`/`⟩` are written only over themselves, except that with
/// `allow_shrink` writing `⟨` over another symbol moving right (or `⟩` moving left) is valid
/// and only noted. Dynamically, each input is run as an ALL and the head must never leave
/// `⟨…⟩`. A machine without `⟨` or `⟩` that uses both `<` and `>` is read with them as the
/// delimiters.
pub fn all_check_with(
    machine: &TuringMachine,
    inputs: &[String],
    max_steps: usize,
    allow_shrink: bool,
) -> Result<AllReport, CatedraError> {
    check_supported(machine)?;
    let machine = &with_delimiter_aliases(machine);
    let mut violations = Vec::new();
    let mut notes = Vec::new();
    for (i, t) in machine.automaton().transitions().iter().enumerate() {
        let a = &t.label.actions()[0];
        let parsed = ReadSymbol::parse(a.read());
        let matches = |d: char| parsed.as_ref().is_some_and(|r| r.matches(d));
        let keeps = |d: char| a.write() == d || a.write() == '~';
        let to_final = machine.automaton().is_final(t.to);
        let halts_here = |d: char| keeps(d) && a.direction() == Direction::Stay && to_final;
        if matches(LEFT_DELIMITER) {
            if halts_here(LEFT_DELIMITER) {
                add_note(
                    &mut notes,
                    halting_note(machine, t.from, LEFT_DELIMITER, t.to),
                );
            } else if !(keeps(LEFT_DELIMITER) && a.direction() == Direction::Right) {
                violations.push(static_violation(
                    machine,
                    i,
                    "delimitador_izquierdo",
                    "lee ⟨: debe escribir ⟨ y moverse a la derecha (D).",
                ));
            }
        }
        if matches(RIGHT_DELIMITER) {
            if halts_here(RIGHT_DELIMITER) {
                add_note(
                    &mut notes,
                    halting_note(machine, t.from, RIGHT_DELIMITER, t.to),
                );
            } else if !(keeps(RIGHT_DELIMITER) && a.direction() == Direction::Left) {
                violations.push(static_violation(
                    machine,
                    i,
                    "delimitador_derecho",
                    "lee ⟩: debe escribir ⟩ y moverse a la izquierda (I).",
                ));
            }
        }
        if a.read().contains(BLANK) || a.write() == BLANK {
            violations.push(static_violation(
                machine,
                i,
                "usa_blanco",
                "lee o escribe Δ: un ALL no usa Δ.",
            ));
        }
        let reads_delimiter = matches!(&parsed, Some(ReadSymbol::Char(c)) if is_delimiter(*c));
        if is_delimiter(a.write()) && !reads_delimiter {
            let read = a.read().replace(BLANK, &DELTA.to_string());
            if allow_shrink && is_shrink(a.write(), a.direction()) {
                add_note(&mut notes, shrink_note(machine, t.from, a.write(), &read));
            } else if allow_shrink {
                violations.push(static_violation(
                    machine,
                    i,
                    "escribe_delimitador",
                    &format!(
                        "escribe {} sobre {read} moviéndose a {}: {}.",
                        a.write(),
                        direction_name(a.direction()),
                        shrink_rule(a.write())
                    ),
                ));
            } else {
                violations.push(static_violation(
                    machine,
                    i,
                    "escribe_delimitador",
                    &format!(
                        "escribe {} sobre {read}: ⟨ y ⟩ solo se escriben sobre sí mismos.",
                        a.write()
                    ),
                ));
            }
        }
    }
    let mut runs = Vec::new();
    for input in inputs {
        let report = run_with(machine, input, Model::All, max_steps, allow_shrink)?;
        violations.extend(report.violation.clone());
        runs.push(report);
    }
    Ok(AllReport {
        ok: violations.is_empty(),
        notes,
        violations,
        runs,
    })
}
