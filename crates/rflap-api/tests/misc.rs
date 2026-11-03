mod common;

use common::{abb_nfa, anbn_pda, err, ok, strings};
use serde_json::{Value, json};

#[test]
fn design_ends_with_ab_is_a_three_state_dfa() {
    let r = ok(json!({
        "op": "design",
        "spec": {
            "alphabet": "ab",
            "language": {"constraint_text": "ends_with(\"ab\")"},
            "examples": {"accept": ["ab", "aab", "bab"], "reject": ["", "a", "ba", "abb"]}
        }
    }));
    assert_eq!(r["kind"], "dfa");
    assert_eq!(r["document"]["type"], "fa");
    assert_eq!(r["document"]["states"].as_object().unwrap().len(), 3);
    assert_eq!(r["stats"]["states"], 3);
    assert_eq!(r["verification"]["ok"], true);
    assert_eq!(r["verification"]["failed_examples"], 0);
    assert!(r.get("automaton").is_none());
    // the designed document works with the rest of the API
    let sim = ok(json!({"op": "simulate", "document": r["document"], "input": "bbab"}));
    assert_eq!(sim["outcome"], "accepted");
    let sim = ok(json!({"op": "simulate", "document": r["document"], "input": "abb"}));
    assert_eq!(sim["outcome"], "rejected");
    let analysis = ok(json!({"op": "analyze", "document": r["document"]}));
    assert_eq!(analysis["kind"], "dfa");
}

#[test]
fn design_a_pda_template_and_options() {
    let r = ok(json!({
        "op": "design",
        "spec": {"alphabet": "ab", "language": {"template": {"name": "an_bn", "a": "a", "b": "b"}}},
        "options": {"check_len": 6}
    }));
    assert_eq!(r["kind"], "pda");
    assert_eq!(r["document"]["type"], "pda");
    let sim = ok(json!({"op": "simulate", "document": r["document"], "input": "aabb"}));
    assert_eq!(sim["outcome"], "accepted");
}

#[test]
fn design_errors_carry_the_core_error() {
    let e = err(json!({
        "op": "design",
        "spec": {"alphabet": "ab", "language": {"constraint_text": "count(a) == count(b)"}}
    }));
    assert_eq!(e["code"], "design_error");
    assert_eq!(e["details"]["error"], "not_regular");
    let e = err(json!({
        "op": "design",
        "spec": {"alphabet": "ab", "language": {"regex": "(a+"}}
    }));
    assert_eq!(e["code"], "design_error");
    assert_eq!(e["details"]["error"], "regex_syntax");
}

#[test]
fn render_svg_marks_states_and_transitions() {
    let r = ok(json!({"op": "render", "document": abb_nfa()}));
    assert_eq!(r["format"], "svg");
    let svg = r["content"].as_str().unwrap();
    assert!(svg.starts_with("<svg"), "{}", &svg[..40.min(svg.len())]);
    assert!(svg.contains("data-state-id=\"0\""));
    assert!(svg.contains("data-state-id=\"3\""));
    assert!(svg.contains("data-transition-index=\"4\""));
    assert!(!svg.contains("data-state-id=\"4\""));
}

#[test]
fn render_highlights_and_dark_mode() {
    let plain = ok(json!({"op": "render", "document": abb_nfa(), "format": "svg"}));
    let lit = ok(
        json!({"op": "render", "document": abb_nfa(), "format": "svg",
                        "highlight_states": ["q1", 2], "highlight_transitions": [0]}),
    );
    let dark = ok(json!({"op": "render", "document": abb_nfa(), "dark": true}));
    assert_ne!(plain["content"], lit["content"]);
    assert_ne!(plain["content"], dark["content"]);
    let e = err(json!({"op": "render", "document": abb_nfa(), "highlight_states": ["nope"]}));
    assert_eq!(e["code"], "unknown_state");
}

#[test]
fn render_text_formats() {
    let md = ok(json!({"op": "render", "document": abb_nfa(), "format": "markdown"}));
    let table = md["content"].as_str().unwrap();
    assert!(table.contains('|') && table.contains("q0"), "{table}");
    let dot = ok(json!({"op": "render", "document": abb_nfa(), "format": "dot"}));
    assert!(dot["content"].as_str().unwrap().contains("digraph"));
    let text = ok(json!({"op": "render", "document": abb_nfa(), "format": "text"}));
    let text = text["content"].as_str().unwrap();
    assert!(text.contains("q0 --a--> q1"), "{text}");
    assert!(text.contains("initial: q0"), "{text}");
    assert!(text.contains("final: q3"), "{text}");
    for doc in [
        anbn_pda(),
        json!({"type": "grammar", "text": "S -> aSb | λ"}),
        json!({"type": "regex", "expression": "a*b"}),
        json!({"type": "moore", "initial": "q", "outputs": {"q": "x"}, "transitions": []}),
    ] {
        for format in ["markdown", "text"] {
            let r = ok(json!({"op": "render", "document": doc.clone(), "format": format}));
            assert!(!r["content"].as_str().unwrap().is_empty());
        }
    }
    let e = err(
        json!({"op": "render", "document": {"type": "regex", "expression": "a"},
                       "format": "svg"}),
    );
    assert_eq!(e["code"], "wrong_document_type");
}

#[test]
fn render_svg_of_other_machines_and_lsystems() {
    let r = ok(json!({"op": "render", "document": anbn_pda()}));
    assert!(r["content"].as_str().unwrap().contains("data-state-id"));
    let r = ok(json!({"op": "render", "document": common::anbncn_tm()}));
    assert!(r["content"].as_str().unwrap().contains("data-state-id"));
    let koch = json!({"type": "lsystem", "axiom": "F",
                      "rules": ["F -> F + F - F - F + F"],
                      "parameters": {"angle": "90", "distance": "4"}});
    let r = ok(json!({"op": "render", "document": koch, "depth": 2}));
    assert!(r["content"].as_str().unwrap().starts_with("<svg"));
}

#[test]
fn explain_describes_the_automaton() {
    let r = ok(json!({"op": "explain", "document": abb_nfa()}));
    let md = r["markdown"].as_str().unwrap();
    assert!(
        md.starts_with("Finite automaton with 4 states and 5 transitions."),
        "{md}"
    );
    assert!(md.contains("Start state: q0."));
    let r = ok(json!({"op": "explain", "document": anbn_pda()}));
    assert!(r["markdown"].as_str().unwrap().contains("q0"));
    let r = ok(json!({"op": "explain", "document": {"type": "grammar", "text": "S -> aS | a"}}));
    assert!(r["markdown"].as_str().unwrap().contains("right-linear"));
}

#[test]
fn lsystem_rendering() {
    let koch = json!({"type": "lsystem", "axiom": "F",
                      "rules": ["F -> F + F - F - F + F"],
                      "parameters": {"angle": "90", "distance": "4"}});
    let r = ok(json!({"op": "lsystem_render", "document": koch.clone(), "depth": 1}));
    assert!(r["svg"].as_str().unwrap().contains("<svg"));
    assert_eq!(r["expansion_length"], 9);
    assert_eq!(r["expansion"], "F + F - F - F + F");
    assert_eq!(r["depth"], 1);
    let deeper = ok(json!({"op": "lsystem_render", "document": koch.clone(), "depth": 3}));
    assert_eq!(deeper["expansion_length"], 249);
    // single-character symbols: "F+F" is F + F
    let compact = json!({"type": "lsystem", "axiom": "F", "single_char_symbols": true,
                         "rules": {"F": "F+F-F-F+F"}});
    let a = ok(json!({"op": "lsystem_render", "document": compact, "depth": 1}));
    assert_eq!(a["expansion"], "F + F - F - F + F");
    let e = err(json!({"op": "lsystem_render", "document": koch.clone(), "depth": 40}));
    assert_eq!(e["code"], "limit_exceeded");
    let explosive = json!({"type": "lsystem", "axiom": "F", "rules": ["F -> F F F F F F F F F F"]});
    let e = err(json!({"op": "lsystem_render", "document": explosive, "depth": 12}));
    assert_eq!(e["code"], "limit_exceeded");
}

#[test]
fn pumping_languages_are_listed() {
    let r = ok(json!({"op": "pumping_languages"}));
    let languages = r["languages"].as_array().unwrap();
    assert_eq!(languages.len(), 26);
    assert_eq!(languages[0]["kind"], "regular");
    assert_eq!(languages[0]["index"], 0);
    assert_eq!(languages[13]["kind"], "context_free");
    assert_eq!(languages[13]["index"], 0);
    assert!(
        languages[0]["title"]
            .as_str()
            .unwrap()
            .starts_with("a^n b^n")
    );
    assert!(languages[0]["explanation"].is_string());
}

#[test]
fn pumping_game_scripted_to_a_result() {
    let new = ok(
        json!({"op": "pumping_new", "kind": "regular", "language": 0,
                        "mode": "computer_first", "seed": 1}),
    );
    assert_eq!(new["document"]["type"], "regular_pumping_lemma");
    assert_eq!(new["view"]["awaiting"], "w");
    let m = new["lemma"]["m"].as_u64().unwrap() as usize;
    assert!(m >= 1);
    let w = format!("{}{}", "a".repeat(m), "b".repeat(m));

    let moved = ok(json!({"op": "pumping_move", "game": new["document"],
                          "move": {"move": "choose_w", "w": w}}));
    assert_eq!(moved["reply"]["accepted"], true);
    assert_eq!(moved["view"]["awaiting"], "i");
    assert_eq!(moved["result"], Value::Null);

    let done = ok(json!({"op": "pumping_move", "game": moved["document"],
                         "move": {"move": "choose_i", "i": 2}}));
    assert_eq!(done["reply"]["accepted"], true);
    assert_eq!(done["result"]["user_wins"], true);
    assert_eq!(done["result"]["in_language"], false);
    assert_eq!(done["lemma"]["w"], w);
    assert_eq!(done["lemma"]["i"], 2);
}

#[test]
fn pumping_game_refuses_a_bad_move_without_failing() {
    let new = ok(
        json!({"op": "pumping_new", "kind": "context_free", "language": "a^n b^n c^n",
                        "mode": "human_first"}),
    );
    assert_eq!(new["document"]["type"], "context_free_pumping_lemma");
    assert_eq!(new["view"]["awaiting"], "m");
    let moved = ok(json!({"op": "pumping_move", "game": new["document"],
                          "move": {"move": "choose_w", "w": "abc"}}));
    assert_eq!(moved["reply"]["accepted"], false);
    assert!(!moved["reply"]["message"].as_str().unwrap().is_empty());
    let e = err(json!({"op": "pumping_new", "kind": "regular", "language": "nope"}));
    assert_eq!(e["code"], "invalid_argument");
    let e = err(json!({"op": "pumping_move", "game": abb_nfa(),
                       "move": {"move": "clear_all"}}));
    assert_eq!(e["code"], "wrong_document_type");
}

#[test]
fn capabilities_lists_every_operation() {
    let r = ok(json!({"op": "capabilities"}));
    let ops: Vec<String> = r["ops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["op"].as_str().unwrap().to_string())
        .collect();
    let expected: Vec<String> = rflap_api::OPS.iter().map(|s| s.to_string()).collect();
    assert_eq!(ops, expected);
    for op in [
        "open_jff",
        "save_jff",
        "normalize",
        "simulate",
        "step",
        "multiple_run",
        "analyze",
        "convert",
        "equivalent",
        "combine",
        "grammar_type",
        "parse",
        "parse_tables",
        "user_parse",
        "layout",
        "render",
        "design",
        "explain",
        "lsystem_render",
        "pumping_languages",
        "pumping_new",
        "pumping_move",
        "capabilities",
    ] {
        assert!(ops.iter().any(|o| o == op), "capabilities lacks {op}");
    }
    let types: Vec<_> = r["document_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["type"].as_str().unwrap())
        .collect();
    assert_eq!(types, rflap_api::TYPES);
    for t in r["document_types"].as_array().unwrap() {
        assert!(t["compact"].is_string() && t["example"].is_object());
    }
    assert!(strings(&r["error_codes"]).contains(&"unknown_state".to_string()));
    assert_eq!(
        strings(&r["parsers"]),
        ["brute", "brute_unrestricted", "ll1", "slr1", "cyk"]
    );
    assert_eq!(strings(&r["formats"]), ["svg", "markdown", "dot", "text"]);
    assert!(strings(&r["layouts"]).contains(&"two-circle".to_string()));
}

#[test]
fn every_example_in_capabilities_runs() {
    let r = ok(json!({"op": "capabilities"}));
    for op in r["ops"].as_array().unwrap() {
        let example = &op["example"];
        assert_eq!(
            example["op"], op["op"],
            "the example of {} has another op",
            op["op"]
        );
        let response = common::send(example.clone());
        assert_eq!(response["ok"], true, "example of {}: {response}", op["op"]);
        assert_eq!(response["op"], op["op"]);
    }
    for t in r["document_types"].as_array().unwrap() {
        let response = common::send(json!({"op": "normalize", "document": t["example"]}));
        assert_eq!(response["ok"], true, "example of {}: {response}", t["type"]);
    }
    for c in r["convert"].as_array().unwrap() {
        let example = &c["example"];
        assert_eq!(example["to"], c["to"]);
        let response = common::send(example.clone());
        assert_eq!(
            response["ok"], true,
            "convert example {}: {response}",
            c["to"]
        );
    }
}
