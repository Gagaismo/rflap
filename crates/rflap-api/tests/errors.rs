mod common;

use common::{err, ok, send};
use serde_json::json;

#[test]
fn malformed_json_is_reported_not_raised() {
    let text = rflap_api::call("{\"op\": \"simulate\", ");
    let response: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(response["ok"], false);
    assert_eq!(response["error"]["code"], "invalid_json");
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not valid JSON")
    );
}

#[test]
fn a_request_that_is_not_an_object_is_invalid() {
    let text = rflap_api::call("[1, 2]");
    let response: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(response["error"]["code"], "invalid_request");
}

#[test]
fn unknown_op_lists_the_operations() {
    let e = err(json!({"op": "frobnicate"}));
    assert_eq!(e["code"], "unknown_op");
    let message = e["message"].as_str().unwrap();
    assert!(message.contains("`frobnicate`"), "{message}");
    assert!(message.contains("simulate"), "{message}");
}

#[test]
fn missing_op_is_an_invalid_request() {
    let e = err(json!({"document": {"type": "fa"}}));
    assert_eq!(e["code"], "invalid_request");
    assert!(e["message"].as_str().unwrap().contains("\"op\""));
}

#[test]
fn missing_argument_names_the_field() {
    let e = err(json!({"op": "simulate", "input": "a"}));
    assert_eq!(e["code"], "invalid_request");
    assert!(e["message"].as_str().unwrap().contains("document"), "{e}");
}

#[test]
fn unknown_argument_is_rejected() {
    let e = err(json!({"op": "capabilities_x"}));
    assert_eq!(e["code"], "unknown_op");
    let e = err(json!({
        "op": "analyze",
        "document": {"type": "fa", "initial": "q0"},
        "colour": "red"
    }));
    assert_eq!(e["code"], "invalid_request");
    assert!(e["message"].as_str().unwrap().contains("colour"), "{e}");
}

#[test]
fn unknown_state_in_a_compact_transition() {
    let e = err(json!({
        "op": "normalize",
        "document": {
            "type": "fa",
            "states": ["q0", "q1"],
            "initial": "q0",
            "finals": ["q1"],
            "transitions": [["q0", "a", "q1"], ["q1", "b", "q9"]]
        }
    }));
    assert_eq!(e["code"], "unknown_state");
    let message = e["message"].as_str().unwrap();
    assert!(message.contains("\"q9\""), "{message}");
    assert!(message.contains("transition #1"), "{message}");
    assert!(message.contains("q0, q1"), "{message}");
    assert_eq!(e["details"]["state"], "q9");
}

#[test]
fn unknown_initial_state_when_states_are_declared() {
    let e = err(json!({
        "op": "normalize",
        "document": {"type": "fa", "states": ["q0"], "initial": "start"}
    }));
    assert_eq!(e["code"], "unknown_state");
    assert!(e["message"].as_str().unwrap().contains("\"initial\""));
}

#[test]
fn bad_labels_are_reported_with_their_transition() {
    let e = err(json!({
        "op": "normalize",
        "document": {"type": "pda", "initial": "q0", "transitions": [["q0", "a,Z", "q0"]]}
    }));
    assert_eq!(e["code"], "invalid_label");
    let message = e["message"].as_str().unwrap();
    assert!(message.contains("transition #0"), "{message}");
    assert!(message.contains("input,pop;push"), "{message}");

    let e = err(json!({
        "op": "normalize",
        "document": {"type": "turing", "initial": "q0", "transitions": [["q0", "a;b,Q", "q0"]]}
    }));
    assert_eq!(e["code"], "invalid_label");
    assert!(e["message"].as_str().unwrap().contains("direction"));

    let e = err(json!({
        "op": "normalize",
        "document": {"type": "mealy", "initial": "q0", "transitions": [["q0", "a", "q0"]]}
    }));
    assert_eq!(e["code"], "invalid_label");
}

#[test]
fn unknown_compact_field_is_rejected() {
    let e = err(json!({
        "op": "normalize",
        "document": {"type": "fa", "initial": "q0", "transition": []}
    }));
    assert_eq!(e["code"], "invalid_document");
    assert!(e["message"].as_str().unwrap().contains("\"transition\""));
}

#[test]
fn unknown_document_type() {
    let e = err(json!({"op": "normalize", "document": {"type": "cube"}}));
    assert_eq!(e["code"], "unknown_document_type");
    let e = err(json!({"op": "normalize", "document": {"initial": "q0"}}));
    assert_eq!(e["code"], "invalid_document");
    let e = err(json!({"op": "normalize", "document": "fa"}));
    assert_eq!(e["code"], "invalid_document");
}

#[test]
fn wrong_document_type_for_an_op() {
    let e = err(json!({
        "op": "simulate",
        "document": {"type": "regex", "expression": "a*"},
        "input": "aa"
    }));
    assert_eq!(e["code"], "wrong_document_type");
    assert!(e["message"].as_str().unwrap().contains("regex"));

    let e = err(json!({
        "op": "convert",
        "document": {"type": "pda", "initial": "q0"},
        "to": "dfa"
    }));
    assert_eq!(e["code"], "wrong_document_type");

    let e = err(json!({
        "op": "grammar_type",
        "grammar": {"type": "fa", "initial": "q0"}
    }));
    assert_eq!(e["code"], "wrong_document_type");

    let e = err(json!({
        "op": "lsystem_render",
        "document": {"type": "fa", "initial": "q0"}
    }));
    assert_eq!(e["code"], "wrong_document_type");
}

#[test]
fn simulate_needs_an_initial_state_and_an_input() {
    let e = err(json!({
        "op": "simulate",
        "document": {"type": "fa", "states": ["q0"]},
        "input": "a"
    }));
    assert_eq!(e["code"], "no_initial_state");

    let e = err(json!({
        "op": "simulate",
        "document": {"type": "fa", "initial": "q0"}
    }));
    assert_eq!(e["code"], "invalid_argument");
}

#[test]
fn options_that_do_not_apply_are_rejected() {
    let e = err(json!({
        "op": "simulate",
        "document": {"type": "fa", "initial": "q0"},
        "input": "",
        "acceptance": "empty_stack"
    }));
    assert_eq!(e["code"], "unsupported_option");
    let e = err(json!({
        "op": "simulate",
        "document": anbn_tm_like(),
        "input": "",
        "mode": "closure"
    }));
    assert_eq!(e["code"], "unsupported_option");
}

fn anbn_tm_like() -> serde_json::Value {
    json!({"type": "turing", "initial": "q0", "transitions": [["q0", "a;a,R", "q0"]]})
}

#[test]
fn invalid_full_form_is_an_invalid_document() {
    let e = err(json!({
        "op": "normalize",
        "document": {"type": "fa", "states": {"0": 5}}
    }));
    assert_eq!(e["code"], "invalid_document");
    // a transition that points at a state that does not exist
    let full = ok(json!({"op": "normalize", "document": {
        "type": "fa", "initial": "a", "transitions": [["a", "x", "b"]]}}))["document"]
        .clone();
    let mut broken = full.clone();
    broken["transitions"][0]["to"] = json!(99);
    let e = err(json!({"op": "normalize", "document": broken}));
    assert_eq!(e["code"], "unknown_state");
}

#[test]
fn bad_grammar_and_regex() {
    let e = err(json!({"op": "grammar_type", "grammar": "S aSb"}));
    assert_eq!(e["code"], "invalid_grammar");
    let e = err(json!({"op": "normalize", "document": {"type": "regex", "expression": "(a+"}}));
    assert_eq!(e["code"], "invalid_regex");
}

#[test]
fn invalid_jff_is_reported() {
    let e = err(json!({"op": "open_jff", "xml": "<structure>"}));
    assert_eq!(e["code"], "jff_error");
    let e = err(json!({"op": "open_jff", "xml": "<structure><type>quantum</type></structure>"}));
    assert_eq!(e["code"], "jff_error");
}

#[test]
fn nothing_panics_on_hostile_input() {
    let requests = [
        json!({"op": "simulate", "document": {"type": "turing"}, "input": "x"}),
        json!({"op": "render", "document": {"type": "fa"}}),
        json!({"op": "layout", "document": {"type": "fa"}, "algorithm": "tree"}),
        json!({"op": "convert", "document": {"type": "fa"}, "to": "minimal_dfa"}),
        json!({"op": "convert", "document": {"type": "fa"}, "to": "regex"}),
        json!({"op": "pumping_move", "game": {"type": "regular_pumping_lemma", "language": 0},
               "move": {"move": "choose_decomposition", "lengths": [-5, 99]}}),
        json!({"op": "design", "spec": {"alphabet": "", "language": {"regex": "("}}}),
        json!({"op": "parse", "grammar": "S -> S", "input": "a"}),
    ];
    for request in requests {
        let response = send(request.clone());
        assert!(response.get("ok").is_some(), "{request} -> {response}");
    }
}

#[test]
fn deserializing_a_document_directly_accepts_both_forms() {
    let compact: rflap_api::Document =
        serde_json::from_value(json!({"type": "fa", "initial": "q0", "finals": ["q0"]})).unwrap();
    let full: rflap_api::Document = serde_json::from_value(compact.to_value()).unwrap();
    assert_eq!(compact, full);
    assert!(serde_json::from_value::<rflap_api::Document>(json!({"type": "nope"})).is_err());
}

#[test]
fn deeply_nested_regex_is_refused_not_a_stack_overflow() {
    let expression = format!("{}a{}", "(".repeat(5000), ")".repeat(5000));
    let e = err(json!({"op": "normalize",
                       "document": {"type": "regex", "expression": expression}}));
    assert_eq!(e["code"], "invalid_regex");
    assert!(e["message"].as_str().unwrap().contains("nested"));
}

#[test]
fn deeply_nested_json_is_invalid_json() {
    let text = format!("{}1{}", "[".repeat(10_000), "]".repeat(10_000));
    let response: serde_json::Value = serde_json::from_str(&rflap_api::call(&text)).unwrap();
    assert_eq!(response["ok"], false);
    assert_eq!(response["error"]["code"], "invalid_json");
}
