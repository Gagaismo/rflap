//! The `notation` and `join_labels` options of `render` for Turing machines.

mod common;

use common::*;
use serde_json::{Value, json};

fn palindrome_corner() -> Value {
    json!({
        "type": "turing",
        "states": ["2", "3"],
        "initial": "2",
        "finals": ["3"],
        "transitions": [
            ["2", "(a,a,D) / (b,b,D)", "2"],
            ["2", "(Δ,Δ,I)", "3"]
        ]
    })
}

fn labels(svg: &str) -> Vec<String> {
    svg.split("<text class=\"label\"")
        .skip(1)
        .map(|chunk| {
            let start = chunk.find('>').unwrap() + 1;
            chunk[start..chunk.find("</text>").unwrap()].to_string()
        })
        .collect()
}

fn render(extra: Value) -> String {
    let mut request = json!({"op": "render", "document": palindrome_corner()});
    for (k, v) in extra.as_object().unwrap() {
        request[k] = v.clone();
    }
    ok(request)["content"].as_str().unwrap().to_string()
}

#[test]
fn a_turing_machine_is_drawn_in_the_cátedra_notation_by_default() {
    assert_eq!(
        labels(&render(json!({}))),
        ["(a,a,D)", "(b,b,D)", "(Δ,Δ,I)"]
    );
    assert_eq!(
        labels(&render(json!({"notation": "catedra"}))),
        ["(a,a,D)", "(b,b,D)", "(Δ,Δ,I)"]
    );
}

#[test]
fn the_jflap_notation_keeps_the_old_labels() {
    assert_eq!(
        labels(&render(json!({"notation": "jflap"}))),
        ["a ; a , R", "b ; b , R", "□ ; □ , L"]
    );
}

#[test]
fn join_labels_puts_parallel_labels_on_one_line() {
    assert_eq!(
        labels(&render(json!({"join_labels": true}))),
        ["(a,a,D) / (b,b,D)", "(Δ,Δ,I)"]
    );
}

#[test]
fn markdown_and_dot_use_the_notation_too() {
    let md = render(json!({"format": "markdown"}));
    assert!(md.contains("(Δ,Δ,I)"), "{md}");
    let md = render(json!({"format": "markdown", "notation": "jflap"}));
    assert!(md.contains("□ ; □ , L"), "{md}");
    let dot = render(json!({"format": "dot"}));
    assert!(dot.contains("(a,a,D)\\n(b,b,D)"), "{dot}");
}

#[test]
fn other_machines_ignore_the_notation() {
    let nfa = json!({"op": "render", "document": abb_nfa()});
    let mut with = nfa.clone();
    with["notation"] = json!("catedra");
    with["join_labels"] = json!(false);
    assert_eq!(ok(nfa)["content"], ok(with)["content"]);
}

#[test]
fn an_unknown_notation_is_refused() {
    let e = err(json!({"op": "render", "document": palindrome_corner(), "notation": "latex"}));
    assert_eq!(e["code"], "invalid_request");
}
