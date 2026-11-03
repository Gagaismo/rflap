//! The MCP protocol over JSON-RPC 2.0: a pure function from a message to its answer, so every
//! transport (stdio, HTTP) and every test drives the same code.

use std::time::Instant;

use serde_json::{Value, json};

use crate::{EDITOR_HTML, EDITOR_URI, MIME_APP, SUPPORTED_VERSIONS, instructions, schemas, tools};

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;
/// MCP: "Resource not found".
const RESOURCE_NOT_FOUND: i64 = -32002;

/// How a server behaves where it runs.
#[derive(Clone, Debug)]
pub struct Config {
    /// Base URL of the local viewer (`http://127.0.0.1:8787/`); visual results link to it.
    pub viewer_url: Option<String>,
    /// Whether `rflap_open_jff` / `rflap_save_jff` may touch the file system (a local server).
    pub allow_files: bool,
    /// Whether every `tools/call` writes one line to stderr ([`crate::call_log_line`]).
    pub log_calls: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            viewer_url: None,
            allow_files: true,
            log_calls: false,
        }
    }
}

/// An MCP server: stateless apart from its configuration.
#[derive(Clone, Debug, Default)]
pub struct Server {
    config: Config,
}

type RpcResult = Result<Value, (i64, String)>;

fn error_response(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message.into()}})
}

/// Runs one message with the default configuration; see [`Server::handle_message`].
pub fn handle_message(message: &Value) -> Option<Value> {
    Server::default().handle_message(message)
}

/// Runs one line of text with the default configuration; see [`Server::handle_line`].
pub fn handle_line(line: &str) -> Option<String> {
    Server::default().handle_line(line)
}

impl Server {
    pub fn new(config: Config) -> Self {
        Server { config }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Answers one JSON-RPC message: `None` for a notification or a response from the client.
    pub fn handle_message(&self, message: &Value) -> Option<Value> {
        let Some(object) = message.as_object() else {
            return Some(error_response(
                Value::Null,
                INVALID_REQUEST,
                "a JSON-RPC message is an object",
            ));
        };
        let id = object.get("id").filter(|id| !id.is_null());
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            // A response to a request of ours (we send none) is ignored.
            if id.is_some() && (object.contains_key("result") || object.contains_key("error")) {
                return None;
            }
            return Some(error_response(
                id.cloned().unwrap_or(Value::Null),
                INVALID_REQUEST,
                "the message has no `method`",
            ));
        };
        let Some(id) = id else {
            // A notification (initialized, cancelled, ...) needs no answer.
            return None;
        };
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.route(method, &params)))
                .unwrap_or_else(|_| Err((INTERNAL_ERROR, "internal error".into())));
        Some(match outcome {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err((code, message)) => error_response(id.clone(), code, message),
        })
    }

    /// Answers one line of text (a message or a batch); `None` when nothing is to be sent.
    pub fn handle_line(&self, line: &str) -> Option<String> {
        match serde_json::from_str::<Value>(line) {
            Ok(parsed) => self.handle_value(&parsed).map(|v| v.to_string()),
            Err(e) => Some(
                error_response(Value::Null, PARSE_ERROR, format!("parse error: {e}")).to_string(),
            ),
        }
    }

    /// Answers a parsed message or batch of messages.
    pub fn handle_value(&self, parsed: &Value) -> Option<Value> {
        let Value::Array(batch) = parsed else {
            return self.handle_message(parsed);
        };
        if batch.is_empty() {
            return Some(error_response(Value::Null, INVALID_REQUEST, "empty batch"));
        }
        let answers: Vec<Value> = batch
            .iter()
            .filter_map(|m| self.handle_message(m))
            .collect();
        (!answers.is_empty()).then_some(Value::Array(answers))
    }

    fn route(&self, method: &str, params: &Value) -> RpcResult {
        match method {
            "initialize" => initialize(params),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(
                json!({"tools": schemas::tools().iter().map(|t| t.to_json()).collect::<Vec<_>>()}),
            ),
            "tools/call" => self.call_tool(params),
            "resources/list" => Ok(json!({"resources": [resource_entry()]})),
            "resources/templates/list" => Ok(json!({"resourceTemplates": []})),
            "resources/read" => read_resource(params),
            "prompts/list" => Ok(json!({"prompts": instructions::prompt_list()})),
            "prompts/get" => get_prompt(params),
            other => Err((METHOD_NOT_FOUND, format!("method not found: {other}"))),
        }
    }

    fn call_tool(&self, params: &Value) -> RpcResult {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return Err((INVALID_PARAMS, "tools/call needs a `name`".into()));
        };
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
        // An unknown tool is a tool result (`isError`), like any other failure of a call: the
        // model sees the message and the list of tools and can correct itself.
        let started = Instant::now();
        let output = tools::run(&self.config, name, &arguments).to_json();
        if self.config.log_calls {
            eprintln!(
                "{}",
                crate::call_log_line(name, &arguments, &output, started.elapsed())
            );
        }
        Ok(output)
    }
}

fn initialize(params: &Value) -> RpcResult {
    let Some(requested) = params.get("protocolVersion").and_then(Value::as_str) else {
        return Err((
            INVALID_PARAMS,
            "initialize needs a `protocolVersion`".into(),
        ));
    };
    let version = if SUPPORTED_VERSIONS.contains(&requested) {
        requested
    } else {
        SUPPORTED_VERSIONS[0]
    };
    Ok(json!({
        "protocolVersion": version,
        "capabilities": {
            "tools": {"listChanged": false},
            "resources": {"subscribe": false, "listChanged": false},
            "prompts": {"listChanged": false},
        },
        "serverInfo": {
            "name": "rflap",
            "title": "Rflap",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "JFLAP 7.1 in Rust: design, simulate, convert and draw automata, grammars and regular expressions.",
        },
        "instructions": instructions::INSTRUCTIONS,
    }))
}

fn resource_meta() -> Value {
    json!({
        "ui": {
            "csp": {"connectDomains": [], "resourceDomains": []},
            "prefersBorder": true,
        },
        "openai/widgetDescription": "Interactive Rflap editor: draws the automaton, grammar or regular expression and lets the student edit it and step through runs.",
        "openai/widgetPrefersBorder": true,
        "openai/widgetCSP": {"connect_domains": [], "resource_domains": []},
    })
}

fn resource_entry() -> Value {
    json!({
        "uri": EDITOR_URI,
        "name": "rflap-editor",
        "title": "Rflap editor",
        "description": "Interactive automaton, grammar and regular expression editor and simulator.",
        "mimeType": MIME_APP,
        "_meta": resource_meta(),
    })
}

fn read_resource(params: &Value) -> RpcResult {
    match params.get("uri").and_then(Value::as_str) {
        Some(EDITOR_URI) => Ok(json!({"contents": [{
            "uri": EDITOR_URI,
            "mimeType": MIME_APP,
            "text": EDITOR_HTML,
            "_meta": resource_meta(),
        }]})),
        Some(other) => Err((RESOURCE_NOT_FOUND, format!("resource not found: {other}"))),
        None => Err((INVALID_PARAMS, "resources/read needs a `uri`".into())),
    }
}

fn get_prompt(params: &Value) -> RpcResult {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Err((INVALID_PARAMS, "prompts/get needs a `name`".into()));
    };
    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
    instructions::prompt(name, &arguments).map_err(|message| (INVALID_PARAMS, message))
}
