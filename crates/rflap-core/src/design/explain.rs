//! Plain-language summary of a finite automaton for chat answers.

use std::collections::BTreeMap;

use crate::analysis::nondeterministic_states;
use crate::automaton::{Label, StateId};
use crate::fa::{Fa, alphabet};

/// States, start, finals and the transition table in Markdown, with the meaning of every
/// state when it has one (`State::label`).
pub fn explain(fa: &Fa) -> String {
    let name = |id: StateId| {
        fa.state(id)
            .map_or_else(|| format!("q{id}"), |s| s.name.clone())
    };
    let mut out = String::new();
    let count = fa.states().count();
    out.push_str(&format!(
        "Finite automaton with {count} state{} and {} transition{}.\n",
        if count == 1 { "" } else { "s" },
        fa.transitions().len(),
        if fa.transitions().len() == 1 { "" } else { "s" },
    ));
    match fa.initial() {
        Some(id) => out.push_str(&format!("Start state: {}.\n", name(id))),
        None => out.push_str("There is no start state.\n"),
    }
    let finals: Vec<String> = fa.finals().iter().map(|f| name(*f)).collect();
    if finals.is_empty() {
        out.push_str("Accepting states: none (the language is empty).\n");
    } else {
        out.push_str(&format!("Accepting states: {}.\n", finals.join(", ")));
    }

    let mut symbols = alphabet(fa);
    symbols.sort();
    let has_lambda = fa.transitions().iter().any(|t| t.label.is_lambda());
    let deterministic = !has_lambda && nondeterministic_states(fa).is_empty();
    let complete = deterministic
        && fa.states().all(|s| {
            symbols
                .iter()
                .all(|c| fa.transitions_from(s.id).any(|t| &t.label.0 == c))
        });
    out.push_str(&format!(
        "It is {}.\n",
        match (deterministic, complete) {
            (true, true) => "a complete DFA (exactly one move per state and symbol)",
            (true, false) => "a DFA with missing moves (a missing move rejects)",
            (false, _) => "an NFA (it has λ moves or several moves on one symbol)",
        }
    ));

    let meanings: Vec<String> = fa
        .states()
        .filter_map(|s| s.label.as_ref().map(|l| format!("- {}: {l}", s.name)))
        .collect();
    if !meanings.is_empty() {
        out.push_str("\nWhat each state means:\n");
        out.push_str(&meanings.join("\n"));
        out.push('\n');
    }

    out.push_str("\nTransition table (→ start, * accepting):\n\n");
    let mut columns = symbols.clone();
    if has_lambda {
        columns.push(String::new());
    }
    let head: Vec<String> = columns
        .iter()
        .map(|c| {
            if c.is_empty() {
                "λ".to_string()
            } else {
                c.clone()
            }
        })
        .collect();
    out.push_str(&format!("| state | {} |\n", head.join(" | ")));
    out.push_str(&format!("|---|{}\n", "---|".repeat(columns.len())));
    for s in fa.states() {
        let mark = format!(
            "{}{}{}",
            if fa.initial() == Some(s.id) {
                "→"
            } else {
                ""
            },
            if fa.is_final(s.id) { "*" } else { "" },
            s.name
        );
        let cells: Vec<String> = columns
            .iter()
            .map(|c| {
                let mut targets: BTreeMap<StateId, ()> = BTreeMap::new();
                for t in fa.transitions_from(s.id).filter(|t| &t.label.0 == c) {
                    targets.insert(t.to, ());
                }
                if targets.is_empty() {
                    "-".to_string()
                } else {
                    targets
                        .keys()
                        .map(|id| name(*id))
                        .collect::<Vec<_>>()
                        .join(",")
                }
            })
            .collect();
        out.push_str(&format!("| {mark} | {} |\n", cells.join(" | ")));
    }
    out
}
