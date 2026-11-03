//! `handle_message` level: the protocol and every tool, without any transport.

use rflap_mcp::{Config, Server, handle_line, handle_message};
use serde_json::{Value, json};

const EDITOR: &str = "ui://rflap/editor.html";

const TOOLS: [&str; 21] = [
    "rflap_design_automaton",
    "rflap_show",
    "rflap_simulate",
    "rflap_step",
    "rflap_convert",
    "rflap_equivalent",
    "rflap_combine",
    "rflap_analyze",
    "rflap_grammar",
    "rflap_parse",
    "rflap_parse_tables",
    "rflap_render",
    "rflap_open_jff",
    "rflap_save_jff",
    "rflap_lsystem_render",
    "rflap_pumping",
    "rflap_capabilities",
    "rflap_call",
    "rflap_mt_catedra",
    "rflap_compose_tm",
    "rflap_membership",
];

const VISUAL: [&str; 16] = [
    "rflap_design_automaton",
    "rflap_show",
    "rflap_simulate",
    "rflap_step",
    "rflap_convert",
    "rflap_combine",
    "rflap_grammar",
    "rflap_parse",
    "rflap_parse_tables",
    "rflap_render",
    "rflap_open_jff",
    "rflap_lsystem_render",
    "rflap_pumping",
    "rflap_mt_catedra",
    "rflap_compose_tm",
    "rflap_membership",
];

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

fn count(v: &Value) -> usize {
    v.as_array()
        .map(Vec::len)
        .or_else(|| v.as_object().map(|o| o.len()))
        .expect("an array or an object")
}

#[test]
fn initialize_negotiates_the_version_and_returns_instructions() {
    for version in ["2025-11-25", "2025-06-18", "2025-03-26"] {
        let answer = rpc(
            "initialize",
            json!({"protocolVersion": version, "capabilities": {},
                   "clientInfo": {"name": "t", "version": "1"}}),
        );
        assert_eq!(answer["jsonrpc"], "2.0");
        assert_eq!(answer["id"], 1);
        assert_eq!(answer["result"]["protocolVersion"], version);
    }
    let answer = rpc(
        "initialize",
        json!({"protocolVersion": "1999-01-01", "capabilities": {}}),
    );
    let result = &answer["result"];
    assert_eq!(result["protocolVersion"], "2025-11-25");
    assert_eq!(result["serverInfo"]["name"], "rflap");
    assert!(result["capabilities"]["tools"].is_object());
    assert!(result["capabilities"]["resources"].is_object());
    assert!(result["capabilities"]["prompts"].is_object());
    let instructions = result["instructions"].as_str().unwrap();
    assert!(instructions.contains("rflap_design_automaton"));
    assert!(instructions.contains("Sintaxis"));
}

#[test]
fn initialize_without_a_version_is_invalid_params() {
    let answer = rpc("initialize", json!({}));
    assert_eq!(answer["error"]["code"], -32602);
}

#[test]
fn ping_and_notifications() {
    assert_eq!(rpc("ping", json!({}))["result"], json!({}));
    let note = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    assert!(handle_message(&note).is_none());
    // A response from the client is not answered either.
    assert!(handle_message(&json!({"jsonrpc": "2.0", "id": 9, "result": {}})).is_none());
}

#[test]
fn unknown_method_is_a_jsonrpc_error() {
    let answer = rpc("does/not/exist", json!({}));
    assert_eq!(answer["error"]["code"], -32601);
    assert_eq!(answer["id"], 1);
}

#[test]
fn malformed_messages_are_jsonrpc_errors() {
    let parse: Value = serde_json::from_str(&handle_line("{not json").unwrap()).unwrap();
    assert_eq!(parse["error"]["code"], -32700);
    assert_eq!(parse["id"], Value::Null);
    let invalid: Value = serde_json::from_str(&handle_line("42").unwrap()).unwrap();
    assert_eq!(invalid["error"]["code"], -32600);
    let missing = rpc("tools/call", json!({}));
    assert_eq!(missing["error"]["code"], -32602);
}

#[test]
fn batches_answer_only_the_requests() {
    let batch = r#"[
        {"jsonrpc":"2.0","id":1,"method":"ping"},
        {"jsonrpc":"2.0","method":"notifications/initialized"},
        {"jsonrpc":"2.0","id":2,"method":"ping"}]"#;
    let answer: Value = serde_json::from_str(&handle_line(batch).unwrap()).unwrap();
    let answers = answer.as_array().unwrap();
    assert_eq!(answers.len(), 2);
    assert_eq!(answers[1]["id"], 2);
    assert!(handle_line(r#"[{"jsonrpc":"2.0","method":"notifications/initialized"}]"#).is_none());
}

#[test]
fn tools_list_has_every_tool_with_a_valid_schema() {
    let answer = rpc("tools/list", json!({}));
    let tools = answer["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in TOOLS {
        assert!(names.contains(&expected), "missing {expected}: {names:?}");
    }
    assert_eq!(names.len(), TOOLS.len(), "unexpected tools: {names:?}");
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        assert!(name.starts_with("rflap_") && name == name.to_lowercase());
        assert!(
            tool["description"].as_str().unwrap().len() > 40,
            "{name} needs a description"
        );
        let schema = &tool["inputSchema"];
        assert_eq!(schema["type"], "object", "{name}");
        let props = schema["properties"].as_object().expect(name);
        for required in schema["required"].as_array().into_iter().flatten() {
            assert!(props.contains_key(required.as_str().unwrap()), "{name}");
        }
        for (key, prop) in props {
            assert!(
                prop.get("description").is_some(),
                "{name}.{key} has no description"
            );
        }
        if let Some(out) = tool.get("outputSchema") {
            assert_eq!(out["type"], "object", "{name}");
        }
        let visual = VISUAL.contains(&name);
        let meta = &tool["_meta"];
        if visual {
            assert_eq!(meta["ui"]["resourceUri"], EDITOR, "{name}");
            assert_eq!(meta["openai/outputTemplate"], EDITOR, "{name}");
            assert_eq!(meta["openai/widgetAccessible"], true, "{name}");
            assert!(meta["openai/toolInvocation/invoking"].is_string(), "{name}");
            assert!(meta["openai/toolInvocation/invoked"].is_string(), "{name}");
        } else {
            assert!(meta["ui"]["resourceUri"].is_null(), "{name}");
        }
    }
}

#[test]
fn design_ends_with_ab_returns_a_three_state_dfa() {
    let result = call(
        "rflap_design_automaton",
        json!({"alphabet": "ab", "constraint_text": "ends_with(\"ab\")",
               "examples": {"accept": ["ab", "aab"], "reject": ["", "ba", "abb"]}}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let document = &result["structuredContent"]["document"];
    assert_eq!(document["type"], "fa");
    assert_eq!(count(&document["states"]), 3);
    let markdown = text(&result);
    assert!(markdown.contains("| State | a | b |"), "{markdown}");
    assert!(markdown.contains("→ q0"), "{markdown}");
    assert!(markdown.contains("Verification"), "{markdown}");
    assert!(markdown.contains("3 states"), "{markdown}");
    assert_eq!(result["structuredContent"]["verification"]["ok"], true);
}

#[test]
fn design_reports_a_failing_example_and_a_bad_spec() {
    let result = call(
        "rflap_design_automaton",
        json!({"alphabet": "ab", "regex": "ab*",
               "examples": {"accept": ["ab"], "reject": ["abb"]}}),
    );
    assert_eq!(result["isError"], false);
    let markdown = text(&result);
    assert!(markdown.contains("❌"), "{markdown}");
    assert!(markdown.contains("abb"), "{markdown}");

    let none = call("rflap_design_automaton", json!({"alphabet": "ab"}));
    assert_eq!(none["isError"], true);
    assert!(text(&none).contains("exactly one"), "{}", text(&none));
    let two = call(
        "rflap_design_automaton",
        json!({"alphabet": "ab", "regex": "a", "constraint_text": "ends_with(\"a\")"}),
    );
    assert_eq!(two["isError"], true);
}

#[test]
fn design_template_builds_a_pda() {
    let result = call(
        "rflap_design_automaton",
        json!({"alphabet": "ab", "template": {"name": "an_bn", "a": "a", "b": "b"},
               "examples": {"accept": ["", "ab", "aabb"], "reject": ["a", "aab"]}}),
    );
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(result["structuredContent"]["document"]["type"], "pda");
}

#[test]
fn a_bad_document_is_a_tool_error_with_the_api_message() {
    let result = call(
        "rflap_analyze",
        json!({"document": {"type": "fa", "initial": "q0", "states": ["q0"],
                            "transitions": [["q0", "a", "zz"]]}}),
    );
    assert_eq!(result["isError"], true);
    let message = text(&result);
    assert!(message.contains("unknown_state"), "{message}");
    assert!(message.contains("zz"), "{message}");
}

#[test]
fn unknown_tool_is_a_tool_error() {
    let result = call("rflap_nope", json!({}));
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("rflap_nope"));
}

fn nfa() -> Value {
    json!({"type": "fa", "initial": "q0", "finals": ["q2"],
           "transitions": [["q0", "a", "q0"], ["q0", "b", "q0"],
                           ["q0", "a", "q1"], ["q1", "b", "q2"]]})
}

#[test]
fn simulate_one_input_and_a_batch() {
    let one = call("rflap_simulate", json!({"document": nfa(), "input": "aab"}));
    assert_eq!(one["isError"], false);
    assert_eq!(one["structuredContent"]["outcome"], "accepted");
    assert_eq!(one["structuredContent"]["document"]["type"], "fa");
    assert!(text(&one).contains("accepted"));

    let batch = call(
        "rflap_simulate",
        json!({"document": nfa(), "inputs": ["ab", "ba"]}),
    );
    let results = batch["structuredContent"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["accepted"], true);
    assert_eq!(results[1]["accepted"], false);
    assert!(text(&batch).contains("| ba |"), "{}", text(&batch));
}

#[test]
fn step_convert_equivalent_combine_analyze() {
    let step = call(
        "rflap_step",
        json!({"document": nfa(), "input": "aab", "steps": 2}),
    );
    assert_eq!(step["structuredContent"]["steps_done"], 2);

    let convert = call(
        "rflap_convert",
        json!({"document": nfa(), "to": "minimal_dfa"}),
    );
    assert_eq!(convert["isError"], false, "{convert}");
    assert_eq!(convert["structuredContent"]["to"], "minimal_dfa");
    assert!(text(&convert).contains("| State |"), "{}", text(&convert));

    let same = call(
        "rflap_equivalent",
        json!({"a": nfa(), "b": {"type": "regex", "expression": "(a+b)*ab"}}),
    );
    assert_eq!(same["structuredContent"]["equivalent"], true);
    let different = call(
        "rflap_equivalent",
        json!({"a": nfa(), "b": {"type": "regex", "expression": "(a+b)*a"}}),
    );
    assert_eq!(different["structuredContent"]["equivalent"], false);
    assert!(text(&different).contains("counterexample"));

    let combined = call("rflap_combine", json!({"a": nfa(), "b": nfa()}));
    assert_eq!(
        count(&combined["structuredContent"]["document"]["states"]),
        6
    );

    let analysis = call("rflap_analyze", json!({"document": nfa()}));
    assert_eq!(analysis["structuredContent"]["kind"], "nfa");
    assert!(text(&analysis).contains("NFA") || text(&analysis).contains("nfa"));
}

#[test]
fn show_renders_any_document() {
    let machine = call("rflap_show", json!({"document": nfa()}));
    assert_eq!(machine["isError"], false);
    assert!(
        machine["structuredContent"]["svg"]
            .as_str()
            .unwrap()
            .contains("<svg")
    );
    assert_eq!(machine["structuredContent"]["analysis"]["kind"], "nfa");
    assert!(text(&machine).contains("| State |"));

    let grammar = call(
        "rflap_show",
        json!({"document": {"type": "grammar", "text": "S -> aSb | λ"}}),
    );
    assert_eq!(grammar["isError"], false, "{grammar}");
    assert!(text(&grammar).contains("S → aSb"));
}

#[test]
fn grammar_parse_and_tables() {
    let kind = call("rflap_grammar", json!({"grammar": "S -> aSb | λ"}));
    assert_eq!(kind["structuredContent"]["classification"], "context_free");
    let cnf = call(
        "rflap_grammar",
        json!({"grammar": "S -> aSb | λ", "action": "cnf"}),
    );
    assert_eq!(cnf["structuredContent"]["document"]["type"], "grammar");

    let parse = call(
        "rflap_parse",
        json!({"grammar": "S -> aSb | λ", "inputs": ["aabb", "aab"], "parser": "ll1"}),
    );
    assert_eq!(
        parse["structuredContent"]["results"][0]["outcome"],
        "accepted"
    );
    assert!(text(&parse).contains("aabb"));
    assert!(text(&parse).contains("⇒"), "{}", text(&parse));

    let tables = call("rflap_parse_tables", json!({"grammar": "S -> aSb | λ"}));
    let markdown = text(&tables);
    assert!(markdown.contains("FIRST"), "{markdown}");
    assert!(markdown.contains("FOLLOW"), "{markdown}");
    assert!(markdown.contains("LL(1)"), "{markdown}");
    assert!(markdown.contains("SLR(1)"), "{markdown}");
}

#[test]
fn render_formats() {
    let markdown = call(
        "rflap_render",
        json!({"document": nfa(), "format": "markdown"}),
    );
    assert!(text(&markdown).contains("| State | a | b |"));
    let dot = call("rflap_render", json!({"document": nfa(), "format": "dot"}));
    assert!(text(&dot).contains("digraph"));
    let svg = call("rflap_render", json!({"document": nfa()}));
    assert!(
        svg["structuredContent"]["content"]
            .as_str()
            .unwrap()
            .contains("<svg")
    );
}

#[test]
fn jff_round_trip_and_files() {
    let saved = call("rflap_save_jff", json!({"document": nfa()}));
    assert_eq!(saved["isError"], false);
    let xml = saved["structuredContent"]["xml"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(text(&saved).contains("<structure>"));
    let opened = call("rflap_open_jff", json!({"xml": xml}));
    assert_eq!(opened["structuredContent"]["document"]["type"], "fa");

    let dir = std::env::temp_dir().join(format!("rflap-mcp-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("nfa.jff");
    let path_text = path.to_str().unwrap();
    let written = call(
        "rflap_save_jff",
        json!({"document": nfa(), "path": path_text}),
    );
    assert_eq!(written["isError"], false, "{written}");
    assert!(path.exists());
    let again = call(
        "rflap_save_jff",
        json!({"document": nfa(), "path": path_text}),
    );
    assert_eq!(again["isError"], true, "overwrite needs overwrite: true");
    let read = call("rflap_open_jff", json!({"path": path_text}));
    assert_eq!(read["structuredContent"]["document"]["type"], "fa");
    let wrong = call(
        "rflap_save_jff",
        json!({"document": nfa(), "path": dir.join("x.txt").to_str().unwrap()}),
    );
    assert_eq!(wrong["isError"], true);
    std::fs::remove_dir_all(&dir).unwrap();

    let server = Server::new(Config {
        viewer_url: None,
        allow_files: false,
        ..Config::default()
    });
    let answer = server
        .handle_message(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "rflap_open_jff", "arguments": {"path": "/etc/passwd"}}}))
        .unwrap();
    assert_eq!(answer["result"]["isError"], true);
}

#[test]
fn lsystem_pumping_capabilities_and_raw_call() {
    let l = call(
        "rflap_lsystem_render",
        json!({"document": {"type": "lsystem", "axiom": "F", "rules": ["F -> F+F-F-F+F"],
                            "parameters": {"angle": "90", "distance": "4"}}, "depth": 2}),
    );
    assert_eq!(l["isError"], false, "{l}");
    assert!(
        l["structuredContent"]["svg"]
            .as_str()
            .unwrap()
            .contains("<svg")
    );

    let languages = call("rflap_pumping", json!({"action": "languages"}));
    assert!(languages["structuredContent"]["languages"].is_array());
    let game = call(
        "rflap_pumping",
        json!({"action": "new", "kind": "regular", "language": 0, "mode": "computer_first"}),
    );
    assert_eq!(game["isError"], false, "{game}");
    let game_doc = game["structuredContent"]["document"].clone();
    let next = call(
        "rflap_pumping",
        json!({"action": "move", "game": game_doc, "move": {"move": "choose_w", "w": "aabb"}}),
    );
    assert_eq!(next["isError"], false, "{next}");

    let caps = call("rflap_capabilities", json!({}));
    assert!(caps["structuredContent"]["ops"].is_array());
    assert!(text(&caps).contains("design"));

    let raw = call(
        "rflap_call",
        json!({"request": {"op": "simulate", "document": nfa(), "input": "ab"}}),
    );
    assert_eq!(raw["structuredContent"]["ok"], true, "{raw}");
    assert_eq!(raw["structuredContent"]["result"]["outcome"], "accepted");
    let bad = call("rflap_call", json!({"request": {"op": "nope"}}));
    assert_eq!(bad["structuredContent"]["ok"], false, "{bad}");
    assert_eq!(bad["structuredContent"]["error"]["code"], "unknown_op");
    assert!(text(&bad).contains("unknown_op"));
}

/// The contract of `web/README.md` ("The engine"): the widget sends `{request: "<json>"}` and
/// expects the `rflap_api::call` envelope as `structuredContent`.
#[test]
fn rflap_call_answers_the_widget_fallback_contract() {
    let request = json!({"op": "simulate", "document": nfa(), "input": "ab"}).to_string();
    let answer = call("rflap_call", json!({"request": request}));
    assert_eq!(answer["isError"], false, "{answer}");
    let envelope = &answer["structuredContent"];
    assert_eq!(envelope["ok"], true, "{answer}");
    assert_eq!(envelope["result"]["outcome"], "accepted");
    // The same envelope is also readable as JSON text content (after the markdown fence).
    let shown = text(&answer);
    let json_text = shown
        .trim()
        .trim_start_matches("```json")
        .trim_end_matches("```")
        .trim();
    assert_eq!(serde_json::from_str::<Value>(json_text).unwrap(), *envelope);

    let bad = call("rflap_call", json!({"request": "{\"op\": \"nope\"}"}));
    assert_eq!(bad["structuredContent"]["ok"], false, "{bad}");
    assert_eq!(bad["structuredContent"]["error"]["code"], "unknown_op");

    let garbage = call("rflap_call", json!({"request": "not json"}));
    assert_eq!(garbage["structuredContent"]["ok"], false, "{garbage}");
}

#[test]
fn rflap_call_is_callable_by_the_app() {
    let list = rpc("tools/list", json!({}));
    let tool = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "rflap_call")
        .unwrap()
        .clone();
    let visibility = tool["_meta"]["ui"]["visibility"].as_array().unwrap();
    assert!(visibility.contains(&json!("app")), "{tool}");
    assert!(tool["_meta"]["ui"]["resourceUri"].is_null(), "{tool}");
}

#[test]
fn visual_results_carry_a_viewer_link_when_configured() {
    let server = Server::new(Config {
        viewer_url: Some("http://127.0.0.1:9/".into()),
        allow_files: true,
        ..Config::default()
    });
    let answer = server
        .handle_message(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "rflap_show", "arguments": {"document": nfa()}}}))
        .unwrap();
    let markdown = answer["result"]["content"][0]["text"].as_str().unwrap();
    assert!(markdown.contains("http://127.0.0.1:9/#doc="), "{markdown}");
    let without = text(&call("rflap_show", json!({"document": nfa()})));
    assert!(!without.contains("#doc="));
}

#[test]
fn editor_resource() {
    let list = rpc("resources/list", json!({}));
    assert_eq!(list["result"]["resources"][0]["uri"], EDITOR);
    let read = rpc("resources/read", json!({"uri": EDITOR}));
    let contents = &read["result"]["contents"][0];
    assert_eq!(contents["uri"], EDITOR);
    assert_eq!(contents["mimeType"], "text/html;profile=mcp-app");
    assert!(contents["text"].as_str().unwrap().contains("<html"));
    assert_eq!(contents["_meta"]["ui"]["prefersBorder"], true);
    assert_eq!(contents["_meta"]["ui"]["csp"]["connectDomains"], json!([]));
    assert_eq!(contents["_meta"]["ui"]["csp"]["resourceDomains"], json!([]));
    let missing = rpc("resources/read", json!({"uri": "ui://rflap/other.html"}));
    assert_eq!(missing["error"]["code"], -32002);
}

#[test]
fn prompts() {
    let list = rpc("prompts/list", json!({}));
    let prompts = list["result"]["prompts"].as_array().unwrap();
    assert!(prompts.iter().any(|p| p["name"] == "ssl_exercise"));
    let got = rpc(
        "prompts/get",
        json!({"name": "ssl_exercise", "arguments": {"exercise": "Cadenas sobre {a,b} que terminan en ab"}}),
    );
    let message = &got["result"]["messages"][0];
    assert_eq!(message["role"], "user");
    let body = message["content"]["text"].as_str().unwrap();
    assert!(body.contains("terminan en ab"));
    assert!(body.contains("rflap_design_automaton"));
    assert_eq!(
        rpc("prompts/get", json!({"name": "ssl_exercise"}))["error"]["code"],
        -32602
    );
    assert_eq!(
        rpc("prompts/get", json!({"name": "nope"}))["error"]["code"],
        -32602
    );
}

#[test]
fn the_tool_reference_in_docs_is_up_to_date() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/mcp-tools.md");
    let on_disk = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        on_disk,
        rflap_mcp::tools_markdown(),
        "regenerate with: cargo run -p rflap-mcp -- --tools-markdown > docs/mcp-tools.md"
    );
}
