//! JFLAP `.jff` file format (read and write).
//!
//! `xml` holds a small element tree and JFLAP's pretty writer, `codec` the generic
//! automaton reader/writer (machine labels are closures), and this module the finite
//! automaton entry points. `machines` (PDA, Mealy, Moore), `tm` (Turing machines with tapes
//! and building blocks) and `text` (grammars, regular expressions) hold the other types, and
//! `structure` the entry point that opens any of them. The element names of every structure
//! type are in `FORMAT.md`.

mod codec;
mod error;
mod machines;
mod structure;
mod text;
mod tm;
mod xml;

pub use codec::{read_automaton, structure_type, structure_type_of, write_automaton};
pub use error::JffError;
pub use machines::{read_mealy, read_moore, read_pda, write_mealy, write_moore, write_pda};
pub use structure::{Structure, read_structure, write_structure};
pub use text::{read_grammar, read_regex, write_grammar, write_regex};
pub use tm::{read_turing, write_turing};
pub use xml::{Element, Node, parse_document, write_document};

use crate::fa::{Fa, FaLabel};

/// Reads a finite automaton from `.jff` text. An empty or missing `<read/>` is λ.
pub fn read_fa(xml: &str) -> Result<Fa, JffError> {
    let root = parse_document(xml)?;
    let found = structure_type_of(&root)?;
    if found != "fa" {
        return Err(JffError::WrongType {
            expected: "fa".to_string(),
            found,
        });
    }
    read_automaton(&root, |t| {
        Ok(FaLabel(t.child_text("read").unwrap_or("").to_string()))
    })
}

/// Writes a finite automaton as `.jff` text in the layout JFLAP 7.1 writes.
pub fn write_fa(fa: &Fa) -> String {
    let root = write_automaton(fa, "fa", |label, t| {
        t.push(Element::with_text("read", &label.0));
    });
    write_document(&root)
}
