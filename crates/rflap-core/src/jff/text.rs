//! `.jff` readers and writers for grammars and regular expressions
//! (JFLAP `GrammarTransducer`, `RETransducer`).

use super::error::JffError;
use super::machines::parse_typed;
use super::xml::{Element, Node, write_document};
use crate::grammar::{Grammar, Production};
use crate::regex::Regex;

fn collect_productions<'a>(el: &'a Element, out: &mut Vec<&'a Element>) {
    for child in el.elements() {
        if child.name == "production" {
            out.push(child);
        }
        collect_productions(child, out);
    }
}

/// Reads a grammar from `.jff` text. Every `<production>` is read in order; a missing
/// `<right>` is the empty string (λ). The start variable is the left side of the first
/// production, since the file does not store one.
pub fn read_grammar(xml: &str) -> Result<Grammar, JffError> {
    let root = parse_typed(xml, "grammar")?;
    let mut elements = Vec::new();
    collect_productions(&root, &mut elements);
    let mut productions = Vec::with_capacity(elements.len());
    for (i, el) in elements.iter().enumerate() {
        let n = i + 1;
        match el.child_text("left") {
            None => {
                return Err(JffError::BadProduction(format!(
                    "production {n} has no <left>"
                )));
            }
            Some("") => {
                return Err(JffError::BadProduction(format!(
                    "production {n} has an empty left side"
                )));
            }
            Some(left) => {
                productions.push(Production::new(left, el.child_text("right").unwrap_or("")))
            }
        }
    }
    Grammar::from_productions(productions).map_err(|e| JffError::BadProduction(e.to_string()))
}

/// Writes a grammar as `.jff` text in the layout JFLAP 7.1 writes.
pub fn write_grammar(grammar: &Grammar) -> String {
    let mut root = Element::new("structure");
    root.push(Element::with_text("type", "grammar"));
    if !grammar.productions().is_empty() {
        root.push_comment("The list of productions.");
    }
    for p in grammar.productions() {
        let mut el = Element::new("production");
        el.push(Element::with_text("left", &p.lhs));
        el.push(Element::with_text("right", &p.rhs));
        root.children.push(Node::Element(el));
    }
    write_document(&root)
}

/// Reads a regular expression from `.jff` text. A missing `<expression>` is the empty
/// expression.
pub fn read_regex(xml: &str) -> Result<Regex, JffError> {
    let root = parse_typed(xml, "re")?;
    let text = root.child_text("expression").unwrap_or("");
    Regex::parse(text).map_err(|e| JffError::BadExpression(e.to_string()))
}

/// Writes a regular expression as `.jff` text, with JFLAP's `!` for the empty string.
pub fn write_regex(regex: &Regex) -> String {
    let mut root = Element::new("structure");
    root.push(Element::with_text("type", "re"));
    root.push_comment("The regular expression.");
    root.push(Element::with_text("expression", &regex.to_jflap_string()));
    write_document(&root)
}
