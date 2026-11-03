//! Every enum value and key in a response is snake_case: no `Accepted`, `NotLl1`, `Left`, ...

mod common;

use common::{EXPRESSION_GRAMMAR, abb_nfa, anbn_pda, anbncn_tm, send};
use serde_json::{Value, json};

const USER_TEXT: [&str; 17] = [
    "productions",
    "lhs",
    "rhs",
    "derivation_forms",
    "sentential",
    "forms",
    "input",
    "name",
    "description",
    "explanation",
    "message",
    "text",
    "stack",
    "entries",
    "caption",
    "content",
    // The text JFLAP stores in the .jff (`Computer`/`Human`), not an enum.
    "first_player",
];

/// Collects the keys and string values that look like a Rust variant name (`Accepted`,
/// `NotLl1`): an uppercase letter followed by a lowercase one. Grammar symbols are single
/// characters, so they never match.
fn camel_case(value: &Value, path: &str, found: &mut Vec<String>) {
    let looks_camel = |s: &str| {
        let mut chars = s.chars();
        matches!((chars.next(), chars.next()), (Some(a), Some(b))
            if a.is_ascii_uppercase() && b.is_ascii_lowercase())
            && s.chars().all(|c| c.is_ascii_alphanumeric())
    };
    match value {
        Value::String(s) if looks_camel(s) => found.push(format!("{path} = {s}")),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                camel_case(item, &format!("{path}[{i}]"), found);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                // User text: grammar symbols and strings, state and variable names.
                if USER_TEXT.contains(&key.as_str()) {
                    continue;
                }
                if looks_camel(key) {
                    found.push(format!("{path}.{key} (key)"));
                }
                camel_case(item, &format!("{path}.{key}"), found);
            }
        }
        _ => {}
    }
}

#[test]
fn no_camel_case_enum_values_in_representative_responses() {
    let grammar = "S -> aSb | λ";
    let requests = vec![
        json!({"op": "parse", "grammar": grammar, "inputs": ["aabb", "aab"], "parser": "brute"}),
        json!({"op": "parse", "grammar": grammar, "inputs": ["aabb"], "parser": "brute_unrestricted"}),
        json!({"op": "parse", "grammar": grammar, "inputs": ["aabb", "ab"], "parser": "ll1"}),
        json!({"op": "parse", "grammar": "S -> Sa | a", "inputs": ["aa"], "parser": "ll1"}),
        json!({"op": "parse", "grammar": EXPRESSION_GRAMMAR, "inputs": ["i+i*i"], "parser": "slr1"}),
        json!({"op": "parse", "grammar": grammar, "inputs": ["ab"], "parser": "cyk"}),
        json!({"op": "parse_tables", "grammar": EXPRESSION_GRAMMAR}),
        json!({"op": "user_parse", "grammar": grammar, "target": "aabb",
               "moves": [{"production": 0}, {"production": 0}, "undo"]}),
        json!({"op": "grammar_type", "grammar": grammar}),
        json!({"op": "convert", "document": {"type": "grammar", "text": "S -> AB | λ\nA -> a | B\nB -> b | C\nC -> c"},
               "to": "remove_lambda"}),
        json!({"op": "convert", "document": {"type": "grammar", "text": "S -> AB | λ\nA -> a | B\nB -> b | C\nC -> c"},
               "to": "remove_unit"}),
        json!({"op": "convert", "document": {"type": "grammar", "text": "S -> aB | b\nA -> a\nB -> b"},
               "to": "remove_useless"}),
        json!({"op": "convert", "document": {"type": "grammar", "text": grammar}, "to": "cnf"}),
        json!({"op": "convert", "document": {"type": "grammar", "text": grammar}, "to": "pda_ll"}),
        json!({"op": "convert", "document": {"type": "grammar", "text": grammar}, "to": "pda_lr"}),
        json!({"op": "convert", "document": abb_nfa(), "to": "minimal_dfa"}),
        json!({"op": "convert", "document": abb_nfa(), "to": "regex"}),
        json!({"op": "convert", "document": abb_nfa(), "to": "regular_grammar"}),
        json!({"op": "convert", "document": anbn_pda(), "to": "cfg"}),
        json!({"op": "convert", "document": anbncn_tm(), "to": "unrestricted_grammar"}),
        json!({"op": "normalize", "document": anbncn_tm()}),
        json!({"op": "normalize", "document": {"type": "regex", "expression": "(a+b)*abb"}}),
        json!({"op": "simulate", "document": anbncn_tm(), "input": "abc"}),
        json!({"op": "analyze", "document": abb_nfa()}),
        json!({"op": "multiple_run", "document": abb_nfa(), "inputs": ["abb", "ab"]}),
        json!({"op": "design", "spec": {"alphabet": "ab",
               "language": {"constraint_text": "ends_with(\"ab\")"}}}),
        json!({"op": "design", "spec": {"alphabet": "ab",
               "language": {"template": {"name": "an_bn", "a": "a", "b": "b"}}}}),
        json!({"op": "pumping_languages"}),
        json!({"op": "pumping_new", "kind": "regular", "language": 0, "mode": "computer_first"}),
        json!({"op": "pumping_new", "kind": "context_free", "language": 0, "mode": "human_first"}),
        json!({"op": "lsystem_render", "document": {"type": "lsystem", "axiom": "F",
               "rules": ["F -> F+F-F-F+F"], "parameters": {"angle": "90", "distance": "4"}}, "depth": 1}),
    ];
    let mut found = Vec::new();
    for request in requests {
        let response = send(request.clone());
        camel_case(&response, &request["op"].to_string(), &mut found);
    }
    found.sort();
    found.dedup();
    assert!(
        found.is_empty(),
        "CamelCase in responses:\n{}",
        found.join("\n")
    );
}
