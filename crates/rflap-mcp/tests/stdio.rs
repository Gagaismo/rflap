//! stdio transport: the real binary, newline-delimited JSON-RPC, nothing else on stdout.

use std::io::{Read, Write};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

#[test]
fn stdio_answers_requests_and_prints_nothing_else() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rflap-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    let mut stdin = child.stdin.take().unwrap();
    let lines = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                          "clientInfo": {"name": "t", "version": "1"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    ];
    for line in &lines {
        writeln!(stdin, "{line}").unwrap();
    }
    writeln!(stdin, "this is not json").unwrap();
    drop(stdin);

    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "{status}");

    let answers: Vec<Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("stdout line {l:?}: {e}")))
        .collect();
    assert_eq!(
        answers.len(),
        3,
        "exactly initialize, tools/list and the parse error: {out}"
    );
    assert_eq!(answers[0]["id"], 1);
    assert_eq!(answers[0]["result"]["serverInfo"]["name"], "rflap");
    assert_eq!(answers[1]["id"], 2);
    assert!(answers[1]["result"]["tools"].as_array().unwrap().len() >= 18);
    assert_eq!(answers[2]["error"]["code"], -32700);
}
