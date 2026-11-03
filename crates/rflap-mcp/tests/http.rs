//! Streamable HTTP transport over a real socket.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread;

use rflap_mcp::{Config, HttpOptions, HttpServer};
use serde_json::{Value, json};

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|e| panic!("{:?}: {e}", self.body))
    }
}

fn request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> Reply {
    let mut stream = TcpStream::connect(addr).unwrap();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (k, v) in headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let raw = String::from_utf8_lossy(&raw).to_string();
    let (head, body) = raw.split_once("\r\n\r\n").expect("a complete response");
    let mut lines = head.lines();
    let status = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();
    let chunked = headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("transfer-encoding") && v.eq_ignore_ascii_case("chunked")
    });
    Reply {
        status,
        headers,
        body: if chunked {
            dechunk(body)
        } else {
            body.to_string()
        },
    }
}

/// The body of a `Transfer-Encoding: chunked` response (the server chunks big answers).
fn dechunk(mut rest: &str) -> String {
    let mut out = String::new();
    while let Some((size, tail)) = rest.split_once(
        "
",
    ) {
        let size = usize::from_str_radix(size.trim(), 16).expect("a chunk size");
        if size == 0 {
            break;
        }
        // the size counts bytes, so cut on bytes and rejoin
        let bytes = tail.as_bytes();
        out.push_str(&String::from_utf8_lossy(&bytes[..size]));
        rest = &tail[size + 2..];
    }
    out
}

fn start(allow_origins: Vec<String>) -> SocketAddr {
    let server = HttpServer::bind(
        "127.0.0.1:0",
        HttpOptions { allow_origins },
        Config::default(),
    )
    .expect("binds a free port");
    let addr = server.local_addr();
    thread::spawn(move || server.run());
    addr
}

const JSON: (&str, &str) = ("Content-Type", "application/json");

fn init_body() -> String {
    json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
           "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                      "clientInfo": {"name": "t", "version": "1"}}})
    .to_string()
}

#[test]
fn session_lifecycle() {
    let addr = start(vec![]);

    let init = request(addr, "POST", "/mcp", &[JSON], &init_body());
    assert_eq!(init.status, 200);
    assert!(
        init.header("content-type")
            .unwrap()
            .starts_with("application/json")
    );
    assert_eq!(init.json()["result"]["serverInfo"]["name"], "rflap");
    let session = init
        .header("mcp-session-id")
        .expect("a session id")
        .to_string();
    assert!(session.len() >= 16);

    let list_body = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}).to_string();
    let list = request(
        addr,
        "POST",
        "/mcp",
        &[JSON, ("Mcp-Session-Id", &session)],
        &list_body,
    );
    assert_eq!(list.status, 200);
    assert!(list.json()["result"]["tools"].as_array().unwrap().len() >= 18);

    let note = json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string();
    let accepted = request(
        addr,
        "POST",
        "/mcp",
        &[JSON, ("Mcp-Session-Id", &session)],
        &note,
    );
    assert_eq!(accepted.status, 202);
    assert!(accepted.body.is_empty());

    let missing = request(addr, "POST", "/mcp", &[JSON], &list_body);
    assert_eq!(missing.status, 400);
    let unknown = request(
        addr,
        "POST",
        "/mcp",
        &[JSON, ("Mcp-Session-Id", "nope")],
        &list_body,
    );
    assert_eq!(unknown.status, 404);

    let get = request(addr, "GET", "/mcp", &[("Mcp-Session-Id", &session)], "");
    assert_eq!(get.status, 405);
    assert_eq!(get.header("allow"), Some("POST, DELETE, OPTIONS"));

    let bad_version = request(
        addr,
        "POST",
        "/mcp",
        &[
            JSON,
            ("Mcp-Session-Id", &session),
            ("MCP-Protocol-Version", "1999-01-01"),
        ],
        &list_body,
    );
    assert_eq!(bad_version.status, 400);

    let bad_json = request(addr, "POST", "/mcp", &[JSON], "{nope");
    assert_eq!(bad_json.status, 400);
    assert_eq!(bad_json.json()["error"]["code"], -32700);

    let delete = request(addr, "DELETE", "/mcp", &[("Mcp-Session-Id", &session)], "");
    assert_eq!(delete.status, 200);
    let after = request(
        addr,
        "POST",
        "/mcp",
        &[JSON, ("Mcp-Session-Id", &session)],
        &list_body,
    );
    assert_eq!(after.status, 404);

    assert_eq!(request(addr, "GET", "/elsewhere", &[], "").status, 404);
}

#[test]
fn origin_is_validated() {
    let addr = start(vec!["https://chat.example.com".into()]);
    let body = init_body();

    let evil = request(
        addr,
        "POST",
        "/mcp",
        &[JSON, ("Origin", "https://evil.example")],
        &body,
    );
    assert_eq!(evil.status, 403);
    let local = request(
        addr,
        "POST",
        "/mcp",
        &[JSON, ("Origin", "http://localhost:5173")],
        &body,
    );
    assert_eq!(local.status, 200);
    assert_eq!(
        local.header("access-control-allow-origin"),
        Some("http://localhost:5173")
    );
    let listed = request(
        addr,
        "POST",
        "/mcp",
        &[JSON, ("Origin", "https://chat.example.com")],
        &body,
    );
    assert_eq!(listed.status, 200);
    let none = request(addr, "POST", "/mcp", &[JSON], &body);
    assert_eq!(none.status, 200);

    let preflight = request(
        addr,
        "OPTIONS",
        "/mcp",
        &[
            ("Origin", "https://chat.example.com"),
            ("Access-Control-Request-Method", "POST"),
        ],
        "",
    );
    assert_eq!(preflight.status, 204);
    assert!(
        preflight
            .header("access-control-allow-headers")
            .unwrap()
            .contains("Mcp-Session-Id")
    );
    let refused = request(
        addr,
        "OPTIONS",
        "/mcp",
        &[("Origin", "https://evil.example")],
        "",
    );
    assert_eq!(refused.status, 403);
}

#[test]
fn viewer_html_is_served_at_the_root() {
    let addr = start(vec![]);
    let page = request(addr, "GET", "/", &[], "");
    assert_eq!(page.status, 200);
    assert!(
        page.header("content-type")
            .unwrap()
            .starts_with("text/html")
    );
    assert!(page.body.contains("<html"));
}
