//! The notation of the course "Sintaxis y Semántica de los Lenguajes": MT and ALL operations.

mod common;

use common::*;
use serde_json::{Value, json};

/// The palindrome MT of CLASE 39 (states 1-8, F = {8}); several labels on one edge.
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

/// The ALL "IGUAL" of CLASE 39 (same number of a and b), states I, A, B, C, D, F.
fn igual() -> Value {
    json!({
        "type": "turing",
        "states": ["I", "A", "B", "C", "D", "F"],
        "initial": "I",
        "finals": ["F"],
        "transitions": [
            ["I", "a,X,I", "A"], ["I", "(b,b,D) / (X,X,D)", "I"], ["I", "(⟩,⟩,I)", "D"],
            ["A", "(b,b,I) / (X,X,I)", "A"], ["A", "(⟨,⟨,D)", "B"],
            ["B", "(a,a,D)", "B"], ["B", "(b,X,I)", "C"], ["B", "(X,X,D)", "B"],
            ["C", "(a,a,I) / (X,X,I)", "C"], ["C", "(⟨,⟨,D)", "I"],
            ["D", "(X,X,I)", "D"], ["D", "(⟨,⟨,D)", "F"]
        ]
    })
}

#[test]
fn compact_turing_accepts_the_catedra_and_the_jflap_label_syntax() {
    let cat = ok(json!({"op": "normalize", "document":
        {"type": "turing", "initial": "p", "transitions": [["p", "(a,X,D)", "p"]]}}));
    let bare = ok(json!({"op": "normalize", "document":
        {"type": "turing", "initial": "p", "transitions": [["p", "a,X,D", "p"]]}}));
    let jflap = ok(json!({"op": "normalize", "document":
        {"type": "turing", "initial": "p", "transitions": [["p", "a;X,R", "p"]]}}));
    assert_eq!(
        cat["document"]["automaton"]["transitions"],
        bare["document"]["automaton"]["transitions"]
    );
    assert_eq!(
        cat["document"]["automaton"]["transitions"],
        jflap["document"]["automaton"]["transitions"]
    );
    let blank = ok(json!({"op": "normalize", "document":
        {"type": "turing", "initial": "p", "transitions": [["p", "(Δ,Δ,N)", "p"]]}}));
    let old = ok(json!({"op": "normalize", "document":
        {"type": "turing", "initial": "p", "transitions": [["p", "_;_,S", "p"]]}}));
    assert_eq!(
        blank["document"]["automaton"]["transitions"],
        old["document"]["automaton"]["transitions"]
    );
}

#[test]
fn an_edge_with_several_labels_makes_several_transitions() {
    let r = ok(json!({"op": "normalize", "document": palindrome()}));
    let transitions = r["document"]["automaton"]["transitions"]
        .as_array()
        .unwrap();
    assert_eq!(transitions.len(), 19);
}

#[test]
fn input_alphabet_can_be_declared() {
    let mut doc = palindrome();
    doc["input_alphabet"] = json!("ab");
    let r = ok(json!({"op": "catedra_definition", "document": doc}));
    assert_eq!(r["input_alphabet_inferred"], false);
    let mut doc = palindrome();
    doc["input_alphabet"] = json!(["a", "b"]);
    let r = ok(json!({"op": "normalize", "document": doc}));
    assert_eq!(r["document"]["input_alphabet"], json!(["a", "b"]));
    let e = err(
        json!({"op": "normalize", "document": {"type": "turing", "initial": "p",
        "input_alphabet": ["ab"], "transitions": [["p", "a,a,D", "p"]]}}),
    );
    assert_eq!(e["code"], "invalid_document");
}

#[test]
fn catedra_definition_of_the_palindrome_machine() {
    let r = ok(json!({"op": "catedra_definition", "document": palindrome()}));
    assert_eq!(r["model"], "mt");
    assert_eq!(r["states"], json!(["1", "2", "3", "4", "5", "6", "7", "8"]));
    assert_eq!(r["input_alphabet"], json!(["a", "b"]));
    assert_eq!(r["input_alphabet_inferred"], true);
    assert_eq!(r["auxiliary_alphabet"], json!(["Δ"]));
    assert_eq!(r["initial"], "1");
    assert_eq!(r["finals"], json!(["8"]));
    assert_eq!(r["deterministic"], true);
    assert_eq!(r["issues"], json!([]));
    assert_eq!(
        r["text"],
        "MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩\nQ = {1, 2, 3, 4, 5, 6, 7, 8}\nΣ_E = {a, b} (inferido)\nΣ_A = {Δ}\nq0 = 1\nF = {8}\n"
    );
}

#[test]
fn catedra_definition_of_igual_is_an_all_and_can_be_forced() {
    let r = ok(json!({"op": "catedra_definition", "document": igual()}));
    assert_eq!(r["model"], "all");
    assert_eq!(r["auxiliary_alphabet"], json!(["⟨", "⟩", "X"]));
    let forced = ok(json!({"op": "catedra_definition", "document": igual(), "model": "mt"}));
    assert_eq!(forced["model"], "mt");
    assert_eq!(forced["auxiliary_alphabet"], json!(["⟨", "⟩", "X", "Δ"]));
}

#[test]
fn catedra_definition_reports_the_issues() {
    let r = ok(json!({"op": "catedra_definition", "document": {
        "type": "turing", "initial": "p", "finals": ["q"],
        "transitions": [["p", "a,a,D", "q"], ["p", "a,b,D", "p"], ["q", "a,a,D", "p"]]}}));
    assert_eq!(r["deterministic"], false);
    let kinds: Vec<&str> = r["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, vec!["no_determinista", "dominio_incluye_final"]);
    assert_eq!(r["issues"][1]["transitions"], json!([2]));
}

#[test]
fn catedra_table_has_markdown_text_and_rows() {
    let r = ok(json!({"op": "catedra_table", "document": palindrome()}));
    let markdown = r["markdown"].as_str().unwrap();
    assert!(markdown.starts_with(
        "| Estado Actual | Símbolo Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal |\n|---|---|---|---|---|\n| →1 | a | 2 | Δ | D |\n"
    ));
    assert!(markdown.ends_with("| 7 | Δ | 1 | Δ | D |\n| *8 | - | - | - | - |\n"));
    let text = r["text"].as_str().unwrap();
    assert!(text.starts_with("Estado Actual  Símbolo Entrada  Estado Nuevo  Símbolo Salida  Movimiento Cabezal\n→1             a                2             Δ               D\n"));
    assert_eq!(r["header"].as_array().unwrap().len(), 5);
    assert_eq!(r["rows"].as_array().unwrap().len(), 20);
    assert_eq!(r["rows"][0], json!(["→1", "a", "2", "Δ", "D"]));
    assert_eq!(r["rows"][19], json!(["*8", "-", "-", "-", "-"]));
}

#[test]
fn catedra_run_reproduces_the_palindrome_examples() {
    let r = ok(json!({"op": "catedra_run", "document": palindrome(),
        "inputs": ["aba", "bb", "ab", ""], "model": "mt", "max_steps": 100}));
    assert_eq!(r["model"], "mt");
    assert_eq!(r["max_steps"], 100);
    let results = r["results"].as_array().unwrap();
    assert_eq!(results.len(), 4);
    assert_eq!(
        results[0]["sequence"],
        "Δ1abaΔ ├─ ΔΔ2baΔ ├─ ΔΔb2aΔ ├─ ΔΔba2Δ ├─ ΔΔb3aΔ ├─ ΔΔ4bΔΔ ├─ Δ4ΔbΔΔ ├─ ΔΔ1bΔΔ ├─ ΔΔΔ5ΔΔ ├─ ΔΔ6ΔΔΔ ├─ ΔΔΔ8ΔΔ final/acepta"
    );
    assert_eq!(results[0]["verdict"], "accepted");
    assert_eq!(results[0]["accepted"], true);
    assert_eq!(results[0]["steps"], 10);
    assert_eq!(results[0]["input"], "aba");
    assert_eq!(results[0]["configurations"].as_array().unwrap().len(), 11);
    assert_eq!(
        results[1]["sequence"],
        "Δ1bbΔ ├─ ΔΔ5bΔ ├─ ΔΔb5Δ ├─ ΔΔ6bΔ ├─ Δ7ΔΔΔ ├─ ΔΔ1ΔΔ ├─ ΔΔΔ8Δ final/acepta"
    );
    assert_eq!(
        results[2]["sequence"],
        "Δ1abΔ ├─ ΔΔ2bΔ ├─ ΔΔb2Δ ├─ ΔΔ3bΔ bloqueo/rechazo"
    );
    assert_eq!(results[2]["verdict"], "rejected");
    assert_eq!(results[2]["accepted"], false);
    assert_eq!(results[3]["sequence"], "Δ1Δ ├─ ΔΔ8Δ final/acepta");
}

#[test]
fn catedra_run_single_input_and_defaults() {
    // the model is detected (mt without delimiters) and the budget defaults to 1000
    let r = ok(json!({"op": "catedra_run", "document": palindrome(), "input": "ab"}));
    assert_eq!(r["model"], "mt");
    assert_eq!(r["max_steps"], 1000);
    assert_eq!(r["results"].as_array().unwrap().len(), 1);
}

#[test]
fn catedra_run_as_all_reproduces_igual() {
    let r = ok(json!({"op": "catedra_run", "document": igual(),
        "inputs": ["ab", "b"], "model": "all"}));
    let results = r["results"].as_array().unwrap();
    assert_eq!(
        results[0]["sequence"],
        "⟨Iab⟩ ├─ A⟨Xb⟩ ├─ ⟨BXb⟩ ├─ ⟨XBb⟩ ├─ ⟨CXX⟩ ├─ C⟨XX⟩ ├─ ⟨IXX⟩ ├─ ⟨XIX⟩ ├─ ⟨XXI⟩ ├─ ⟨XDX⟩ ├─ ⟨DXX⟩ ├─ D⟨XX⟩ ├─ ⟨FXX⟩ final/acepta"
    );
    assert_eq!(
        results[1]["sequence"],
        "⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza"
    );
    assert_eq!(results[1]["verdict"], "rejected");
}

#[test]
fn catedra_run_reports_machines_that_do_not_stop() {
    let r = ok(json!({"op": "catedra_run", "max_steps": 3, "input": "",
        "document": {"type": "turing", "initial": "p", "finals": ["q"],
                     "transitions": [["p", "(Δ,Δ,D)", "p"]]}}));
    assert_eq!(r["results"][0]["verdict"], "no_halt");
    assert_eq!(
        r["results"][0]["sequence"],
        "ΔpΔ ├─ ΔΔpΔ ├─ ΔΔΔpΔ ├─ ΔΔΔΔpΔ no se detiene (límite de 3 pasos)"
    );
}

#[test]
fn all_check_accepts_igual() {
    let r = ok(json!({"op": "all_check", "document": igual(), "inputs": ["ab", "b"]}));
    assert_eq!(r["ok"], true);
    assert_eq!(r["violations"], json!([]));
    assert_eq!(r["runs"].as_array().unwrap().len(), 2);
    assert_eq!(
        r["runs"][1]["sequence"],
        "⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza"
    );
}

#[test]
fn all_check_names_the_transition_and_the_step() {
    let doc = json!({"type": "turing", "states": ["I", "D", "F"], "initial": "I", "finals": ["F"],
        "transitions": [["I", "(b,b,D)", "I"], ["I", "(⟩,X,I)", "D"], ["D", "(⟨,⟨,D)", "F"]]});
    let r = ok(json!({"op": "all_check", "document": doc, "inputs": ["b"]}));
    assert_eq!(r["ok"], false);
    assert_eq!(
        r["violations"],
        json!([
            {"kind": "delimitador_derecho", "transition": 1,
             "transition_text": "I --(⟩,X,I)--> D",
             "message": "La transición I --(⟩,X,I)--> D lee ⟩: debe escribir ⟩ y moverse a la izquierda (I)."},
            {"kind": "sobrescribe_delimitador", "step": 2, "input": "b", "transition": 1,
             "transition_text": "I --(⟩,X,I)--> D",
             "message": "Paso 2 con la entrada \"b\": la transición I --(⟩,X,I)--> D escribe X sobre el delimitador ⟩."}
        ])
    );
    assert_eq!(r["runs"][0]["verdict"], "violation");
    assert_eq!(r["runs"][0]["accepted"], false);
}

#[test]
fn all_check_without_inputs_is_static_only() {
    let r = ok(json!({"op": "all_check", "document": igual()}));
    assert_eq!(r["ok"], true);
    assert_eq!(r["runs"], json!([]));
}

#[test]
fn compose_tm_chains_two_machines() {
    let a = json!({"type": "turing", "initial": "s0", "finals": ["s1"],
        "transitions": [["s0", "(a,a,D)", "s1"]]});
    let b = json!({"type": "turing", "initial": "s0", "finals": ["s1"],
        "transitions": [["s0", "(b,b,D)", "s1"]]});
    let r = ok(json!({"op": "compose_tm", "a": a, "b": b}));
    let doc = &r["document"];
    assert_eq!(doc["type"], "turing");
    let names: Vec<&str> = doc["automaton"]["states"]
        .as_object()
        .unwrap()
        .values()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["s0", "s1", "s0'", "s1'"]);
    assert_eq!(r["definition"]["finals"], json!(["s1'"]));
    // the composite runs the first machine and then the second
    let run = ok(json!({"op": "catedra_run", "document": doc, "input": "ab"}));
    assert_eq!(
        run["results"][0]["sequence"],
        "Δs0abΔ ├─ Δas0'bΔ ├─ Δabs1'Δ final/acepta"
    );
}

#[test]
fn membership_tk_reproduces_the_course_example() {
    let grammar = "S -> T\nDB -> BD\nD -> c\nT -> aTBD\nT -> abD\nbB -> bb";
    let r = ok(json!({"op": "membership_tk", "grammar": grammar, "input": "abbc"}));
    assert_eq!(r["member"], false);
    assert_eq!(r["bound"], 4);
    assert_eq!(
        r["sets"],
        json!([
            ["S"],
            ["S", "T"],
            ["S", "T", "aTBD", "abD"],
            ["S", "T", "aTBD", "abD", "aTBc", "abc"],
            ["S", "T", "aTBD", "abD", "aTBc", "abc"]
        ])
    );
    assert_eq!(
        r["text"],
        "T0 = { S }\nT1 = { S, T }\nT2 = { S, T, aTBD, abD }\nT3 = { S, T, aTBD, abD, aTBc, abc }\nT4 = T3\nComo T4 = T3, \"abbc\" no pertenece al lenguaje.\n"
    );
    let yes = ok(json!({"op": "membership_tk", "grammar": grammar, "input": "abc"}));
    assert_eq!(yes["member"], true);
}

#[test]
fn analyze_adds_multipista_and_the_catedra_warnings() {
    let r = ok(json!({"op": "analyze", "document": palindrome()}));
    assert_eq!(
        r["multipista"],
        json!({"tapes": 1, "applicable": false, "is_multipista": true, "offending": []})
    );
    assert_eq!(r["catedra_warnings"], json!([]));

    let r = ok(json!({"op": "analyze", "document": {
        "type": "turing", "initial": "p", "finals": ["q"],
        "transitions": [["p", "a,a,D", "q"], ["q", "a,a,D", "p"]]}}));
    assert_eq!(
        r["catedra_warnings"],
        json!([
            "El dominio no incluye a F: el estado final q tiene la transición q --(a,a,D)--> p."
        ])
    );

    let r = ok(json!({"op": "analyze", "document": {
        "type": "turing", "initial": "p", "finals": ["q"],
        "transitions": [["p", "a;a,R|b;b,R", "q"], ["q", "a;a,R|b;b,S", "p"]]}}));
    assert_eq!(
        r["multipista"],
        json!({"tapes": 2, "applicable": true, "is_multipista": false, "offending": [1]})
    );
    assert_eq!(r["catedra_warnings"], json!([]));
}

#[test]
fn errors_have_precise_codes() {
    let e = err(json!({"op": "catedra_run", "input": "a", "document": {
        "type": "turing", "initial": "p", "transitions": [["p", "a;a,R|a;a,R", "p"]]}}));
    assert_eq!(e["code"], "unsupported_option");

    let e =
        err(json!({"op": "catedra_run", "input": "a", "model": "pda", "document": palindrome()}));
    assert_eq!(e["code"], "invalid_argument");
    assert!(e["message"].as_str().unwrap().contains("mt"));

    let e = err(json!({"op": "catedra_run", "document": palindrome()}));
    assert_eq!(e["code"], "invalid_argument");
    let e =
        err(json!({"op": "catedra_run", "input": "a", "inputs": ["b"], "document": palindrome()}));
    assert_eq!(e["code"], "invalid_argument");
    let e = err(
        json!({"op": "catedra_run", "input": "a", "max_steps": 100000000, "document": palindrome()}),
    );
    assert_eq!(e["code"], "limit_exceeded");

    let e = err(json!({"op": "catedra_run", "input": "a", "document": abb_nfa()}));
    assert_eq!(e["code"], "wrong_document_type");

    let e = err(json!({"op": "catedra_run", "input": "a", "document":
        {"type": "turing", "transitions": [["p", "a,a,D", "p"]]}}));
    assert_eq!(e["code"], "no_initial_state");

    let e = err(json!({"op": "catedra_table", "document":
        {"type": "turing", "states": ["p"], "initial": "p", "transitions": [["p", "a,a,D", "z"]]}}));
    assert_eq!(e["code"], "unknown_state");

    let e = err(json!({"op": "catedra_table", "document":
        {"type": "turing", "initial": "p", "transitions": [["p", "(a,b)", "p"]]}}));
    assert_eq!(e["code"], "invalid_label");

    let e = err(json!({"op": "all_check", "inputs": ["a⟩"], "document": igual()}));
    assert_eq!(e["code"], "invalid_argument");

    let e = err(json!({"op": "compose_tm", "a": palindrome(), "b": abb_nfa()}));
    assert_eq!(e["code"], "wrong_document_type");
    let e = err(json!({"op": "compose_tm", "a": palindrome(), "b":
        {"type": "turing", "initial": "p", "transitions": [["p", "a;a,R|a;a,R", "p"]]}}));
    assert_eq!(e["code"], "invalid_argument");

    let e =
        err(json!({"op": "membership_tk", "grammar": "S -> aAb\naA -> a\nA -> c", "input": "ab"}));
    assert_eq!(e["code"], "contracting_grammar");
    let e = err(json!({"op": "membership_tk", "grammar": abb_nfa(), "input": "ab"}));
    assert_eq!(e["code"], "wrong_document_type");
    let e = err(
        json!({"op": "membership_tk", "grammar": "S -> SS | a | b", "input": "abababababab", "max_forms": 50}),
    );
    assert_eq!(e["code"], "limit_exceeded");
}

#[test]
fn capabilities_documents_the_catedra_operations() {
    let r = ok(json!({"op": "capabilities"}));
    let ops = r["ops"].as_array().unwrap();
    for name in [
        "catedra_definition",
        "catedra_table",
        "catedra_run",
        "all_check",
        "compose_tm",
        "membership_tk",
    ] {
        let entry = ops
            .iter()
            .find(|o| o["op"] == name)
            .unwrap_or_else(|| panic!("{name} is not in capabilities"));
        assert_eq!(entry["example"]["op"], name);
        assert!(!entry["summary"].as_str().unwrap().is_empty());
        // the example runs
        ok(entry["example"].clone());
    }
    assert!(
        r["error_codes"]
            .as_array()
            .unwrap()
            .contains(&json!("contracting_grammar"))
    );
}
