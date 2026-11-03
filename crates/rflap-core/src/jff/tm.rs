//! `.jff` reader and writer for Turing machines (JFLAP `TMTransducer`): several tapes and
//! building blocks.
//!
//! JFLAP writes every state as a `<block>` with a `<tag>`, and after the transitions one
//! element per tag, named after it, that holds the inner machine of that block (empty for
//! an ordinary state). Reading finds the element of a tag as the first element of the
//! document with that name, like JFLAP does.

use std::collections::BTreeMap;

use super::codec::{read_automaton, write_automaton};
use super::error::JffError;
use super::machines::parse_typed;
use super::xml::{Element, Node, write_document};
use crate::automaton::StateId;
use crate::turing::{BLANK, TapeAction, TmLabel, TuringMachine};

const MAX_TAPES: usize = 5;

fn read_tapes(root: &Element) -> Result<usize, JffError> {
    let Some(text) = root.child_text("tapes") else {
        return Ok(1);
    };
    match text.trim().parse::<usize>() {
        Ok(n) if (1..=MAX_TAPES).contains(&n) => Ok(n),
        _ => Err(JffError::bad_number("number of tapes (1 to 5)", text)),
    }
}

/// The 0-based tape a `<read>`, `<write>` or `<move>` element belongs to.
fn tape_of(el: &Element, tapes: usize) -> Result<usize, JffError> {
    let text = el.attr("tape").filter(|t| !t.is_empty()).unwrap_or("1");
    let tape: usize = text
        .parse()
        .map_err(|_| JffError::bad_number(format!("tape of <{}>", el.name), text))?;
    if tape < 1 || tape > tapes {
        return Err(JffError::TapeOutOfRange { tape, tapes });
    }
    Ok(tape - 1)
}

/// The label of a `<transition>`: one action per tape. A block transition
/// (`block="true"`) only has `<read>`; JFLAP reads it as write `~`, move `S`.
fn read_label(t: &Element, tapes: usize) -> Result<TmLabel, JffError> {
    let block = t.attr("block") == Some("true");
    let mut read = vec![String::new(); tapes];
    let mut write = vec![String::new(); tapes];
    let mut moves = vec!["R".to_string(); tapes];
    let mut present = vec![false; tapes];
    for (name, values) in [
        ("read", &mut read),
        ("write", &mut write),
        ("move", &mut moves),
    ] {
        for el in t.children_named(name) {
            let tape = tape_of(el, tapes)?;
            present[tape] = true;
            values[tape] = el.text.clone();
        }
    }
    let found = present.iter().filter(|p| **p).count();
    if found != tapes {
        return Err(JffError::TapeCount {
            expected: tapes,
            found,
        });
    }
    if block {
        write.iter_mut().for_each(|w| *w = "~".to_string());
        moves.iter_mut().for_each(|m| *m = "S".to_string());
    }
    let actions = (0..tapes)
        .map(|i| TapeAction::new(&read[i], &write[i], &moves[i]))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| JffError::BadLabel(e.to_string()))?;
    TmLabel::new(actions).map_err(|e| JffError::BadLabel(e.to_string()))
}

/// Reads one machine from `node` (an `<automaton>` or the element of a block tag).
/// `open` holds the tags of the blocks being read, to catch a block that contains itself.
fn read_machine(
    doc: &Element,
    node: &Element,
    tapes: usize,
    open: &mut Vec<String>,
) -> Result<TuringMachine, JffError> {
    // Keep what the generic reader understands, with <block> read as <state>.
    let mut shell = Element::new("automaton");
    let mut tags: Vec<(StateId, String)> = Vec::new();
    for el in node.elements() {
        match el.name.as_str() {
            "block" | "state" => {
                let id_text = el.attr("id").unwrap_or("");
                let id: StateId = id_text
                    .parse()
                    .map_err(|_| JffError::bad_number("id of a state", id_text))?;
                if let Some(tag) = el.child_text("tag") {
                    tags.push((id, tag.to_string()));
                }
                let mut state = el.clone();
                state.name = "state".to_string();
                shell.push(state);
            }
            "transition" | "note" => shell.push(el.clone()),
            _ => {}
        }
    }
    let mut holder = Element::new("structure");
    holder.push(shell);
    let automaton = read_automaton(&holder, |t| read_label(t, tapes))?;

    let mut blocks: BTreeMap<StateId, TuringMachine> = BTreeMap::new();
    for (id, tag) in tags {
        if open.contains(&tag) {
            return Err(JffError::BlockCycle { tag });
        }
        let inner_node = doc
            .descendant(&tag)
            .ok_or_else(|| JffError::UnknownBlock { tag: tag.clone() })?;
        open.push(tag);
        let inner = read_machine(doc, inner_node, tapes, open)?;
        open.pop();
        // A machine without states is an ordinary state.
        if inner.automaton().states().next().is_some() {
            blocks.insert(id, inner);
        }
    }
    // The model keeps its parts private; its serde form is the way to build it whole,
    // control points and notes included.
    let value = serde_json::json!({
        "automaton": serde_json::to_value(&automaton).map_err(internal)?,
        "tapes": tapes,
        "blocks": serde_json::to_value(&blocks).map_err(internal)?,
    });
    serde_json::from_value(value).map_err(internal)
}

fn internal(e: serde_json::Error) -> JffError {
    JffError::BadXml(format!("could not build the machine: {e}"))
}

/// Reads a Turing machine (1 to 5 tapes, building blocks included) from `.jff` text.
/// A transition must describe every tape of the machine.
pub fn read_turing(xml: &str) -> Result<TuringMachine, JffError> {
    let root = parse_typed(xml, "turing")?;
    let tapes = read_tapes(&root)?;
    let node = root.descendant("automaton").unwrap_or(&root);
    read_machine(&root, node, tapes, &mut Vec::new())
}

fn write_label(label: &TmLabel, t: &mut Element) {
    let multi = label.tapes() > 1;
    for (i, a) in label.actions().iter().enumerate() {
        let part = |name: &str, text: &str| {
            let mut el = Element::with_text(name, text);
            if multi {
                el.set_attr("tape", &(i + 1).to_string());
            }
            el
        };
        let blank = |s: &str| {
            if s.chars().eq([BLANK]) {
                String::new()
            } else {
                s.to_string()
            }
        };
        t.push(part("read", &blank(a.read())));
        t.push(part("write", &blank(&a.write().to_string())));
        t.push(part("move", &a.direction().symbol().to_string()));
    }
}

/// The `<automaton>` node of a machine. Block tags are `<prefix><id>`, where the prefix
/// carries the path of the enclosing blocks, so no two blocks of a file share a tag: JFLAP
/// resolves a tag to the first element with that name, and it cannot reload a file where
/// the default `MachineN` names of different levels collide.
fn write_machine(tm: &TuringMachine, prefix: &str) -> Element {
    let root = write_automaton(tm.automaton(), "turing", write_label);
    let mut node = root
        .children_named("automaton")
        .next()
        .cloned()
        .expect("write_automaton writes an <automaton>");
    let mut machines: Vec<Node> = Vec::new();
    for child in &mut node.children {
        let Node::Element(el) = child else { continue };
        if el.name != "state" {
            continue;
        }
        let id: StateId = el.attr("id").and_then(|i| i.parse().ok()).unwrap_or(0);
        let inner = tm
            .block(id)
            .filter(|b| b.automaton().states().next().is_some());
        let tag = match inner {
            Some(_) => format!("{prefix}{id}"),
            None => format!("Machine{id}"),
        };
        // <block> = <state> + <tag> first and no <output>.
        let mut block = Element::new("block");
        block.attributes = el.attributes.clone();
        block.push(Element::with_text("tag", &tag));
        for c in el.elements().filter(|c| c.name != "output") {
            block.push(c.clone());
        }
        *el = block;
        let mut machine = match inner {
            Some(b) => write_machine(b, &format!("{tag}_")),
            None => Element::new("automaton"),
        };
        machine.name = tag;
        machines.push(Node::Element(machine));
    }
    if !machines.is_empty() {
        let at = node
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.name == "note"))
            .unwrap_or(node.children.len());
        machines.insert(0, Node::Comment("The list of automata".to_string()));
        node.children.splice(at..at, machines);
    }
    node
}

/// Writes a Turing machine as `.jff` text in the layout JFLAP 7.1 writes: `<tapes>` for
/// more than one tape, a `<block>` per state and one element per block with its inner
/// machine. A state label is written as `<label>` in its `<block>` (JFLAP reads it back,
/// but does not write it).
pub fn write_turing(tm: &TuringMachine) -> String {
    let mut root = Element::new("structure");
    root.push(Element::with_text("type", "turing"));
    if tm.tapes() > 1 {
        root.push(Element::with_text("tapes", &tm.tapes().to_string()));
    }
    root.push(write_machine(tm, "Block"));
    write_document(&root)
}
