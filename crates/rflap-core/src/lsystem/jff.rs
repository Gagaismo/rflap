//! `.jff` files of type `lsystem` (JFLAP `LSystemTransducer`).

use std::collections::HashMap;

use super::LSystem;
use crate::jff::{Element, JffError, parse_document, structure_type_of, write_document};

const COMMENT_AXIOM: &str = "The L-system axiom.";
const COMMENT_RULE: &str = "The rewriting rules.";
const COMMENT_PARAMETER: &str = "The drawing parameters.";

/// `getElementsByTagName`: every descendant with the name, in document order.
fn descendants<'a>(el: &'a Element, name: &str, out: &mut Vec<&'a Element>) {
    for child in el.elements() {
        if child.name == name {
            out.push(child);
        }
        descendants(child, name, out);
    }
}

fn all<'a>(el: &'a Element, name: &str) -> Vec<&'a Element> {
    let mut out = Vec::new();
    descendants(el, name, &mut out);
    out
}

/// `containedText`: the text of the element, `None` when it has none.
fn contained_text(el: &Element) -> Option<&str> {
    (!el.text.is_empty()).then_some(el.text.as_str())
}

/// `elementsToText`: tag name to text of the child elements, the last one winning.
fn elements_to_text(el: &Element) -> HashMap<&str, Option<&str>> {
    el.elements()
        .map(|c| (c.name.as_str(), contained_text(c)))
        .collect()
}

/// Reads an L-system from `.jff` text.
pub fn read(xml: &str) -> Result<LSystem, JffError> {
    let root = parse_document(xml)?;
    let found = structure_type_of(&root)?;
    if found != "lsystem" {
        return Err(JffError::WrongType {
            expected: "lsystem".to_string(),
            found,
        });
    }
    let axiom = all(&root, "axiom")
        .last()
        .map(|a| contained_text(a).unwrap_or("").to_string())
        .ok_or_else(|| JffError::missing("axiom", &root.name))?;

    let mut productions: Vec<(String, String)> = Vec::new();
    for rule in all(&root, "production") {
        let left = elements_to_text(rule)
            .get("left")
            .copied()
            .flatten()
            .unwrap_or("")
            .to_string();
        for right in all(rule, "right") {
            productions.push((
                left.clone(),
                contained_text(right).unwrap_or("").to_string(),
            ));
        }
    }

    let mut parameters: Vec<(String, String)> = Vec::new();
    for parameter in all(&root, "parameter") {
        let map = elements_to_text(parameter);
        let Some(name) = map.get("name").copied().flatten() else {
            continue;
        };
        parameters.push((
            name.to_string(),
            map.get("value")
                .copied()
                .flatten()
                .unwrap_or("")
                .to_string(),
        ));
    }

    let productions: Vec<(&str, &str)> = productions
        .iter()
        .map(|(l, r)| (l.as_str(), r.as_str()))
        .collect();
    let parameters: Vec<(&str, &str)> = parameters
        .iter()
        .map(|(n, v)| (n.as_str(), v.as_str()))
        .collect();
    Ok(LSystem::new(&axiom, &productions, &parameters))
}

/// Writes an L-system as `.jff` text in the layout JFLAP 7.1 writes.
pub fn write(lsystem: &LSystem) -> String {
    let mut root = Element::new("structure");
    root.push(Element::with_text("type", "lsystem"));
    root.push_comment(COMMENT_AXIOM);
    root.push(Element::with_text("axiom", &lsystem.axiom().join(" ")));
    let mut symbols = lsystem.symbols_with_replacements().peekable();
    if symbols.peek().is_some() {
        root.push_comment(COMMENT_RULE);
    }
    for symbol in symbols {
        let mut production = Element::new("production");
        production.push(Element::with_text("left", symbol));
        for right in lsystem.replacements(symbol) {
            production.push(Element::with_text("right", &right.join(" ")));
        }
        root.push(production);
    }
    let parameters = lsystem.parameters_in_jflap_order();
    if !parameters.is_empty() {
        root.push_comment(COMMENT_PARAMETER);
    }
    for (name, value) in parameters {
        let mut parameter = Element::new("parameter");
        parameter.push(Element::with_text("name", name));
        parameter.push(Element::with_text("value", value));
        root.push(parameter);
    }
    write_document(&root)
}
