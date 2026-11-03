#![allow(dead_code)]

use serde_json::Value;

/// Sends a request through the JSON entry point.
pub fn send(request: Value) -> Value {
    let text = rflap_api::call(&request.to_string());
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("the response is not JSON ({e}): {text}"))
}

/// The `result` of a request that must succeed.
pub fn ok(request: Value) -> Value {
    let response = send(request.clone());
    assert_eq!(
        response["ok"],
        Value::Bool(true),
        "request {request} failed: {response}"
    );
    response["result"].clone()
}

/// The `error` of a request that must fail.
pub fn err(request: Value) -> Value {
    let response = send(request.clone());
    assert_eq!(
        response["ok"],
        Value::Bool(false),
        "request {request} should have failed: {response}"
    );
    response["error"].clone()
}

pub fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("not an array: {value}"))
        .iter()
        .map(|v| {
            v.as_str()
                .unwrap_or_else(|| panic!("not a string: {v}"))
                .to_string()
        })
        .collect()
}

/// The classic NFA for `(a+b)*abb`, compact.
pub fn abb_nfa() -> Value {
    serde_json::json!({
        "type": "fa",
        "initial": "q0",
        "finals": ["q3"],
        "transitions": [
            ["q0", "a", "q0"], ["q0", "b", "q0"], ["q0", "a", "q1"],
            ["q1", "b", "q2"], ["q2", "b", "q3"]
        ]
    })
}

/// A PDA for `a^n b^n`, accepting by final state.
pub fn anbn_pda() -> Value {
    serde_json::json!({
        "type": "pda",
        "initial": "q0",
        "finals": ["q2"],
        "transitions": [
            ["q0", "a,Z;AZ", "q0"], ["q0", "a,A;AA", "q0"],
            ["q0", "b,A;λ", "q1"], ["q1", "b,A;λ", "q1"],
            ["q1", "λ,Z;Z", "q2"], ["q0", "λ,Z;Z", "q2"]
        ]
    })
}

/// A one-tape Turing machine for `a^n b^n c^n`.
pub fn anbncn_tm() -> Value {
    serde_json::json!({
        "type": "turing",
        "initial": "q0",
        "finals": ["qf"],
        "transitions": [
            ["q0", "a;X,R", "q1"], ["q0", "Y;Y,R", "q4"], ["q0", "_;_,S", "qf"],
            ["q1", "a;a,R", "q1"], ["q1", "Y;Y,R", "q1"], ["q1", "b;Y,R", "q2"],
            ["q2", "b;b,R", "q2"], ["q2", "Z;Z,R", "q2"], ["q2", "c;Z,L", "q3"],
            ["q3", "a;a,L", "q3"], ["q3", "b;b,L", "q3"], ["q3", "Y;Y,L", "q3"],
            ["q3", "Z;Z,L", "q3"], ["q3", "X;X,R", "q0"],
            ["q4", "Y;Y,R", "q4"], ["q4", "Z;Z,R", "q4"], ["q4", "_;_,R", "qf"]
        ]
    })
}

/// The classic expression grammar.
pub const EXPRESSION_GRAMMAR: &str = "E -> E+T | T\nT -> T*F | F\nF -> (E) | i";
