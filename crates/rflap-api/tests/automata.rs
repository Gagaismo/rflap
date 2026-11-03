mod common;

use common::{abb_nfa, anbn_pda, anbncn_tm, err, ok, strings};
use serde_json::{Value, json};

fn lambda_fa() -> Value {
    json!({
        "type": "fa",
        "initial": "q0",
        "finals": ["q2"],
        "transitions": [["q0", "a", "q1"], ["q1", "", "q2"]]
    })
}

#[test]
fn compact_fa_accepts_with_a_trace() {
    let r = ok(json!({"op": "simulate", "document": lambda_fa(), "input": "a"}));
    assert_eq!(r["outcome"], "accepted");
    assert_eq!(r["accepted"], true);
    let trace = r["trace"].as_array().unwrap();
    let states: Vec<_> = trace.iter().map(|c| c["state"].as_str().unwrap()).collect();
    assert_eq!(states, ["q0", "q1", "q2"]);
    let remaining: Vec<_> = trace
        .iter()
        .map(|c| c["remaining"].as_str().unwrap())
        .collect();
    assert_eq!(remaining, ["a", "", ""]);
    assert_eq!(trace[2]["state_id"], 2);
}

#[test]
fn compact_fa_rejects() {
    for input in ["", "b", "aa", "ab"] {
        let r = ok(json!({"op": "simulate", "document": lambda_fa(), "input": input}));
        assert_eq!(r["outcome"], "rejected", "input {input:?}");
        assert_eq!(r["accepted"], false);
        assert_eq!(r["trace"], json!([]));
    }
}

#[test]
fn closure_mode_folds_the_lambda_steps() {
    let r = ok(json!({
        "op": "simulate", "document": lambda_fa(), "input": "a", "mode": "closure"
    }));
    assert_eq!(r["outcome"], "accepted");
    let states: Vec<_> = r["trace"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["state"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(states, ["q0", "q2"]);
}

#[test]
fn a_budget_that_runs_out_is_undecided() {
    let r = ok(json!({
        "op": "simulate",
        "document": {"type": "fa", "initial": "q", "transitions": [["q", "a", "q"]]},
        "input": "aaaaaaaaaa", "max_configs": 3
    }));
    assert_eq!(r["outcome"], "undecided");
}

#[test]
fn fa_ranges_in_labels() {
    let doc = json!({
        "type": "fa", "initial": "s", "finals": ["d"],
        "transitions": [["s", "[0-9]", "d"], ["d", "[0-9]", "d"]]
    });
    let r = ok(json!({"op": "simulate", "document": doc.clone(), "input": "2024"}));
    assert_eq!(r["outcome"], "accepted");
    let r = ok(json!({"op": "simulate", "document": doc, "input": "20x4"}));
    assert_eq!(r["outcome"], "rejected");
}

#[test]
fn normalize_expands_names_and_lays_out() {
    let r = ok(json!({"op": "normalize", "document": lambda_fa()}));
    let d = &r["document"];
    assert_eq!(d["type"], "fa");
    assert_eq!(d["initial"], 0);
    assert_eq!(d["finals"], json!([2]));
    let states = d["states"].as_object().unwrap();
    assert_eq!(states.len(), 3);
    let names: Vec<_> = (0..3)
        .map(|i| states[&i.to_string()]["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["q0", "q1", "q2"]);
    let points: Vec<_> = (0..3)
        .map(|i| {
            let p = &states[&i.to_string()]["point"];
            (p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap())
        })
        .collect();
    assert!(
        points[0] != points[1] && points[1] != points[2] && points[0] != points[2],
        "states were not laid out: {points:?}"
    );
    assert_eq!(d["transitions"].as_array().unwrap().len(), 2);
    assert_eq!(d["transitions"][1]["label"], "");
    // the full form is accepted again and normalizing is idempotent
    let again = ok(json!({"op": "normalize", "document": d.clone()}));
    assert_eq!(again["document"], *d);
}

#[test]
fn explicit_points_are_kept() {
    let r = ok(json!({"op": "normalize", "document": {
        "type": "fa", "initial": "a", "finals": ["b"],
        "transitions": [["a", "x", "b"]],
        "points": {"a": [10, 20], "b": [300, 40]}
    }}));
    let states = r["document"]["states"].as_object().unwrap();
    assert_eq!(states["0"]["point"], json!({"x": 10.0, "y": 20.0}));
    assert_eq!(states["1"]["point"], json!({"x": 300.0, "y": 40.0}));
}

#[test]
fn partial_points_fix_the_given_states() {
    let r = ok(json!({"op": "normalize", "document": {
        "type": "fa", "initial": "a", "finals": ["c"],
        "transitions": [["a", "x", "b"], ["b", "y", "c"]],
        "points": {"a": [10, 20]},
        "layout": "spiral"
    }}));
    let states = r["document"]["states"].as_object().unwrap();
    assert_eq!(states.len(), 3);
    assert_eq!(states["0"]["point"], json!({"x": 10.0, "y": 20.0}));
    assert_ne!(states["1"]["point"], states["2"]["point"]);
}

#[test]
fn compact_save_open_roundtrip() {
    let normalized = ok(json!({"op": "normalize", "document": lambda_fa()}))["document"].clone();
    let saved = ok(json!({"op": "save_jff", "document": lambda_fa()}));
    let xml = saved["xml"].as_str().unwrap();
    assert!(xml.contains("<type>fa</type>"), "{xml}");
    let reopened = ok(json!({"op": "open_jff", "xml": xml}))["document"].clone();
    assert_eq!(reopened["type"], "fa");
    assert_eq!(reopened["initial"], normalized["initial"]);
    assert_eq!(reopened["finals"], normalized["finals"]);
    let transitions = |d: &Value| -> Vec<(u64, u64, String)> {
        d["transitions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                (
                    t["from"].as_u64().unwrap(),
                    t["to"].as_u64().unwrap(),
                    t["label"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    };
    assert_eq!(transitions(&reopened), transitions(&normalized));
    // the second trip is a fixed point
    let again = ok(json!({"op": "save_jff", "document": reopened.clone()}));
    assert_eq!(again["xml"], saved["xml"]);
    // and the reopened machine still works
    let r = ok(json!({"op": "simulate", "document": reopened, "input": "a"}));
    assert_eq!(r["outcome"], "accepted");
}

#[test]
fn every_document_type_survives_jff() {
    let documents = [
        anbn_pda(),
        anbncn_tm(),
        json!({"type": "mealy", "initial": "q0", "transitions": [["q0", "a;1", "q0"]]}),
        json!({"type": "moore", "initial": "q0", "outputs": {"q0": "x"},
               "transitions": [["q0", "a", "q0"]]}),
        json!({"type": "grammar", "text": "S -> aSb | λ"}),
        json!({"type": "regex", "expression": "(a+b)*abb"}),
        json!({"type": "lsystem", "axiom": "F",
               "rules": ["F -> F + F"], "parameters": {"angle": "90"}}),
        json!({"type": "regular_pumping_lemma", "language": 0}),
        json!({"type": "context_free_pumping_lemma", "language": 0, "mode": "human_first"}),
    ];
    for doc in documents {
        let kind = doc["type"].as_str().unwrap().to_string();
        let full = ok(json!({"op": "normalize", "document": doc}))["document"].clone();
        let xml = ok(json!({"op": "save_jff", "document": full.clone()}))["xml"]
            .as_str()
            .unwrap()
            .to_string();
        let back = ok(json!({"op": "open_jff", "xml": xml.clone()}))["document"].clone();
        assert_eq!(back["type"], kind, "{xml}");
        let xml2 = ok(json!({"op": "save_jff", "document": back}))["xml"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(xml2, xml, "{kind} is not a fixed point of save/open");
    }
}

#[test]
fn pda_final_state_a_n_b_n() {
    let accepts = ["", "ab", "aabb", "aaabbb"];
    let rejects = ["a", "b", "abb", "aab", "ba", "aabbb"];
    for input in accepts {
        let r = ok(json!({"op": "simulate", "document": anbn_pda(), "input": input}));
        assert_eq!(r["outcome"], "accepted", "input {input:?}");
    }
    for input in rejects {
        let r = ok(json!({"op": "simulate", "document": anbn_pda(), "input": input}));
        assert_eq!(r["outcome"], "rejected", "input {input:?}");
    }
    let r = ok(json!({"op": "simulate", "document": anbn_pda(), "input": "ab"}));
    let last = r["trace"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["state"], "q2");
    assert_eq!(last["stack"], "Z");
    assert_eq!(last["remaining"], "");
}

#[test]
fn pda_closure_mode_and_empty_stack() {
    let r = ok(
        json!({"op": "simulate", "document": anbn_pda(), "input": "aabb",
                      "mode": "closure"}),
    );
    assert_eq!(r["outcome"], "accepted");
    let empty_stack = json!({
        "type": "pda", "initial": "q0",
        "transitions": [["q0", "a,Z;λ", "q0"]]
    });
    let r = ok(
        json!({"op": "simulate", "document": empty_stack.clone(), "input": "a",
                      "acceptance": "empty_stack"}),
    );
    assert_eq!(r["outcome"], "accepted");
    let r = ok(
        json!({"op": "simulate", "document": empty_stack, "input": "a",
                      "acceptance": "final_state"}),
    );
    assert_eq!(r["outcome"], "rejected");
}

#[test]
fn turing_machine_a_n_b_n_c_n() {
    for input in ["", "abc", "aabbcc", "aaabbbccc"] {
        let r = ok(json!({"op": "simulate", "document": anbncn_tm(), "input": input}));
        assert_eq!(r["outcome"], "accepted", "input {input:?}");
    }
    for input in ["a", "ab", "abcc", "aabbc", "cba", "abab", "aabcc"] {
        let r = ok(json!({"op": "simulate", "document": anbncn_tm(), "input": input}));
        assert_eq!(r["outcome"], "rejected", "input {input:?}");
    }
    let r = ok(json!({"op": "simulate", "document": anbncn_tm(), "input": "abc"}));
    let last = r["trace"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["state"], "qf");
    assert!(
        last["tapes"][0]["content"]
            .as_str()
            .unwrap()
            .contains("XYZ")
    );
}

#[test]
fn turing_tape_outputs_are_the_final_tape_without_surrounding_blanks() {
    // The head ends past the data (on the blank after `aa`): the output is still `aa`.
    let walk = json!({
        "type": "turing", "initial": "q0", "finals": ["q1"],
        "transitions": [["q0", "a;a,R", "q0"], ["q0", "_;_,S", "q1"]]
    });
    let r = ok(json!({"op": "simulate", "document": walk, "input": "aa"}));
    assert_eq!(r["outcome"], "accepted");
    assert_eq!(r["tape_outputs"], json!(["aa"]));
}

#[test]
fn turing_machine_output_and_halting() {
    let rewrite = json!({
        "type": "turing", "initial": "q0", "finals": ["q2"],
        "transitions": [
            ["q0", "a;b,R", "q0"], ["q0", "_;_,L", "q1"],
            ["q1", "b;b,L", "q1"], ["q1", "_;_,R", "q2"]
        ]
    });
    let r = ok(json!({"op": "simulate", "document": rewrite, "input": "aa"}));
    assert_eq!(r["outcome"], "accepted");
    assert_eq!(r["tape_outputs"], json!(["bb"]));

    let halting = json!({"type": "turing", "initial": "q0",
                         "transitions": [["q0", "a;a,R", "q0"]]});
    let by_final = ok(json!({"op": "simulate", "document": halting.clone(), "input": "aa"}));
    assert_eq!(by_final["outcome"], "rejected");
    let by_halt = ok(
        json!({"op": "simulate", "document": halting.clone(), "input": "aa",
                            "acceptance": "halting"}),
    );
    assert_eq!(by_halt["outcome"], "accepted");
    let both = ok(json!({"op": "simulate", "document": halting, "input": "aa",
                         "acceptance": "both"}));
    assert_eq!(both["outcome"], "accepted");
}

#[test]
fn multi_tape_turing_machine() {
    let equal = json!({
        "type": "turing", "initial": "q0", "finals": ["qf"],
        "transitions": [
            ["q0", "a;a,R|a;a,R", "q0"], ["q0", "b;b,R|b;b,R", "q0"],
            ["q0", "_;_,S|_;_,S", "qf"]
        ]
    });
    let norm = ok(json!({"op": "normalize", "document": equal.clone()}));
    assert_eq!(norm["document"]["tapes"], 2);
    let r = ok(json!({"op": "simulate", "document": equal.clone(), "inputs": ["ab", "ab"]}));
    assert_eq!(r["outcome"], "accepted");
    let r = ok(json!({"op": "simulate", "document": equal.clone(), "inputs": ["ab", "aa"]}));
    assert_eq!(r["outcome"], "rejected");
    let r = ok(json!({"op": "simulate", "document": equal.clone(), "input": "ab"}));
    assert_eq!(r["outcome"], "accepted");
    let e = err(json!({"op": "simulate", "document": equal, "inputs": ["ab"]}));
    assert_eq!(e["code"], "invalid_argument");
}

#[test]
fn turing_building_blocks_in_the_compact_form() {
    let doc = json!({
        "type": "turing", "initial": "start", "finals": ["done"],
        "transitions": [["start", "a;a,S", "block"], ["block", "_;_,S", "done"]],
        "blocks": {"block": {
            "initial": "m0", "finals": ["m1"],
            "transitions": [["m0", "a;b,R", "m1"]]
        }}
    });
    let r = ok(json!({"op": "simulate", "document": doc, "input": "a"}));
    assert_eq!(r["outcome"], "accepted");
}

#[test]
fn mealy_output() {
    let doc = json!({
        "type": "mealy", "initial": "q0",
        "transitions": [["q0", "a;1", "q0"], ["q0", "b;0", "q0"]]
    });
    let r = ok(json!({"op": "simulate", "document": doc, "input": "abba"}));
    assert_eq!(r["outcome"], "accepted");
    assert_eq!(r["output"], "1001");
    let norm = ok(json!({"op": "normalize", "document": {
        "type": "mealy", "initial": "q0", "transitions": [["q0", "a;1", "q0"]]}}));
    assert_eq!(
        norm["document"]["transitions"][0]["label"],
        json!({"input": "a", "output": "1"})
    );
}

#[test]
fn moore_output() {
    let doc = json!({
        "type": "moore", "initial": "q0", "outputs": {"q0": "x", "q1": "y"},
        "transitions": [["q0", "a", "q1"], ["q1", "a", "q0"]]
    });
    let r = ok(json!({"op": "simulate", "document": doc, "input": "aa"}));
    assert_eq!(r["output"], "xyx");
}

#[test]
fn step_returns_the_frontier() {
    let doc = json!({
        "type": "fa", "initial": "q0", "finals": ["q1"],
        "transitions": [["q0", "a", "q0"], ["q0", "a", "q1"]]
    });
    let r = ok(json!({"op": "step", "document": doc.clone(), "input": "a", "steps": 1}));
    assert_eq!(r["steps_done"], 1);
    assert_eq!(r["finished"], false);
    let frontier: Vec<_> = r["frontier"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["state"].as_str().unwrap())
        .collect();
    assert_eq!(frontier, ["q0", "q1"]);
    let accepted = r["accepted"].as_array().unwrap();
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0]["config"]["state"], "q1");
    assert_eq!(accepted[0]["trace"].as_array().unwrap().len(), 2);

    let r = ok(json!({"op": "step", "document": doc.clone(), "input": "a", "steps": 0}));
    assert_eq!(r["steps_done"], 0);
    assert_eq!(r["frontier"].as_array().unwrap().len(), 1);

    // after more steps than the input allows the run is finished and the dead ends are rejected
    let r = ok(json!({"op": "step", "document": doc, "input": "a", "steps": 5}));
    assert_eq!(r["finished"], true);
    assert_eq!(r["steps_done"], 2);
    assert_eq!(r["rejected"].as_array().unwrap().len(), 1);
}

#[test]
fn multiple_run_gives_one_result_per_input() {
    let r = ok(json!({"op": "multiple_run", "document": abb_nfa(),
                      "inputs": ["abb", "aabb", "ab", ""]}));
    let results = r["results"].as_array().unwrap();
    let outcomes: Vec<_> = results
        .iter()
        .map(|x| x["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(outcomes, ["accepted", "accepted", "rejected", "rejected"]);
    assert_eq!(results[1]["input"], "aabb");
    let r = ok(json!({"op": "multiple_run",
        "document": {"type": "mealy", "initial": "q", "transitions": [["q", "a;x", "q"]]},
        "inputs": ["aa", "ab"]}));
    assert_eq!(r["results"][0]["output"], "xx");
    assert_eq!(r["results"][1]["outcome"], "rejected");
}

#[test]
fn analyze_an_nfa() {
    let doc = json!({
        "type": "fa", "initial": "q0", "finals": ["q2"],
        "transitions": [
            ["q0", "a", "q0"], ["q0", "a", "q1"], ["q1", "", "q2"], ["q3", "b", "q3"]
        ]
    });
    let r = ok(json!({"op": "analyze", "document": doc}));
    assert_eq!(r["type"], "fa");
    assert_eq!(r["kind"], "nfa");
    assert_eq!(r["deterministic"], false);
    assert_eq!(strings(&r["alphabet"]), ["a", "b"]);
    assert_eq!(strings(&r["nondeterministic_states"]), ["q0", "q1"]);
    assert_eq!(r["lambda_transitions"].as_array().unwrap().len(), 1);
    assert_eq!(r["lambda_transitions"][0]["from"], "q1");
    assert_eq!(r["lambda_transitions"][0]["index"], 2);
    assert_eq!(strings(&r["unreachable_states"]), ["q3"]);
    assert_eq!(strings(&r["useless_states"]), ["q3"]);
    assert_eq!(r["initial"], "q0");
    assert_eq!(strings(&r["finals"]), ["q2"]);
    assert_eq!(r["states"].as_array().unwrap().len(), 4);
    assert_eq!(r["complete"], false);
}

#[test]
fn analyze_a_dfa_and_other_machines() {
    let dfa = json!({
        "type": "fa", "initial": "p", "finals": ["p"],
        "transitions": [["p", "a", "q"], ["p", "b", "p"], ["q", "a", "p"], ["q", "b", "q"]]
    });
    let r = ok(json!({"op": "analyze", "document": dfa}));
    assert_eq!(r["kind"], "dfa");
    assert_eq!(r["deterministic"], true);
    assert_eq!(r["complete"], true);
    assert_eq!(r["nondeterministic_states"], json!([]));

    let r = ok(json!({"op": "analyze", "document": anbn_pda()}));
    assert_eq!(r["kind"], "npda");
    let r = ok(json!({"op": "analyze", "document": anbncn_tm()}));
    assert_eq!(r["type"], "turing");
    assert_eq!(r["tapes"], 1);
}

#[test]
fn analyze_documents_that_are_not_graphs() {
    let r = ok(json!({"op": "analyze", "document": {"type": "grammar", "text": "S -> aS | a"}}));
    assert_eq!(r["type"], "grammar");
    assert_eq!(r["start"], "S");
    assert_eq!(r["classification"], "right_linear");
    let r = ok(json!({"op": "analyze", "document": {"type": "regex", "expression": "a*b"}}));
    assert_eq!(r["nullable"], false);
    assert_eq!(strings(&r["alphabet"]), ["a", "b"]);
}

#[test]
fn layout_moves_the_states() {
    let doc = json!({
        "type": "fa", "initial": "a", "finals": ["d"],
        "transitions": [["a", "x", "b"], ["a", "x", "c"], ["b", "x", "d"], ["c", "x", "d"]]
    });
    let circle = ok(json!({"op": "layout", "document": doc.clone(), "algorithm": "circle"}));
    let tree = ok(json!({"op": "layout", "document": doc.clone(), "algorithm": "tree"}));
    assert_ne!(circle["document"]["states"], tree["document"]["states"]);
    assert_eq!(
        circle["document"]["transitions"],
        tree["document"]["transitions"]
    );
    for algorithm in [
        "circle",
        "gem",
        "random",
        "spiral",
        "tree",
        "tree-degree",
        "two-circle",
    ] {
        let r = ok(json!({"op": "layout", "document": doc.clone(), "algorithm": algorithm}));
        assert_eq!(r["document"]["states"].as_object().unwrap().len(), 4);
    }
    let again = ok(json!({"op": "layout", "document": doc, "algorithm": "gem",
                          "options": {"seed": 7, "width": 400, "height": 300}}));
    assert!(
        again["document"]["states"]["0"]["point"]["x"]
            .as_f64()
            .unwrap()
            <= 400.0
    );
    let e = err(
        json!({"op": "layout", "document": {"type": "fa", "initial": "a"},
                       "algorithm": "zigzag"}),
    );
    assert_eq!(e["code"], "invalid_argument");
}

#[test]
fn combine_appends_the_second_machine() {
    let r = ok(json!({"op": "combine", "a": lambda_fa(), "b": abb_nfa()}));
    let d = &r["document"];
    assert_eq!(d["states"].as_object().unwrap().len(), 3 + 4);
    assert_eq!(d["transitions"].as_array().unwrap().len(), 2 + 5);
    assert_eq!(d["initial"], 0);
}

#[test]
fn calls_are_deterministic() {
    let request = json!({"op": "normalize", "document": abb_nfa()}).to_string();
    assert_eq!(rflap_api::call(&request), rflap_api::call(&request));
    let request = json!({"op": "layout", "document": abb_nfa(), "algorithm": "random"}).to_string();
    assert_eq!(rflap_api::call(&request), rflap_api::call(&request));
}

#[test]
fn combine_appends_the_second_turing_machine_with_its_blocks() {
    let first = json!({"type": "turing", "initial": "p0", "finals": ["p1"],
                       "transitions": [["p0", "a;a,R", "p1"]]});
    let second = json!({
        "type": "turing", "initial": "start", "finals": ["done"],
        "transitions": [["start", "a;a,S", "block"], ["block", "_;_,S", "done"]],
        "blocks": {"block": {"initial": "m0", "finals": ["m1"],
                             "transitions": [["m0", "a;b,R", "m1"]]}}
    });
    let r = ok(json!({"op": "combine", "a": first, "b": second}));
    let d = &r["document"];
    assert_eq!(d["type"], "turing");
    assert_eq!(d["automaton"]["states"].as_object().unwrap().len(), 2 + 3);
    assert_eq!(
        d["automaton"]["transitions"].as_array().unwrap().len(),
        1 + 2
    );
    assert_eq!(d["blocks"].as_object().unwrap().len(), 1, "{d}");
    // The combined machine still runs the first machine's initial state.
    let sim = ok(json!({"op": "simulate", "document": d.clone(), "input": "a"}));
    assert_eq!(sim["outcome"], "accepted");

    let two_tapes = json!({"type": "turing", "tapes": 2, "initial": "q", "finals": ["q"],
                           "transitions": []});
    let e = err(json!({"op": "combine", "a": d.clone(), "b": two_tapes}));
    assert_eq!(e["code"], "invalid_argument", "{e}");
}
