mod common;

use common::{abb_nfa, anbn_pda, err, ok, strings};
use serde_json::{Value, json};

fn state_count(doc: &Value) -> usize {
    doc["states"].as_object().unwrap().len()
}

fn equivalent(a: &Value, b: &Value) -> Value {
    ok(json!({"op": "equivalent", "a": a, "b": b}))
}

#[test]
fn nfa_to_dfa_has_the_classic_four_states() {
    let r = ok(json!({"op": "convert", "document": abb_nfa(), "to": "dfa"}));
    assert_eq!(r["to"], "dfa");
    assert_eq!(r["document"]["type"], "fa");
    assert_eq!(state_count(&r["document"]), 4);
    assert_eq!(r["document"]["transitions"].as_array().unwrap().len(), 8);
    assert_eq!(r["dfa_states"].as_array().unwrap().len(), 4);
    assert!(!r["steps"].as_array().unwrap().is_empty());
    let analysis = ok(json!({"op": "analyze", "document": r["document"]}));
    assert_eq!(analysis["kind"], "dfa");
    assert_eq!(equivalent(&abb_nfa(), &r["document"])["equivalent"], true);
}

#[test]
fn minimal_dfa_of_the_nfa() {
    let r = ok(json!({"op": "convert", "document": abb_nfa(), "to": "minimal_dfa"}));
    assert_eq!(state_count(&r["document"]), 4);
    assert!(r["tree"].is_object());
    assert!(!r["steps"].as_array().unwrap().is_empty());
    assert_eq!(equivalent(&abb_nfa(), &r["document"])["equivalent"], true);

    // a redundant DFA shrinks: states 1 and 2 are equivalent
    let big = json!({
        "type": "fa", "initial": "s", "finals": ["x", "y"],
        "transitions": [["s", "a", "x"], ["s", "b", "y"], ["x", "a", "x"], ["x", "b", "x"],
                        ["y", "a", "y"], ["y", "b", "y"]]
    });
    let r = ok(json!({"op": "convert", "document": big.clone(), "to": "minimal_dfa"}));
    assert_eq!(state_count(&r["document"]), 2);
    assert_eq!(equivalent(&big, &r["document"])["equivalent"], true);
}

#[test]
fn fa_to_regex_is_equivalent() {
    let r = ok(json!({"op": "convert", "document": abb_nfa(), "to": "regex"}));
    assert_eq!(r["document"]["type"], "regex");
    let expression = r["document"]["expression"].as_str().unwrap();
    assert!(!expression.is_empty());
    assert!(!r["steps"].as_array().unwrap().is_empty());
    let check = equivalent(&abb_nfa(), &r["document"]);
    assert_eq!(check["equivalent"], true, "regex {expression}");
    assert_eq!(check["counterexample"], Value::Null);
}

#[test]
fn regex_to_nfa_and_back() {
    let regex = json!({"type": "regex", "expression": "(a+b)*abb"});
    let r = ok(json!({"op": "convert", "document": regex.clone(), "to": "nfa"}));
    assert_eq!(r["document"]["type"], "fa");
    assert!(!r["steps"].as_array().unwrap().is_empty());
    assert_eq!(equivalent(&regex, &r["document"])["equivalent"], true);
    assert_eq!(equivalent(&abb_nfa(), &regex)["equivalent"], true);
    // regexes convert straight to a minimal DFA too
    let r = ok(json!({"op": "convert", "document": regex, "to": "minimal_dfa"}));
    assert_eq!(state_count(&r["document"]), 4);
}

#[test]
fn equivalent_gives_a_counterexample() {
    let other = json!({"type": "regex", "expression": "(a+b)*ab"});
    let r = equivalent(&abb_nfa(), &other);
    assert_eq!(r["equivalent"], false);
    let w = r["counterexample"].as_str().unwrap();
    assert!(!w.is_empty());
    // the counterexample is accepted by exactly one of them
    let other_fa = ok(json!({"op": "convert", "document": other, "to": "nfa"}))["document"].clone();
    let a = ok(json!({"op": "simulate", "document": abb_nfa(), "input": w}));
    let b = ok(json!({"op": "simulate", "document": other_fa, "input": w}));
    assert_ne!(a["accepted"], b["accepted"], "counterexample {w:?}");
}

#[test]
fn complete_adds_the_trap_state() {
    let dfa = json!({
        "type": "fa", "initial": "p", "finals": ["q"],
        "transitions": [["p", "a", "q"], ["p", "b", "p"], ["q", "a", "p"]]
    });
    let r = ok(json!({"op": "convert", "document": dfa, "to": "complete"}));
    assert_eq!(state_count(&r["document"]), 3);
    assert_eq!(r["trap_state"], "q2");
    assert_eq!(r["steps"][0]["kind"], "add_trap_state");
    assert_eq!(r["steps"][0]["transitions_added"], 3);
    assert_eq!(
        r["document"]["transitions"].as_array().unwrap().len(),
        3 + 3
    );
    let analysis = ok(json!({"op": "analyze", "document": r["document"]}));
    assert_eq!(analysis["complete"], true);
}

#[test]
fn split_labels_and_remove_useless_states() {
    let doc = json!({
        "type": "fa", "initial": "a", "finals": ["b"],
        "transitions": [["a", "xyz", "b"], ["b", "q", "dead"]]
    });
    let r = ok(json!({"op": "convert", "document": doc.clone(), "to": "split_labels"}));
    assert_eq!(state_count(&r["document"]), 3 + 2);
    assert_eq!(r["steps"][0]["kind"], "split_label");
    assert_eq!(r["steps"][0]["label"], "xyz");
    let sim = ok(json!({"op": "simulate", "document": r["document"], "input": "xyz"}));
    assert_eq!(sim["outcome"], "accepted");
    let r = ok(json!({"op": "convert", "document": doc, "to": "remove_useless_states"}));
    assert_eq!(state_count(&r["document"]), 2);
    assert_eq!(r["steps"][0]["states"], json!(["dead"]));
}

#[test]
fn fa_and_regular_grammar() {
    let r = ok(json!({"op": "convert", "document": abb_nfa(), "to": "regular_grammar"}));
    assert_eq!(r["document"]["type"], "grammar");
    assert_eq!(r["document"]["start"], "S");
    let back = ok(json!({"op": "convert", "document": r["document"], "to": "fa"}));
    assert_eq!(back["document"]["type"], "fa");
    assert_eq!(
        equivalent(&abb_nfa(), &back["document"])["equivalent"],
        true
    );

    let grammar = json!({"type": "grammar", "text": "S -> aS | bA\nA -> bA | λ"});
    let r = ok(json!({"op": "convert", "document": grammar, "to": "fa"}));
    for (input, outcome) in [("aab", "accepted"), ("a", "rejected"), ("bbb", "accepted")] {
        let s = ok(json!({"op": "simulate", "document": r["document"], "input": input}));
        assert_eq!(s["outcome"], outcome, "input {input}");
    }
    let e = err(json!({"op": "convert", "to": "fa",
        "document": {"type": "grammar", "text": "S -> Sa | a"}}));
    assert_eq!(e["code"], "conversion_failed");
}

#[test]
fn cfg_to_pda_both_constructions() {
    let grammar = json!({"type": "grammar", "text": "S -> aSb | λ"});
    for to in ["pda_ll", "pda_lr"] {
        let r = ok(json!({"op": "convert", "document": grammar.clone(), "to": to}));
        assert_eq!(r["document"]["type"], "pda", "{to}");
        for input in ["", "ab", "aabb"] {
            let s = ok(json!({"op": "simulate", "document": r["document"], "input": input}));
            assert_eq!(s["outcome"], "accepted", "{to} on {input:?}");
        }
        // the LR machine may reduce forever, so a bad string is rejected or undecided
        let s = ok(json!({"op": "simulate", "document": r["document"], "input": "aab"}));
        assert_ne!(s["outcome"], "accepted", "{to} on aab");
    }
}

#[test]
fn pda_to_cfg() {
    // pushes and pops one symbol per step, ends in one final state popping Z
    let pda = json!({
        "type": "pda", "initial": "q0", "finals": ["qf"],
        "transitions": [
            ["q0", "a,Z;AZ", "q0"], ["q0", "a,A;AA", "q0"], ["q0", "b,A;λ", "q1"],
            ["q1", "b,A;λ", "q1"], ["q1", "λ,Z;A", "qf"]
        ]
    });
    let e = err(json!({"op": "convert", "document": pda.clone(), "to": "cfg"}));
    assert_eq!(e["code"], "conversion_failed");
    assert!(e["message"].as_str().unwrap().contains("pop"));

    let ok_pda = json!({
        "type": "pda", "initial": "q0", "finals": ["qf"],
        "transitions": [
            ["q0", "a,Z;AZ", "q0"], ["q0", "a,A;AA", "q0"], ["q0", "b,A;λ", "q1"],
            ["q1", "b,A;λ", "q1"], ["q1", "λ,Z;λ", "qf"]
        ]
    });
    let r = ok(json!({"op": "convert", "document": ok_pda, "to": "cfg"}));
    assert_eq!(r["document"]["type"], "grammar");
    assert!(!r["document"]["productions"].as_array().unwrap().is_empty());
}

#[test]
fn grammar_transformations() {
    let g = json!({"type": "grammar", "text": "S -> aSb | A | λ\nA -> a | B\nB -> b"});
    let r = ok(json!({"op": "convert", "document": g.clone(), "to": "remove_lambda"}));
    let text = grammar_text(&r["document"]);
    assert!(!text.contains("λ"), "{text}");
    assert!(r["lambda_set"].is_array());

    let r = ok(json!({"op": "convert", "document": g.clone(), "to": "remove_unit"}));
    let text = grammar_text(&r["document"]);
    assert!(
        !text.lines().any(|l| l == "S → A" || l == "A → B"),
        "{text}"
    );

    let g2 = json!({"type": "grammar", "text": "S -> aS | a | B\nB -> Bb\nC -> c"});
    let r = ok(json!({"op": "convert", "document": g2, "to": "remove_useless"}));
    let text = grammar_text(&r["document"]);
    assert!(!text.contains('B') && !text.contains('C'), "{text}");

    let r = ok(json!({"op": "convert", "document": g, "to": "cnf"}));
    assert_eq!(r["document"]["type"], "grammar");
    let check = ok(json!({"op": "grammar_type", "grammar": r["document"]}));
    assert_eq!(check["report"]["cnf"], true);
    assert!(r["steps"].is_object());
}

fn grammar_text(doc: &Value) -> String {
    doc["productions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            format!(
                "{} → {}",
                p["lhs"].as_str().unwrap(),
                if p["rhs"] == "" {
                    "λ"
                } else {
                    p["rhs"].as_str().unwrap()
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn turing_machine_to_unrestricted_grammar() {
    let tm = json!({
        "type": "turing", "initial": "q0", "finals": ["q1"],
        "transitions": [["q0", "a;a,R", "q0"], ["q0", "_;_,S", "q1"]]
    });
    let r = ok(json!({"op": "convert", "document": tm, "to": "unrestricted_grammar"}));
    assert_eq!(r["document"]["type"], "grammar");
    assert!(r["warnings"].is_array());
    let e = err(json!({"op": "convert", "document": anbncn_tm_two_tapes(),
                       "to": "unrestricted_grammar"}));
    assert_eq!(e["code"], "conversion_failed");
}

fn anbncn_tm_two_tapes() -> Value {
    json!({"type": "turing", "initial": "q", "transitions": [["q", "a;a,R|a;a,R", "q"]]})
}

#[test]
fn conversions_reject_the_wrong_type() {
    for (doc, to) in [
        (anbn_pda(), "dfa"),
        (abb_nfa(), "cnf"),
        (abb_nfa(), "cfg"),
        (anbn_pda(), "regex"),
        (abb_nfa(), "pda_ll"),
        (abb_nfa(), "unrestricted_grammar"),
        (json!({"type": "regex", "expression": "a"}), "complete"),
    ] {
        let e = err(json!({"op": "convert", "document": doc, "to": to}));
        assert_eq!(e["code"], "wrong_document_type", "to {to}");
    }
    let e = err(json!({"op": "convert", "document": abb_nfa(), "to": "teleport"}));
    assert_eq!(e["code"], "invalid_request");
    assert_eq!(
        strings(
            &ok(json!({"op": "capabilities"}))["convert"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["to"].clone())
                .collect::<Value>()
        )
        .len(),
        17
    );
}
