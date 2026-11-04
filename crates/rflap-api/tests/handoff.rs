//! Fixes of the handoff feedback: default layered layout, `title`/`name`, `<` `>` as the
//! delimiters of an ALL, Σ_E and Σ_A in `analyze`, course labels in the generic operations.

mod common;

use common::*;
use serde_json::{Value, json};

/// The ALL of the handoff (inciso a), with `lt` and `gt` as its delimiters.
fn all_machine(lt: &str, gt: &str) -> Value {
    let rows = [
        ["q0", "(a,a,D)", "q1"],
        ["q1", "(a,a,D)", "q1"],
        ["q1", "(b,b,D)", "q2"],
        ["q1", "(>,>,I)", "q3"],
        ["q2", "(b,b,D)", "q2"],
        ["q2", "(>,>,I)", "q3"],
        ["q3", "(a,a,I) / (b,b,I)", "q3"],
        ["q3", "(<,<,D)", "q4"],
        ["q4", "(a,X,D)", "q5"],
        ["q4", "(Y,Y,D)", "q7"],
        ["q5", "(a,a,D) / (Y,Y,D)", "q5"],
        ["q5", "(b,Y,I)", "q6"],
        ["q5", "(>,>,I)", "qf"],
        ["q6", "(a,a,I) / (Y,Y,I)", "q6"],
        ["q6", "(X,X,D)", "q4"],
        ["q7", "(Y,Y,D)", "q7"],
        ["q7", "(b,b,D)", "qf"],
    ];
    let transitions: Vec<Value> = rows
        .iter()
        .map(|[from, label, to]| json!([from, label.replace('<', lt).replace('>', gt), to]))
        .collect();
    json!({"type": "turing", "initial": "q0", "finals": ["qf"], "transitions": transitions})
}

const INPUTS: [&str; 12] = [
    "", "b", "ba", "aba", "ab", "aabb", "a", "aa", "aab", "abb", "aaabb", "abbb",
];
const ACCEPTED: [bool; 12] = [
    false, false, false, false, false, false, true, true, true, true, true, true,
];

fn points(r: &Value) -> Vec<(f64, f64)> {
    let states = r["document"]["states"].as_object().unwrap();
    (0..states.len())
        .map(|i| {
            let p = &states[&i.to_string()]["point"];
            (p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap())
        })
        .collect()
}

fn normalize(doc: Value) -> Value {
    ok(json!({"op": "normalize", "document": doc}))
}

// S4

#[test]
fn compact_without_layout_puts_the_initial_state_left_and_orders_by_bfs() {
    let r = normalize(json!({"type": "fa", "initial": "s", "finals": ["d"],
        "transitions": [["s", "a", "b"], ["s", "a", "c"], ["b", "a", "d"], ["c", "a", "d"]],
        "states": ["d", "c", "b", "s"]}));
    // creation order: d=0, c=1, b=2, s=3
    let p = points(&r);
    assert!(p[3].0 < p[1].0 && p[3].0 < p[2].0, "{p:?}");
    assert_eq!(p[1].0, p[2].0, "{p:?}");
    assert!(p[1].0 < p[0].0, "{p:?}");
    assert!(p[1].1 < p[2].1, "creation order inside a column: {p:?}");
    // the default is exactly `layered`
    let explicit = normalize(json!({"type": "fa", "initial": "s", "finals": ["d"],
        "transitions": [["s", "a", "b"], ["s", "a", "c"], ["b", "a", "d"], ["c", "a", "d"]],
        "states": ["d", "c", "b", "s"], "layout": "layered"}));
    assert_eq!(r, explicit);
}

#[test]
fn compact_layout_values_are_accepted_and_points_still_win() {
    let doc = |extra: Value| {
        let mut d = json!({"type": "fa", "initial": "a", "finals": ["c"],
            "transitions": [["a", "x", "b"], ["b", "y", "c"]]});
        d.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        d
    };
    for layout in [
        "circle",
        "gem",
        "random",
        "spiral",
        "tree",
        "tree-degree",
        "two-circle",
        "layered",
    ] {
        normalize(doc(json!({"layout": layout})));
    }
    let circle = normalize(doc(json!({"layout": "circle"})));
    let layered = normalize(doc(json!({})));
    assert_ne!(points(&circle), points(&layered));
    let fixed = normalize(doc(
        json!({"points": {"a": [500, 20], "b": [10, 20], "c": [200, 300]}}),
    ));
    assert_eq!(
        points(&fixed),
        [(500.0, 20.0), (10.0, 20.0), (200.0, 300.0)]
    );
    let e = err(json!({"op": "normalize", "document": doc(json!({"layout": "zigzag"}))}));
    assert_eq!(e["code"], "invalid_argument");
}

// S2

#[test]
fn every_compact_type_accepts_title_and_name_as_metadata() {
    let docs = [
        json!({"type": "fa", "initial": "q0", "transitions": [["q0", "a", "q0"]]}),
        json!({"type": "pda", "initial": "q0", "transitions": [["q0", "a,Z;AZ", "q0"]]}),
        json!({"type": "turing", "initial": "q0", "transitions": [["q0", "(a,a,D)", "q0"]]}),
        json!({"type": "mealy", "initial": "q0", "transitions": [["q0", "a;1", "q0"]]}),
        json!({"type": "moore", "initial": "q0", "outputs": {"q0": "x"},
               "transitions": [["q0", "a", "q0"]]}),
        json!({"type": "grammar", "text": "S -> aSb | λ"}),
        json!({"type": "regex", "expression": "(a+b)*abb"}),
        json!({"type": "lsystem", "axiom": "F", "rules": ["F -> F + F"],
               "parameters": {"angle": "90"}}),
        json!({"type": "regular_pumping_lemma", "language": 0, "mode": "computer_first",
               "seed": 1}),
        json!({"type": "context_free_pumping_lemma", "language": 0, "mode": "computer_first",
               "seed": 1}),
    ];
    for doc in docs {
        let plain = normalize(doc.clone());
        for key in ["title", "name"] {
            let mut titled = doc.clone();
            titled[key] = json!("Ejercicio 2.1 a)");
            assert_eq!(normalize(titled), plain, "{} with {key}", doc["type"]);
        }
        let mut bad = doc.clone();
        bad["title"] = json!(5);
        assert_eq!(
            err(json!({"op": "normalize", "document": bad}))["code"],
            "invalid_document",
            "{}",
            doc["type"]
        );
    }
    // other unknown fields are still errors
    let e = err(json!({"op": "normalize", "document":
        {"type": "turing", "initial": "q0", "label": "x", "transitions": []}}));
    assert_eq!(e["code"], "invalid_document");
    assert!(
        e["message"]
            .as_str()
            .unwrap()
            .contains("unknown field \"label\"")
    );
}

// S8

fn check(doc: Value) -> Value {
    ok(json!({"op": "all_check", "document": doc, "inputs": INPUTS, "max_steps": 1000}))
}

fn verdicts(runs: &Value) -> Vec<bool> {
    runs.as_array()
        .unwrap()
        .iter()
        .map(|r| r["verdict"] == "accepted")
        .collect()
}

#[test]
fn all_check_reads_lt_gt_as_the_delimiters_of_an_all() {
    let real = check(all_machine("⟨", "⟩"));
    assert_eq!(real["ok"], true, "{real}");
    assert_eq!(verdicts(&real["runs"]), ACCEPTED);
    let aliased = check(all_machine("<", ">"));
    assert_eq!(aliased["ok"], true, "{aliased}");
    assert_eq!(verdicts(&aliased["runs"]), ACCEPTED);
    assert_eq!(aliased["runs"], real["runs"]);
}

#[test]
fn catedra_run_reads_lt_gt_as_delimiters_with_or_without_an_explicit_model() {
    let run = |model: Option<&str>| {
        let mut req = json!({"op": "catedra_run", "document": all_machine("<", ">"),
                             "inputs": INPUTS});
        if let Some(m) = model {
            req["model"] = json!(m);
        }
        ok(req)
    };
    let real = ok(
        json!({"op": "catedra_run", "document": all_machine("⟨", "⟩"),
                         "inputs": INPUTS, "model": "all"}),
    );
    // a machine using both `<` and `>` is normalized to ⟨ ⟩, so it is detected as an ALL
    for model in [None, Some("all")] {
        let r = run(model);
        assert_eq!(r["model"], "all");
        let accepted: Vec<bool> = r["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["accepted"] == true)
            .collect();
        assert_eq!(accepted, ACCEPTED);
        assert_eq!(r["results"], real["results"]);
    }
}

#[test]
fn catedra_definition_shows_the_normalized_delimiters() {
    let d = ok(
        json!({"op": "catedra_definition", "document": all_machine("<", ">"),
                      "model": "all"}),
    );
    assert_eq!(strings(&d["auxiliary_alphabet"]), ["⟨", "⟩", "X", "Y"]);
}

#[test]
fn a_machine_mixing_delimiters_and_lt_keeps_lt_literal() {
    let mut doc = all_machine("⟨", "⟩");
    doc["transitions"]
        .as_array_mut()
        .unwrap()
        .push(json!(["q0", "(<,<,D)", "qf"]));
    // the machine already uses ⟨ ⟩: `<` is a plain symbol read by the extra transition
    let d = ok(json!({"op": "catedra_definition", "document": doc, "model": "all"}));
    assert!(
        strings(&d["auxiliary_alphabet"]).contains(&"<".to_string()),
        "{d}"
    );
}

// S11

#[test]
fn analyze_reports_sigma_e_and_sigma_a_of_a_turing_machine() {
    let r = ok(json!({"op": "analyze", "document": all_machine("⟨", "⟩")}));
    assert_eq!(strings(&r["alphabet"]), ["a", "b"]);
    assert_eq!(strings(&r["tape_alphabet"]), ["⟨", "⟩", "X", "Y"]);
    let mt = ok(
        json!({"op": "analyze", "document": {"type": "turing", "initial": "q0",
        "finals": ["q1"], "input_alphabet": "ab",
        "transitions": [["q0", "(a,X,D)", "q0"], ["q0", "(Δ,Δ,N)", "q1"]]}}),
    );
    assert_eq!(strings(&mt["alphabet"]), ["a", "b"]);
    assert_eq!(strings(&mt["tape_alphabet"]), ["X", "Δ"]);
}

// S7

#[test]
fn compact_turing_accepts_catedra_labels_in_simulate_step_and_normalize() {
    let doc = json!({"type": "turing", "initial": "q0", "finals": ["q2"], "transitions": [
        ["q0", "(a,X,D)", "q1"], ["q1", "(a,a,I) / (b,b,I)", "q1"], ["q1", "(Δ,Δ,D)", "q2"]]});
    let n = normalize(doc.clone());
    let edges = n["document"]["automaton"]["transitions"]
        .as_array()
        .unwrap();
    assert_eq!(edges.len(), 4);
    let action = |i: usize| edges[i]["label"]["actions"][0].clone();
    assert_eq!(
        action(0),
        json!({"read": "a", "write": "X", "direction": "right"})
    );
    assert_eq!(
        action(1),
        json!({"read": "a", "write": "a", "direction": "left"})
    );
    assert_eq!(
        action(2),
        json!({"read": "b", "write": "b", "direction": "left"})
    );
    // Δ is the blank on both sides
    assert_eq!(action(3)["read"], action(3)["write"]);
    assert_eq!(action(3)["direction"], "right");
    let s = ok(json!({"op": "simulate", "document": doc.clone(), "input": "a"}));
    assert_eq!(s["outcome"], "accepted", "{s}");
    let st = ok(json!({"op": "step", "document": doc, "input": "a"}));
    assert_eq!(st["steps_done"], 1, "{st}");
}
