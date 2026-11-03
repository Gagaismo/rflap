//! User-controlled parsing (JFLAP `UserParser`, `RestrictedUserParser`, `UnrestrictedUserParser`):
//! the user picks a production and where to apply it, step by step, towards the target.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::brute::{BruteKind, kind_of, possible_with};
use super::{
    Derivation, DerivationStep, ParseError, ParseTree, check_input, occurrences, replace_at,
};
use crate::grammar::model::Grammar;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserParseError {
    /// There is no production with that index.
    NoSuchProduction {
        production: usize,
    },
    /// The left side of the production does not occur in the current form.
    NotApplicable {
        production: usize,
    },
    /// The left side does not occur at that position.
    NoMatchAt {
        production: usize,
        position: usize,
    },
    /// Two of the chosen places overlap (or are not in increasing order).
    Overlap {
        production: usize,
        position: usize,
    },
    NothingToUndo,
}

impl fmt::Display for UserParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserParseError::NoSuchProduction { production } => {
                write!(f, "there is no production {production}")
            }
            UserParseError::NotApplicable { production } => {
                write!(
                    f,
                    "production {production} does not apply to the current form"
                )
            }
            UserParseError::NoMatchAt {
                production,
                position,
            } => {
                write!(
                    f,
                    "production {production} does not match at position {position}"
                )
            }
            UserParseError::Overlap {
                production,
                position,
            } => {
                write!(f, "production {production} overlaps at position {position}")
            }
            UserParseError::NothingToUndo => write!(f, "there is nothing to undo"),
        }
    }
}

impl std::error::Error for UserParseError {}

/// A derivation built one production at a time. Production indices refer to
/// `grammar().productions()`; positions are character indices into the current form.
#[derive(Clone, Debug)]
pub struct UserParser {
    grammar: Grammar,
    target: String,
    kind: BruteKind,
    smaller: HashSet<char>,
    forms: Vec<String>,
    steps: Vec<DerivationStep>,
    /// How many forms/steps each user move added.
    marks: Vec<usize>,
}

impl UserParser {
    /// Starts at the grammar's start variable. The target must only use the grammar's terminals.
    pub fn new(g: &Grammar, target: &str) -> Result<Self, ParseError> {
        check_input(g, target)?;
        Ok(UserParser {
            grammar: g.clone(),
            target: target.to_string(),
            kind: kind_of(g),
            smaller: super::brute::smaller_symbols_of(g.productions()),
            forms: vec![g.start().to_string()],
            steps: Vec::new(),
            marks: Vec::new(),
        })
    }

    pub fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    /// The current sentential form.
    pub fn sentential(&self) -> &str {
        self.forms.last().expect("there is always a form")
    }

    pub fn is_finished(&self) -> bool {
        self.sentential() == self.target
    }

    /// Where the left side of production `production` occurs in the current form.
    pub fn positions(&self, production: usize) -> Vec<usize> {
        let Some(p) = self.grammar.productions().get(production) else {
            return Vec::new();
        };
        let form: Vec<char> = self.sentential().chars().collect();
        let lhs: Vec<char> = p.lhs.chars().collect();
        occurrences(&form, &lhs)
    }

    /// Applies a production at `position`, or at its first occurrence when `None`.
    pub fn apply(
        &mut self,
        production: usize,
        position: Option<usize>,
    ) -> Result<(), UserParseError> {
        let found = self.positions(production);
        self.check_production(production)?;
        let at = match position {
            None => *found
                .first()
                .ok_or(UserParseError::NotApplicable { production })?,
            Some(p) if found.contains(&p) => p,
            Some(position) => {
                return Err(UserParseError::NoMatchAt {
                    production,
                    position,
                });
            }
        };
        self.apply_at(production, &[at])
    }

    /// Applies one production at several places of the current form at once (JFLAP lets the
    /// user pick every occurrence to replace). Positions are in increasing order and must not
    /// overlap.
    pub fn apply_many(
        &mut self,
        production: usize,
        positions: &[usize],
    ) -> Result<(), UserParseError> {
        self.check_production(production)?;
        if positions.is_empty() {
            return Err(UserParseError::NotApplicable { production });
        }
        let found = self.positions(production);
        let len = self.grammar.productions()[production].lhs.chars().count();
        let mut previous_end = 0usize;
        for &position in positions {
            if !found.contains(&position) {
                return Err(UserParseError::NoMatchAt {
                    production,
                    position,
                });
            }
            if position < previous_end {
                return Err(UserParseError::Overlap {
                    production,
                    position,
                });
            }
            previous_end = position + len;
        }
        self.apply_at(production, positions)
    }

    fn check_production(&self, production: usize) -> Result<(), UserParseError> {
        if production < self.grammar.productions().len() {
            Ok(())
        } else {
            Err(UserParseError::NoSuchProduction { production })
        }
    }

    fn apply_at(&mut self, production: usize, positions: &[usize]) -> Result<(), UserParseError> {
        let p = self.grammar.productions()[production].clone();
        let (lhs_len, rhs_len) = (p.lhs.chars().count(), p.rhs.chars().count());
        let mut current = self.sentential().to_string();
        let mut delta: isize = 0;
        let mut new_forms = Vec::new();
        let mut new_steps = Vec::new();
        for &pos in positions {
            let at = (pos as isize + delta) as usize;
            current = replace_at(&current, at, lhs_len, &p.rhs);
            delta += rhs_len as isize - lhs_len as isize;
            new_steps.push(DerivationStep {
                production: p.clone(),
                position: at,
                result: current.clone(),
            });
            new_forms.push(current.clone());
        }
        // one user move is recorded as one step per replaced place, but undone as a whole
        self.forms.extend(new_forms);
        self.steps.extend(new_steps);
        self.marks.push(positions.len());
        Ok(())
    }

    /// Takes back the last move.
    pub fn undo(&mut self) -> Result<(), UserParseError> {
        let n = self.marks.pop().ok_or(UserParseError::NothingToUndo)?;
        for _ in 0..n {
            self.forms.pop();
            self.steps.pop();
        }
        Ok(())
    }

    /// JFLAP's pruning: could `form` still lead to the target?
    pub fn is_possible_derivation(&self, form: &str) -> bool {
        let form: Vec<char> = form.chars().collect();
        let target: Vec<char> = self.target.chars().collect();
        possible_with(self.kind, &form, &target, &self.smaller)
    }

    pub fn derivation(&self) -> Derivation {
        Derivation {
            start: self.grammar.start().to_string(),
            steps: self.steps.clone(),
        }
    }

    /// The parse tree so far (context-free grammars only).
    pub fn tree(&self) -> Option<ParseTree> {
        self.derivation().tree()
    }
}
