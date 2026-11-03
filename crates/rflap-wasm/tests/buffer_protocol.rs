//! The raw C ABI, called natively exactly as the JavaScript loader calls it in wasm:
//! alloc a buffer, copy the request in, call, read `[u32 le length][UTF-8 JSON]`, free both.

use rflap_wasm::{rflap_alloc, rflap_call, rflap_free, rflap_free_result};
use serde_json::Value;

/// Runs one request through the ABI and returns the decoded answer text.
fn call_raw(request: &[u8]) -> String {
    unsafe {
        let ptr = rflap_alloc(request.len());
        std::ptr::copy_nonoverlapping(request.as_ptr(), ptr, request.len());
        let out = rflap_call(ptr, request.len());
        rflap_free(ptr, request.len());
        assert!(!out.is_null());
        let len = u32::from_le_bytes(std::slice::from_raw_parts(out, 4).try_into().unwrap());
        let body = std::slice::from_raw_parts(out.add(4), len as usize).to_vec();
        rflap_free_result(out);
        String::from_utf8(body).expect("the answer is UTF-8")
    }
}

#[test]
fn capabilities_roundtrip() {
    let answer: Value = serde_json::from_str(&call_raw(br#"{"op": "capabilities"}"#)).unwrap();
    assert_eq!(answer["ok"], true);
    assert_eq!(answer["op"], "capabilities");
}

#[test]
fn simulate_matches_the_api() {
    let request = r#"{"op": "simulate", "input": "ab", "document": {
        "type": "fa", "initial": "q0", "finals": ["q2"],
        "transitions": [["q0", "a", "q1"], ["q1", "b", "q2"]]}}"#;
    assert_eq!(call_raw(request.as_bytes()), rflap_api::call(request));
}

#[test]
fn length_prefix_counts_bytes_not_chars() {
    // λ is two bytes in UTF-8: the prefix must be the byte length of the JSON.
    let answer = call_raw(r#"{"op": "nope", "x": "λλλ"}"#.as_bytes());
    let value: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(value["ok"], false);
    unsafe {
        let request = r#"{"op": "grammar_type", "grammar": "S -> λ"}"#.as_bytes();
        let ptr = rflap_alloc(request.len());
        std::ptr::copy_nonoverlapping(request.as_ptr(), ptr, request.len());
        let out = rflap_call(ptr, request.len());
        rflap_free(ptr, request.len());
        let len = u32::from_le_bytes(std::slice::from_raw_parts(out, 4).try_into().unwrap());
        let expected = rflap_api::call(std::str::from_utf8(request).unwrap()).len();
        assert_eq!(len as usize, expected);
        rflap_free_result(out);
    }
}

#[test]
fn invalid_utf8_is_an_error_answer_not_a_trap() {
    let answer: Value = serde_json::from_str(&call_raw(&[0xff, 0xfe, 0x7b])).unwrap();
    assert_eq!(answer["ok"], false);
    assert_eq!(answer["error"]["code"], "invalid_json");
}

#[test]
fn empty_request_is_an_error_answer() {
    let answer: Value = serde_json::from_str(&call_raw(b"")).unwrap();
    assert_eq!(answer["ok"], false);
}

#[test]
fn many_calls_do_not_interfere() {
    for i in 0..50 {
        let request = format!(
            r#"{{"op": "simulate", "input": "{}", "document": {{"type": "fa", "initial": "q0", "finals": ["q0"], "transitions": [["q0", "a", "q0"]]}}}}"#,
            "a".repeat(i)
        );
        let answer: Value = serde_json::from_str(&call_raw(request.as_bytes())).unwrap();
        assert_eq!(answer["result"]["outcome"], "accepted");
    }
}
