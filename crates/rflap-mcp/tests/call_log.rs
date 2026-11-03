//! One stderr line per `tools/call`, so a host log (Claude Desktop's `mcp-server-rflap.log`)
//! shows which tool ran, with which arguments and how it ended.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};

#[test]
fn the_line_names_the_tool_the_outcome_the_time_the_arguments_and_the_answer() {
    let output = json!({
        "content": [{"type": "text", "text": "\n**Designed DFA** for `ends_with(\"ab\")`: 3 states\n\nmore"}],
        "isError": false,
    });
    let line = rflap_mcp::call_log_line(
        "rflap_design_automaton",
        &json!({"alphabet": "ab", "constraint_text": "ends_with(\"ab\")"}),
        &output,
        Duration::from_millis(4),
    );
    assert_eq!(
        line,
        "rflap-mcp: tools/call rflap_design_automaton ok 4ms \
         args={\"alphabet\":\"ab\",\"constraint_text\":\"ends_with(\\\"ab\\\")\"} \
         -> **Designed DFA** for `ends_with(\"ab\")`: 3 states"
    );
}

#[test]
fn a_failed_call_says_error_and_long_arguments_are_cut() {
    let output =
        json!({"content": [{"type": "text", "text": "unknown state `q9`"}], "isError": true});
    let long = "a".repeat(5000);
    let line = rflap_mcp::call_log_line(
        "rflap_simulate",
        &json!({"input": long}),
        &output,
        Duration::from_millis(12),
    );
    assert!(
        line.starts_with("rflap-mcp: tools/call rflap_simulate error 12ms args={\"input\":\"aaa"),
        "{line}"
    );
    assert!(line.contains("… (+3012 chars)"), "{line}");
    assert!(line.ends_with(" -> unknown state `q9`"), "{line}");
    assert!(!line.contains('\n'));
}

fn run_binary(extra: &[&str]) -> (String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rflap-mcp"))
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    let mut stdin = child.stdin.take().unwrap();
    for line in [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-11-25", "capabilities": {},
                          "clientInfo": {"name": "t", "version": "1"}}}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
               "params": {"name": "rflap_design_automaton",
                          "arguments": {"alphabet": "ab", "constraint_text": "ends_with(\"ab\")"}}}),
    ] {
        writeln!(stdin, "{line}").unwrap();
    }
    drop(stdin);
    let (mut out, mut err) = (String::new(), String::new());
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    assert!(child.wait().unwrap().success());
    for line in out.lines() {
        serde_json::from_str::<Value>(line).unwrap_or_else(|e| panic!("stdout line {line:?}: {e}"));
    }
    assert_eq!(out.lines().count(), 2, "{out}");
    (out, err)
}

#[test]
fn the_binary_logs_calls_to_stderr_and_quiet_turns_it_off() {
    let (_, err) = run_binary(&[]);
    let logged: Vec<&str> = err.lines().filter(|l| l.contains("tools/call")).collect();
    assert_eq!(logged.len(), 1, "{err}");
    assert!(
        logged[0].starts_with("rflap-mcp: tools/call rflap_design_automaton ok "),
        "{err}"
    );
    assert!(logged[0].contains("args={\"alphabet\":\"ab\""), "{err}");

    let (_, err) = run_binary(&["--quiet"]);
    assert!(!err.contains("tools/call"), "{err}");
}
