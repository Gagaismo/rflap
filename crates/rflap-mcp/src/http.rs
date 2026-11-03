//! The Streamable HTTP transport (MCP 2025-03-26 and later) on a single `/mcp` endpoint.
//!
//! POST carries one JSON-RPC message (or a batch) and is answered with `application/json`;
//! notifications and responses are answered `202`. GET is `405` (this server never opens a
//! server-to-client stream). `Mcp-Session-Id` is issued on `initialize` and required after.
//! `Origin` is validated against DNS rebinding: absent is fine (not a browser), loopback origins
//! are always fine, anything else must be in the allow list.

use std::collections::VecDeque;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{self, Read};
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use tiny_http::{Header, Method, Request, Response, ResponseBox};

use crate::{Config, SUPPORTED_VERSIONS, Server, viewer};

const MAX_BODY: u64 = 8 * 1024 * 1024;
const MAX_SESSIONS: usize = 1000;
const SESSION_HEADER: &str = "Mcp-Session-Id";
const ENDPOINT: &str = "/mcp";

/// Settings of the HTTP transport.
#[derive(Clone, Debug, Default)]
pub struct HttpOptions {
    /// Origins allowed besides the loopback ones (`https://chatgpt.com`); `*` allows all.
    pub allow_origins: Vec<String>,
}

/// A bound, not yet running, HTTP server.
pub struct HttpServer {
    listener: TcpListener,
    options: HttpOptions,
    server: Server,
}

struct State {
    server: Server,
    options: HttpOptions,
    sessions: Mutex<VecDeque<String>>,
    counter: AtomicU64,
}

/// Binds `addr` and serves forever.
pub fn serve_http(addr: &str, options: HttpOptions, config: Config) -> io::Result<()> {
    HttpServer::bind(addr, options, config)?.run();
    Ok(())
}

impl HttpServer {
    /// Binds `addr` (`127.0.0.1:0` picks a free port). Without `viewer_url`, the widget served
    /// at `/` of this same server becomes the viewer.
    pub fn bind(addr: &str, options: HttpOptions, mut config: Config) -> io::Result<HttpServer> {
        let listener = TcpListener::bind(addr)?;
        if config.viewer_url.is_none() {
            let local = listener.local_addr()?;
            let host = match local.ip() {
                ip if ip.is_unspecified() || ip.is_loopback() => IpAddr::from([127, 0, 0, 1]),
                ip => ip,
            };
            config.viewer_url = Some(format!("http://{}/", SocketAddr::new(host, local.port())));
        }
        Ok(HttpServer {
            listener,
            options,
            server: Server::new(config),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("a bound listener has an address")
    }

    /// The URL of the MCP endpoint, for the startup message.
    pub fn endpoint_url(&self) -> String {
        format!("http://{}{ENDPOINT}", self.local_addr())
    }

    /// The viewer URL links point to.
    pub fn viewer_url(&self) -> Option<&str> {
        self.server.config().viewer_url.as_deref()
    }

    /// Serves requests, one thread each, until the process ends.
    pub fn run(self) {
        let http = match tiny_http::Server::from_listener(self.listener, None) {
            Ok(http) => http,
            Err(e) => {
                eprintln!("rflap-mcp: cannot serve HTTP: {e}");
                return;
            }
        };
        let state = Arc::new(State {
            server: self.server,
            options: self.options,
            sessions: Mutex::new(VecDeque::new()),
            counter: AtomicU64::new(0),
        });
        for request in http.incoming_requests() {
            let state = Arc::clone(&state);
            thread::spawn(move || handle(&state, request));
        }
    }
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("a valid header")
}

fn request_header(request: &Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str().to_string())
}

fn plain(status: u16, body: &str) -> ResponseBox {
    Response::from_string(body).with_status_code(status).boxed()
}

fn empty(status: u16) -> ResponseBox {
    Response::from_string("").with_status_code(status).boxed()
}

fn json_body(status: u16, body: &Value) -> ResponseBox {
    Response::from_string(body.to_string())
        .with_status_code(status)
        .with_header(header("Content-Type", "application/json"))
        .boxed()
}

/// The loopback origins (`http://localhost:5173`, `http://127.0.0.1`, `http://[::1]:3000`).
fn is_loopback_origin(origin: &str) -> bool {
    let Some(rest) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };
    if rest.contains('/') {
        return false;
    }
    let host = if rest.starts_with('[') {
        rest.split_once(']')
            .map_or(rest, |(h, _)| &rest[..=h.len()])
    } else {
        rest.split(':').next().unwrap_or(rest)
    };
    matches!(host, "localhost" | "127.0.0.1" | "[::1]")
}

fn origin_allowed(origin: &str, options: &HttpOptions) -> bool {
    is_loopback_origin(origin)
        || options.allow_origins.iter().any(|allowed| {
            allowed == "*" || allowed.trim_end_matches('/').eq_ignore_ascii_case(origin)
        })
}

fn new_session_id(counter: &AtomicU64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let n = counter.fetch_add(1, Ordering::Relaxed);
    // `RandomState` is seeded by the OS per instance: unguessable ids without a dependency.
    (0..2)
        .map(|round| {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(n);
            hasher.write_u64(round);
            hasher.write_u128(nanos);
            format!("{:016x}", hasher.finish())
        })
        .collect()
}

fn handle(state: &State, mut request: Request) {
    let origin = request_header(&request, "Origin");
    if let Some(origin) = &origin
        && !origin_allowed(origin, &state.options)
    {
        let _ = request.respond(plain(403, "forbidden origin"));
        return;
    }
    let mut response = route(state, &mut request);
    if let Some(origin) = origin {
        response = with_cors(response, &origin);
    }
    let _ = request.respond(response);
}

fn with_cors(response: ResponseBox, origin: &str) -> ResponseBox {
    // `ResponseBox` is built by value; headers are added by rebuilding through the trait object.
    let mut response = response;
    for h in [
        header("Access-Control-Allow-Origin", origin),
        header("Vary", "Origin"),
        header("Access-Control-Expose-Headers", SESSION_HEADER),
    ] {
        response.add_header(h);
    }
    response
}

fn route(state: &State, request: &mut Request) -> ResponseBox {
    let path = request
        .url()
        .split(['?', '#'])
        .next()
        .unwrap_or("/")
        .to_string();
    let method = request.method().clone();
    if path == "/" || path == "/index.html" {
        return match method {
            Method::Get | Method::Head => viewer::html_response(),
            _ => plain(405, "method not allowed"),
        };
    }
    if path != ENDPOINT {
        return plain(404, "not found");
    }
    match method {
        Method::Post => post(state, request),
        Method::Delete => delete(state, request),
        Method::Options => {
            let mut response = empty(204);
            for h in [
                header("Access-Control-Allow-Methods", "POST, DELETE, OPTIONS"),
                header(
                    "Access-Control-Allow-Headers",
                    "Content-Type, Accept, Mcp-Session-Id, MCP-Protocol-Version, Authorization",
                ),
                header("Access-Control-Max-Age", "600"),
            ] {
                response.add_header(h);
            }
            response
        }
        _ => {
            let mut response = plain(
                405,
                "this server has no server-to-client stream; POST JSON-RPC messages",
            );
            response.add_header(header("Allow", "POST, DELETE, OPTIONS"));
            response
        }
    }
}

/// `None` when the session header is valid; otherwise the error response.
fn check_session(state: &State, request: &Request) -> Option<ResponseBox> {
    let Some(id) = request_header(request, SESSION_HEADER) else {
        return Some(plain(
            400,
            "missing Mcp-Session-Id header; send initialize first",
        ));
    };
    let known = state.sessions.lock().is_ok_and(|s| s.contains(&id));
    (!known).then(|| plain(404, "unknown session; send initialize again"))
}

fn delete(state: &State, request: &Request) -> ResponseBox {
    if let Some(error) = check_session(state, request) {
        return error;
    }
    if let (Some(id), Ok(mut sessions)) = (
        request_header(request, SESSION_HEADER),
        state.sessions.lock(),
    ) {
        sessions.retain(|s| *s != id);
    }
    empty(200)
}

fn post(state: &State, request: &mut Request) -> ResponseBox {
    let mut body = Vec::new();
    if request
        .as_reader()
        .take(MAX_BODY + 1)
        .read_to_end(&mut body)
        .is_err()
    {
        return plain(400, "unreadable body");
    }
    if body.len() as u64 > MAX_BODY {
        return plain(413, "body too large");
    }
    let parsed: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            let error = json!({"jsonrpc": "2.0", "id": null,
                               "error": {"code": -32700, "message": format!("parse error: {e}")}});
            return json_body(400, &error);
        }
    };
    if let Some(version) = request_header(request, "MCP-Protocol-Version")
        && !SUPPORTED_VERSIONS.contains(&version.as_str())
    {
        return plain(
            400,
            &format!(
                "unsupported MCP-Protocol-Version `{version}`; supported: {}",
                SUPPORTED_VERSIONS.join(", ")
            ),
        );
    }
    let initializing = parsed["method"] == "initialize" && !parsed["id"].is_null();
    if !initializing && let Some(error) = check_session(state, request) {
        return error;
    }
    let Some(answer) = state.server.handle_value(&parsed) else {
        return empty(202);
    };
    let mut response = json_body(200, &answer);
    if initializing && answer.get("result").is_some() {
        let id = new_session_id(&state.counter);
        if let Ok(mut sessions) = state.sessions.lock() {
            if sessions.len() >= MAX_SESSIONS {
                sessions.pop_front();
            }
            sessions.push_back(id.clone());
        }
        response.add_header(header(SESSION_HEADER, &id));
    }
    response
}
