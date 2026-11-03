//! Parsers (JFLAP `grammar.parse`): brute force, LL(1), SLR(1), CYK and user-controlled.
//!
//! Every parser returns an outcome, a [`Derivation`] (one production at one position per step)
//! and a [`ParseTree`], all serde-serializable.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::model::{Grammar, LAMBDA, Production, is_variable_symbol};

mod brute;
mod cyk;
mod ll;
mod lr;
mod multi;
mod ops;
mod user;

pub use brute::{BruteKind, BruteResult, brute_parse};
pub use cyk::{CykResult, CykTable, cyk_parse};
pub use ll::{LlAction, LlConflict, LlParseResult, LlStep, LlTable, ll1_parse, ll1_table};
pub use lr::{
    LrAction, LrAutomaton, LrConflict, LrItem, LrParseResult, LrState, LrStep, LrStepAction,
    LrTable, slr_automaton, slr_parse, slr_table,
};
pub use multi::{MultiOutcome, MultiResult, ParseMethod, parse_many};
pub use ops::{END_MARKER, first_of_string, first_sets, follow_sets};
pub use user::{UserParseError, UserParser};

/// Result of a parse attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseOutcome {
    Accepted,
    Rejected,
    /// The search ran out of its budget before deciding.
    Undecided,
}

/// Why an input could not even be attempted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseError {
    /// The input has a symbol that is not a terminal of the grammar (JFLAP's "String to parse
    /// has nonterminal").
    UnknownSymbol { symbol: char },
    /// CYK needs Chomsky normal form.
    NotChomsky { production: Production },
    /// The LL(1) table has cells with several entries.
    NotLl1 { conflicts: Vec<LlConflict> },
    /// The SLR(1) table has cells with several actions.
    NotSlr1 { conflicts: Vec<LrConflict> },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnknownSymbol { symbol } => {
                write!(f, "String to parse has nonterminal {symbol}.")
            }
            ParseError::NotChomsky { production } => {
                write!(
                    f,
                    "the production {production} is not in Chomsky normal form"
                )
            }
            ParseError::NotLl1 { conflicts } => {
                write!(
                    f,
                    "the grammar is not LL(1): {} conflicting cell(s)",
                    conflicts.len()
                )
            }
            ParseError::NotSlr1 { conflicts } => {
                write!(
                    f,
                    "the grammar is not SLR(1): {} conflicting cell(s)",
                    conflicts.len()
                )
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// One derivation step: `production` replaces the occurrence of its left side that starts at
/// character `position` of the previous sentential form, giving `result`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationStep {
    pub production: Production,
    pub position: usize,
    pub result: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Derivation {
    pub start: String,
    pub steps: Vec<DerivationStep>,
}

impl Derivation {
    pub fn new(start: &str) -> Self {
        Derivation {
            start: start.to_string(),
            steps: Vec::new(),
        }
    }

    /// The start symbol followed by the form after each step.
    pub fn sentential_forms(&self) -> Vec<String> {
        std::iter::once(self.start.clone())
            .chain(self.steps.iter().map(|s| s.result.clone()))
            .collect()
    }

    /// The last sentential form.
    pub fn result(&self) -> &str {
        self.steps
            .last()
            .map_or(self.start.as_str(), |s| s.result.as_str())
    }

    /// Replays the derivation: every production belongs to `g`, matches at its position and
    /// produces the recorded form.
    pub fn is_valid_for(&self, g: &Grammar) -> bool {
        let mut current = self.start.clone();
        for s in &self.steps {
            if !g.contains(&s.production) {
                return false;
            }
            let chars: Vec<char> = current.chars().collect();
            let lhs: Vec<char> = s.production.lhs.chars().collect();
            let end = s.position + lhs.len();
            if end > chars.len() || chars[s.position..end] != lhs[..] {
                return false;
            }
            current = replace_at(&current, s.position, lhs.len(), &s.production.rhs);
            if current != s.result {
                return false;
            }
        }
        true
    }

    /// The parse tree this derivation builds; `None` unless every step has a single-variable
    /// left side (a context-free derivation).
    pub fn tree(&self) -> Option<ParseTree> {
        let mut arena = vec![Node {
            symbol: self.start.clone(),
            children: Vec::new(),
        }];
        let mut frontier: Vec<usize> = vec![0];
        if self.start.chars().count() != 1 {
            return None;
        }
        for s in &self.steps {
            if s.production.lhs.chars().count() != 1 || s.position >= frontier.len() {
                return None;
            }
            let parent = frontier[s.position];
            let mut kids = Vec::new();
            if s.production.rhs.is_empty() {
                arena.push(Node {
                    symbol: LAMBDA.to_string(),
                    children: Vec::new(),
                });
                let id = arena.len() - 1;
                arena[parent].children.push(id);
            } else {
                for c in s.production.rhs.chars() {
                    arena.push(Node {
                        symbol: c.to_string(),
                        children: Vec::new(),
                    });
                    let id = arena.len() - 1;
                    arena[parent].children.push(id);
                    kids.push(id);
                }
            }
            frontier.splice(s.position..=s.position, kids);
        }
        Some(build(&arena, 0))
    }
}

struct Node {
    symbol: String,
    children: Vec<usize>,
}

fn build(arena: &[Node], id: usize) -> ParseTree {
    ParseTree {
        symbol: arena[id].symbol.clone(),
        children: arena[id]
            .children
            .iter()
            .map(|&c| build(arena, c))
            .collect(),
    }
}

/// A parse tree. A variable replaced by λ has a single child with symbol [`LAMBDA`]; variables
/// not expanded yet (a rejected parse stops early) are leaves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseTree {
    pub symbol: String,
    pub children: Vec<ParseTree>,
}

impl ParseTree {
    /// The terminals at the leaves, left to right.
    pub fn yield_string(&self) -> String {
        let mut out = String::new();
        self.collect_yield(&mut out);
        out
    }

    fn collect_yield(&self, out: &mut String) {
        if self.children.is_empty() {
            let mut cs = self.symbol.chars();
            if let (Some(c), None) = (cs.next(), cs.next())
                && !is_variable_symbol(c)
            {
                out.push(c);
            }
            return;
        }
        if !self.has_lambda_marker() {
            for c in &self.children {
                c.collect_yield(out);
            }
        }
    }

    fn has_lambda_marker(&self) -> bool {
        self.children.len() == 1
            && self.children[0].children.is_empty()
            && self.children[0].symbol == LAMBDA
            && self.symbol.chars().all(is_variable_symbol)
    }

    /// The leftmost derivation that expands this tree (CFG trees only). A variable leaf, as in
    /// a partial tree, is left unexpanded.
    pub fn leftmost_derivation(&self) -> Derivation {
        let mut d = Derivation::new(&self.symbol);
        let mut current = self.symbol.clone();
        let mut emitted = 0usize;
        self.expand_leftmost(&mut d, &mut current, &mut emitted);
        d
    }

    fn expand_leftmost(&self, d: &mut Derivation, current: &mut String, emitted: &mut usize) {
        if !self.children.is_empty() && self.symbol.chars().all(is_variable_symbol) {
            let rhs: String = if self.has_lambda_marker() {
                String::new()
            } else {
                self.children.iter().map(|c| c.symbol.as_str()).collect()
            };
            let production = Production {
                lhs: self.symbol.clone(),
                rhs,
            };
            *current = replace_at(current, *emitted, 1, &production.rhs);
            d.steps.push(DerivationStep {
                production,
                position: *emitted,
                result: current.clone(),
            });
            if !self.has_lambda_marker() {
                for c in &self.children {
                    c.expand_leftmost(d, current, emitted);
                }
            }
        } else if self.symbol != LAMBDA {
            *emitted += 1;
        }
    }
}

/// Replaces `len` characters of `s` starting at character `position` with `with`.
pub(crate) fn replace_at(s: &str, position: usize, len: usize, with: &str) -> String {
    let mut out: String = s.chars().take(position).collect();
    out.push_str(with);
    out.extend(s.chars().skip(position + len));
    out
}

/// Every character index where `needle` occurs in `hay` (overlaps included).
pub(crate) fn occurrences(hay: &[char], needle: &[char]) -> Vec<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return Vec::new();
    }
    (0..=hay.len() - needle.len())
        .filter(|&i| hay[i..i + needle.len()] == *needle)
        .collect()
}

/// The first character index at or after `from` where `needle` occurs.
pub(crate) fn find_from(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
    if from > hay.len() {
        return None;
    }
    occurrences(&hay[from..], needle).first().map(|i| i + from)
}

/// Rejects input characters that are not terminals of `g` (JFLAP checks this before parsing).
pub(crate) fn check_input(g: &Grammar, input: &str) -> Result<(), ParseError> {
    match input.chars().find(|&c| !g.is_terminal(c)) {
        Some(symbol) => Err(ParseError::UnknownSymbol { symbol }),
        None => Ok(()),
    }
}
