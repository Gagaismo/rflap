//! `jflap_check` (S8): what classic JFLAP 7.1 cannot represent or runs differently.

mod common;

use common::*;
use serde_json::{Value, json};

const B: &str = include_str!("fixtures/catedra/ALL_2.1b_complemento_palindromo.jff");
const C: &str = include_str!("fixtures/catedra/ALL_2.1c_central_igual_primero.jff");
const D: &str = include_str!("fixtures/catedra/ALL_2.1d_n_igual_m_mas_p.jff");
const E: &str = include_str!("fixtures/catedra/ALL_2.1e_m_igual_2p_mas_1.jff");

fn check(document: Value) -> Value {
    ok(json!({"op": "jflap_check", "document": document}))
}

fn codes(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|i| i["code"].as_str().unwrap().to_string())
        .collect()
}

fn open(xml: &str) -> Value {
    ok(json!({"op": "open_jff", "xml": xml}))["document"].clone()
}

#[test]
fn the_operation_is_listed_and_documented() {
    assert!(rflap_api::OPS.contains(&"jflap_check"));
    let caps = ok(json!({"op": "capabilities"}));
    let listed = caps["ops"]
        .as_array()
        .unwrap()
        .iter()
        .any(|o| o["op"] == "jflap_check");
    assert!(listed, "{caps}");
}

#[test]
fn a_plain_machine_of_every_kind_is_compatible_with_no_issue_and_no_note() {
    let machines = [
        abb_nfa(),
        anbn_pda(),
        anbncn_tm(),
        json!({"type": "mealy", "initial": "q0", "transitions": [["q0", "a;1", "q0"]]}),
        json!({"type": "moore", "initial": "q0", "outputs": {"q0": "x"},
               "transitions": [["q0", "a", "q0"]]}),
    ];
    for machine in machines {
        let r = check(machine.clone());
        assert_eq!(r["compatible"], true, "{machine}: {r}");
        assert_eq!(r["issues"], json!([]), "{machine}");
        assert_eq!(r["notes"], json!([]), "{machine}");
    }
}

#[test]
fn documents_that_are_not_machines_are_compatible() {
    for document in [
        json!({"type": "grammar", "text": "S -> aSb | λ"}),
        json!({"type": "regex", "expression": "(a+b)*abb"}),
    ] {
        let r = check(document);
        assert_eq!(r["compatible"], true, "{r}");
        assert_eq!(r["issues"], json!([]));
    }
}

#[test]
fn a_machine_with_more_than_five_tapes_is_incompatible() {
    let label = ["a;a,R"; 6].join("|");
    let r = check(json!({"type": "turing", "initial": "q0", "finals": ["q1"],
                         "transitions": [["q0", label, "q1"]]}));
    assert_eq!(r["compatible"], false, "{r}");
    assert_eq!(codes(&r["issues"]), ["too_many_tapes"]);
    let message = r["issues"][0]["message"].as_str().unwrap();
    assert!(
        message.contains("6 tapes") && message.contains("1 to 5"),
        "{message}"
    );
}

#[test]
fn five_tapes_are_fine() {
    let label = ["a;a,R"; 5].join("|");
    let r = check(json!({"type": "turing", "initial": "q0", "finals": ["q1"],
                         "transitions": [["q0", label, "q1"]]}));
    assert_eq!(r["compatible"], true, "{r}");
}

#[test]
fn a_range_into_a_state_with_a_lambda_exit_is_incompatible() {
    // JFLAP's run (closure simulator) stops one state early after a range: it rejects "5".
    let r = check(json!({"type": "fa", "initial": "q0", "finals": ["q2"],
                         "transitions": [["q0", "[0-9]", "q1"], ["q1", "", "q2"]]}));
    assert_eq!(r["compatible"], false, "{r}");
    assert_eq!(codes(&r["issues"]), ["range_before_lambda"]);
    assert_eq!(r["issues"][0]["transition"], json!(["q0", "[0-9]", "q1"]));
    let message = r["issues"][0]["message"].as_str().unwrap();
    assert!(
        message.contains("[0-9]") && message.contains("q1"),
        "{message}"
    );
}

#[test]
fn a_range_into_a_state_without_lambda_exit_is_fine() {
    let r = check(json!({"type": "fa", "initial": "q0", "finals": ["q1"],
                         "transitions": [["q0", "[0-9]", "q1"], ["q1", "[0-9]", "q1"]]}));
    assert_eq!(r["compatible"], true, "{r}");
}

#[test]
fn an_all_opens_the_same_and_carries_the_note_on_how_to_run_it_in_jflap() {
    for xml in [B, C, D, E] {
        let r = check(open(xml));
        assert_eq!(r["compatible"], true, "{r}");
        assert_eq!(r["issues"], json!([]));
        assert_eq!(codes(&r["notes"]), ["all_model"], "{r}");
        let note = r["notes"][0]["message"].as_str().unwrap();
        for needle in [
            "ALL",
            "⟨w⟩",
            "primer símbolo",
            "(⟨,⟨,D)",
            "solo valen en Rflap",
        ] {
            assert!(note.contains(needle), "{needle} not in {note}");
        }
    }
}

#[test]
fn an_all_written_with_ascii_delimiters_gets_the_same_note() {
    let r = check(json!({"type": "turing", "initial": "q0", "finals": ["q1"],
                         "transitions": [["q0", "(a,<,D)", "q1"], ["q1", "(c,>,I)", "q0"]]}));
    assert_eq!(codes(&r["notes"]), ["all_model"], "{r}");
}

#[test]
fn a_declared_input_alphabet_is_not_stored_in_the_jff() {
    let r = check(json!({"type": "turing", "initial": "q0", "finals": ["q1"],
                         "input_alphabet": "ab", "transitions": [["q0", "(a,a,D)", "q1"]]}));
    assert_eq!(r["compatible"], true, "{r}");
    assert_eq!(codes(&r["notes"]), ["input_alphabet_not_saved"], "{r}");
}

#[test]
fn a_machine_without_final_states_is_accepted_only_by_halting() {
    let tm = check(json!({"type": "turing", "initial": "q0", "finals": [],
                          "transitions": [["q0", "a;a,R", "q0"]]}));
    assert_eq!(codes(&tm["notes"]), ["accepts_by_halting"], "{tm}");
    let pda = check(json!({"type": "pda", "initial": "q0", "finals": [],
                           "transitions": [["q0", "a,Z;Z", "q0"]]}));
    assert_eq!(codes(&pda["notes"]), ["accepts_by_empty_stack"], "{pda}");
}

#[test]
fn the_document_errors_keep_their_codes() {
    let e = err(json!({"op": "jflap_check", "document": {"type": "nope"}}));
    assert_eq!(e["code"], "unknown_document_type");
}
