//! ALL with a shrinking tape (S1), delimiters in the input (S4), final-state stays (S6) and
//! the normalization of `<` `>` to ⟨ ⟩ (S7), through the API.

mod common;

use common::*;
use serde_json::{Value, json};

const E: &str = include_str!("fixtures/catedra/ALL_2.1e_m_igual_2p_mas_1.jff");
const B: &str = include_str!("fixtures/catedra/ALL_2.1b_complemento_palindromo.jff");
const C: &str = include_str!("fixtures/catedra/ALL_2.1c_central_igual_primero.jff");
const D: &str = include_str!("fixtures/catedra/ALL_2.1d_n_igual_m_mas_p.jff");

fn open(xml: &str) -> Value {
    ok(json!({"op": "open_jff", "xml": xml}))["document"].clone()
}

fn verdicts(runs: &Value) -> Vec<(String, String)> {
    runs.as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["input"].as_str().unwrap().to_string(),
                r["verdict"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn notes(v: &Value) -> Vec<String> {
    strings(v)
}

// S1

#[test]
fn s1_e_of_the_professor_passes_all_check_with_shrink() {
    let r = ok(json!({"op": "all_check", "document": open(E),
                      "inputs": ["aab", "aabbbc", "aabb"]}));
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(r["violations"], json!([]));
    assert_eq!(
        verdicts(&r["runs"]),
        [
            ("aab".to_string(), "accepted".to_string()),
            ("aabbbc".to_string(), "accepted".to_string()),
            ("aabb".to_string(), "rejected".to_string()),
        ]
    );
    let all = notes(&r["notes"]);
    assert!(all.contains(&"q0 escribe ⟨ sobre a: la cinta se achica".to_string()));
    assert!(all.contains(&"q5 escribe ⟩ sobre c: la cinta se achica".to_string()));
    // each shrinking transition once
    let mut unique = all.clone();
    unique.dedup();
    assert_eq!(unique, all);
}

#[test]
fn s1_e_shows_the_real_cells_while_the_tape_shrinks() {
    let r = ok(json!({"op": "catedra_run", "document": open(E), "input": "aab"}));
    let run = &r["results"][0];
    assert_eq!(run["model"], "all");
    assert_eq!(
        strings(&run["configurations"]),
        ["⟨q0aab⟩", "⟨⟨q1ab⟩", "⟨⟨⟨q2b⟩", "⟨⟨⟨⟨q3⟩", "⟨⟨⟨qf⟨⟩"]
    );
    assert_eq!(
        notes(&run["notes"]),
        [
            "q0 escribe ⟨ sobre a: la cinta se achica",
            "q1 escribe ⟨ sobre a: la cinta se achica",
            "q2 escribe ⟨ sobre b: la cinta se achica"
        ]
    );
    assert_eq!(run["violation"], Value::Null);
}

#[test]
fn s1_1_allow_shrink_false_restores_the_strict_rule_and_its_messages() {
    let r = ok(
        json!({"op": "all_check", "document": open(E), "allow_shrink": false,
                      "inputs": ["aab", "aabbbc", "aabb"]}),
    );
    assert_eq!(r["ok"], false);
    assert_eq!(r["notes"], json!([]));
    let first = &r["violations"][0];
    assert_eq!(first["kind"], "escribe_delimitador");
    assert_eq!(
        first["message"],
        "La transición q0 --(a,⟨,D)--> q1 escribe ⟨ sobre a: ⟨ y ⟩ solo se escriben sobre sí mismos."
    );
    assert!(
        r["runs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|run| run["verdict"] == "violation")
    );
    let run = ok(
        json!({"op": "catedra_run", "document": open(E), "input": "aab",
                        "allow_shrink": false}),
    );
    assert_eq!(run["results"][0]["verdict"], "violation");
    assert_eq!(run["results"][0]["notes"], json!([]));
}

#[test]
fn s1_2_the_shrink_is_a_note_and_never_a_violation() {
    let r = ok(json!({"op": "catedra_run", "document": open(E), "inputs": ["aab"]}));
    let run = &r["results"][0];
    assert!(run.get("violation").is_none() || run["violation"].is_null());
    assert_eq!(run["accepted"], true);
    assert_eq!(run["notes"].as_array().unwrap().len(), 3);
}

#[test]
fn s1_3_the_wrong_move_when_writing_a_delimiter_is_a_violation_with_shrink() {
    let doc = json!({"type": "turing", "initial": "q0", "finals": ["q1"],
                     "transitions": [["q0", "(a,⟨,I)", "q1"]]});
    let r = ok(json!({"op": "all_check", "document": doc, "inputs": ["a"]}));
    assert_eq!(r["ok"], false);
    assert_eq!(r["violations"][0]["kind"], "escribe_delimitador");
    assert_eq!(r["notes"], json!([]));
}

#[test]
fn s1_4_b_accepts_only_abab_by_final_state() {
    let r = ok(
        json!({"op": "catedra_run", "document": open(B), "model": "all",
                      "inputs": ["aba", "abab", "babab", "aaa", "a", "b", "bb"]}),
    );
    let accepted: Vec<(&str, bool)> = r["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (x["input"].as_str().unwrap(), x["accepted"] == true))
        .collect();
    assert_eq!(
        accepted,
        [
            ("aba", false),
            ("abab", true),
            ("babab", false),
            ("aaa", false),
            ("a", false),
            ("b", false),
            ("bb", false)
        ]
    );
}

#[test]
fn s1_c_and_d_run_through_all_check() {
    let c = ok(json!({"op": "all_check", "document": open(C),
                      "inputs": ["a", "b", "ab", "aaa", "aab", "bab", "aabaa"]}));
    assert_eq!(c["ok"], true, "{c}");
    let accepted = |r: &Value| -> Vec<String> {
        verdicts(&r["runs"])
            .into_iter()
            .filter(|(_, v)| v == "accepted")
            .map(|(i, _)| i)
            .collect()
    };
    assert_eq!(accepted(&c), ["a", "b", "aaa", "aab"]);
    let d = ok(json!({"op": "all_check", "document": open(D),
                      "inputs": ["abc", "aabc", "aaabbc", "aabbcc", "aaaabbcc"]}));
    assert_eq!(d["ok"], true, "{d}");
    assert_eq!(accepted(&d), ["aabc", "aaabbc", "aaaabbcc"]);
}

// S6

fn stay_into(final_state: bool) -> Value {
    let mut doc = json!({"type": "turing", "initial": "q0",
        "transitions": [["q0", "(a,a,D)", "q0"], ["q0", "(⟩,⟩,N)", "q1"],
                        ["q0", "(⟨,⟨,D)", "q0"]]});
    if final_state {
        doc["finals"] = json!(["q1"]);
    } else {
        doc["finals"] = json!(["q0"]);
    }
    doc
}

#[test]
fn s6_staying_on_a_delimiter_into_a_final_state_is_valid_with_a_note() {
    let r = ok(json!({"op": "all_check", "document": stay_into(true), "inputs": ["a"]}));
    assert_eq!(r["ok"], true, "{r}");
    let expected =
        "q0 lee ⟩ y escribe ⟩ sin moverse (N) hacia q1: los estados finales son de parada";
    assert_eq!(notes(&r["notes"]), [expected]);
    assert_eq!(notes(&r["runs"][0]["notes"]), [expected]);
    assert_eq!(r["runs"][0]["verdict"], "accepted");
}

#[test]
fn s6_staying_into_a_non_final_state_is_still_a_violation() {
    let r = ok(json!({"op": "all_check", "document": stay_into(false)}));
    assert_eq!(r["ok"], false);
    assert_eq!(r["violations"][0]["kind"], "delimitador_derecho");
}

#[test]
fn s6_the_wrong_move_into_a_final_state_is_still_a_violation() {
    let doc = json!({"type": "turing", "initial": "q0", "finals": ["q1"],
        "transitions": [["q0", "(⟨,⟨,I)", "q1"], ["q0", "(⟩,⟩,D)", "q1"]]});
    let r = ok(json!({"op": "all_check", "document": doc}));
    let kinds: Vec<&str> = r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["delimitador_izquierdo", "delimitador_derecho"]);
}

// S4

#[test]
fn s4_an_input_with_its_delimiters_is_trimmed_and_noted() {
    let plain = ok(
        json!({"op": "catedra_run", "document": open(B), "model": "all",
                          "input": "abab"}),
    );
    for given in ["<abab>", "⟨abab⟩"] {
        let r = ok(
            json!({"op": "catedra_run", "document": open(B), "model": "all",
                          "input": given}),
        );
        let run = &r["results"][0];
        assert_eq!(run["steps"], plain["results"][0]["steps"], "{given}");
        assert_eq!(run["sequence"], plain["results"][0]["sequence"], "{given}");
        assert_eq!(run["accepted"], true, "{given}");
        // the trimming comes first, then the shrink notes of the run
        assert_eq!(
            notes(&run["notes"])[0],
            format!(
                "La entrada «{given}» ya traía los delimitadores; se quitaron y se corre ⟨abab⟩."
            )
        );
    }
    assert!(!notes(&plain["results"][0]["notes"])[0].starts_with("La entrada"));
}

#[test]
fn s4_all_check_trims_too() {
    let r = ok(json!({"op": "all_check", "document": open(B), "inputs": ["<aba>"]}));
    assert_eq!(r["ok"], true, "{r}");
    assert!(r["runs"][0]["steps"].as_u64().unwrap() > 0);
    assert!(notes(&r["runs"][0]["notes"])[0].starts_with("La entrada «<aba>» ya traía"));
}

// S7

fn ascii_machine() -> Value {
    json!({"type": "turing", "initial": "q0", "finals": ["q2"],
           "transitions": [["q0", "(a,<,D)", "q1"], ["q1", "(c,>,I)", "q2"]]})
}

#[test]
fn s7_a_compact_machine_with_lt_and_gt_is_shown_with_the_delimiters() {
    let normalized = ok(json!({"op": "normalize", "document": ascii_machine()}));
    let text = normalized["document"].to_string();
    assert!(text.contains("⟨") && text.contains("⟩"), "{text}");
    assert!(!text.contains("\"<\"") && !text.contains("\">\""), "{text}");
    let table = ok(json!({"op": "catedra_table", "document": ascii_machine()}));
    let t = table["text"].as_str().unwrap();
    assert!(
        t.contains("⟨") && t.contains("⟩") && !t.contains('<'),
        "{t}"
    );
    let definition = ok(json!({"op": "catedra_definition", "document": ascii_machine()}));
    assert_eq!(definition["model"], "all");
}

#[test]
fn s7_save_jff_writes_the_delimiters_and_not_the_ascii_ones() {
    let saved = ok(json!({"op": "save_jff", "document": ascii_machine()}));
    let xml = saved["xml"].as_str().unwrap();
    assert!(xml.contains("<write>⟨</write>"), "{xml}");
    assert!(xml.contains("<write>⟩</write>"), "{xml}");
    assert!(!xml.contains("&lt;") && !xml.contains("&gt;"), "{xml}");
}

#[test]
fn s7_the_full_form_is_normalized_too() {
    let plain = json!({"type": "turing", "initial": "q0", "finals": ["q2"],
        "transitions": [["q0", "(a,x,D)", "q1"], ["q1", "(c,y,I)", "q2"]]});
    let full = ok(json!({"op": "normalize", "document": plain}))["document"].to_string();
    let ascii = full
        .replace("\"write\":\"x\"", "\"write\":\"<\"")
        .replace("\"write\":\"y\"", "\"write\":\">\"");
    let document: Value = serde_json::from_str(&ascii).unwrap();
    let again = ok(json!({"op": "normalize", "document": document}))["document"].to_string();
    assert!(
        again.contains("\"⟨\"") && again.contains("\"⟩\""),
        "{again}"
    );
    assert!(!again.contains("\"<\""), "{again}");
}

#[test]
fn s7_an_old_jff_with_ascii_delimiters_opens_with_the_delimiters_as_an_all() {
    let ascii = B.replace('⟨', "&lt;").replace('⟩', "&gt;");
    assert!(ascii.contains("&lt;") && !ascii.contains('⟨'));
    let document = open(&ascii);
    let saved = ok(json!({"op": "save_jff", "document": document}));
    let xml = saved["xml"].as_str().unwrap();
    assert!(xml.contains('⟨') && xml.contains('⟩'));
    assert!(!xml.contains("&lt;"), "{xml}");
    let definition = ok(json!({"op": "catedra_definition", "document": document}));
    assert_eq!(definition["model"], "all");
    let r = ok(json!({"op": "catedra_run", "document": document,
                      "inputs": ["aba", "abab", "babab"]}));
    assert_eq!(r["model"], "all");
    let accepted: Vec<bool> = r["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["accepted"] == true)
        .collect();
    assert_eq!(accepted, [false, true, false]);
}

#[test]
fn s7_a_machine_using_only_one_of_them_keeps_it_as_a_symbol() {
    for symbol in ["<", ">"] {
        let doc = json!({"type": "turing", "initial": "q0", "finals": ["q1"],
            "transitions": [["q0", format!("(a,{symbol},D)"), "q1"]]});
        let saved = ok(json!({"op": "save_jff", "document": doc}));
        let xml = saved["xml"].as_str().unwrap();
        assert!(!xml.contains('⟨') && !xml.contains('⟩'), "{xml}");
        assert!(
            xml.contains(if symbol == "<" { "&lt;" } else { "&gt;" }),
            "{xml}"
        );
        let definition = ok(json!({"op": "catedra_definition", "document": doc}));
        assert_eq!(definition["model"], "mt");
    }
}
