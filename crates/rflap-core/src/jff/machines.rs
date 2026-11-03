//! `.jff` readers and writers for pushdown automata, Mealy and Moore machines
//! (JFLAP `PDATransducer`, `MealyTransducer`, `MooreTransducer`).

use super::codec::{read_automaton, structure_type_of, write_automaton};
use super::error::JffError;
use super::xml::{Element, Node, parse_document, write_document};
use crate::automaton::StateId;
use crate::mealy::{Mealy, MealyLabel, Moore, MooreLabel};
use crate::pda::{Pda, PdaLabel};

/// Parses a document and checks its `<type>`.
pub(super) fn parse_typed(xml: &str, expected: &str) -> Result<Element, JffError> {
    let root = parse_document(xml)?;
    let found = structure_type_of(&root)?;
    if found != expected {
        return Err(JffError::WrongType {
            expected: expected.to_string(),
            found,
        });
    }
    Ok(root)
}

/// The `<automaton>` element of a written structure.
pub(super) fn automaton_node_mut(root: &mut Element) -> Option<&mut Element> {
    root.children.iter_mut().find_map(|n| match n {
        Node::Element(e) if e.name == "automaton" => Some(e),
        _ => None,
    })
}

fn text_or_empty<'a>(t: &'a Element, name: &str) -> &'a str {
    t.child_text(name).unwrap_or("")
}

/// Reads a pushdown automaton from `.jff` text. Missing `<read>`, `<pop>` and `<push>` are
/// empty.
pub fn read_pda(xml: &str) -> Result<Pda, JffError> {
    let root = parse_typed(xml, "pda")?;
    read_automaton(&root, |t| {
        Ok(PdaLabel::new(
            text_or_empty(t, "read"),
            text_or_empty(t, "pop"),
            text_or_empty(t, "push"),
        ))
    })
}

/// Writes a pushdown automaton as `.jff` text in the layout JFLAP 7.1 writes.
pub fn write_pda(pda: &Pda) -> String {
    let root = write_automaton(pda, "pda", |label, t| {
        t.push(Element::with_text("read", &label.input));
        t.push(Element::with_text("pop", &label.pop));
        t.push(Element::with_text("push", &label.push));
    });
    write_document(&root)
}

/// Reads a Mealy machine from `.jff` text. Missing `<read>` and `<transout>` are empty.
pub fn read_mealy(xml: &str) -> Result<Mealy, JffError> {
    let root = parse_typed(xml, "mealy")?;
    read_automaton(&root, |t| {
        Ok(MealyLabel::new(
            text_or_empty(t, "read"),
            text_or_empty(t, "transout"),
        ))
    })
}

/// Writes a Mealy machine as `.jff` text in the layout JFLAP 7.1 writes.
pub fn write_mealy(mealy: &Mealy) -> String {
    let root = write_automaton(mealy, "mealy", |label, t| {
        t.push(Element::with_text("read", &label.input));
        t.push(Element::with_text("transout", &label.output));
    });
    write_document(&root)
}

/// Reads a Moore machine from `.jff` text. The output of a state is its `<output>`; an
/// empty output is no output. JFLAP copies the `<transout>` of every transition into the
/// output of the transition's destination when it loads a file; so does this reader, but
/// only for transitions that have a `<transout>` element.
pub fn read_moore(xml: &str) -> Result<Moore, JffError> {
    let root = parse_typed(xml, "moore")?;
    let mut m: Moore = read_automaton(&root, |t| {
        Ok(MooreLabel(text_or_empty(t, "read").to_string()))
    })?;
    let node = root.descendant("automaton").unwrap_or(&root);
    for t in node.children_named("transition") {
        let (Some(transout), Some(to)) = (t.child_text("transout"), t.child_text("to")) else {
            continue;
        };
        if let Some(state) = to.parse::<StateId>().ok().and_then(|id| m.state_mut(id)) {
            state.output = Some(transout.to_string());
        }
    }
    let ids: Vec<StateId> = m.states().map(|s| s.id).collect();
    for id in ids {
        if let Some(state) = m.state_mut(id)
            && state.output.as_deref() == Some("")
        {
            state.output = None;
        }
    }
    Ok(m)
}

/// Writes a Moore machine as `.jff` text. Like JFLAP, every state gets an `<output>` and
/// every transition the output of its destination as `<transout>`.
pub fn write_moore(moore: &Moore) -> String {
    let mut root = write_automaton(moore, "moore", |label, t| {
        t.push(Element::with_text("read", &label.0));
    });
    if let Some(node) = automaton_node_mut(&mut root) {
        for child in &mut node.children {
            let Node::Element(el) = child else { continue };
            match el.name.as_str() {
                "state" if el.child("output").is_none() => el.push(Element::new("output")),
                "transition" => {
                    let output = el
                        .child_text("to")
                        .and_then(|to| to.parse::<StateId>().ok())
                        .map(|id| crate::mealy::moore_output(moore, id))
                        .unwrap_or("");
                    el.push(Element::with_text("transout", output));
                }
                _ => {}
            }
        }
    }
    write_document(&root)
}
