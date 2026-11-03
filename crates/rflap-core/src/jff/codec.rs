//! Generic automaton codec (JFLAP `AutomatonTransducer`); machine-specific parts are closures.

use std::collections::BTreeMap;

use super::error::JffError;
use super::xml::{Element, parse_document};
use crate::automaton::{Automaton, Label, Note, Point, State, StateId, Transition};

/// The `<type>` text of a parsed document (JFLAP `TransducerFactory.getTransducer`).
pub fn structure_type_of(root: &Element) -> Result<String, JffError> {
    root.descendant("type")
        .map(|t| t.text.clone())
        .ok_or_else(|| JffError::missing("type", &root.name))
}

/// The structure type named by a `.jff` text: `fa`, `pda`, `turing`, `mealy`, `moore`,
/// `grammar`, `re`, `lsystem`, `regular pumping lemma`, `context-free pumping lemma`.
pub fn structure_type(xml: &str) -> Result<String, JffError> {
    structure_type_of(&parse_document(xml)?)
}

fn parse_id(text: &str, what: &str) -> Result<StateId, JffError> {
    text.parse().map_err(|_| JffError::bad_number(what, text))
}

fn parse_f64(text: &str, what: &str) -> Result<f64, JffError> {
    match text.trim().parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(JffError::bad_number(what, text)),
    }
}

/// Reads `<x>`/`<y>`; a missing coordinate counts as 0 (JFLAP lays such states out).
fn read_point(el: &Element, what: &str) -> Result<Point, JffError> {
    let coord = |name: &str| match el.child(name) {
        Some(c) => parse_f64(&c.text, &format!("{name} of {what}")),
        None => Ok(0.0),
    };
    Ok(Point {
        x: coord("x")?,
        y: coord("y")?,
    })
}

fn read_state<L: Label>(el: &Element, a: &mut Automaton<L>) -> Result<(), JffError> {
    let id = parse_id(el.attr("id").unwrap_or(""), "id of a state")?;
    let what = format!("state {id}");
    let point = read_point(el, &what)?;
    if a.create_state_with_id(id, point).is_none() {
        return Err(JffError::DuplicateState(id));
    }
    let state = a.state_mut(id).expect("state was just created");
    state.name = match (el.child_text("name"), el.attr("name")) {
        (Some(n), _) => n.to_string(),
        (None, Some(n)) if !n.is_empty() => n.to_string(),
        _ => format!("q{id}"),
    };
    state.label = el
        .child_text("label")
        .filter(|l| !l.is_empty())
        .map(str::to_string);
    state.output = el.child_text("output").map(str::to_string);
    if el.child("final").is_some() {
        a.set_final(id, true);
    }
    if el.child("initial").is_some() {
        a.set_initial(Some(id));
    }
    Ok(())
}

fn read_endpoint<L: Label>(
    el: &Element,
    name: &str,
    a: &Automaton<L>,
) -> Result<StateId, JffError> {
    let text = el
        .child_text(name)
        .ok_or_else(|| JffError::missing(name, &el.name))?;
    let id = parse_id(text, &format!("{name} of a transition"))?;
    if a.state(id).is_none() {
        return Err(JffError::UnknownState {
            id,
            context: format!("{name} of a transition"),
        });
    }
    Ok(id)
}

fn read_control(el: &Element) -> Result<Option<Point>, JffError> {
    match (el.child_text("controlx"), el.child_text("controly")) {
        (Some(x), Some(y)) => Ok(Some(Point {
            x: parse_f64(x, "controlx of a transition")?,
            y: parse_f64(y, "controly of a transition")?,
        })),
        _ => Ok(None),
    }
}

/// Reads the automaton of a `<structure>` element. States and transitions are looked up
/// in the first `<automaton>` descendant (JFLAP 7 layout) or else directly in `root`
/// (older layout). `read_label` builds the label from a `<transition>` element.
pub fn read_automaton<L: Label>(
    root: &Element,
    read_label: impl Fn(&Element) -> Result<L, JffError>,
) -> Result<Automaton<L>, JffError> {
    let node = root.descendant("automaton").unwrap_or(root);
    let mut a = Automaton::new();
    // JFLAP creates states in id order, whatever the order in the file.
    let mut states: BTreeMap<StateId, &Element> = BTreeMap::new();
    for el in node.children_named("state") {
        let id = parse_id(el.attr("id").unwrap_or(""), "id of a state")?;
        if states.insert(id, el).is_some() {
            return Err(JffError::DuplicateState(id));
        }
    }
    for el in states.values() {
        read_state(el, &mut a)?;
    }
    for el in node.children_named("transition") {
        let from = read_endpoint(el, "from", &a)?;
        let to = read_endpoint(el, "to", &a)?;
        let label = read_label(el)?;
        let mut t = Transition::new(from, to, label);
        t.control = read_control(el)?;
        a.add_transition(t);
    }
    for el in node.children_named("note") {
        let Some(text) = el.child("text").map(|t| t.text.clone()) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        let point = read_point(el, &format!("the note \"{text}\""))?;
        a.add_note(Note { point, text });
    }
    Ok(a)
}

/// Java `Double.toString` for the values a canvas holds.
fn number(v: f64) -> String {
    format!("{v:?}")
}

fn write_state<L: Label>(a: &Automaton<L>, s: &State) -> Element {
    let mut el = Element::new("state");
    el.set_attr("id", &s.id.to_string());
    el.set_attr("name", &s.name);
    el.push(Element::with_text("x", &number(s.point.x)));
    el.push(Element::with_text("y", &number(s.point.y)));
    if let Some(label) = &s.label {
        el.push(Element::with_text("label", label));
    }
    if a.initial() == Some(s.id) {
        el.push(Element::new("initial"));
    }
    if a.is_final(s.id) {
        el.push(Element::new("final"));
    }
    if let Some(output) = &s.output {
        el.push(Element::with_text("output", output));
    }
    el
}

/// Builds the `<structure>` element of an automaton in the JFLAP 7 layout. `write_label`
/// appends the label elements (`<read>`, ...) to each `<transition>`, after `<from>`,
/// `<to>` and the control point. The caller may add more children before serializing
/// with [`write_document`](super::write_document).
pub fn write_automaton<L: Label>(
    a: &Automaton<L>,
    type_name: &str,
    write_label: impl Fn(&L, &mut Element),
) -> Element {
    let mut root = Element::new("structure");
    root.push(Element::with_text("type", type_name));
    let mut node = Element::new("automaton");
    if a.states().next().is_some() {
        node.push_comment("The list of states.");
    }
    for s in a.states() {
        node.push(write_state(a, s));
    }
    if !a.transitions().is_empty() {
        node.push_comment("The list of transitions.");
    }
    for t in a.transitions() {
        let mut el = Element::new("transition");
        el.push(Element::with_text("from", &t.from.to_string()));
        el.push(Element::with_text("to", &t.to.to_string()));
        if let Some(c) = t.control {
            // JFLAP stores the control point as integers.
            el.push(Element::with_text(
                "controlx",
                &(c.x.round() as i64).to_string(),
            ));
            el.push(Element::with_text(
                "controly",
                &(c.y.round() as i64).to_string(),
            ));
        }
        write_label(&t.label, &mut el);
        node.push(el);
    }
    for n in a.notes() {
        let mut el = Element::new("note");
        el.push(Element::with_text("text", &n.text));
        el.push(Element::with_text("x", &number(n.point.x)));
        el.push(Element::with_text("y", &number(n.point.y)));
        node.push(el);
    }
    root.push(node);
    root
}
