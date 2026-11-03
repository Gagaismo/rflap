//! The compact document form: what an AI (or a person) writes by hand.
//!
//! States are created by name, in order of first use, and laid out automatically when no
//! coordinates are given. See the crate docs for the syntax of every type; the labels are
//! read here.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;

use rflap_core::automaton::{Automaton, Label, Point, StateId};
use rflap_core::fa::{Fa, FaLabel};
use rflap_core::grammar::Grammar;
use rflap_core::layout::{self, Algorithm, LayoutOptions};
use rflap_core::lsystem::LSystem;
use rflap_core::mealy::{Mealy, MealyLabel, Moore, MooreLabel};
use rflap_core::pda::{Pda, PdaLabel};
use rflap_core::pumping::{Game, Kind, Language, Mode, catalogue};
use rflap_core::turing::catedra::{parse_label, split_labels};
use rflap_core::turing::{TmLabel, TuringMachine};
use serde_json::{Map, Value};

use crate::document::{Document, RegexDoc};
use crate::error::{ApiError, code};

type Object = Map<String, Value>;

/// Whether the object is a compact document of this type (anything else is read as full form).
pub fn is_compact(kind: &str, object: &Object) -> bool {
    match kind {
        // the full form of a Turing machine nests its graph under "automaton"
        "turing" => !object.contains_key("automaton"),
        "fa" | "pda" | "mealy" | "moore" => !matches!(object.get("states"), Some(Value::Object(_))),
        "grammar" => {
            object.contains_key("text")
                || object.contains_key("rules")
                || matches!(object.get("productions"),
                    Some(Value::Array(items)) if items.iter().any(Value::is_string))
        }
        "regex" => true,
        "lsystem" => !matches!(object.get("axiom"), Some(Value::Array(_))),
        _ => !object.contains_key("lemma"),
    }
}

/// Expands a compact document.
pub fn build(kind: &str, object: &Object) -> Result<Document, ApiError> {
    match kind {
        "fa" => {
            check_keys(kind, object, &MACHINE_KEYS)?;
            let mut fa = Fa::new();
            let shape = Shape::read(object)?;
            populate(&mut fa, &shape, 50.0, |s| {
                Ok(FaLabel(lambda(s).to_string()))
            })?;
            Ok(Document::Fa(fa))
        }
        "pda" => {
            check_keys(kind, object, &MACHINE_KEYS)?;
            let mut pda = Pda::new();
            let shape = Shape::read(object)?;
            populate(&mut pda, &shape, 80.0, pda_label)?;
            Ok(Document::Pda(pda))
        }
        "mealy" => {
            check_keys(kind, object, &MACHINE_KEYS)?;
            let mut mealy = Mealy::new();
            let shape = Shape::read(object)?;
            populate(&mut mealy, &shape, 65.0, mealy_label)?;
            Ok(Document::Mealy(mealy))
        }
        "moore" => {
            check_keys(kind, object, &MOORE_KEYS)?;
            let mut moore = Moore::new();
            let shape = Shape::read(object)?;
            populate(&mut moore, &shape, 65.0, |s| {
                Ok(MooreLabel(lambda(s).to_string()))
            })?;
            Ok(Document::Moore(moore))
        }
        "turing" => Ok(Document::Turing(build_turing(object, None)?)),
        "grammar" => build_grammar(object),
        "regex" => build_regex(object),
        "lsystem" => build_lsystem(object),
        _ => build_pumping(kind, object),
    }
}

const MACHINE_KEYS: [&str; 7] = [
    "type",
    "states",
    "initial",
    "finals",
    "transitions",
    "points",
    "layout",
];
const MOORE_KEYS: [&str; 8] = [
    "type",
    "states",
    "initial",
    "finals",
    "transitions",
    "points",
    "layout",
    "outputs",
];
const TURING_KEYS: [&str; 10] = [
    "type",
    "states",
    "initial",
    "finals",
    "transitions",
    "points",
    "layout",
    "tapes",
    "blocks",
    "input_alphabet",
];

fn check_keys(kind: &str, object: &Object, allowed: &[&str]) -> Result<(), ApiError> {
    match object.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(key) => Err(ApiError::invalid_document(format!(
            "unknown field \"{key}\" in a compact {kind} document; the fields are {}",
            allowed
                .iter()
                .filter(|k| **k != "type")
                .copied()
                .collect::<Vec<_>>()
                .join(", ")
        ))),
        None => Ok(()),
    }
}

/// `λ`, `ε` and the empty string all mean "nothing".
fn lambda(s: &str) -> &str {
    match s.trim() {
        "λ" | "ε" => "",
        t => t,
    }
}

fn bad_label(what: &str, label: &str, expected: &str) -> ApiError {
    ApiError::new(
        code::INVALID_LABEL,
        format!("invalid {what} label `{label}`: expected {expected}"),
    )
}

/// `input,pop;push` (JFLAP text), any part may be empty or `λ`.
fn pda_label(s: &str) -> Result<PdaLabel, ApiError> {
    let expected = "`input,pop;push` such as `a,Z;aZ` (use λ or nothing for empty parts)";
    let (left, push) = s
        .split_once(';')
        .ok_or_else(|| bad_label("pda", s, expected))?;
    let (input, pop) = left
        .split_once(',')
        .ok_or_else(|| bad_label("pda", s, expected))?;
    Ok(PdaLabel::new(lambda(input), lambda(pop), lambda(push)))
}

/// `input;output`.
fn mealy_label(s: &str) -> Result<MealyLabel, ApiError> {
    let (input, output) = s.split_once(';').ok_or_else(|| {
        bad_label(
            "mealy",
            s,
            "`input;output` such as `a;1` (use λ or nothing for empty)",
        )
    })?;
    Ok(MealyLabel::new(lambda(input), lambda(output)))
}

/// `(read,write,move)` (the course's notation) or `read;write,direction` per tape joined by `|`
/// (JFLAP's); `Δ` and `_` are the blank.
fn tm_label(s: &str) -> Result<TmLabel, ApiError> {
    let expected = "`(read,write,move)` such as `(a,X,D)` (move I, N or D; Δ is the blank) or JFLAP's `read;write,direction` per tape joined by `|` such as `a;b,R` (direction L, R or S; `_` is the blank). Several labels of one edge are joined with ` / `";
    parse_label(s).map_err(|e| bad_label("turing", s, &format!("{expected} ({e})")))
}

/// The common shape of the compact machine documents.
struct Shape {
    declared: Option<Vec<String>>,
    initial: Option<String>,
    finals: Vec<String>,
    transitions: Vec<(String, String, String)>,
    points: BTreeMap<String, Point>,
    outputs: BTreeMap<String, String>,
    layout: Algorithm,
    tapes: Option<usize>,
    blocks: Vec<(String, Value)>,
}

fn name_of(value: &Value, what: &str) -> Result<String, ApiError> {
    let name = match value {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        other => {
            return Err(ApiError::invalid_document(format!(
                "{what} must be a string, got {other}"
            )));
        }
    };
    if name.is_empty() {
        return Err(ApiError::invalid_document(format!(
            "{what} must not be empty"
        )));
    }
    Ok(name)
}

fn name_list(value: &Value, what: &str) -> Result<Vec<String>, ApiError> {
    match value {
        Value::Array(items) => items
            .iter()
            .map(|v| name_of(v, &format!("an entry of {what}")))
            .collect(),
        other => Err(ApiError::invalid_document(format!(
            "{what} must be an array of state names, got {other}"
        ))),
    }
}

/// Algorithm by its name (`circle`, `gem`, `random`, `spiral`, `tree`, `tree-degree`, `two-circle`).
pub fn parse_algorithm(name: &str) -> Result<Algorithm, ApiError> {
    Algorithm::from_str(name)
        .or_else(|_| serde_json::from_value(Value::String(name.to_string())))
        .map_err(|_| {
            ApiError::new(
                code::INVALID_ARGUMENT,
                format!(
                    "unknown layout algorithm `{name}`; the algorithms are {}",
                    Algorithm::ALL.map(Algorithm::name).join(", ")
                ),
            )
        })
}

impl Shape {
    fn read(object: &Object) -> Result<Shape, ApiError> {
        let declared = object
            .get("states")
            .map(|v| name_list(v, "\"states\""))
            .transpose()?;
        if let Some(names) = &declared {
            let mut seen = BTreeSet::new();
            if let Some(dup) = names.iter().find(|n| !seen.insert(n.as_str())) {
                return Err(ApiError::invalid_document(format!(
                    "the state \"{dup}\" is declared twice in \"states\""
                )));
            }
        }
        let initial = object
            .get("initial")
            .filter(|v| !v.is_null())
            .map(|v| name_of(v, "\"initial\""))
            .transpose()?;
        let finals = object
            .get("finals")
            .map(|v| name_list(v, "\"finals\""))
            .transpose()?
            .unwrap_or_default();
        let mut transitions = Vec::new();
        match object.get("transitions") {
            None | Some(Value::Null) => {}
            Some(Value::Array(items)) => {
                for (i, item) in items.iter().enumerate() {
                    transitions.push(transition(i, item)?);
                }
            }
            Some(other) => {
                return Err(ApiError::invalid_document(format!(
                    "\"transitions\" must be an array of [from, label, to], got {other}"
                )));
            }
        }
        let mut points = BTreeMap::new();
        match object.get("points") {
            None | Some(Value::Null) => {}
            Some(Value::Object(map)) => {
                for (name, p) in map {
                    let xy = p
                        .as_array()
                        .filter(|a| a.len() == 2)
                        .and_then(|a| Some((a[0].as_f64()?, a[1].as_f64()?)));
                    let Some((x, y)) = xy else {
                        return Err(ApiError::invalid_document(format!(
                            "the point of \"{name}\" must be [x, y], got {p}"
                        )));
                    };
                    points.insert(name.clone(), Point { x, y });
                }
            }
            Some(other) => {
                return Err(ApiError::invalid_document(format!(
                    "\"points\" must be an object {{\"state\": [x, y]}}, got {other}"
                )));
            }
        }
        let mut outputs = BTreeMap::new();
        match object.get("outputs") {
            None | Some(Value::Null) => {}
            Some(Value::Object(map)) => {
                for (name, out) in map {
                    outputs.insert(name.clone(), name_or_text(out, "an output")?);
                }
            }
            Some(other) => {
                return Err(ApiError::invalid_document(format!(
                    "\"outputs\" must be an object {{\"state\": \"output\"}}, got {other}"
                )));
            }
        }
        let layout = match object.get("layout") {
            None | Some(Value::Null) => Algorithm::Circle,
            Some(Value::String(name)) => parse_algorithm(name)?,
            Some(other) => {
                return Err(ApiError::invalid_document(format!(
                    "\"layout\" must be an algorithm name, got {other}"
                )));
            }
        };
        let tapes = match object.get("tapes") {
            None | Some(Value::Null) => None,
            Some(v) => match v.as_u64() {
                Some(n) if n >= 1 => Some(n as usize),
                _ => {
                    return Err(ApiError::invalid_document(format!(
                        "\"tapes\" must be a positive integer, got {v}"
                    )));
                }
            },
        };
        let blocks = match object.get("blocks") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Object(map)) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            Some(other) => {
                return Err(ApiError::invalid_document(format!(
                    "\"blocks\" must be an object {{\"state\": <turing document>}}, got {other}"
                )));
            }
        };
        Ok(Shape {
            declared,
            initial,
            finals,
            transitions,
            points,
            outputs,
            layout,
            tapes,
            blocks,
        })
    }

    /// State names in creation order. When `states` is declared every name used elsewhere
    /// must be in it.
    fn names(&self) -> Result<Vec<String>, ApiError> {
        let mut order: Vec<String> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        if let Some(declared) = &self.declared {
            for n in declared {
                seen.insert(n.clone());
                order.push(n.clone());
            }
        }
        let strict = self.declared.is_some();
        let mut touch = |name: &str, place: String| -> Result<(), ApiError> {
            if seen.contains(name) {
                return Ok(());
            }
            if strict {
                let known = self.declared.clone().unwrap_or_default();
                return Err(ApiError::new(
                    code::UNKNOWN_STATE,
                    format!(
                        "unknown state \"{name}\" in {place}; the declared states are [{}]",
                        known.join(", ")
                    ),
                )
                .with_details(serde_json::json!({"state": name, "declared": known})));
            }
            seen.insert(name.to_string());
            order.push(name.to_string());
            Ok(())
        };
        if let Some(i) = &self.initial {
            touch(i, "\"initial\"".to_string())?;
        }
        for (i, (from, label, to)) in self.transitions.iter().enumerate() {
            let place = format!("transition #{i} [\"{from}\", \"{label}\", \"{to}\"]");
            touch(from, place.clone())?;
            touch(to, place)?;
        }
        for f in &self.finals {
            touch(f, "\"finals\"".to_string())?;
        }
        for n in self.outputs.keys() {
            touch(n, "\"outputs\"".to_string())?;
        }
        for n in self.points.keys() {
            touch(n, "\"points\"".to_string())?;
        }
        for (n, _) in &self.blocks {
            touch(n, "\"blocks\"".to_string())?;
        }
        Ok(order)
    }
}

fn name_or_text(value: &Value, what: &str) -> Result<String, ApiError> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        other => Err(ApiError::invalid_document(format!(
            "{what} must be a string, got {other}"
        ))),
    }
}

fn transition(i: usize, item: &Value) -> Result<(String, String, String), ApiError> {
    let shape = "[from, label, to] (or {\"from\", \"label\", \"to\"})";
    let bad = |got: &Value| {
        ApiError::invalid_document(format!("transition #{i} must be {shape}, got {got}"))
    };
    let (from, label, to) = match item {
        Value::Array(parts) if parts.len() == 3 => (&parts[0], &parts[1], &parts[2]),
        Value::Object(map) => match (map.get("from"), map.get("label"), map.get("to")) {
            (Some(f), Some(l), Some(t)) => (f, l, t),
            _ => return Err(bad(item)),
        },
        other => return Err(bad(other)),
    };
    let what = format!("a state of transition #{i}");
    Ok((
        name_of(from, &what)?,
        name_or_text(label, &format!("the label of transition #{i}"))?,
        name_of(to, &what)?,
    ))
}

/// The writable face of the machines a compact document can build.
trait Target {
    type L: Label;
    fn auto(&self) -> &Automaton<Self::L>;
    fn create(&mut self, name: &str) -> StateId;
    fn set_initial(&mut self, id: StateId);
    fn set_final(&mut self, id: StateId);
    fn set_output(&mut self, id: StateId, output: &str);
    fn set_point(&mut self, id: StateId, point: Point);
    fn edge(&mut self, from: StateId, to: StateId, label: Self::L) -> Result<(), ApiError>;
}

macro_rules! automaton_target {
    ($label:ty) => {
        impl Target for Automaton<$label> {
            type L = $label;
            fn auto(&self) -> &Automaton<$label> {
                self
            }
            fn create(&mut self, name: &str) -> StateId {
                let id = self.create_state(Point::default());
                if let Some(s) = self.state_mut(id) {
                    s.name = name.to_string();
                }
                id
            }
            fn set_initial(&mut self, id: StateId) {
                Automaton::set_initial(self, Some(id));
            }
            fn set_final(&mut self, id: StateId) {
                Automaton::set_final(self, id, true);
            }
            fn set_output(&mut self, id: StateId, output: &str) {
                if let Some(s) = self.state_mut(id) {
                    s.output = Some(output.to_string());
                }
            }
            fn set_point(&mut self, id: StateId, point: Point) {
                if let Some(s) = self.state_mut(id) {
                    s.point = point;
                }
            }
            fn edge(&mut self, from: StateId, to: StateId, label: $label) -> Result<(), ApiError> {
                self.add_transition(rflap_core::automaton::Transition::new(from, to, label));
                Ok(())
            }
        }
    };
}

automaton_target!(FaLabel);
automaton_target!(PdaLabel);
automaton_target!(MealyLabel);
automaton_target!(MooreLabel);

impl Target for TuringMachine {
    type L = TmLabel;
    fn auto(&self) -> &Automaton<TmLabel> {
        self.automaton()
    }
    fn create(&mut self, name: &str) -> StateId {
        let id = self.create_state(Point::default());
        if let Some(s) = self.state_mut(id) {
            s.name = name.to_string();
        }
        id
    }
    fn set_initial(&mut self, id: StateId) {
        TuringMachine::set_initial(self, Some(id));
    }
    fn set_final(&mut self, id: StateId) {
        TuringMachine::set_final(self, id, true);
    }
    fn set_output(&mut self, id: StateId, output: &str) {
        if let Some(s) = self.state_mut(id) {
            s.output = Some(output.to_string());
        }
    }
    fn set_point(&mut self, id: StateId, point: Point) {
        if let Some(s) = self.state_mut(id) {
            s.point = point;
        }
    }
    fn edge(&mut self, from: StateId, to: StateId, label: TmLabel) -> Result<(), ApiError> {
        self.add_transition(from, to, label)
            .map(|_| ())
            .map_err(|e| ApiError::new(code::INVALID_LABEL, e.to_string()))
    }
}

/// Creates the states and transitions of `shape`, then places the states.
fn populate<T: Target>(
    target: &mut T,
    shape: &Shape,
    vertex_buffer: f64,
    parse: impl Fn(&str) -> Result<T::L, ApiError>,
) -> Result<HashMap<String, StateId>, ApiError> {
    let order = shape.names()?;
    let mut ids: HashMap<String, StateId> = HashMap::new();
    for name in &order {
        let id = target.create(name);
        ids.insert(name.clone(), id);
    }
    if let Some(i) = &shape.initial {
        target.set_initial(ids[i]);
    }
    for f in &shape.finals {
        target.set_final(ids[f]);
    }
    for (name, out) in &shape.outputs {
        target.set_output(ids[name], out);
    }
    for (i, (from, label, to)) in shape.transitions.iter().enumerate() {
        let parsed = parse(label).map_err(|mut e| {
            e.message = format!(
                "transition #{i} [\"{from}\", \"{label}\", \"{to}\"]: {}",
                e.message
            );
            e
        })?;
        target.edge(ids[from], ids[to], parsed)?;
    }
    place(target, shape, &ids, vertex_buffer);
    Ok(ids)
}

/// Gives the named points, then lays out the states that have none.
fn place<T: Target>(
    target: &mut T,
    shape: &Shape,
    ids: &HashMap<String, StateId>,
    vertex_buffer: f64,
) {
    let mut fixed = BTreeSet::new();
    for (name, point) in &shape.points {
        let id = ids[name];
        target.set_point(id, *point);
        fixed.insert(id);
    }
    if fixed.len() == ids.len() {
        return;
    }
    let options = LayoutOptions {
        vertex_buffer,
        fixed,
        ..LayoutOptions::default()
    };
    let positions = layout::compute(target.auto(), shape.layout, &options);
    for (id, point) in positions {
        target.set_point(id, point);
    }
    // the final fit of the layout moves every state; the given coordinates win
    for (name, point) in &shape.points {
        target.set_point(ids[name], *point);
    }
}

fn build_turing(object: &Object, tapes_hint: Option<usize>) -> Result<TuringMachine, ApiError> {
    check_keys("turing", object, &TURING_KEYS)?;
    let mut shape = Shape::read(object)?;
    shape.transitions = shape
        .transitions
        .into_iter()
        .flat_map(|(from, label, to)| {
            let parts = split_labels(&label);
            if parts.len() < 2 {
                return vec![(from, label, to)];
            }
            parts
                .into_iter()
                .map(|p| (from.clone(), p.to_string(), to.clone()))
                .collect()
        })
        .collect();
    let counted = shape
        .transitions
        .first()
        .map(|(_, label, _)| label.split('|').count());
    let tapes = shape.tapes.or(counted).or(tapes_hint).unwrap_or(1);
    if let Some(hint) = tapes_hint
        && hint != tapes
    {
        return Err(ApiError::invalid_document(format!(
            "a building block has {tapes} tapes but its machine has {hint}"
        )));
    }
    let mut tm = TuringMachine::new(tapes);
    let ids = populate(&mut tm, &shape, 80.0 * tapes as f64, tm_label)?;
    if let Some(alphabet) = object.get("input_alphabet").filter(|v| !v.is_null()) {
        tm.set_input_alphabet(Some(input_alphabet(alphabet)?));
    }
    for (name, inner) in &shape.blocks {
        let Value::Object(inner_object) = inner else {
            return Err(ApiError::invalid_document(format!(
                "the block of \"{name}\" must be a turing document, got {inner}"
            )));
        };
        let machine = build_turing(inner_object, Some(tapes))?;
        tm.set_block(ids[name], machine)
            .map_err(|e| ApiError::invalid_document(format!("the block of \"{name}\": {e}")))?;
    }
    Ok(tm)
}

fn build_grammar(object: &Object) -> Result<Document, ApiError> {
    check_keys(
        "grammar",
        object,
        &["type", "text", "rules", "productions", "start"],
    )?;
    let sources: Vec<&str> = ["text", "rules", "productions"]
        .into_iter()
        .filter(|k| object.contains_key(*k))
        .collect();
    if sources.len() != 1 {
        return Err(ApiError::new(
            code::INVALID_GRAMMAR,
            "a compact grammar needs exactly one of \"text\" (lines such as `S -> aSb | λ`), \
             \"rules\" or \"productions\" (an array of such lines)",
        ));
    }
    let value = &object[sources[0]];
    let text = match value {
        Value::String(s) => s.clone(),
        Value::Array(lines) => {
            let mut text = Vec::new();
            for line in lines {
                let Value::String(line) = line else {
                    return Err(ApiError::new(
                        code::INVALID_GRAMMAR,
                        format!(
                            "every line of \"{}\" must be a string, got {line}",
                            sources[0]
                        ),
                    ));
                };
                text.push(line.as_str());
            }
            text.join("\n")
        }
        other => {
            return Err(ApiError::new(
                code::INVALID_GRAMMAR,
                format!(
                    "\"{}\" must be a string or an array of strings, got {other}",
                    sources[0]
                ),
            ));
        }
    };
    let mut grammar = Grammar::parse(&text)
        .map_err(|e| ApiError::new(code::INVALID_GRAMMAR, format!("grammar: {e}")))?;
    if let Some(start) = object.get("start") {
        let start = name_of(start, "\"start\"")?;
        grammar.set_start(&start);
    }
    Ok(Document::Grammar(grammar))
}

fn build_regex(object: &Object) -> Result<Document, ApiError> {
    check_keys("regex", object, &["type", "expression", "text", "regex"])?;
    let found: Vec<&str> = ["expression", "text", "regex"]
        .into_iter()
        .filter(|k| object.contains_key(*k))
        .collect();
    let [key] = found[..] else {
        return Err(ApiError::new(
            code::INVALID_REGEX,
            "a regex document needs exactly one \"expression\" string such as \"(a+b)*abb\"",
        ));
    };
    let Value::String(text) = &object[key] else {
        return Err(ApiError::new(
            code::INVALID_REGEX,
            format!("\"{key}\" must be a string"),
        ));
    };
    Ok(Document::Regex(RegexDoc::new(text)?))
}

fn spaced(text: &str, chars: bool) -> String {
    if chars {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .map(String::from)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        text.to_string()
    }
}

fn build_lsystem(object: &Object) -> Result<Document, ApiError> {
    check_keys(
        "lsystem",
        object,
        &[
            "type",
            "axiom",
            "rules",
            "parameters",
            "single_char_symbols",
        ],
    )?;
    let chars = match object.get("single_char_symbols") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(other) => {
            return Err(ApiError::invalid_document(format!(
                "\"single_char_symbols\" must be true or false, got {other}"
            )));
        }
    };
    let Some(Value::String(axiom)) = object.get("axiom") else {
        return Err(ApiError::invalid_document(
            "a compact lsystem needs an \"axiom\" string of whitespace-separated symbols, such as \"F\"",
        ));
    };
    let mut rules: Vec<(String, String)> = Vec::new();
    let split_rule = |line: &str| -> Result<(String, String), ApiError> {
        let (lhs, rhs) = line
            .split_once("->")
            .or_else(|| line.split_once('→'))
            .ok_or_else(|| {
                ApiError::invalid_document(format!("the rule `{line}` must look like `F -> F + F`"))
            })?;
        Ok((lhs.trim().to_string(), spaced(rhs.trim(), chars)))
    };
    match object.get("rules") {
        None | Some(Value::Null) => {}
        Some(Value::Array(lines)) => {
            for line in lines {
                let Value::String(line) = line else {
                    return Err(ApiError::invalid_document(format!(
                        "every rule must be a string such as `F -> F + F`, got {line}"
                    )));
                };
                rules.push(split_rule(line)?);
            }
        }
        Some(Value::Object(map)) => {
            for (lhs, rhs) in map {
                let alternatives: Vec<&Value> = match rhs {
                    Value::Array(items) => items.iter().collect(),
                    one => vec![one],
                };
                for alt in alternatives {
                    let Value::String(alt) = alt else {
                        return Err(ApiError::invalid_document(format!(
                            "the replacement of `{lhs}` must be a string (or an array of strings), got {alt}"
                        )));
                    };
                    rules.push((lhs.trim().to_string(), spaced(alt, chars)));
                }
            }
        }
        Some(other) => {
            return Err(ApiError::invalid_document(format!(
                "\"rules\" must be an array of `lhs -> rhs` strings or an object, got {other}"
            )));
        }
    }
    let mut parameters: Vec<(String, String)> = Vec::new();
    match object.get("parameters") {
        None | Some(Value::Null) => {}
        Some(Value::Object(map)) => {
            for (name, value) in map {
                parameters.push((name.clone(), name_or_text(value, "a parameter value")?));
            }
        }
        Some(Value::Array(items)) => {
            for item in items {
                let Value::String(item) = item else {
                    return Err(ApiError::invalid_document(format!(
                        "every parameter must be a `name=value` string, got {item}"
                    )));
                };
                let (name, value) = item.split_once('=').ok_or_else(|| {
                    ApiError::invalid_document(format!(
                        "the parameter `{item}` must look like `angle=90`"
                    ))
                })?;
                parameters.push((name.trim().to_string(), value.trim().to_string()));
            }
        }
        Some(other) => {
            return Err(ApiError::invalid_document(format!(
                "\"parameters\" must be an object {{\"angle\": \"90\"}} or an array of `name=value`, got {other}"
            )));
        }
    }
    let productions: Vec<(&str, &str)> = rules
        .iter()
        .map(|(l, r)| (l.as_str(), r.as_str()))
        .collect();
    let params: Vec<(&str, &str)> = parameters
        .iter()
        .map(|(n, v)| (n.as_str(), v.as_str()))
        .collect();
    Ok(Document::Lsystem(LSystem::new(
        &spaced(axiom, chars),
        &productions,
        &params,
    )))
}

fn build_pumping(kind: &str, object: &Object) -> Result<Document, ApiError> {
    check_keys(kind, object, &["type", "language", "mode", "seed"])?;
    let lemma = if kind == "regular_pumping_lemma" {
        Kind::Regular
    } else {
        Kind::ContextFree
    };
    let titles = || {
        catalogue(lemma)
            .into_iter()
            .map(|l| format!("\"{}\"", l.title()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let language = match object.get("language") {
        Some(Value::String(title)) => Language::from_title(lemma, title),
        Some(Value::Number(n)) => n
            .as_u64()
            .and_then(|i| Language::from_index(lemma, i as usize)),
        _ => None,
    }
    .ok_or_else(|| {
        ApiError::invalid_document(format!(
            "\"language\" must be the title or the index of a predefined language: {}",
            titles()
        ))
    })?;
    let mode = match object.get("mode").and_then(Value::as_str) {
        None | Some("computer_first") => Mode::ComputerFirst,
        Some("human_first") => Mode::HumanFirst,
        Some(other) => {
            return Err(ApiError::invalid_document(format!(
                "\"mode\" must be \"human_first\" or \"computer_first\", got \"{other}\""
            )));
        }
    };
    let seed = object.get("seed").and_then(Value::as_i64).unwrap_or(1);
    let game = Game::new(language, mode, seed);
    Ok(if lemma == Kind::Regular {
        Document::RegularPumpingLemma(game)
    } else {
        Document::ContextFreePumpingLemma(game)
    })
}

/// `"ab"` or `["a", "b"]`: the declared input alphabet Σ_E of a Turing machine.
fn input_alphabet(value: &Value) -> Result<BTreeSet<char>, ApiError> {
    let bad = || {
        ApiError::invalid_document(format!(
            "\"input_alphabet\" must be a string such as \"ab\" or an array of one-character strings, got {value}"
        ))
    };
    match value {
        Value::String(s) => Ok(s
            .chars()
            .filter(|c| !c.is_whitespace() && *c != ',')
            .collect()),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                let s = item.as_str().ok_or_else(bad)?;
                let mut chars = s.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => Ok(c),
                    _ => Err(bad()),
                }
            })
            .collect(),
        _ => Err(bad()),
    }
}
