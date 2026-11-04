//! Differential tests against the real JFLAP 7.1 (S8): what Rflap designs and saves opens in
//! JFLAP with the same graph and runs with the same verdicts as JFLAP's own simulators.
//!
//! They run `crates/rflap-core/tests/fixtures/jff/harness/HarnessSim.java` with `java` and the
//! jar (never committed; found in `$JFLAP_JAR`, `JFLAP7_1.JAR` at the workspace root, or at the
//! root of the main checkout of this repository). Without Java or the jar every test prints
//! `SKIPPED` and passes, like the CI without the jar.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::*;
use rflap_api::Document;
use rflap_core::automaton::{Automaton, Label};
use serde_json::{Value, json};

const B: &str = include_str!("fixtures/catedra/ALL_2.1b_complemento_palindromo.jff");
const C: &str = include_str!("fixtures/catedra/ALL_2.1c_central_igual_primero.jff");
const D: &str = include_str!("fixtures/catedra/ALL_2.1d_n_igual_m_mas_p.jff");
const E: &str = include_str!("fixtures/catedra/ALL_2.1e_m_igual_2p_mas_1.jff");

// ---------------------------------------------------------------------------------------------
// Running JFLAP
// ---------------------------------------------------------------------------------------------

fn jar() -> Option<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(path) = std::env::var_os("JFLAP_JAR") {
        candidates.push(path.into());
    }
    candidates.push(manifest.join("../../JFLAP7_1.JAR"));
    // a linked worktree: the jar sits at the root of the main checkout
    if let Ok(out) = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(manifest)
        .output()
        && let Ok(common) = String::from_utf8(out.stdout)
        && let Some(root) = Path::new(common.trim()).parent()
    {
        candidates.push(root.join("JFLAP7_1.JAR"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn java_runs() -> bool {
    Command::new("java")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The jar, when the differential tests can run; prints why not otherwise.
fn jflap() -> Option<PathBuf> {
    match (jar(), java_runs()) {
        (Some(jar), true) => Some(jar),
        _ => {
            eprintln!("SKIPPED: needs `java` and JFLAP7_1.JAR (set JFLAP_JAR)");
            None
        }
    }
}

fn scratch() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "rflap-jflap-diff-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Loads `xml` with JFLAP's `XMLCodec` and runs `inputs` (one entry per input, tapes joined
/// by a tab) in JFLAP's own simulators: the lines of the harness, or JFLAP's error.
fn run_jflap(jar: &Path, xml: &str, inputs: &[String]) -> Result<Vec<String>, String> {
    let dir = scratch();
    let (file, list, out) = (
        dir.join("machine.jff"),
        dir.join("inputs.txt"),
        dir.join("out.txt"),
    );
    std::fs::write(&file, xml).unwrap();
    std::fs::write(
        &list,
        inputs.iter().map(|i| format!("{i}\n")).collect::<String>(),
    )
    .unwrap();
    let harness = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rflap-core/tests/fixtures/jff/harness/HarnessSim.java");
    let output = Command::new("java")
        .args(["-Djava.awt.headless=true", "-cp"])
        .arg(jar)
        .arg(harness)
        .args([&file, &list, &out])
        .output()
        .expect("java starts");
    let result = if output.status.success() {
        Ok(std::fs::read_to_string(&out)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    };
    let _ = std::fs::remove_dir_all(&dir);
    result
}

// ---------------------------------------------------------------------------------------------
// The graph as Rflap sees it, in the harness's canonical lines
// ---------------------------------------------------------------------------------------------

fn lines<L: Label>(
    class: &str,
    a: &Automaton<L>,
    describe: impl Fn(&L) -> String,
    moore: bool,
) -> Vec<String> {
    let mut out = vec![
        format!("type {class}"),
        format!(
            "initial {}",
            a.initial().map_or("none".to_string(), |i| i.to_string())
        ),
    ];
    for s in a.states() {
        let output = if moore {
            format!(" output={}", s.output.clone().unwrap_or_default())
        } else {
            String::new()
        };
        out.push(format!(
            "state {} name={} label={} final={}{output}",
            s.id,
            s.name,
            s.label.clone().unwrap_or_else(|| "null".to_string()),
            a.is_final(s.id)
        ));
    }
    let mut transitions: Vec<String> = a
        .transitions()
        .iter()
        .map(|t| format!("transition {} {} {}", t.from, t.to, describe(&t.label)))
        .collect();
    transitions.sort();
    out.extend(transitions);
    out
}

fn rflap_graph(document: &Value) -> Vec<String> {
    match Document::from_value(document.clone()).expect("a document") {
        Document::Fa(a) => lines("FiniteStateAutomaton", &a, |l| l.description(), false),
        Document::Pda(a) => lines("PushdownAutomaton", &a, |l| l.description(), false),
        Document::Turing(t) => lines("TuringMachine", t.automaton(), |l| l.description(), false),
        Document::Mealy(a) => lines("MealyMachine", &a, |l| l.description(), false),
        Document::Moore(a) => lines("MooreMachine", &a, |l| l.description(), true),
        other => panic!("not a machine: {}", other.type_name()),
    }
}

fn graph_part(jflap_lines: &[String]) -> Vec<String> {
    jflap_lines
        .iter()
        .filter(|l| !l.starts_with("run "))
        .cloned()
        .collect()
}

fn verdicts_of_jflap(jflap_lines: &[String]) -> Vec<String> {
    jflap_lines
        .iter()
        .filter_map(|l| l.strip_prefix("run "))
        .map(|l| l.split_once(' ').unwrap().1.to_string())
        .collect()
}

/// Rflap's verdict in the harness's words: `accept`, `reject`, with ` output=…` for the
/// Mealy and Moore machines.
fn rflap_verdict(document: &Value, tapes: &[&str]) -> String {
    let request = if tapes.len() == 1 {
        json!({"op": "simulate", "document": document, "input": tapes[0]})
    } else {
        json!({"op": "simulate", "document": document, "inputs": tapes})
    };
    let r = ok(request);
    match r["outcome"].as_str().unwrap() {
        "accepted" => match r["output"].as_str() {
            Some(output) => format!("accept output={output}"),
            None => "accept".to_string(),
        },
        "rejected" => "reject".to_string(),
        other => panic!("{other}: {r}"),
    }
}

/// Saves `document` with Rflap, loads it in JFLAP and compares the graph and the verdict of
/// every input. Returns false (the test was skipped) without Java or the jar.
fn same_in_jflap(document: &Value, inputs: &[&[&str]]) -> bool {
    let Some(jar) = jflap() else { return false };
    let xml = ok(json!({"op": "save_jff", "document": document}))["xml"]
        .as_str()
        .unwrap()
        .to_string();
    let joined: Vec<String> = inputs.iter().map(|tapes| tapes.join("\t")).collect();
    let out = run_jflap(&jar, &xml, &joined).unwrap_or_else(|e| panic!("JFLAP refused it: {e}"));
    assert_eq!(graph_part(&out), rflap_graph(document), "the graph differs");
    let jflap_verdicts = verdicts_of_jflap(&out);
    let rflap_verdicts: Vec<String> = inputs.iter().map(|t| rflap_verdict(document, t)).collect();
    assert_eq!(
        jflap_verdicts, rflap_verdicts,
        "verdicts differ for the inputs {inputs:?} (JFLAP left, Rflap right)"
    );
    true
}

fn designed(spec: Value) -> Value {
    ok(json!({"op": "design", "spec": spec}))["document"].clone()
}

fn open(xml: &str) -> Value {
    ok(json!({"op": "open_jff", "xml": xml}))["document"].clone()
}

fn one(words: &[&'static str]) -> Vec<Vec<&'static str>> {
    words.iter().map(|w| vec![*w]).collect()
}

fn run_same(document: &Value, words: &[&'static str]) -> bool {
    let inputs = one(words);
    let refs: Vec<&[&str]> = inputs.iter().map(Vec::as_slice).collect();
    same_in_jflap(document, &refs)
}

// ---------------------------------------------------------------------------------------------
// Finite automata
// ---------------------------------------------------------------------------------------------

#[test]
fn designed_dfa_for_a_constraint_opens_and_runs_the_same_in_jflap() {
    let doc = designed(json!({
        "alphabet": "ab",
        "language": {"constraint_text": "ends_with(\"ab\") and count(a) % 2 == 0"},
    }));
    run_same(
        &doc,
        &["", "ab", "aab", "bab", "aabab", "abb", "b", "aaab", "abab"],
    );
}

#[test]
fn designed_dfa_for_a_regex_opens_and_runs_the_same_in_jflap() {
    let doc = designed(json!({"alphabet": "01", "language": {"regex": "(0+1)*101"}}));
    run_same(&doc, &["", "101", "0101", "1101", "10", "1010", "00101"]);
}

#[test]
fn designed_dfa_for_a_regular_grammar_opens_and_runs_the_same_in_jflap() {
    let doc = designed(json!({
        "alphabet": "ab",
        "language": {"regular_grammar": "S -> aS | bA\nA -> bA | λ"},
    }));
    run_same(&doc, &["", "b", "ab", "aabb", "ba", "a"]);
}

#[test]
fn an_nfa_with_lambda_transitions_and_multi_character_labels_runs_the_same_in_jflap() {
    let doc = json!({"type": "fa", "initial": "q0", "finals": ["q3"], "transitions": [
        ["q0", "ab", "q1"], ["q1", "", "q2"], ["q2", "cd", "q3"], ["q2", "a", "q2"],
        ["q0", "λ", "q3"], ["q3", "[0-9]", "q3"]]});
    run_same(
        &doc,
        &["", "abcd", "abaacd", "abcd7", "ab", "abc", "7", "abcd77x"],
    );
}

// ---------------------------------------------------------------------------------------------
// Pushdown automata
// ---------------------------------------------------------------------------------------------

#[test]
fn designed_pda_for_a_template_opens_and_runs_the_same_in_jflap() {
    let doc = designed(json!({
        "alphabet": "ab",
        "language": {"template": {"name": "an_bn", "a": "a", "b": "b"}},
    }));
    run_same(
        &doc,
        &["", "ab", "aabb", "aab", "abb", "ba", "abab", "aaabbb"],
    );
}

#[test]
fn a_pda_with_multi_symbol_push_and_lambda_moves_runs_the_same_in_jflap() {
    let doc = json!({"type": "pda", "initial": "q0", "finals": ["q2"], "transitions": [
        ["q0", "a,Z;AAZ", "q0"], ["q0", "a,A;AAA", "q0"], ["q0", "b,A;λ", "q1"],
        ["q1", "b,A;λ", "q1"], ["q1", "λ,Z;Z", "q2"], ["q0", "λ,Z;Z", "q2"]]});
    run_same(&doc, &["", "ab", "abb", "aabbb", "aab", "b", "aabbbb"]);
}

// ---------------------------------------------------------------------------------------------
// Mealy and Moore machines
// ---------------------------------------------------------------------------------------------

#[test]
fn a_mealy_machine_opens_and_gives_the_same_outputs_in_jflap() {
    let doc = json!({"type": "mealy", "initial": "q0", "transitions": [
        ["q0", "a;1", "q0"], ["q0", "b;0", "q1"], ["q1", "a;", "q0"], ["q1", "b;01", "q1"]]});
    run_same(&doc, &["", "a", "ab", "abb", "ba", "c", "abba"]);
}

#[test]
fn a_moore_machine_opens_and_gives_the_same_outputs_in_jflap() {
    let doc = json!({"type": "moore", "initial": "q0", "outputs": {"q0": "x", "q1": "yy"},
                     "transitions": [["q0", "a", "q1"], ["q1", "a", "q0"], ["q1", "b", "q1"]]});
    run_same(&doc, &["", "a", "aa", "ab", "abb", "b", "aab"]);
}

// ---------------------------------------------------------------------------------------------
// Turing machines
// ---------------------------------------------------------------------------------------------

/// The palindrome machine of the course (CLASE 39), cátedra notation.
fn palindrome() -> Value {
    json!({
        "type": "turing",
        "states": ["1", "2", "3", "4", "5", "6", "7", "8"],
        "initial": "1",
        "finals": ["8"],
        "transitions": [
            ["1", "(a,Δ,D)", "2"], ["1", "(b,Δ,D)", "5"], ["1", "(Δ,Δ,D)", "8"],
            ["2", "(a,a,D) / (b,b,D)", "2"], ["2", "(Δ,Δ,I)", "3"],
            ["3", "(a,Δ,I)", "4"], ["3", "(Δ,Δ,D)", "8"],
            ["4", "(a,a,I) / (b,b,I)", "4"], ["4", "(Δ,Δ,D)", "1"],
            ["5", "(a,a,D) / (b,b,D)", "5"], ["5", "(Δ,Δ,I)", "6"],
            ["6", "(b,Δ,I)", "7"], ["6", "(Δ,Δ,D)", "8"],
            ["7", "(a,a,I) / (b,b,I)", "7"], ["7", "(Δ,Δ,D)", "1"]
        ]
    })
}

#[test]
fn the_palindrome_machine_of_the_course_opens_and_runs_the_same_in_jflap() {
    run_same(
        &palindrome(),
        &[
            "", "a", "aba", "abba", "ab", "abb", "baab", "babab", "aab", "bbaa",
        ],
    );
}

#[test]
fn a_machine_with_n_moves_and_blank_transitions_runs_the_same_in_jflap() {
    let doc = anbncn_tm();
    run_same(&doc, &["", "abc", "aabbcc", "aabbc", "abcabc", "cba", "a"]);
}

#[test]
fn a_two_tape_machine_opens_and_runs_the_same_in_jflap() {
    let doc = json!({"type": "turing", "initial": "d", "finals": ["e"], "transitions": [
        ["d", "a;a,R|_;a,R", "d"], ["d", "b;b,R|_;b,R", "d"], ["d", "_;_,S|_;_,L", "e"]]});
    let inputs: Vec<Vec<&str>> = ["", "a", "ab", "aab", "ac"]
        .iter()
        .map(|w| vec![*w, ""])
        .collect();
    let refs: Vec<&[&str]> = inputs.iter().map(Vec::as_slice).collect();
    same_in_jflap(&doc, &refs);
}

#[test]
fn a_nondeterministic_machine_takes_the_first_transition_in_both() {
    let doc = json!({"type": "turing", "initial": "q0", "finals": ["qf"], "transitions": [
        ["q0", "a;a,R", "q1"], ["q0", "a;a,R", "q0"], ["q1", "b;b,S", "qf"]]});
    run_same(&doc, &["ab", "aab", "a", "b"]);
}

// ---------------------------------------------------------------------------------------------
// What jflap_check reports, against JFLAP
// ---------------------------------------------------------------------------------------------

#[test]
fn jflap_refuses_a_machine_with_six_tapes_and_jflap_check_says_so() {
    let label = ["a;a,R"; 6].join("|");
    let doc = json!({"type": "turing", "initial": "q0", "finals": ["q1"],
                     "transitions": [["q0", label, "q1"]]});
    let check = ok(json!({"op": "jflap_check", "document": doc}));
    assert_eq!(check["compatible"], false, "{check}");
    let Some(jar) = jflap() else { return };
    let xml = ok(json!({"op": "save_jff", "document": doc}))["xml"]
        .as_str()
        .unwrap()
        .to_string();
    let refused = run_jflap(&jar, &xml, &[]).expect_err("JFLAP should refuse six tapes");
    assert!(refused.contains("invalid # of tapes"), "{refused}");
}

#[test]
fn a_range_before_a_lambda_exit_is_accepted_by_rflap_and_rejected_by_jflap() {
    let doc = json!({"type": "fa", "initial": "q0", "finals": ["q2"],
                     "transitions": [["q0", "[0-9]", "q1"], ["q1", "", "q2"]]});
    let check = ok(json!({"op": "jflap_check", "document": doc}));
    assert_eq!(check["compatible"], false, "{check}");
    let Some(jar) = jflap() else { return };
    let xml = ok(json!({"op": "save_jff", "document": doc}))["xml"]
        .as_str()
        .unwrap()
        .to_string();
    let out = run_jflap(&jar, &xml, &["5".to_string()]).unwrap();
    assert_eq!(verdicts_of_jflap(&out), ["reject"]);
    assert_eq!(rflap_verdict(&doc, &["5"]), "accept");
}

#[test]
fn the_same_range_without_the_lambda_exit_agrees_and_is_compatible() {
    let doc = json!({"type": "fa", "initial": "q0", "finals": ["q1"],
                     "transitions": [["q0", "[0-9]", "q1"], ["q1", "[a-c]", "q1"]]});
    let check = ok(json!({"op": "jflap_check", "document": doc}));
    assert_eq!(check["compatible"], true, "{check}");
    run_same(&doc, &["5", "5ab", "x", "", "55"]);
}

#[test]
fn a_machine_without_final_states_is_rejected_by_jflap_unless_it_accepts_by_halting() {
    let doc = json!({"type": "turing", "initial": "q0", "finals": [],
                     "transitions": [["q0", "a;a,R", "q0"]]});
    let check = ok(json!({"op": "jflap_check", "document": doc}));
    assert_eq!(check["notes"][0]["code"], "accepts_by_halting", "{check}");
    let Some(jar) = jflap() else { return };
    let xml = ok(json!({"op": "save_jff", "document": doc}))["xml"]
        .as_str()
        .unwrap()
        .to_string();
    let out = run_jflap(&jar, &xml, &["a".to_string()]).unwrap();
    assert_eq!(verdicts_of_jflap(&out), ["reject"]);
    let halting = ok(json!({"op": "simulate", "document": doc, "input": "a",
                            "acceptance": "halting"}));
    assert_eq!(halting["outcome"], "accepted");
}

// ---------------------------------------------------------------------------------------------
// The ALL fixtures
// ---------------------------------------------------------------------------------------------

#[test]
fn the_four_all_fixtures_load_in_jflap_with_the_same_graph_and_carry_the_all_note() {
    let jar = jflap();
    for (name, xml) in [("b", B), ("c", C), ("d", D), ("e", E)] {
        let document = open(xml);
        let check = ok(json!({"op": "jflap_check", "document": document}));
        assert_eq!(check["compatible"], true, "{name}: {check}");
        assert_eq!(check["notes"][0]["code"], "all_model", "{name}: {check}");
        let Some(jar) = &jar else { continue };
        // the file as the professor wrote it, and as Rflap saves it
        let saved = ok(json!({"op": "save_jff", "document": document}))["xml"]
            .as_str()
            .unwrap()
            .to_string();
        for (which, text) in [("original", xml.to_string()), ("saved", saved)] {
            let out = run_jflap(jar, &text, &[]).unwrap_or_else(|e| panic!("{name} {which}: {e}"));
            assert_eq!(
                graph_part(&out),
                rflap_graph(&document),
                "{name} ({which}): the graph differs"
            );
        }
    }
}

/// The recipe of the ALL note: a new initial state with `(⟨,⟨,D)` to the old one, and the word
/// typed as `⟨w⟩`. JFLAP then gives the verdicts of the cátedra ALL.
fn with_left_delimiter_step(document: &Value) -> Value {
    let mut doc = document.clone();
    let automaton = &mut doc["automaton"];
    let old = automaton["initial"].clone();
    let new_id = automaton["states"].as_object().unwrap().len() + 100;
    automaton["states"][new_id.to_string()] = json!({
        "id": new_id, "name": "s0", "label": null, "output": null,
        "point": {"x": 5.0, "y": 5.0}});
    automaton["initial"] = json!(new_id);
    automaton["transitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
        "from": new_id, "to": old, "control": null,
        "label": {"actions": [{"read": "⟨", "write": "⟨", "direction": "right"}]}}));
    doc
}

#[test]
fn the_recipe_of_the_all_note_gives_in_jflap_the_verdicts_of_the_all() {
    let Some(jar) = jflap() else { return };
    let words = [
        "", "a", "ab", "aba", "abab", "babab", "aaa", "bb", "aab", "aabbbc", "aabb", "aabc",
        "aaabbc",
    ];
    for (name, xml) in [("b", B), ("c", C), ("d", D), ("e", E)] {
        let document = open(xml);
        let expected = ok(
            json!({"op": "catedra_run", "document": document, "model": "all",
                                  "inputs": words}),
        );
        let patched = with_left_delimiter_step(&document);
        let saved = ok(json!({"op": "save_jff", "document": patched}))["xml"]
            .as_str()
            .unwrap()
            .to_string();
        let typed: Vec<String> = words.iter().map(|w| format!("⟨{w}⟩")).collect();
        let out = run_jflap(&jar, &saved, &typed).unwrap();
        let jflap_verdicts = verdicts_of_jflap(&out);
        let all_verdicts: Vec<String> = expected["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                if r["accepted"] == true {
                    "accept"
                } else {
                    "reject"
                }
                .to_string()
            })
            .collect();
        assert_eq!(jflap_verdicts, all_verdicts, "ALL 2.1{name}: {words:?}");
    }
}
