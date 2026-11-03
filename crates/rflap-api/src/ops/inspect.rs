//! `analyze`, `explain`, `render`, `layout` and `combine`.

use std::collections::BTreeSet;

use rflap_core::analysis::{
    lambda_transitions, nondeterministic_states, unreachable_states, useless_states,
};
use rflap_core::automaton::{Automaton, Label, Point, StateId};
use rflap_core::design;
use rflap_core::fa;
use rflap_core::grammar::Grammar;
use rflap_core::layout::{self, Algorithm, LayoutOptions};
use rflap_core::regex::Regex;
use rflap_core::render::{self, SvgOptions};
use serde_json::{Value, json};

use super::{Object, snake, state_name, to_value, with_automaton, wrong_type};
use crate::compact::parse_algorithm;
use crate::document::Document;
use crate::error::{ApiError, code};
use crate::request::{Format, LayoutArgs, RenderArgs, StateRef};

// ---------------------------------------------------------------------------------------------
// analyze
// ---------------------------------------------------------------------------------------------

fn names<L: Label>(a: &Automaton<L>, ids: impl IntoIterator<Item = StateId>) -> Vec<String> {
    ids.into_iter().map(|id| state_name(a, id)).collect()
}

/// What every graph-like document reports.
fn automaton_report<L: Label>(a: &Automaton<L>) -> Object {
    let mut out = Object::new();
    let states: Vec<Value> = a
        .states()
        .map(|s| {
            let mut v = json!({
                "id": s.id,
                "name": s.name,
                "initial": a.initial() == Some(s.id),
                "final": a.is_final(s.id),
            });
            if let Some(label) = &s.label {
                v["label"] = json!(label);
            }
            if let Some(output) = &s.output {
                v["output"] = json!(output);
            }
            v
        })
        .collect();
    let nondeterministic = nondeterministic_states(a);
    out.insert("states".into(), Value::Array(states));
    out.insert(
        "initial".into(),
        a.initial().map_or(Value::Null, |i| json!(state_name(a, i))),
    );
    out.insert("finals".into(), json!(names(a, a.finals().iter().copied())));
    out.insert("transitions".into(), json!(a.transitions().len()));
    out.insert("deterministic".into(), json!(nondeterministic.is_empty()));
    out.insert(
        "nondeterministic_states".into(),
        json!(names(a, nondeterministic)),
    );
    let lambdas: Vec<Value> = a
        .transitions()
        .iter()
        .enumerate()
        .filter(|(_, t)| t.label.is_lambda())
        .map(|(index, t)| {
            json!({
                "index": index,
                "from": state_name(a, t.from),
                "to": state_name(a, t.to),
                "label": t.label.description(),
            })
        })
        .collect();
    debug_assert_eq!(lambdas.len(), lambda_transitions(a).len());
    out.insert("lambda_transitions".into(), Value::Array(lambdas));
    out.insert(
        "unreachable_states".into(),
        json!(names(a, unreachable_states(a))),
    );
    out.insert(
        "useless_states".into(),
        useless_states(a).map_or(Value::Null, |u| json!(names(a, u))),
    );
    out
}

fn distinct(symbols: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = Vec::new();
    for s in symbols {
        if !s.is_empty() && !seen.contains(&s) {
            seen.push(s);
        }
    }
    seen
}

fn regex_symbols(r: &Regex, out: &mut BTreeSet<char>) {
    match r {
        Regex::Symbol(c) => {
            out.insert(*c);
        }
        Regex::Union(v) | Regex::Concat(v) => v.iter().for_each(|x| regex_symbols(x, out)),
        Regex::Star(x) => regex_symbols(x, out),
        Regex::Empty | Regex::Lambda => {}
    }
}

pub fn analyze(doc: &Document) -> Result<Value, ApiError> {
    let mut out = Object::new();
    out.insert("type".into(), json!(doc.type_name()));
    match doc {
        Document::Fa(a) => {
            out.extend(automaton_report(a));
            let deterministic = out["deterministic"] == json!(true);
            let alphabet = fa::alphabet(a);
            let complete = deterministic
                && a.states().all(|s| {
                    alphabet
                        .iter()
                        .all(|x| a.transitions_from(s.id).any(|t| &t.label.0 == x))
                });
            out.insert(
                "kind".into(),
                json!(if deterministic { "dfa" } else { "nfa" }),
            );
            out.insert("alphabet".into(), json!(alphabet));
            out.insert("complete".into(), json!(complete));
            out.insert(
                "has_multi_char_labels".into(),
                json!(fa::has_multi_char_labels(a)),
            );
        }
        Document::Pda(a) => {
            out.extend(automaton_report(a));
            let deterministic = out["deterministic"] == json!(true);
            out.insert(
                "kind".into(),
                json!(if deterministic { "dpda" } else { "npda" }),
            );
            out.insert(
                "alphabet".into(),
                json!(distinct(
                    a.transitions().iter().map(|t| t.label.input.clone())
                )),
            );
            out.insert(
                "stack_alphabet".into(),
                json!(distinct(
                    a.transitions()
                        .iter()
                        .flat_map(|t| t.label.pop.chars().chain(t.label.push.chars()))
                        .map(String::from)
                )),
            );
        }
        Document::Turing(t) => {
            out.extend(automaton_report(t.automaton()));
            out.insert("kind".into(), json!("turing"));
            out.insert("tapes".into(), json!(t.tapes()));
            out.insert("blocks".into(), json!(t.blocks().len()));
        }
        Document::Mealy(a) => {
            out.extend(automaton_report(a));
            out.insert("kind".into(), json!("mealy"));
            out.insert(
                "alphabet".into(),
                json!(distinct(
                    a.transitions().iter().map(|t| t.label.input.clone())
                )),
            );
        }
        Document::Moore(a) => {
            out.extend(automaton_report(a));
            out.insert("kind".into(), json!("moore"));
            out.insert(
                "alphabet".into(),
                json!(distinct(a.transitions().iter().map(|t| t.label.0.clone()))),
            );
        }
        Document::Grammar(g) => {
            let report = g.type_report();
            out.insert("start".into(), json!(g.start()));
            out.insert(
                "variables".into(),
                json!(
                    g.variables()
                        .iter()
                        .map(char::to_string)
                        .collect::<Vec<_>>()
                ),
            );
            out.insert(
                "terminals".into(),
                json!(
                    g.terminals()
                        .iter()
                        .map(char::to_string)
                        .collect::<Vec<_>>()
                ),
            );
            out.insert("productions".into(), json!(g.productions().len()));
            out.insert(
                "classification".into(),
                json!(snake(&format!("{:?}", report.classification))),
            );
            out.insert("message".into(), json!(report.classification.message()));
        }
        Document::Regex(r) => {
            let regex = r.parse()?;
            let mut symbols = BTreeSet::new();
            regex_symbols(&regex, &mut symbols);
            out.insert("expression".into(), json!(r.expression));
            out.insert("canonical".into(), json!(regex.to_string()));
            out.insert("nullable".into(), json!(regex.is_nullable()));
            out.insert(
                "alphabet".into(),
                json!(symbols.iter().map(char::to_string).collect::<Vec<_>>()),
            );
        }
        Document::Lsystem(l) => {
            out.insert("axiom".into(), json!(l.axiom().join(" ")));
            out.insert(
                "rules".into(),
                json!(l.symbols_with_replacements().collect::<Vec<_>>()),
            );
            out.insert("parameters".into(), json!(l.parameters()));
            out.insert("nondeterministic".into(), json!(l.is_nondeterministic()));
        }
        Document::RegularPumpingLemma(g) | Document::ContextFreePumpingLemma(g) => {
            out.insert("language".into(), to_value(&g.lemma().info()));
            out.insert("mode".into(), to_value(&g.mode()));
        }
    }
    Ok(Value::Object(out))
}

// ---------------------------------------------------------------------------------------------
// explain
// ---------------------------------------------------------------------------------------------

fn machine_summary<L: Label>(title: &str, a: &Automaton<L>) -> String {
    let count = a.states().count();
    let mut out = format!(
        "{title} with {count} state{} and {} transition{}.\n",
        if count == 1 { "" } else { "s" },
        a.transitions().len(),
        if a.transitions().len() == 1 { "" } else { "s" },
    );
    match a.initial() {
        Some(id) => out.push_str(&format!("Start state: {}.\n", state_name(a, id))),
        None => out.push_str("There is no start state.\n"),
    }
    let finals = names(a, a.finals().iter().copied());
    if finals.is_empty() {
        out.push_str("Accepting states: none.\n");
    } else {
        out.push_str(&format!("Accepting states: {}.\n", finals.join(", ")));
    }
    out.push('\n');
    out.push_str(&render::transition_table(a));
    out
}

pub fn explain(doc: &Document) -> Result<Value, ApiError> {
    let markdown = match doc {
        Document::Fa(a) => design::explain(a),
        Document::Pda(a) => machine_summary("Pushdown automaton", a),
        Document::Turing(t) => machine_summary(
            &format!("Turing machine with {} tape(s)", t.tapes()),
            t.automaton(),
        ),
        Document::Mealy(a) => machine_summary("Mealy machine", a),
        Document::Moore(a) => machine_summary("Moore machine", a),
        Document::Grammar(g) => format!(
            "{}\n\nStart variable: {}.\n\n{}",
            g.type_report().classification.message(),
            g.start(),
            g.to_text()
        ),
        Document::Regex(r) => {
            let regex = r.parse()?;
            format!(
                "Regular expression `{}`; the empty string is {}in the language.",
                regex,
                if regex.is_nullable() { "" } else { "not " }
            )
        }
        Document::Lsystem(l) => format!(
            "L-system with axiom `{}` and {} rewritten symbol(s).",
            l.axiom().join(" "),
            l.symbols_with_replacements().count()
        ),
        Document::RegularPumpingLemma(g) | Document::ContextFreePumpingLemma(g) => {
            let info = g.lemma().info();
            format!("{}\n\n{}", info.title, info.explanation)
        }
    };
    Ok(json!({"markdown": markdown}))
}

// ---------------------------------------------------------------------------------------------
// render
// ---------------------------------------------------------------------------------------------

fn resolve_state<L: Label>(a: &Automaton<L>, r: &StateRef) -> Result<StateId, ApiError> {
    let found = match r {
        StateRef::Name(name) => a
            .states()
            .find(|s| &s.name == name)
            .map(|s| s.id)
            .or_else(|| name.parse().ok().filter(|id| a.state(*id).is_some())),
        StateRef::Id(id) => a.state(*id).map(|s| s.id),
    };
    found.ok_or_else(|| {
        let shown = match r {
            StateRef::Name(n) => n.clone(),
            StateRef::Id(i) => i.to_string(),
        };
        ApiError::new(
            code::UNKNOWN_STATE,
            format!(
                "unknown state \"{shown}\"; the states are [{}]",
                a.states()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    })
}

fn text_listing<L: Label>(title: &str, a: &Automaton<L>) -> String {
    let count = a.states().count();
    let mut out = format!(
        "{title}: {count} state{}, {} transition{}\n",
        if count == 1 { "" } else { "s" },
        a.transitions().len(),
        if a.transitions().len() == 1 { "" } else { "s" },
    );
    out.push_str(&format!(
        "initial: {}\n",
        a.initial().map_or("none".to_string(), |i| state_name(a, i))
    ));
    let finals = names(a, a.finals().iter().copied());
    out.push_str(&format!(
        "final: {}\n",
        if finals.is_empty() {
            "none".to_string()
        } else {
            finals.join(", ")
        }
    ));
    let outputs: Vec<String> = a
        .states()
        .filter_map(|s| s.output.as_ref().map(|o| format!("{}={o}", s.name)))
        .collect();
    if !outputs.is_empty() {
        out.push_str(&format!("outputs: {}\n", outputs.join(", ")));
    }
    for t in a.transitions() {
        out.push_str(&format!(
            "{} --{}--> {}\n",
            state_name(a, t.from),
            t.label.description(),
            state_name(a, t.to)
        ));
    }
    out
}

fn automaton_title(doc: &Document) -> &'static str {
    match doc {
        Document::Fa(_) => "Finite automaton",
        Document::Pda(_) => "Pushdown automaton",
        Document::Turing(_) => "Turing machine",
        Document::Mealy(_) => "Mealy machine",
        _ => "Moore machine",
    }
}

pub fn render(r: RenderArgs) -> Result<Value, ApiError> {
    let format = r.format.unwrap_or(Format::Svg);
    let doc = &r.document;
    let name = to_value(&format);
    let content = if doc.is_automaton() {
        with_automaton!(doc, a => {
            match format {
                Format::Svg => {
                    let mut options = SvgOptions {
                        dark_mode: r.dark,
                        ..SvgOptions::default()
                    };
                    for state in &r.highlight_states {
                        options.highlight_states.insert(resolve_state(a, state)?);
                    }
                    for &index in &r.highlight_transitions {
                        if index >= a.transitions().len() {
                            return Err(ApiError::new(
                                code::INVALID_ARGUMENT,
                                format!(
                                    "transition index {index} is out of range: the document has {} transitions",
                                    a.transitions().len()
                                ),
                            ));
                        }
                        options.highlight_transitions.insert(index);
                    }
                    render::to_svg(a, &options)
                }
                Format::Markdown => render::transition_table(a),
                Format::Dot => render::to_dot(a),
                Format::Text => text_listing(automaton_title(doc), a),
            }
        }, _ => unreachable!("is_automaton() checked"))
    } else {
        match (doc, format) {
            (Document::Lsystem(l), Format::Svg) => l.to_svg(r.depth.unwrap_or(3).min(12)),
            (Document::Grammar(g), Format::Markdown | Format::Text) => grammar_text(g, format),
            (Document::Regex(re), Format::Markdown | Format::Text) => {
                let regex = re.parse()?;
                if format == Format::Markdown {
                    format!("`{regex}`")
                } else {
                    regex.to_string()
                }
            }
            (Document::Lsystem(_), Format::Markdown | Format::Text)
            | (
                Document::RegularPumpingLemma(_) | Document::ContextFreePumpingLemma(_),
                Format::Markdown | Format::Text,
            ) => explain(doc)?["markdown"].as_str().unwrap_or("").to_string(),
            _ => {
                return Err(wrong_type(
                    "render",
                    &format!(
                        "a document that can be drawn as `{}` (svg and dot need a machine or an lsystem; markdown and text work for every type)",
                        name.as_str().unwrap_or("?")
                    ),
                    doc,
                ));
            }
        }
    };
    Ok(json!({"format": name, "content": content}))
}

fn grammar_text(g: &Grammar, format: Format) -> String {
    if format == Format::Text {
        return g.to_text();
    }
    let mut out = String::from("| # | production |\n|---|---|\n");
    for (i, p) in g.productions().iter().enumerate() {
        out.push_str(&format!(
            "| {i} | {} → {} |\n",
            p.lhs,
            if p.rhs.is_empty() { "λ" } else { &p.rhs }
        ));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// layout
// ---------------------------------------------------------------------------------------------

fn vertex_buffer(doc: &Document) -> f64 {
    match doc {
        Document::Fa(_) => 50.0,
        Document::Mealy(_) | Document::Moore(_) => 65.0,
        Document::Pda(_) => 80.0,
        Document::Turing(t) => 80.0 * t.tapes() as f64,
        _ => 50.0,
    }
}

fn place<L: Label>(
    a: &Automaton<L>,
    algorithm: Algorithm,
    options: &LayoutOptions,
) -> Vec<(StateId, Point)> {
    layout::compute(a, algorithm, options).into_iter().collect()
}

pub fn layout(r: LayoutArgs) -> Result<Value, ApiError> {
    let algorithm = match &r.algorithm {
        Some(name) => parse_algorithm(name)?,
        None => Algorithm::Circle,
    };
    let mut doc = r.document;
    if !doc.is_automaton() {
        return Err(wrong_type(
            "layout",
            "a fa, pda, turing, mealy or moore document",
            &doc,
        ));
    }
    let options = r.options.unwrap_or_else(|| LayoutOptions {
        vertex_buffer: vertex_buffer(&doc),
        ..LayoutOptions::default()
    });
    macro_rules! relocate {
        ($a:expr, $moves:expr) => {{
            for (id, point) in $moves {
                if let Some(s) = $a.state_mut(id) {
                    s.point = point;
                }
            }
        }};
    }
    match &mut doc {
        Document::Fa(a) => relocate!(a, place(a, algorithm, &options)),
        Document::Pda(a) => relocate!(a, place(a, algorithm, &options)),
        Document::Mealy(a) => relocate!(a, place(a, algorithm, &options)),
        Document::Moore(a) => relocate!(a, place(a, algorithm, &options)),
        Document::Turing(t) => {
            let moves = place(t.automaton(), algorithm, &options);
            relocate!(t, moves);
        }
        _ => {}
    }
    Ok(json!({"document": doc}))
}

// ---------------------------------------------------------------------------------------------
// combine
// ---------------------------------------------------------------------------------------------

pub fn combine(a: &Document, b: &Document) -> Result<Value, ApiError> {
    let document = match (a, b) {
        (Document::Fa(x), Document::Fa(y)) => Document::Fa(fa::combine(x, y)),
        (Document::Pda(x), Document::Pda(y)) => Document::Pda(fa::combine(x, y)),
        (Document::Mealy(x), Document::Mealy(y)) => Document::Mealy(fa::combine(x, y)),
        (Document::Moore(x), Document::Moore(y)) => Document::Moore(fa::combine(x, y)),
        (Document::Turing(_), Document::Turing(_)) => {
            return Err(ApiError::new(
                code::UNSUPPORTED_OPTION,
                "`combine` does not support turing machines",
            ));
        }
        (x, y) if x.is_automaton() && y.is_automaton() => {
            return Err(ApiError::new(
                code::WRONG_DOCUMENT_TYPE,
                format!(
                    "`combine` needs two documents of the same type, got `{}` and `{}`",
                    x.type_name(),
                    y.type_name()
                ),
            ));
        }
        (x, _) => {
            return Err(wrong_type(
                "combine",
                "two fa, pda, mealy or moore documents",
                x,
            ));
        }
    };
    Ok(json!({"document": document}))
}
