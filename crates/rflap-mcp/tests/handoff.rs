//! The handoff of 3 Nov 2026 (exercise 2.1, ALL): titles, the ALL model in the generic tools,
//! the output notation, `rflap_capabilities` sections, descriptions and the hang regression.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rflap_mcp::handle_message;
use serde_json::{Value, json};

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

/// The ALL of exercise 2.1 a).
fn all_machine() -> Value {
    json!({
        "type": "turing", "initial": "q0", "finals": ["qf"],
        "transitions": [
            ["q0","(a,a,D)","q1"],["q1","(a,a,D)","q1"],["q1","(b,b,D)","q2"],
            ["q1","(⟩,⟩,I)","q3"],["q2","(b,b,D)","q2"],["q2","(⟩,⟩,I)","q3"],
            ["q3","(a,a,I) / (b,b,I)","q3"],["q3","(⟨,⟨,D)","q4"],["q4","(a,X,D)","q5"],
            ["q4","(Y,Y,D)","q7"],["q5","(a,a,D) / (Y,Y,D)","q5"],["q5","(b,Y,I)","q6"],
            ["q5","(⟩,⟩,I)","qf"],["q6","(a,a,I) / (Y,Y,I)","q6"],["q6","(X,X,D)","q4"],
            ["q7","(Y,Y,D)","q7"],["q7","(b,b,D)","qf"]
        ]
    })
}

/// A plain machine (no ⟨ ⟩) that reads Δ, so it breaks the rules of an ALL.
fn blank_machine() -> Value {
    json!({
        "type": "turing", "initial": "q0", "finals": ["qf"],
        "transitions": [["q0","(a,a,D)","q0"],["q0","(Δ,Δ,D)","qf"]]
    })
}

fn with(mut document: Value, key: &str, value: Value) -> Value {
    document[key] = value;
    document
}

// ---------------------------------------------------------------------------------------------
// S1: the call that "hung" and the hard limits
// ---------------------------------------------------------------------------------------------

/// One `tools/call` over the real stdio binary, with the time it took.
fn stdio_call(name: &str, arguments: Value) -> (Value, Duration) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rflap-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the binary starts");
    let mut stdin = child.stdin.take().unwrap();
    let started = Instant::now();
    let initialize = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                   "clientInfo": {"name": "t", "version": "1"}}});
    let request = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": name, "arguments": arguments}});
    writeln!(stdin, "{initialize}").unwrap();
    writeln!(stdin, "{request}").unwrap();
    drop(stdin);
    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    let elapsed = started.elapsed();
    child.wait().unwrap();
    let answer = out
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|a| a["id"] == 2)
        .unwrap_or_else(|| panic!("no answer to tools/call: {out}"));
    (answer["result"].clone(), elapsed)
}

#[test]
fn s1_1_the_handoff_all_check_answers_in_under_five_seconds_with_the_expected_verdicts() {
    let inputs = [
        "", "a", "aa", "ab", "aab", "abb", "aabb", "aaabb", "abbb", "b", "ba", "aba",
    ];
    let (result, elapsed) = stdio_call(
        "rflap_mt_catedra",
        json!({"action": "all_check", "model": "all", "input_alphabet": "ab",
               "inputs": inputs, "document": all_machine()}),
    );
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(result["structuredContent"]["ok"], true, "{result}");
    let accepted = ["a", "aa", "aab", "abb", "aaabb", "abbb"];
    let runs = result["structuredContent"]["runs"].as_array().unwrap();
    assert_eq!(runs.len(), inputs.len());
    for run in runs {
        let input = run["input"].as_str().unwrap();
        assert_eq!(
            run["accepted"] == true,
            accepted.contains(&input),
            "input `{input}`: {run}"
        );
    }
}

#[test]
fn s1_2_a_loop_inside_the_delimiters_reports_not_halting_and_does_not_claim_a_clean_run() {
    let looping = json!({
        "type": "turing", "initial": "q0", "finals": ["qf"],
        "transitions": [["q0","(a,a,D)","q1"],["q1","(⟩,⟩,I)","q0"],["q0","(⟨,⟨,D)","q0"]]
    });
    let (result, elapsed) = stdio_call(
        "rflap_mt_catedra",
        json!({"action": "all_check", "document": looping, "inputs": ["a"], "max_steps": 20}),
    );
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(md.contains("no se detiene"), "{md}");
    assert!(
        !md.contains("el cabezal nunca sale"),
        "an undecided run must not be reported as a clean check: {md}"
    );
}

#[test]
fn s1_2_an_invalid_document_is_an_error_with_a_message_never_an_empty_text() {
    let (result, _) = stdio_call(
        "rflap_mt_catedra",
        json!({"action": "run", "inputs": ["a"],
               "document": {"type": "turing", "initial": "q0",
                            "transitions": [["q0", "(a,a", "q1"]]}}),
    );
    assert_eq!(result["isError"], true, "{result}");
    assert!(text(&result).contains("invalid_label"), "{result}");
}

// ---------------------------------------------------------------------------------------------
// S2: titles
// ---------------------------------------------------------------------------------------------

#[test]
fn s2_show_puts_the_title_on_the_first_line_and_in_structured_content() {
    for field in ["title", "name"] {
        let result = call(
            "rflap_show",
            json!({"document": with(blank_machine(), field, json!("Ejercicio 2.1 a"))}),
        );
        assert_eq!(result["isError"], false, "{field}: {result}");
        assert_eq!(
            text(&result).lines().next(),
            Some("**Ejercicio 2.1 a**"),
            "{field}"
        );
        assert_eq!(result["structuredContent"]["title"], "Ejercicio 2.1 a");
    }
}

#[test]
fn s2_without_a_title_the_text_and_structured_content_have_none() {
    let result = call("rflap_show", json!({"document": blank_machine()}));
    assert!(!text(&result).starts_with("**"), "{}", text(&result));
    assert!(result["structuredContent"].get("title").is_none());
}

#[test]
fn s2_every_tool_that_takes_a_document_accepts_title_and_name() {
    let titled = with(blank_machine(), "title", json!("Mi máquina"));
    let named = with(blank_machine(), "name", json!("Mi máquina"));
    let calls = [
        ("rflap_simulate", json!({"document": titled, "input": "a"})),
        ("rflap_step", json!({"document": named, "input": "a"})),
        ("rflap_analyze", json!({"document": titled})),
        (
            "rflap_mt_catedra",
            json!({"document": named, "action": "table"}),
        ),
        ("rflap_render", json!({"document": titled})),
    ];
    for (tool, arguments) in calls {
        let result = call(tool, arguments);
        assert_eq!(result["isError"], false, "{tool}: {result}");
        assert_eq!(
            text(&result).lines().next(),
            Some("**Mi máquina**"),
            "{tool}"
        );
        assert_eq!(result["structuredContent"]["title"], "Mi máquina", "{tool}");
    }
}

#[test]
fn s2_a_grammar_document_keeps_its_title_too() {
    let result = call(
        "rflap_grammar",
        json!({"grammar": {"type": "grammar", "text": "S -> aSb | λ", "title": "Gramática 1"}}),
    );
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(text(&result).lines().next(), Some("**Gramática 1**"));
    assert_eq!(result["structuredContent"]["title"], "Gramática 1");
}

// ---------------------------------------------------------------------------------------------
// S6: the ALL model in simulate, step and show
// ---------------------------------------------------------------------------------------------

#[test]
fn s6_simulate_with_model_all_runs_the_course_tape_and_reports_each_input() {
    let result = call(
        "rflap_simulate",
        json!({"document": all_machine(), "model": "all", "inputs": ["aab", "ab"]}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(md.contains("⟨q0aab⟩ ├─ ⟨aq1ab⟩"), "{md}");
    assert!(md.contains("final/acepta"), "{md}");
    assert!(md.contains("bloqueo/rechaza"), "{md}");
    let results = result["structuredContent"]["results"].as_array().unwrap();
    assert_eq!(results[0]["accepted"], true);
    assert_eq!(results[1]["accepted"], false);
    assert_eq!(result["structuredContent"]["model"], "all");
}

#[test]
fn s6_simulate_detects_the_all_by_its_delimiters_when_there_is_no_model() {
    let result = call(
        "rflap_simulate",
        json!({"document": all_machine(), "input": "a"}),
    );
    assert_eq!(result["isError"], false, "{result}");
    assert!(
        text(&result).contains("⟨q0a⟩ ├─ ⟨aq1⟩"),
        "{}",
        text(&result)
    );
}

#[test]
fn s6_simulate_with_model_mt_keeps_the_blank_tape_of_jflap() {
    let result = call(
        "rflap_simulate",
        json!({"document": all_machine(), "model": "mt", "input": "a"}),
    );
    assert_eq!(result["isError"], false, "{result}");
    assert!(!text(&result).contains("⟨q0a⟩"), "{}", text(&result));
}

#[test]
fn s6_every_all_result_warns_about_delta_transitions() {
    let result = call(
        "rflap_simulate",
        json!({"document": blank_machine(), "model": "all", "input": "a"}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(md.contains("⚠️"), "{md}");
    assert!(md.contains("Δ"), "{md}");
}

#[test]
fn s6_step_with_model_all_returns_the_first_configurations() {
    let result = call(
        "rflap_step",
        json!({"document": all_machine(), "model": "all", "input": "ab", "steps": 3}),
    );
    assert_eq!(result["isError"], false, "{result}");
    assert!(
        text(&result).contains("⟨q0ab⟩ ├─ ⟨aq1b⟩ ├─ ⟨abq2⟩ ├─ ⟨aq3b⟩"),
        "{}",
        text(&result)
    );
    assert_eq!(
        result["structuredContent"]["configurations"],
        json!(["⟨q0ab⟩", "⟨aq1b⟩", "⟨abq2⟩", "⟨aq3b⟩"])
    );
}

#[test]
fn s6_show_with_model_all_adds_the_definition_and_the_static_check() {
    let result = call(
        "rflap_show",
        json!({"document": blank_machine(), "model": "all"}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(md.contains("Definición formal (ALL)"), "{md}");
    assert!(md.contains("no es un ALL válido"), "{md}");
    assert_eq!(result["structuredContent"]["model"], "all");
}

#[test]
fn s6_show_detects_the_all_and_does_not_add_the_hint() {
    let result = call("rflap_show", json!({"document": all_machine()}));
    let md = text(&result);
    assert!(md.contains("Definición formal (ALL)"), "{md}");
    assert!(
        md.contains("La máquina cumple las reglas de un ALL"),
        "{md}"
    );
    assert!(!md.contains("rflap_mt_catedra"), "{md}");
}

#[test]
fn s6_show_of_a_plain_machine_says_how_to_check_it_as_an_all() {
    let md = text(&call("rflap_show", json!({"document": blank_machine()})));
    assert!(md.contains("all_check"), "{md}");
    assert!(md.contains("model"), "{md}");
    assert!(!md.contains("Definición formal (ALL)"), "{md}");
}

#[test]
fn s6_an_unknown_model_is_an_error_that_names_the_choices() {
    let result = call(
        "rflap_simulate",
        json!({"document": all_machine(), "model": "xx", "input": "a"}),
    );
    assert_eq!(result["isError"], true, "{result}");
    assert!(text(&result).contains("mt") && text(&result).contains("all"));
}

// ---------------------------------------------------------------------------------------------
// S7: notation
// ---------------------------------------------------------------------------------------------

#[test]
fn s7_course_labels_work_as_input_in_simulate_step_and_show() {
    let doc = blank_machine();
    for (tool, arguments) in [
        ("rflap_simulate", json!({"document": doc, "input": "aa"})),
        (
            "rflap_step",
            json!({"document": doc, "input": "aa", "steps": 2}),
        ),
        ("rflap_show", json!({"document": doc})),
    ] {
        let result = call(tool, arguments);
        assert_eq!(result["isError"], false, "{tool}: {result}");
        assert!(!text(&result).is_empty(), "{tool}");
    }
}

#[test]
fn s7_show_defaults_to_the_course_table_and_jflap_switches_it() {
    let default = call("rflap_show", json!({"document": blank_machine()}));
    assert!(
        text(&default).contains("| →q0 | a | q0 | a | D |"),
        "{}",
        text(&default)
    );
    assert!(default["structuredContent"].get("notation").is_none());

    let jflap = call(
        "rflap_show",
        json!({"document": blank_machine(), "notation": "jflap"}),
    );
    assert_eq!(jflap["isError"], false, "{jflap}");
    let md = text(&jflap);
    assert!(!md.contains("Estado Actual"), "{md}");
    assert!(md.contains('□'), "{md}");
    assert_eq!(jflap["structuredContent"]["notation"], "jflap");
    let svg = jflap["structuredContent"]["svg"].as_str().unwrap();
    assert!(
        svg.contains('□') && !svg.contains('Δ'),
        "the SVG follows the notation"
    );
}

#[test]
fn s7_simulate_with_notation_reports_it_in_structured_content() {
    for notation in ["catedra", "jflap"] {
        let result = call(
            "rflap_simulate",
            json!({"document": blank_machine(), "input": "a", "notation": notation}),
        );
        assert_eq!(result["isError"], false, "{result}");
        assert_eq!(result["structuredContent"]["notation"], notation);
    }
    let jflap = call(
        "rflap_simulate",
        json!({"document": blank_machine(), "input": "a", "notation": "jflap"}),
    );
    assert!(!text(&jflap).contains("Estado Actual"), "{}", text(&jflap));
}

#[test]
fn s7_an_unknown_notation_is_an_error() {
    let result = call(
        "rflap_show",
        json!({"document": blank_machine(), "notation": "latex"}),
    );
    assert_eq!(result["isError"], true, "{result}");
}

// ---------------------------------------------------------------------------------------------
// S9: the summary never says 0 states
// ---------------------------------------------------------------------------------------------

#[test]
fn s9_show_never_says_zero_states_for_the_compact_or_the_full_form() {
    let compact = call("rflap_show", json!({"document": all_machine()}));
    assert_eq!(compact["isError"], false, "{compact}");
    let full = compact["structuredContent"]["document"].clone();
    assert!(full.get("automaton").is_some(), "the full form: {full}");
    let again = call("rflap_show", json!({"document": full}));
    assert_eq!(again["isError"], false, "{again}");
    for result in [&compact, &again] {
        let md = text(result);
        assert!(!md.contains(" 0 states"), "{md}");
        assert!(!md.contains("(0 states"), "{md}");
        assert!(md.contains("Turing machine with 9 states"), "{md}");
        assert!(md.contains("(9 states"), "{md}");
    }
}

// ---------------------------------------------------------------------------------------------
// S10: capabilities
// ---------------------------------------------------------------------------------------------

#[test]
fn s10_an_unknown_section_is_an_error_that_lists_the_valid_ones() {
    let result = call("rflap_capabilities", json!({"section": "bogus"}));
    assert_eq!(result["isError"], true, "{result}");
    let md = text(&result);
    for section in [
        "overview",
        "operations",
        "document_types",
        "convert",
        "layout",
        "all",
    ] {
        assert!(md.contains(section), "{section}: {md}");
    }
}

#[test]
fn s10_the_layout_section_lists_the_algorithms_and_the_compact_fields() {
    let result = call("rflap_capabilities", json!({"section": "layout"}));
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    for needle in ["tree-degree", "two-circle", "points", "title", "layout"] {
        assert!(md.contains(needle), "{needle}: {md}");
    }
}

// ---------------------------------------------------------------------------------------------
// S12 and the docs half of S4: descriptions
// ---------------------------------------------------------------------------------------------

fn tools_list() -> Vec<Value> {
    rpc("tools/list", json!({}))["result"]["tools"]
        .as_array()
        .unwrap()
        .clone()
}

fn tool(name: &str) -> Value {
    tools_list()
        .into_iter()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn description_of(name: &str) -> String {
    tool(name)["description"].as_str().unwrap().to_string()
}

#[test]
fn s12_simulate_step_and_show_point_to_rflap_mt_catedra_for_an_all() {
    for name in ["rflap_simulate", "rflap_step", "rflap_show"] {
        let description = description_of(name);
        for needle in ["rflap_mt_catedra", "ALL", "UTN FRT", "model"] {
            assert!(description.contains(needle), "{name}: {needle}");
        }
    }
}

#[test]
fn s12_mt_catedra_description_starts_with_the_keywords() {
    let description = description_of("rflap_mt_catedra");
    let start: String = description.chars().take(120).collect();
    for keyword in [
        "ALL",
        "autómata linealmente acotado",
        "linear bounded automaton",
    ] {
        assert!(start.contains(keyword), "{keyword}: {start}");
    }
}

#[test]
fn s12_show_documents_layout_points_and_title() {
    let description = description_of("rflap_show");
    for needle in ["layout", "points", "title", "layered", "tree-degree"] {
        assert!(description.contains(needle), "{needle}");
    }
}

#[test]
fn s12_the_model_and_notation_arguments_are_in_the_schemas() {
    for name in ["rflap_simulate", "rflap_step", "rflap_show"] {
        let properties = tool(name)["inputSchema"]["properties"].clone();
        assert_eq!(properties["model"]["enum"], json!(["mt", "all"]), "{name}");
    }
    for name in ["rflap_simulate", "rflap_show"] {
        let properties = tool(name)["inputSchema"]["properties"].clone();
        assert_eq!(
            properties["notation"]["enum"],
            json!(["catedra", "jflap"]),
            "{name}"
        );
    }
}
