//! `open_jff` and `save_jff`.

use rflap_core::jff::{self, JffError, Structure};
use rflap_core::lsystem;
use rflap_core::pumping::{self, Game};
use serde_json::{Value, json};

use crate::document::{Document, RegexDoc};
use crate::error::{ApiError, code};

fn jff_error(e: JffError) -> ApiError {
    ApiError::new(code::JFF_ERROR, e.to_string())
}

/// Reads a `.jff` file of any type JFLAP 7.1 writes.
pub fn open(xml: &str) -> Result<Value, ApiError> {
    let kind = jff::structure_type(xml).map_err(jff_error)?;
    let document = match kind.as_str() {
        "lsystem" => Document::Lsystem(lsystem::jff::read(xml).map_err(jff_error)?),
        "regular pumping lemma" => Document::RegularPumpingLemma(Game::from_lemma(
            pumping::jff::read(xml).map_err(jff_error)?,
        )),
        "context-free pumping lemma" => Document::ContextFreePumpingLemma(Game::from_lemma(
            pumping::jff::read(xml).map_err(jff_error)?,
        )),
        _ => match jff::read_structure(xml).map_err(jff_error)? {
            Structure::Fa(a) => Document::Fa(a),
            Structure::Pda(a) => Document::Pda(a),
            Structure::Turing(a) => Document::Turing(a),
            Structure::Mealy(a) => Document::Mealy(a),
            Structure::Moore(a) => Document::Moore(a),
            Structure::Grammar(g) => Document::Grammar(g),
            Structure::Regex(r) => Document::Regex(RegexDoc::of(&r)),
        },
    };
    Ok(json!({"document": document}))
}

/// Writes a document as `.jff` text.
pub fn save(document: &Document) -> Result<Value, ApiError> {
    let xml = match document {
        Document::Fa(a) => jff::write_fa(a),
        Document::Pda(a) => jff::write_pda(a),
        Document::Turing(a) => jff::write_turing(a),
        Document::Mealy(a) => jff::write_mealy(a),
        Document::Moore(a) => jff::write_moore(a),
        Document::Grammar(g) => jff::write_grammar(g),
        Document::Regex(r) => jff::write_regex(&r.parse()?),
        Document::Lsystem(l) => lsystem::jff::write(l),
        Document::RegularPumpingLemma(g) | Document::ContextFreePumpingLemma(g) => {
            pumping::jff::write(g.lemma())
        }
    };
    Ok(json!({"xml": xml, "type": document.type_name()}))
}
