//! The single entry point that opens any supported `.jff` (JFLAP `TransducerFactory`).

use serde::{Deserialize, Serialize};

use super::codec::structure_type;
use super::error::JffError;
use super::machines::{read_mealy, read_moore, read_pda, write_mealy, write_moore, write_pda};
use super::text::{read_grammar, read_regex, write_grammar, write_regex};
use super::tm::{read_turing, write_turing};
use super::{read_fa, write_fa};
use crate::fa::Fa;
use crate::grammar::Grammar;
use crate::mealy::{Mealy, Moore};
use crate::pda::Pda;
use crate::regex::Regex;
use crate::turing::TuringMachine;

/// Any structure a `.jff` file can hold that this module reads and writes. L-systems and
/// pumping lemma games have their own readers (`lsystem::jff`, `pumping::jff`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum Structure {
    Fa(Fa),
    Pda(Pda),
    Turing(TuringMachine),
    Mealy(Mealy),
    Moore(Moore),
    Grammar(Grammar),
    Regex(Regex),
}

/// Reads a `.jff` text of any supported type, dispatching on `<type>`. Types this module
/// does not read (`lsystem`, the pumping lemma games, unknown ones) are
/// [`JffError::Unsupported`].
pub fn read_structure(xml: &str) -> Result<Structure, JffError> {
    match structure_type(xml)?.as_str() {
        "fa" => read_fa(xml).map(Structure::Fa),
        "pda" => read_pda(xml).map(Structure::Pda),
        "turing" => read_turing(xml).map(Structure::Turing),
        "mealy" => read_mealy(xml).map(Structure::Mealy),
        "moore" => read_moore(xml).map(Structure::Moore),
        "grammar" => read_grammar(xml).map(Structure::Grammar),
        "re" => read_regex(xml).map(Structure::Regex),
        other => Err(JffError::Unsupported {
            structure_type: other.to_string(),
        }),
    }
}

/// Writes any structure as `.jff` text.
pub fn write_structure(structure: &Structure) -> String {
    match structure {
        Structure::Fa(a) => write_fa(a),
        Structure::Pda(a) => write_pda(a),
        Structure::Turing(a) => write_turing(a),
        Structure::Mealy(a) => write_mealy(a),
        Structure::Moore(a) => write_moore(a),
        Structure::Grammar(g) => write_grammar(g),
        Structure::Regex(r) => write_regex(r),
    }
}
