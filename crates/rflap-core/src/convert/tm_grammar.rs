//! Turing machine to unrestricted grammar (JFLAP `TuringToGrammarConverter` driven by
//! `TuringConvertController`).
//!
//! The grammar writes the input as cells `V(xy)` (original symbol `x`, current symbol `y`)
//! and `V(xqy)` for the cell under the head in state `q`; `=` stands for the blank. Moves
//! rewrite two neighbouring cells and, when they enter a final state, the cells collapse
//! back into the original input symbols.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::ConvertError;
use crate::automaton::{Label, StateId};
use crate::grammar::{Grammar, Production};
use crate::turing::{BLANK, Direction, TuringMachine};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TmToGrammarStep {
    /// The productions JFLAP reveals for the initial state: the template `S`, `T` and `=`.
    Initial {
        state: StateId,
        productions: Vec<Production>,
    },
    /// The productions of one transition.
    Transition {
        from: StateId,
        to: StateId,
        label: String,
        productions: Vec<Production>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmToGrammar {
    pub grammar: Grammar,
    pub steps: Vec<TmToGrammarStep>,
    /// Limits of JFLAP's construction that apply to this machine.
    pub warnings: Vec<String>,
}

/// The tape symbol as the construction writes it: the blank is `=`.
fn symbol(c: &str) -> String {
    if c == BLANK.to_string() {
        "=".to_string()
    } else {
        c.to_string()
    }
}

fn cell(x: &str, y: &str) -> String {
    format!("V({x}{y})")
}

fn head_cell(x: &str, q: StateId, y: &str) -> String {
    format!("V({x}{q}{y})")
}

fn push_unique(set: &mut Vec<String>, s: String) -> bool {
    if set.contains(&s) {
        false
    } else {
        set.push(s);
        true
    }
}

/// JFLAP's construction for a one-tape machine without building blocks. Tape symbols must
/// be plain characters (no `~`, `!x` or `a,b}v` patterns).
///
/// Differences from JFLAP: productions keep the order they are generated in (the template
/// first, then each transition) instead of JFLAP's sort, and the symbol sets are iterated in
/// order of first use. Limits that JFLAP shares and that are reported in `warnings`:
/// transitions that stay (`S`) produce nothing, the empty input cannot be derived, and a
/// move only fires next to cells whose current symbol some transition writes.
pub fn tm_to_grammar(tm: &TuringMachine) -> Result<TmToGrammar, ConvertError> {
    let machine = tm.automaton();
    if tm.tapes() != 1 {
        return Err(ConvertError::NotSingleTape { tapes: tm.tapes() });
    }
    if tm
        .blocks()
        .values()
        .any(|b| b.automaton().states().next().is_some())
    {
        return Err(ConvertError::UnsupportedBlocks);
    }
    let initial = machine.initial().ok_or(ConvertError::NoInitialState)?;

    // Plain symbols only.
    for t in machine.transitions() {
        let action = &t.label.actions()[0];
        let read = action.read();
        let plain_read = read.chars().count() == 1 && read != "~";
        if !plain_read || action.write() == '~' {
            let shown = if plain_read {
                action.write().to_string()
            } else {
                read.to_string()
            };
            return Err(ConvertError::UnsupportedTapeSymbol {
                from: t.from,
                to: t.to,
                symbol: shown,
            });
        }
    }

    // Initial state: the template and the cells for every symbol some transition reads.
    let mut readable = vec!["=".to_string()];
    let mut writable: Vec<String> = Vec::new();
    let mut init = vec![
        Production::new("S", "V(==)S"),
        Production::new("S", "SV(==)"),
        Production::new("S", "T"),
    ];
    for t in machine.transitions() {
        let action = &t.label.actions()[0];
        let (read, write) = (symbol(action.read()), symbol(&action.write().to_string()));
        push_unique(&mut writable, write);
        if push_unique(&mut readable, read.clone()) {
            init.push(Production::new("T", &format!("T{}", cell(&read, &read))));
            init.push(Production::new("T", &head_cell(&read, initial, &read)));
        }
    }
    init.push(Production::new("=", ""));

    let mut all = init.clone();
    let mut steps = vec![TmToGrammarStep::Initial {
        state: initial,
        productions: init,
    }];
    let mut warnings = Vec::new();

    for t in machine.transitions() {
        let action = &t.label.actions()[0];
        let (read, write) = (symbol(action.read()), symbol(&action.write().to_string()));
        let target_final = machine.is_final(t.to);
        let mut made = Vec::new();
        match action.direction() {
            Direction::Stay => warnings.push(format!(
                "The transition q{} -> q{} stays ({}): JFLAP generates no productions for it.",
                t.from,
                t.to,
                t.label.description()
            )),
            dir => {
                for x4 in &readable {
                    for x5 in &readable {
                        for x6 in &writable {
                            let (head, other) = (head_cell(x5, t.from, &read), cell(x4, x6));
                            let (new_head, new_cell) = (head_cell(x4, t.to, x6), cell(x5, &write));
                            let (before, after) = if dir == Direction::Right {
                                (format!("{head}{other}"), format!("{new_cell}{new_head}"))
                            } else {
                                (format!("{other}{head}"), format!("{new_head}{new_cell}"))
                            };
                            made.push(Production::new(&before, &after));
                            if target_final {
                                made.push(Production::new(&head_cell(x4, t.to, x6), x4));
                                let (left, right) = (cell(x5, x6), x4.as_str());
                                if dir == Direction::Right {
                                    made.push(Production::new(
                                        &format!("{left}{right}"),
                                        &format!("{x5}{x4}"),
                                    ));
                                    made.push(Production::new(
                                        &format!("{x4}{left}"),
                                        &format!("{x4}{x5}"),
                                    ));
                                } else {
                                    made.push(Production::new(
                                        &format!("{x4}{left}"),
                                        &format!("{x4}{x5}"),
                                    ));
                                    made.push(Production::new(
                                        &format!("{left}{right}"),
                                        &format!("{x5}{x4}"),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        // Productions repeat across symbol combinations; the grammar keeps the first.
        let mut seen = BTreeSet::new();
        made.retain(|p| seen.insert(p.clone()));
        all.extend(made.iter().cloned());
        steps.push(TmToGrammarStep::Transition {
            from: t.from,
            to: t.to,
            label: t.label.description(),
            productions: made,
        });
    }

    let unwritten: Vec<&String> = readable.iter().filter(|r| !writable.contains(r)).collect();
    if !unwritten.is_empty() {
        let list: Vec<&str> = unwritten.iter().map(|s| s.as_str()).collect();
        warnings.push(format!(
            "JFLAP only moves the head next to cells whose current symbol some transition writes; \
             no transition writes {}.",
            list.join(", ")
        ));
    }
    warnings.push(
        "The empty input cannot be derived: the grammar always has at least one input cell."
            .to_string(),
    );

    let grammar = Grammar::from_productions(all).expect("left sides are never empty");
    Ok(TmToGrammar {
        grammar,
        steps,
        warnings,
    })
}
