//! JFLAP `.jff` file format (read and write).
//!
//! `xml` holds a small element tree and JFLAP's pretty writer, `codec` the generic
//! automaton reader/writer (machine labels are closures), and this module the finite
//! automaton entry points. The element names of every structure type are in `FORMAT.md`.

mod codec;
mod error;
mod xml;

pub use codec::{read_automaton, structure_type, structure_type_of, write_automaton};
pub use error::JffError;
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
