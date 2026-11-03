//! Conversions between models: FA and regular grammars, PDA and CFG, Turing machine to unrestricted grammar.
//!
//! Every conversion returns its result together with serde-serializable steps, in the order
//! JFLAP shows them, so a UI can replay the construction. Preconditions are the ones JFLAP
//! enforces (with JFLAP's messages in [`ConvertError`]'s `Display`).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::automaton::{Point, StateId};
use crate::grammar::Production;
use crate::pda::PdaLabel;

mod cfg_pda;
mod fa_grammar;
mod pda_cfg;
mod tm_grammar;

pub use cfg_pda::{CfgToPda, CfgToPdaStep, cfg_to_pda_ll, cfg_to_pda_lr};
pub use fa_grammar::{
    FaToGrammar, FaToGrammarStep, GrammarToFa, GrammarToFaStep, StateVariable, VariableState,
    fa_to_grammar, grammar_to_fa,
};
pub use pda_cfg::{
    PdaToCfg, PdaToCfgStep, TripleProduction, TripleVariable, check_pda_form, pda_to_cfg,
};
pub use tm_grammar::{TmToGrammar, TmToGrammarStep, tm_to_grammar};

/// A PDA transition named by its endpoints and label, for error reports.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionRef {
    pub from: StateId,
    pub to: StateId,
    pub label: PdaLabel,
}

/// Why a conversion could not be done.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConvertError {
    /// The automaton has no initial state.
    NoInitialState,
    /// FA to grammar needs a letter per state.
    TooManyStates { count: usize },
    /// FA to grammar: capital letters are reserved for variables.
    CapitalInTransition {
        from: StateId,
        to: StateId,
        label: String,
    },
    /// The grammar has no productions.
    EmptyGrammar,
    /// Grammar to FA: JFLAP only converts right-linear grammars.
    NotRightLinear { production: Production },
    /// CFG to PDA: the left side of a production is not a single variable.
    NotContextFree { production: Production },
    /// PDA to CFG: JFLAP needs exactly one final state.
    NotExactlyOneFinalState { count: usize },
    /// PDA to CFG: transitions to the final state must pop only `Z`.
    FinalTransitionsMustPopZ { transitions: Vec<TransitionRef> },
    /// PDA to CFG: every transition must pop one symbol and push zero or two.
    TransitionsMustPopOnePushZeroOrTwo { transitions: Vec<TransitionRef> },
    /// PDA to CFG: more variables than letters, even after trimming.
    TooManyVariables { count: usize },
    /// TM to grammar: JFLAP's construction is for one tape.
    NotSingleTape { tapes: usize },
    /// TM to grammar: building blocks are not expanded by the construction.
    UnsupportedBlocks,
    /// TM to grammar: only plain tape symbols can be turned into variables.
    UnsupportedTapeSymbol {
        from: StateId,
        to: StateId,
        symbol: String,
    },
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::NoInitialState => f.write_str("There must be an initial state!"),
            ConvertError::TooManyStates { .. } => {
                f.write_str("There may be at most 26 states for conversion.")
            }
            ConvertError::CapitalInTransition { .. } => {
                f.write_str("Capital letters are reserved for grammar variables.")
            }
            ConvertError::EmptyGrammar => f.write_str("The grammar should exist."),
            ConvertError::NotRightLinear { .. } => {
                f.write_str("The production is not right linear.")
            }
            ConvertError::NotContextFree { production } => {
                write!(
                    f,
                    "The production {production} is unrestricted on the left hand side."
                )
            }
            ConvertError::NotExactlyOneFinalState { .. } => {
                f.write_str("There must be exactly one final state!")
            }
            ConvertError::FinalTransitionsMustPopZ { .. } => {
                f.write_str("Transitions to final must pop only 'Z'.")
            }
            ConvertError::TransitionsMustPopOnePushZeroOrTwo { .. } => {
                f.write_str("Transitions must pop 1 and push 0 or 2.")
            }
            ConvertError::TooManyVariables { .. } => {
                f.write_str("There are more variables than can be uniquely represented.")
            }
            ConvertError::NotSingleTape { tapes } => write!(
                f,
                "The conversion to a grammar needs a one-tape machine, found {tapes} tapes."
            ),
            ConvertError::UnsupportedBlocks => {
                f.write_str("The conversion to a grammar does not support building blocks.")
            }
            ConvertError::UnsupportedTapeSymbol { symbol, .. } => {
                write!(
                    f,
                    "The tape symbol {symbol:?} cannot be converted to a grammar."
                )
            }
        }
    }
}

impl std::error::Error for ConvertError {}

/// Position of the `index`-th created state when a construction lays its states in a row.
fn row_point(index: usize) -> Point {
    Point {
        x: 80.0 + 160.0 * index as f64,
        y: 120.0,
    }
}
