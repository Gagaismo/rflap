//! S8: every automaton the AI designs, shows or saves is checked against classic JFLAP 7.1;
//! the issues and notes come in the text and in `structuredContent.jflap`.

use rflap_mcp::handle_message;
use serde_json::{Value, json};

const B: &str =
    include_str!("../../rflap-api/tests/fixtures/catedra/ALL_2.1b_complemento_palindromo.jff");

fn rpc(method: &str, params: Value) -> Value {
    handle_message(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
        .expect("a request is answered")
}

fn call(name: &str, arguments: Value) -> Value {
    let answer = rpc("tools/call", json!({"name": name, "arguments": arguments}));
    assert!(answer.get("error").is_none(), "protocol error: {answer}");
    answer["result"].clone()
}

fn text(result: &Value) -> String {
    result["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn six_tapes() -> Value {
    let label = ["a;a,R"; 6].join("|");
    json!({"type": "turing", "initial": "q0", "finals": ["q1"],
           "transitions": [["q0", label, "q1"]]})
}

fn all_machine() -> Value {
    json!({"type": "turing", "initial": "q0", "finals": ["q1"],
           "transitions": [["q0", "(a,⟨,D)", "q1"], ["q1", "(c,⟩,I)", "q0"]]})
}

fn ends_with_ab() -> Value {
    json!({"alphabet": "ab", "constraint_text": "ends_with(\"ab\")",
           "examples": {"accept": ["ab", "aab"], "reject": ["", "ba"]}})
}

fn warning_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.starts_with("⚠️ JFLAP 7.1:"))
        .collect()
}

fn note_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.starts_with("ℹ️ JFLAP 7.1:"))
        .collect()
}

#[test]
fn a_clean_design_says_nothing_and_structured_content_says_compatible() {
    let result = call("rflap_design_automaton", ends_with_ab());
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(!md.contains("JFLAP 7.1:"), "{md}");
    let jflap = &result["structuredContent"]["jflap"];
    assert_eq!(jflap["compatible"], true, "{jflap}");
    assert_eq!(jflap["issues"], json!([]));
    assert_eq!(jflap["notes"], json!([]));
}

#[test]
fn show_warns_about_what_jflap_cannot_open() {
    let result = call("rflap_show", json!({"document": six_tapes()}));
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    let warnings = warning_lines(&md);
    assert_eq!(warnings.len(), 1, "{md}");
    assert!(warnings[0].contains("6 tapes"), "{}", warnings[0]);
    let jflap = &result["structuredContent"]["jflap"];
    assert_eq!(jflap["compatible"], false);
    assert_eq!(jflap["issues"][0]["code"], "too_many_tapes");
}

#[test]
fn show_of_an_all_adds_the_note_on_how_to_run_it_in_jflap() {
    let result = call("rflap_show", json!({"document": all_machine()}));
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(warning_lines(&md).is_empty(), "{md}");
    let notes = note_lines(&md);
    assert_eq!(notes.len(), 1, "{md}");
    assert!(
        notes[0].contains("JFLAP no tiene el modelo ALL"),
        "{}",
        notes[0]
    );
    assert!(notes[0].contains("(⟨,⟨,D)"), "{}", notes[0]);
    let jflap = &result["structuredContent"]["jflap"];
    assert_eq!(jflap["compatible"], true);
    assert_eq!(jflap["notes"][0]["code"], "all_model");
}

#[test]
fn the_catedra_definition_and_all_check_of_an_all_carry_the_note() {
    for action in ["definition", "all_check"] {
        let result = call(
            "rflap_mt_catedra",
            json!({"document": all_machine(), "action": action}),
        );
        assert_eq!(result["isError"], false, "{action}: {result}");
        assert_eq!(
            note_lines(&text(&result)).len(),
            1,
            "{action}: {}",
            text(&result)
        );
        assert_eq!(
            result["structuredContent"]["jflap"]["notes"][0]["code"],
            "all_model"
        );
    }
}

#[test]
fn save_jff_without_a_path_warns_and_still_returns_the_file() {
    let result = call("rflap_save_jff", json!({"document": six_tapes()}));
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert_eq!(warning_lines(&md).len(), 1, "{md}");
    assert!(md.contains("<type>turing</type>"), "{md}");
    assert_eq!(result["structuredContent"]["jflap"]["compatible"], false);
    assert!(
        result["structuredContent"]["xml"]
            .as_str()
            .unwrap()
            .contains("<tapes>6</tapes>")
    );
}

#[test]
fn save_jff_of_an_incompatible_document_saves_it_unchanged_and_warns() {
    let dir = std::env::temp_dir().join(format!("rflap-jflap-mcp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("six.jff");
    let result = call(
        "rflap_save_jff",
        json!({"document": six_tapes(), "path": path.to_str().unwrap()}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(md.contains("Saved"), "{md}");
    assert!(!md.contains("JFLAP 7.1 opens it"), "{md}");
    assert_eq!(warning_lines(&md).len(), 1, "{md}");
    let written = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        written,
        result["structuredContent"]["xml"].as_str().unwrap()
    );
    assert!(written.contains("<tapes>6</tapes>"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_jff_of_a_compatible_document_says_jflap_opens_it() {
    let dir = std::env::temp_dir().join(format!("rflap-jflap-mcp-ok-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ok.jff");
    let nfa = json!({"type": "fa", "initial": "q0", "finals": ["q1"],
                     "transitions": [["q0", "a", "q1"]]});
    let result = call(
        "rflap_save_jff",
        json!({"document": nfa, "path": path.to_str().unwrap()}),
    );
    let md = text(&result);
    assert!(md.contains("JFLAP 7.1 opens it"), "{md}");
    assert!(
        warning_lines(&md).is_empty() && note_lines(&md).is_empty(),
        "{md}"
    );
    assert_eq!(result["structuredContent"]["jflap"]["compatible"], true);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_other_tools_that_return_a_document_carry_the_check_too() {
    let nfa = json!({"type": "fa", "initial": "q0", "finals": ["q1"],
                     "transitions": [["q0", "[0-9]", "q1"], ["q1", "", "q0"]]});
    for (tool, arguments) in [
        ("rflap_convert", json!({"document": nfa, "to": "dfa"})),
        ("rflap_combine", json!({"a": nfa, "b": nfa})),
        ("rflap_render", json!({"document": nfa})),
    ] {
        let result = call(tool, arguments);
        assert_eq!(result["isError"], false, "{tool}: {result}");
        assert!(
            result["structuredContent"]["jflap"]["compatible"].is_boolean(),
            "{tool}: {}",
            result["structuredContent"]
        );
    }
    // the render of the document with the range into a λ exit warns
    let result = call("rflap_render", json!({"document": nfa}));
    assert_eq!(warning_lines(&text(&result)).len(), 1, "{}", text(&result));
}

#[test]
fn opening_a_file_does_not_judge_it() {
    let result = call("rflap_open_jff", json!({"xml": B}));
    assert!(result["structuredContent"].get("jflap").is_none());
}

#[test]
fn the_rule_is_in_the_instructions_and_in_the_descriptions() {
    let init = rpc(
        "initialize",
        json!({"protocolVersion": "2025-06-18", "capabilities": {},
                                        "clientInfo": {"name": "t", "version": "1"}}),
    );
    let instructions = init["result"]["instructions"].as_str().unwrap();
    for needle in [
        "JFLAP 7.1",
        "must open in classic JFLAP 7.1",
        "Rflap-only",
        "ALL",
        "⚠️ JFLAP 7.1",
    ] {
        assert!(
            instructions.contains(needle),
            "{needle} not in the instructions"
        );
    }
    let tools = rpc("tools/list", json!({}));
    for name in ["rflap_design_automaton", "rflap_show", "rflap_save_jff"] {
        let tool = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap();
        let description = tool["description"].as_str().unwrap();
        assert!(
            description.contains("JFLAP 7.1 compatib"),
            "{name}: {description}"
        );
    }
}

#[test]
fn the_skill_states_the_rule() {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skills/rflap/SKILL.md"),
    )
    .unwrap();
    for needle in [
        "JFLAP 7.1 compatible",
        "Rflap-only",
        "⚠️ JFLAP 7.1",
        "ℹ️ JFLAP 7.1",
    ] {
        assert!(skill.contains(needle), "{needle} not in SKILL.md");
    }
}
