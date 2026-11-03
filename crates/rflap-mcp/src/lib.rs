//! Rflap as an MCP server, so AI chats and agents (ChatGPT, Claude, Claude Code, Codex) can
//! design, run, convert and draw automata, grammars and regular expressions.
//!
//! The server only adapts [`rflap_api`]: every tool turns its arguments into one or more API
//! requests and answers with
//!
//! * `content`: concise Markdown (summary, transition table, verification report, a viewer link
//!   when one is configured) for chats that cannot render the widget, and
//! * `structuredContent`: the API result, which the widget (`ui://rflap/editor.html`, the
//!   single page `web/dist/rflap.html`) reads in hosts that support MCP Apps.
//!
//! Failures of a tool are results with `isError: true`, never JSON-RPC errors.
//!
//! The protocol is hand-rolled JSON-RPC 2.0 (no async runtime): [`Server::handle_message`] is a
//! pure function from a message to its answer, [`serve_stdio`] and [`serve_http`] are the two
//! transports, and the binary `rflap-mcp` wires them to the command line.
//!
//! ```
//! let answer = rflap_mcp::handle_message(&serde_json::json!(
//!     {"jsonrpc": "2.0", "id": 1, "method": "ping"}));
//! assert_eq!(answer.unwrap()["result"], serde_json::json!({}));
//! ```

mod content;
mod docs;
mod http;
mod instructions;
mod log;
mod schemas;
mod server;
mod stdio;
mod tools;
mod viewer;

pub use docs::tools_markdown;
pub use http::{HttpOptions, HttpServer, serve_http};
pub use log::call_log_line;
pub use server::{Config, Server, handle_line, handle_message};
pub use stdio::{serve, serve_stdio};
pub use viewer::{base64url, link as viewer_link, spawn as spawn_viewer};

/// The widget, a single self-contained page built by the web editor.
pub const EDITOR_HTML: &str = include_str!("../../../web/dist/rflap.html");
/// The `ui://` URI of the widget (`_meta.ui.resourceUri`).
pub const EDITOR_URI: &str = "ui://rflap/editor.html";
/// The MCP Apps MIME type of the widget resource.
pub const MIME_APP: &str = "text/html;profile=mcp-app";
/// Protocol versions this server speaks, newest first.
pub const SUPPORTED_VERSIONS: [&str; 3] = ["2025-11-25", "2025-06-18", "2025-03-26"];
