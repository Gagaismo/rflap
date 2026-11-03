//! The notation of the course "Sintaxis y Semántica de los Lenguajes" through the MCP tools.

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
fn the_three_tools_are_listed_with_widget_meta_and_examples() {
    let answer = rpc("tools/list", json!({}));
    let tools = answer["result"]["tools"].as_array().unwrap();
    for name in ["rflap_mt_catedra", "rflap_compose_tm", "rflap_membership"] {
        let tool = tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        assert_eq!(tool["_meta"]["ui"]["resourceUri"], "ui://rflap/editor.html");
        assert_eq!(tool["_meta"]["openai/widgetAccessible"], true);
        assert!(tool["description"].as_str().unwrap().contains("Example"));
    }
    let schema = &tools
        .iter()
        .find(|t| t["name"] == "rflap_mt_catedra")
        .unwrap()["inputSchema"];
    assert_eq!(schema["required"], json!(["document", "action"]));
    assert_eq!(
        schema["properties"]["action"]["enum"],
        json!(["definition", "table", "run", "all_check"])
    );
    assert_eq!(schema["properties"]["model"]["enum"], json!(["mt", "all"]));
    for key in ["inputs", "max_steps", "input_alphabet"] {
        assert!(schema["properties"][key].is_object(), "{key}");
    }
}

#[test]
fn run_prints_the_configuration_sequences_of_the_palindrome_machine() {
    let result = call(
        "rflap_mt_catedra",
        json!({"document": palindrome(), "action": "run", "inputs": ["aba", "ab"]}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(
        md.contains("Δ1abaΔ ├─ ΔΔ2baΔ ├─ ΔΔb2aΔ ├─ ΔΔba2Δ ├─ ΔΔb3aΔ"),
        "{md}"
    );
    assert!(md.contains("ΔΔΔ8ΔΔ final/acepta"), "{md}");
    assert!(
        md.contains("Δ1abΔ ├─ ΔΔ2bΔ ├─ ΔΔb2Δ ├─ ΔΔ3bΔ bloqueo/rechazo"),
        "{md}"
    );
    assert!(md.contains("```"), "{md}");
    let structured = &result["structuredContent"];
    assert_eq!(structured["tool"], "rflap_mt_catedra");
    assert_eq!(structured["document"]["type"], "turing");
    assert_eq!(structured["results"].as_array().unwrap().len(), 2);
    assert_eq!(structured["results"][0]["accepted"], true);
}

#[test]
fn definition_prints_the_six_tuple_and_the_sets() {
    let result = call(
        "rflap_mt_catedra",
        json!({"document": palindrome(), "action": "definition"}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(md.contains("MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩"), "{md}");
    assert!(md.contains("Q = {1, 2, 3, 4, 5, 6, 7, 8}"), "{md}");
    assert!(md.contains("Σ_E = {a, b} (inferido)"), "{md}");
    assert!(md.contains("Σ_A = {Δ}"), "{md}");
    assert!(md.contains("q0 = 1"), "{md}");
    assert!(md.contains("F = {8}"), "{md}");
    assert_eq!(result["structuredContent"]["initial"], "1");
    assert_eq!(result["structuredContent"]["document"]["type"], "turing");
}

#[test]
fn input_alphabet_declares_sigma_e() {
    let result = call(
        "rflap_mt_catedra",
        json!({"document": palindrome(), "action": "definition", "input_alphabet": "ab"}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(md.contains("Σ_E = {a, b}"), "{md}");
    assert!(!md.contains("(inferido)"), "{md}");
}

#[test]
fn table_has_the_course_header() {
    let result = call(
        "rflap_mt_catedra",
        json!({"document": palindrome(), "action": "table"}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(
        md.contains(
            "| Estado Actual | Símbolo Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal |"
        ),
        "{md}"
    );
    assert!(md.contains("| →1 | a | 2 | Δ | D |"), "{md}");
    assert!(md.contains("| *8 | - | - | - | - |"), "{md}");
}

#[test]
fn all_check_passes_on_igual_and_lists_the_violations_of_a_broken_all() {
    let good = call(
        "rflap_mt_catedra",
        json!({"document": igual(), "action": "all_check", "inputs": ["ab", "b"]}),
    );
    assert_eq!(good["isError"], false, "{good}");
    let md = text(&good);
    assert!(md.contains("cumple"), "{md}");
    assert!(md.contains("⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza"), "{md}");
    assert_eq!(good["structuredContent"]["ok"], true);

    let broken = json!({"type": "turing", "states": ["I", "D", "F"], "initial": "I",
        "finals": ["F"],
        "transitions": [["I", "(b,b,D)", "I"], ["I", "(⟩,X,I)", "D"], ["D", "(⟨,⟨,D)", "F"]]});
    let bad = call(
        "rflap_mt_catedra",
        json!({"document": broken, "action": "all_check", "inputs": ["b"]}),
    );
    assert_eq!(bad["isError"], false, "{bad}");
    let md = text(&bad);
    assert!(
        md.contains(
            "La transición I --(⟩,X,I)--> D lee ⟩: debe escribir ⟩ y moverse a la izquierda (I)."
        ),
        "{md}"
    );
    assert!(md.contains("escribe X sobre el delimitador ⟩"), "{md}");
    assert_eq!(bad["structuredContent"]["ok"], false);
}

#[test]
fn run_of_an_all_uses_the_delimiters() {
    let result = call(
        "rflap_mt_catedra",
        json!({"document": igual(), "action": "run", "model": "all", "inputs": ["ab"]}),
    );
    let md = text(&result);
    assert!(md.contains("⟨Iab⟩ ├─ A⟨Xb⟩ ├─ ⟨BXb⟩"), "{md}");
    assert!(md.contains("⟨FXX⟩ final/acepta"), "{md}");
}

#[test]
fn bad_calls_are_errors() {
    let result = call(
        "rflap_mt_catedra",
        json!({"document": palindrome(), "action": "run"}),
    );
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("inputs"), "{}", text(&result));
    let result = call(
        "rflap_mt_catedra",
        json!({"document": palindrome(), "action": "run", "inputs": ["a"], "model": "xx"}),
    );
    assert_eq!(result["isError"], true);
    let result = call(
        "rflap_mt_catedra",
        json!({"document": {"type": "fa", "initial": "q"}, "action": "table"}),
    );
    assert_eq!(result["isError"], true);
}

#[test]
fn membership_reproduces_the_sets_t0_to_t4() {
    let grammar = "S -> T\nDB -> BD\nD -> c\nT -> aTBD\nT -> abD\nbB -> bb";
    let result = call(
        "rflap_membership",
        json!({"grammar": grammar, "input": "abbc"}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    for line in [
        "T0 = { S }",
        "T1 = { S, T }",
        "T2 = { S, T, aTBD, abD }",
        "T3 = { S, T, aTBD, abD, aTBc, abc }",
        "T4 = T3",
    ] {
        assert!(md.contains(line), "{line} in {md}");
    }
    assert!(md.contains("\"abbc\" no pertenece"), "{md}");
    assert_eq!(result["structuredContent"]["member"], false);
    assert_eq!(result["structuredContent"]["document"]["type"], "grammar");
    let yes = call(
        "rflap_membership",
        json!({"grammar": grammar, "input": "abc"}),
    );
    assert!(text(&yes).contains("pertenece"), "{}", text(&yes));
    let contracting = call(
        "rflap_membership",
        json!({"grammar": "S -> ab\nab -> b", "input": "ab"}),
    );
    assert_eq!(contracting["isError"], true);
}

#[test]
fn compose_chains_two_machines() {
    let a = json!({"type": "turing", "initial": "s0", "finals": ["s1"],
        "transitions": [["s0", "(a,a,D)", "s1"]]});
    let b = json!({"type": "turing", "initial": "s0", "finals": ["s1"],
        "transitions": [["s0", "(b,b,D)", "s1"]]});
    let result = call("rflap_compose_tm", json!({"a": a, "b": b}));
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(md.contains("MT3 = ⟨Q1∪Q2, Σ_E, Σ_A, q01, F2, f3⟩"), "{md}");
    assert!(md.contains("F = {s1'}"), "{md}");
    assert!(md.contains("| Estado Actual |"), "{md}");
    assert_eq!(result["structuredContent"]["document"]["type"], "turing");
}

#[test]
fn simulate_a_single_tape_machine_adds_the_catedra_table_and_sequences() {
    let result = call(
        "rflap_simulate",
        json!({"document": palindrome(), "input": "ab"}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let md = text(&result);
    assert!(
        md.contains(
            "| Estado Actual | Símbolo Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal |"
        ),
        "{md}"
    );
    assert!(
        md.contains("Δ1abΔ ├─ ΔΔ2bΔ ├─ ΔΔb2Δ ├─ ΔΔ3bΔ bloqueo/rechazo"),
        "{md}"
    );
    let batch = call(
        "rflap_simulate",
        json!({"document": palindrome(), "inputs": ["aba", "ab"]}),
    );
    let md = text(&batch);
    assert!(md.contains("ΔΔΔ8ΔΔ final/acepta"), "{md}");
    assert!(md.contains("bloqueo/rechazo"), "{md}");
}

#[test]
fn show_and_analyze_add_the_catedra_table() {
    for tool in ["rflap_show", "rflap_analyze"] {
        let result = call(tool, json!({"document": palindrome()}));
        assert_eq!(result["isError"], false, "{result}");
        let md = text(&result);
        assert!(md.contains("| →1 | a | 2 | Δ | D |"), "{tool}: {md}");
    }
    let fa = call(
        "rflap_analyze",
        json!({"document": {"type": "fa", "initial": "q", "transitions": [["q", "a", "q"]]}}),
    );
    assert!(!text(&fa).contains("Estado Actual"));
}

#[test]
fn analyze_reports_multipista_for_a_multitape_machine() {
    let together = json!({"type": "turing", "initial": "p", "tapes": 2,
        "transitions": [["p", "a;a,R|_;a,R", "p"]]});
    let md = text(&call("rflap_analyze", json!({"document": together})));
    assert!(md.contains("- multipista: sí"), "{md}");
    assert!(!md.contains("Estado Actual"), "{md}");
    let apart = json!({"type": "turing", "initial": "p", "tapes": 2,
        "transitions": [["p", "a;a,R|_;a,L", "p"]]});
    let md = text(&call("rflap_analyze", json!({"document": apart})));
    assert!(md.contains("- multipista: no"), "{md}");
    assert!(md.contains("[0]"), "{md}");
}

#[test]
fn instructions_teach_the_catedra_conventions() {
    let init = rpc(
        "initialize",
        json!({"protocolVersion": "2025-06-18", "capabilities": {},
               "clientInfo": {"name": "t", "version": "0"}}),
    );
    let instructions = init["result"]["instructions"].as_str().unwrap();
    for needle in [
        "rflap_mt_catedra",
        "all_check",
        "(e,s,m)",
        "Δ",
        "bloqueo",
        "MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩",
        "rflap_membership",
        "rflap_compose_tm",
    ] {
        assert!(instructions.contains(needle), "{needle}");
    }
}

#[test]
fn the_mt_catedra_prompt_and_the_updated_ssl_exercise() {
    let list = rpc("prompts/list", json!({}));
    let prompts = list["result"]["prompts"].as_array().unwrap();
    let prompt = prompts.iter().find(|p| p["name"] == "mt_catedra").unwrap();
    assert_eq!(prompt["arguments"][0]["name"], "enunciado");
    assert_eq!(prompt["arguments"][0]["required"], true);
    let got = rpc(
        "prompts/get",
        json!({"name": "mt_catedra", "arguments": {"enunciado": "MT que reconoce palíndromos"}}),
    );
    let body = got["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(body.contains("palíndromos"));
    assert!(body.contains("rflap_mt_catedra"));
    assert!(body.contains("all_check"));
    assert_eq!(
        rpc("prompts/get", json!({"name": "mt_catedra"}))["error"]["code"],
        -32602
    );
    let ssl = rpc(
        "prompts/get",
        json!({"name": "ssl_exercise", "arguments": {"exercise": "x"}}),
    );
    let body = ssl["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(body.contains("rflap_mt_catedra"));
}
