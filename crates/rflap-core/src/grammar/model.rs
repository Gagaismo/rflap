//! Productions and grammars (JFLAP `grammar.Production`, `grammar.Grammar`).
//!
//! JFLAP conventions kept here: a variable is an uppercase character, every other character
//! is a terminal, an empty right-hand side is λ, and the start variable is the left side of
//! the first production. Productions are unique inside a grammar and keep insertion order.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

/// How JFLAP writes the empty string.
pub const LAMBDA: &str = "λ";

/// JFLAP's `ProductionChecker.isVariable`.
pub fn is_variable_symbol(c: char) -> bool {
    c.is_uppercase()
}

/// A production `lhs → rhs`; an empty `rhs` is λ.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Production {
    pub lhs: String,
    pub rhs: String,
}

impl Production {
    pub fn new(lhs: &str, rhs: &str) -> Self {
        Production {
            lhs: lhs.to_string(),
            rhs: rhs.to_string(),
        }
    }

    pub fn variables_on_lhs(&self) -> Vec<char> {
        self.lhs
            .chars()
            .filter(|c| is_variable_symbol(*c))
            .collect()
    }

    pub fn variables_on_rhs(&self) -> Vec<char> {
        self.rhs
            .chars()
            .filter(|c| is_variable_symbol(*c))
            .collect()
    }

    pub fn terminals_on_lhs(&self) -> Vec<char> {
        self.lhs
            .chars()
            .filter(|c| !is_variable_symbol(*c))
            .collect()
    }

    pub fn terminals_on_rhs(&self) -> Vec<char> {
        self.rhs
            .chars()
            .filter(|c| !is_variable_symbol(*c))
            .collect()
    }

    /// Distinct variables, right-hand side first (the order JFLAP registers them in).
    pub fn variables(&self) -> Vec<char> {
        distinct(
            self.variables_on_rhs()
                .into_iter()
                .chain(self.variables_on_lhs()),
        )
    }

    /// Distinct terminals, right-hand side first.
    pub fn terminals(&self) -> Vec<char> {
        distinct(
            self.terminals_on_rhs()
                .into_iter()
                .chain(self.terminals_on_lhs()),
        )
    }

    /// Exactly one symbol on the left and it is a variable.
    pub fn is_restricted_on_lhs(&self) -> bool {
        self.lhs.chars().count() == 1 && self.variables_on_lhs().len() == 1
    }

    /// `A → B`.
    pub fn is_unit(&self) -> bool {
        self.is_restricted_on_lhs()
            && self.rhs.chars().count() == 1
            && self.variables_on_rhs().len() == 1
    }

    /// `A → λ`.
    pub fn is_lambda(&self) -> bool {
        self.is_restricted_on_lhs() && self.rhs.is_empty()
    }

    fn is_linear_without_variable(&self) -> bool {
        self.is_restricted_on_lhs() && self.rhs.chars().count() == self.terminals_on_rhs().len()
    }

    /// `A → wB` (one variable, last) or `A → w`.
    pub fn is_right_linear(&self) -> bool {
        (self.is_restricted_on_lhs()
            && self.variables_on_rhs().len() == 1
            && self.rhs.chars().next_back().is_some_and(is_variable_symbol))
            || self.is_linear_without_variable()
    }

    /// `A → Bw` (one variable, first) or `A → w`.
    pub fn is_left_linear(&self) -> bool {
        (self.is_restricted_on_lhs()
            && self.variables_on_rhs().len() == 1
            && self.rhs.chars().next().is_some_and(is_variable_symbol))
            || self.is_linear_without_variable()
    }

    pub fn is_linear(&self) -> bool {
        self.is_right_linear() || self.is_left_linear()
    }
}

fn distinct(items: impl Iterator<Item = char>) -> Vec<char> {
    let mut out = Vec::new();
    for c in items {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

impl fmt::Display for Production {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rhs = if self.rhs.is_empty() {
            LAMBDA
        } else {
            &self.rhs
        };
        write!(f, "{} → {}", self.lhs, rhs)
    }
}

/// Why a grammar could not be built.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrammarError {
    /// A line without an arrow (`line` is 1-based).
    Syntax { line: usize, message: String },
    /// A production whose left side is empty.
    EmptyLhs { line: usize },
    /// JFLAP needs the first production to be restricted so the start variable is a variable.
    FirstProductionUnrestricted,
}

impl fmt::Display for GrammarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrammarError::Syntax { line, message } => write!(f, "line {line}: {message}"),
            GrammarError::EmptyLhs { line } => {
                write!(f, "line {line}: the left hand side cannot be empty")
            }
            GrammarError::FirstProductionUnrestricted => {
                write!(f, "the first production must be restricted")
            }
        }
    }
}

impl std::error::Error for GrammarError {}

#[derive(Deserialize)]
struct GrammarRepr {
    start: String,
    productions: Vec<Production>,
}

impl TryFrom<GrammarRepr> for Grammar {
    type Error = GrammarError;

    fn try_from(r: GrammarRepr) -> Result<Self, GrammarError> {
        Grammar::from_parts(&r.start, r.productions)
    }
}

/// An ordered set of productions plus a start variable. Variables and terminals are derived
/// from the productions, like JFLAP does when a production is added.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "GrammarRepr")]
pub struct Grammar {
    start: String,
    productions: Vec<Production>,
}

impl Default for Grammar {
    fn default() -> Self {
        Grammar::empty()
    }
}

impl Grammar {
    /// A grammar with no productions and no start variable.
    pub fn empty() -> Self {
        Grammar {
            start: String::new(),
            productions: Vec::new(),
        }
    }

    /// Builds a grammar from productions, adding them one by one: the start variable is the
    /// left side of the first one, duplicates are dropped.
    pub fn from_productions(productions: Vec<Production>) -> Result<Self, GrammarError> {
        let mut g = Grammar::empty();
        for (i, p) in productions.into_iter().enumerate() {
            g.add_checked(p, i + 1)?;
        }
        Ok(g)
    }

    /// `from_productions` over `(lhs, rhs)` pairs.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Result<Self, GrammarError> {
        Grammar::from_productions(pairs.iter().map(|(l, r)| Production::new(l, r)).collect())
    }

    /// A grammar with an explicit start variable (which need not occur in any production, as
    /// after removing every useless production). Duplicates are dropped.
    pub fn from_parts(start: &str, productions: Vec<Production>) -> Result<Self, GrammarError> {
        let mut g = Grammar {
            start: start.to_string(),
            productions: Vec::new(),
        };
        for (i, p) in productions.into_iter().enumerate() {
            if p.lhs.is_empty() {
                return Err(GrammarError::EmptyLhs { line: i + 1 });
            }
            if !g.productions.contains(&p) {
                g.productions.push(p);
            }
        }
        Ok(g)
    }

    /// Parses `S -> aSb | λ` lines (`→` also works). Blank lines are skipped, blanks inside a
    /// production are ignored, and `λ`, `ε` or nothing at all is the empty string.
    pub fn parse(text: &str) -> Result<Self, GrammarError> {
        let mut g = Grammar::empty();
        for (i, raw) in text.lines().enumerate() {
            let line = i + 1;
            if raw.trim().is_empty() {
                continue;
            }
            let (arrow_at, arrow_len) = find_arrow(raw).ok_or_else(|| GrammarError::Syntax {
                line,
                message: format!("expected `->` in `{}`", raw.trim()),
            })?;
            let lhs: String = raw[..arrow_at]
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if lhs.is_empty() {
                return Err(GrammarError::EmptyLhs { line });
            }
            for alt in raw[arrow_at + arrow_len..].split('|') {
                let rhs: String = alt.chars().filter(|c| !c.is_whitespace()).collect();
                let rhs = if rhs == "λ" || rhs == "ε" {
                    String::new()
                } else {
                    rhs
                };
                g.add_checked(
                    Production {
                        lhs: lhs.clone(),
                        rhs,
                    },
                    line,
                )?;
            }
        }
        Ok(g)
    }

    /// Adds a production; `Ok(false)` if it was already there.
    pub fn add_production(&mut self, p: Production) -> Result<bool, GrammarError> {
        let n = self.productions.len() + 1;
        self.add_checked(p, n)
    }

    fn add_checked(&mut self, p: Production, line: usize) -> Result<bool, GrammarError> {
        if p.lhs.is_empty() {
            return Err(GrammarError::EmptyLhs { line });
        }
        if self.productions.is_empty() && !p.is_restricted_on_lhs() {
            return Err(GrammarError::FirstProductionUnrestricted);
        }
        if self.productions.contains(&p) {
            return Ok(false);
        }
        if self.productions.is_empty() {
            self.start = p.lhs.clone();
        }
        self.productions.push(p);
        Ok(true)
    }

    /// Removes a production; `true` if it was present. The start variable is kept.
    pub fn remove_production(&mut self, p: &Production) -> bool {
        match self.productions.iter().position(|q| q == p) {
            Some(i) => {
                self.productions.remove(i);
                true
            }
            None => false,
        }
    }

    pub fn productions(&self) -> &[Production] {
        &self.productions
    }

    pub fn start(&self) -> &str {
        &self.start
    }

    pub fn set_start(&mut self, start: &str) {
        self.start = start.to_string();
    }

    pub fn contains(&self, p: &Production) -> bool {
        self.productions.contains(p)
    }

    /// The productions whose left side is exactly `lhs`, in order.
    pub fn productions_for<'a>(&'a self, lhs: &'a str) -> impl Iterator<Item = &'a Production> {
        self.productions.iter().filter(move |p| p.lhs == lhs)
    }

    /// Every variable that occurs in some production.
    pub fn variables(&self) -> BTreeSet<char> {
        self.productions
            .iter()
            .flat_map(|p| p.variables())
            .collect()
    }

    /// Every terminal that occurs in some production.
    pub fn terminals(&self) -> BTreeSet<char> {
        self.productions
            .iter()
            .flat_map(|p| p.terminals())
            .collect()
    }

    pub fn is_variable(&self, c: char) -> bool {
        is_variable_symbol(c) && self.productions.iter().any(|p| has(p, c))
    }

    pub fn is_terminal(&self, c: char) -> bool {
        !is_variable_symbol(c) && self.productions.iter().any(|p| has(p, c))
    }

    /// The variables in the order JFLAP iterates them (`Grammar.getVariables()` walks a Java
    /// `HashSet<String>`): bucket order of the one-character strings, ties by insertion order.
    /// The transformations use it so their steps come out in the same order as JFLAP's.
    pub fn variables_jflap_order(&self) -> Vec<char> {
        let mut inserted: Vec<char> = Vec::new();
        for p in &self.productions {
            for v in p.variables() {
                if !inserted.contains(&v) {
                    inserted.push(v);
                }
            }
        }
        let mut capacity = 16usize;
        while inserted.len() > capacity * 3 / 4 {
            capacity *= 2;
        }
        let mut keyed: Vec<(usize, usize, char)> = inserted
            .iter()
            .enumerate()
            .map(|(i, &c)| (bucket(c, capacity), i, c))
            .collect();
        keyed.sort();
        keyed.into_iter().map(|(_, _, c)| c).collect()
    }

    /// One production per line (`S -> aSb`, λ for the empty string); `parse` reads it back.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for p in &self.productions {
            let rhs = if p.rhs.is_empty() { LAMBDA } else { &p.rhs };
            out.push_str(&format!("{} -> {}\n", p.lhs, rhs));
        }
        out
    }
}

fn has(p: &Production, c: char) -> bool {
    p.lhs.contains(c) || p.rhs.contains(c)
}

/// `HashMap` bucket of the one-character string `c` in a table of `capacity` slots.
fn bucket(c: char, capacity: usize) -> usize {
    let mut units = [0u16; 2];
    let mut h: i32 = 0;
    for u in c.encode_utf16(&mut units) {
        h = h.wrapping_mul(31).wrapping_add(i32::from(*u));
    }
    let h = h as u32;
    ((h ^ (h >> 16)) as usize) & (capacity - 1)
}

fn find_arrow(line: &str) -> Option<(usize, usize)> {
    let ascii = line.find("->").map(|i| (i, 2));
    let unicode = line.find('→').map(|i| (i, '→'.len_utf8()));
    match (ascii, unicode) {
        (Some(a), Some(u)) => Some(if a.0 <= u.0 { a } else { u }),
        (a, u) => a.or(u),
    }
}
