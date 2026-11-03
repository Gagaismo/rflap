//! Text renderings: Markdown transition table and Graphviz DOT.

use std::collections::BTreeSet;

use crate::automaton::{Automaton, Label, State, StateId};

/// State name, followed by ` / output` for Moore machines.
fn display_name(s: &State) -> String {
    match &s.output {
        Some(out) => format!("{} / {out}", s.name),
        None => s.name.clone(),
    }
}

fn md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// Markdown transition table: one row per state (`→` marks the initial state, `*` final
/// states), one column per distinct transition label (λ last), each cell listing the target
/// states or `∅`.
pub fn transition_table<L: Label>(automaton: &Automaton<L>) -> String {
    let mut columns: Vec<(bool, String)> = automaton
        .transitions()
        .iter()
        .map(|t| (t.label.is_lambda(), t.label.description()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    columns.sort();
    let mut out = String::from("| State |");
    for (_, label) in &columns {
        out.push_str(&format!(" {} |", md_cell(label)));
    }
    out.push_str("\n|---|");
    for _ in &columns {
        out.push_str("---|");
    }
    out.push('\n');
    for s in automaton.states() {
        let mark = match (automaton.initial() == Some(s.id), automaton.is_final(s.id)) {
            (true, true) => "→* ",
            (true, false) => "→ ",
            (false, true) => "* ",
            (false, false) => "",
        };
        out.push_str(&format!("| {mark}{} |", md_cell(&display_name(s))));
        for (_, label) in &columns {
            let targets: BTreeSet<StateId> = automaton
                .transitions_from(s.id)
                .filter(|t| &t.label.description() == label)
                .map(|t| t.to)
                .collect();
            let cell = if targets.is_empty() {
                "∅".to_string()
            } else {
                targets
                    .iter()
                    .filter_map(|&id| automaton.state(id))
                    .map(|st| st.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            out.push_str(&format!(" {} |", md_cell(&cell)));
        }
        out.push('\n');
    }
    out
}

fn dot_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Graphviz DOT source: left-to-right, final states as double circles, the initial state
/// reached from an invisible-label point, parallel transitions merged into one edge.
pub fn to_dot<L: Label>(automaton: &Automaton<L>) -> String {
    let mut out = String::from("digraph automaton {\n  rankdir=LR;\n  node [shape=circle];\n");
    if automaton.initial().is_some() {
        out.push_str("  __start [shape=point];\n");
    }
    for s in automaton.states() {
        let shape = if automaton.is_final(s.id) {
            ", shape=doublecircle"
        } else {
            ""
        };
        out.push_str(&format!(
            "  s{} [label=\"{}\"{shape}];\n",
            s.id,
            dot_escape(&display_name(s))
        ));
    }
    if let Some(i) = automaton.initial() {
        out.push_str(&format!("  __start -> s{i};\n"));
    }
    let mut pairs: Vec<(StateId, StateId, Vec<String>)> = Vec::new();
    for t in automaton.transitions() {
        let label = t.label.description();
        match pairs
            .iter_mut()
            .find(|(f, to, _)| *f == t.from && *to == t.to)
        {
            Some((_, _, labels)) => labels.push(label),
            None => pairs.push((t.from, t.to, vec![label])),
        }
    }
    for (from, to, labels) in pairs {
        out.push_str(&format!(
            "  s{from} -> s{to} [label=\"{}\"];\n",
            dot_escape(&labels.join("\n"))
        ));
    }
    out.push_str("}\n");
    out
}
