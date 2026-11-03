//! LL(1) parse table and table-driven parsing (JFLAP `LLParseTable`, `LLParseTableGenerator`,
//! `LLParseController`).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::ops::{END_MARKER, first_of_string, first_sets, follow_sets};
use super::{Derivation, DerivationStep, ParseError, ParseOutcome, ParseTree};
use crate::grammar::model::{Grammar, Production};

/// The LL(1) table: rows are variables, columns the terminals followed by `$`; each cell holds
/// the right-hand sides to use (`""` is λ).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlTable {
    pub variables: Vec<String>,
    /// Sorted terminals, then `$`.
    pub terminals: Vec<String>,
    pub cells: Vec<Vec<BTreeSet<String>>>,
}

/// A cell with more than one entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlConflict {
    pub variable: String,
    pub terminal: String,
    pub entries: Vec<String>,
}

impl LlTable {
    /// The cell for `variable` on `terminal` (`"$"` for end of input); `None` if either is not
    /// in the table.
    pub fn get(&self, variable: &str, terminal: &str) -> Option<&BTreeSet<String>> {
        let row = self.variables.iter().position(|v| v == variable)?;
        let col = self.terminals.iter().position(|t| t == terminal)?;
        Some(&self.cells[row][col])
    }

    pub fn conflicts(&self) -> Vec<LlConflict> {
        let mut out = Vec::new();
        for (r, row) in self.cells.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if cell.len() > 1 {
                    out.push(LlConflict {
                        variable: self.variables[r].clone(),
                        terminal: self.terminals[c].clone(),
                        entries: cell.iter().cloned().collect(),
                    });
                }
            }
        }
        out
    }

    pub fn is_ll1(&self) -> bool {
        self.cells.iter().flatten().all(|c| c.len() <= 1)
    }
}

/// Builds the table from FIRST and FOLLOW: `A → α` goes under every terminal of FIRST(α) and,
/// when α can vanish, under every member of FOLLOW(A).
pub fn ll1_table(g: &Grammar) -> LlTable {
    let first = first_sets(g);
    let follow = follow_sets(g);
    let variables: Vec<String> = g.variables().iter().map(char::to_string).collect();
    let mut terminals: Vec<String> = g.terminals().iter().map(char::to_string).collect();
    terminals.push(END_MARKER.to_string());
    let mut cells = vec![vec![BTreeSet::new(); terminals.len()]; variables.len()];
    for p in g.productions() {
        let Some(row) = variables.iter().position(|v| *v == p.lhs) else {
            continue;
        };
        let f = first_of_string(&first, &p.rhs);
        let mut put = |t: &str| {
            if let Some(col) = terminals.iter().position(|x| x == t) {
                cells[row][col].insert(p.rhs.clone());
            }
        };
        for t in f.iter().filter(|t| !t.is_empty()) {
            put(t);
        }
        if f.contains("") {
            for t in follow.get(&p.lhs).into_iter().flatten() {
                put(t);
            }
        }
    }
    LlTable {
        variables,
        terminals,
        cells,
    }
}

/// One move of the table-driven parser, recorded with the configuration it started from. The
/// stack is written top first and the remaining input ends with `$`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlStep {
    pub stack: String,
    pub input: String,
    pub action: LlAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlAction {
    /// Pop the variable and push the right side of `production`.
    Replace {
        production: Production,
    },
    /// The terminal on top of the stack equals the lookahead: pop both.
    Match {
        symbol: String,
    },
    Accept,
    Reject {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlParseResult {
    pub outcome: ParseOutcome,
    pub steps: Vec<LlStep>,
    /// Leftmost derivation of what was parsed (up to the error when rejected).
    pub derivation: Derivation,
    pub tree: Option<ParseTree>,
}

/// Parses `input` with the LL(1) table. Fails with [`ParseError::NotLl1`] when the table has
/// conflicts, as JFLAP refuses to parse then. Symbols outside the grammar are just rejected.
pub fn ll1_parse(g: &Grammar, input: &str) -> Result<LlParseResult, ParseError> {
    let table = ll1_table(g);
    let conflicts = table.conflicts();
    if !conflicts.is_empty() {
        return Err(ParseError::NotLl1 { conflicts });
    }
    let text: Vec<char> = input.chars().chain(std::iter::once('$')).collect();
    let mut p = 0usize;
    let mut stack: Vec<char> = g.start().chars().collect();
    let mut form: String = g.start().to_string();
    let mut derivation = Derivation::new(g.start());
    let mut steps = Vec::new();
    let outcome;
    let mut guard = 0usize;
    loop {
        guard += 1;
        let lookahead = text[p];
        let snapshot = |action: LlAction, stack: &[char], p: usize| LlStep {
            stack: stack.iter().rev().collect(),
            input: text[p..].iter().collect(),
            action,
        };
        if guard > 100_000 {
            steps.push(snapshot(
                LlAction::Reject {
                    reason: "step limit exceeded".into(),
                },
                &stack,
                p,
            ));
            outcome = ParseOutcome::Undecided;
            break;
        }
        let Some(&top) = stack.last() else {
            if lookahead == '$' {
                steps.push(snapshot(LlAction::Accept, &stack, p));
                outcome = ParseOutcome::Accepted;
            } else {
                let reason = "The stack is empty, but the input is not.".to_string();
                steps.push(snapshot(LlAction::Reject { reason }, &stack, p));
                outcome = ParseOutcome::Rejected;
            }
            break;
        };
        if !crate::grammar::model::is_variable_symbol(top) {
            if top == lookahead {
                steps.push(snapshot(
                    LlAction::Match {
                        symbol: top.to_string(),
                    },
                    &stack,
                    p,
                ));
                stack.pop();
                p += 1;
            } else {
                let reason = "Stack and input don't match.".to_string();
                steps.push(snapshot(LlAction::Reject { reason }, &stack, p));
                outcome = ParseOutcome::Rejected;
                break;
            }
            continue;
        }
        let entry = table
            .get(&top.to_string(), &lookahead.to_string())
            .and_then(|cell| cell.iter().next())
            .cloned();
        let Some(rhs) = entry else {
            let reason = format!("No rule for {top} with {lookahead} as lookahead.");
            steps.push(snapshot(LlAction::Reject { reason }, &stack, p));
            outcome = ParseOutcome::Rejected;
            break;
        };
        let production = Production {
            lhs: top.to_string(),
            rhs,
        };
        steps.push(snapshot(
            LlAction::Replace {
                production: production.clone(),
            },
            &stack,
            p,
        ));
        let position = form.chars().position(|c| c == top).unwrap_or(0);
        form = super::replace_at(&form, position, 1, &production.rhs);
        derivation.steps.push(DerivationStep {
            production: production.clone(),
            position,
            result: form.clone(),
        });
        stack.pop();
        stack.extend(production.rhs.chars().rev());
    }
    let tree = derivation.tree();
    Ok(LlParseResult {
        outcome,
        steps,
        derivation,
        tree,
    })
}
