//! CYK parsing for grammars in Chomsky normal form (JFLAP `CYKParser`).

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use super::{Derivation, ParseError, ParseOutcome, ParseTree};
use crate::grammar::check::is_chomsky_production;
use crate::grammar::model::Grammar;

/// The CYK table: `get(i, j)` holds the variables that derive characters `i..=j` of the input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CykTable {
    pub length: usize,
    /// `cells[i][j]`, meaningful for `i ≤ j`.
    pub cells: Vec<Vec<BTreeSet<String>>>,
}

impl CykTable {
    /// The cell for the substring `i..=j`; empty outside the table.
    pub fn get(&self, i: usize, j: usize) -> &BTreeSet<String> {
        static EMPTY: BTreeSet<String> = BTreeSet::new();
        self.cells
            .get(i)
            .and_then(|row| row.get(j))
            .unwrap_or(&EMPTY)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CykResult {
    pub outcome: ParseOutcome,
    pub table: CykTable,
    /// Leftmost derivation read off the table; only when accepted.
    pub derivation: Option<Derivation>,
    pub tree: Option<ParseTree>,
}

/// Runs CYK. The grammar must be in Chomsky normal form ([`ParseError::NotChomsky`] otherwise);
/// like JFLAP, the empty string is rejected, and so is any input with a symbol no production
/// produces.
pub fn cyk_parse(g: &Grammar, input: &str) -> Result<CykResult, ParseError> {
    if !g.is_context_free() || !g.is_chomsky() {
        let production = g
            .productions()
            .iter()
            .find(|p| !p.is_restricted_on_lhs() || !is_chomsky_production(p))
            .cloned()
            .expect("a non-CNF grammar has a production that is not in CNF");
        return Err(ParseError::NotChomsky { production });
    }
    let text: Vec<char> = input.chars().collect();
    let n = text.len();
    let mut cells = vec![vec![BTreeSet::<String>::new(); n]; n];
    // (i, j, variable) -> (split k, left variable, right variable)
    let mut back: HashMap<(usize, usize, String), (usize, String, String)> = HashMap::new();
    for (i, &c) in text.iter().enumerate() {
        for p in g.productions() {
            if p.rhs == c.to_string() {
                cells[i][i].insert(p.lhs.clone());
            }
        }
    }
    for span in 2..=n {
        for i in 0..=n - span {
            let j = i + span - 1;
            for k in i..j {
                for p in g.productions() {
                    let rhs: Vec<char> = p.rhs.chars().collect();
                    let [b, c] = rhs.as_slice() else { continue };
                    let (b, c) = (b.to_string(), c.to_string());
                    if cells[i][k].contains(&b) && cells[k + 1][j].contains(&c) {
                        let key = (i, j, p.lhs.clone());
                        if cells[i][j].insert(p.lhs.clone()) {
                            back.insert(key, (k, b, c));
                        }
                    }
                }
            }
        }
    }
    let accepted = n > 0 && cells[0][n - 1].contains(g.start());
    let table = CykTable { length: n, cells };
    if !accepted {
        return Ok(CykResult {
            outcome: ParseOutcome::Rejected,
            table,
            derivation: None,
            tree: None,
        });
    }
    let tree = node(&text, &back, 0, n - 1, g.start());
    let derivation = tree.leftmost_derivation();
    Ok(CykResult {
        outcome: ParseOutcome::Accepted,
        table,
        derivation: Some(derivation),
        tree: Some(tree),
    })
}

fn node(
    text: &[char],
    back: &HashMap<(usize, usize, String), (usize, String, String)>,
    i: usize,
    j: usize,
    var: &str,
) -> ParseTree {
    let children = match back.get(&(i, j, var.to_string())) {
        Some((k, b, c)) => vec![node(text, back, i, *k, b), node(text, back, k + 1, j, c)],
        None => vec![ParseTree {
            symbol: text[i].to_string(),
            children: Vec::new(),
        }],
    };
    ParseTree {
        symbol: var.to_string(),
        children,
    }
}
