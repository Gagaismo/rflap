//! Multiple run: parse many inputs with one method (JFLAP "Multiple Brute Force Parse",
//! "Multiple CYK Parse", and the table-driven parsers' input lists).

use serde::{Deserialize, Serialize};

use super::{ParseOutcome, brute_parse, cyk_parse, ll1_parse, slr_parse};
use crate::grammar::model::Grammar;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseMethod {
    Brute { max_nodes: usize },
    Ll1,
    Slr1,
    Cyk,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiOutcome {
    Accepted,
    Rejected,
    Undecided,
    /// The input (or the grammar, for this method) could not be parsed at all.
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiResult {
    pub input: String,
    pub outcome: MultiOutcome,
}

fn lift(outcome: ParseOutcome) -> MultiOutcome {
    match outcome {
        ParseOutcome::Accepted => MultiOutcome::Accepted,
        ParseOutcome::Rejected => MultiOutcome::Rejected,
        ParseOutcome::Undecided => MultiOutcome::Undecided,
    }
}

/// Parses every input with `method`, one result per input, in order.
pub fn parse_many(g: &Grammar, inputs: &[&str], method: ParseMethod) -> Vec<MultiResult> {
    inputs
        .iter()
        .map(|&input| {
            let outcome = match method {
                ParseMethod::Brute { max_nodes } => {
                    brute_parse(g, input, max_nodes).map(|r| r.outcome)
                }
                ParseMethod::Ll1 => ll1_parse(g, input).map(|r| r.outcome),
                ParseMethod::Slr1 => slr_parse(g, input).map(|r| r.outcome),
                ParseMethod::Cyk => cyk_parse(g, input).map(|r| r.outcome),
            };
            MultiResult {
                input: input.to_string(),
                outcome: outcome.map_or_else(|e| MultiOutcome::Error(e.to_string()), lift),
            }
        })
        .collect()
}
