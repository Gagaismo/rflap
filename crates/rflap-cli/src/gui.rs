//! `gui`: serves the embedded web editor on 127.0.0.1 and opens it in the browser.

use std::io::Write;
use std::process::Command;

use serde_json::json;
use tiny_http::{Header, Response, Server, StatusCode};

use crate::api;
use crate::error::{CliError, Result};
use crate::input;

/// The single-file editor built by the web worker (`web/dist/rflap.html`).
const EDITOR: &str = include_str!("../../../web/dist/rflap.html");

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Base64url without padding.
pub fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..=chunk.len() {
            out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

fn open_browser(url: &str) {
    let spawned = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/c", "start", "", url]).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
    if spawned.is_err() {
        eprintln!("could not open the browser; open the URL above by hand");
    }
}

fn serve_error(what: impl std::fmt::Display) -> CliError {
    CliError::new("io_error", format!("cannot serve the editor: {what}"))
}

/// Starts the server and blocks serving requests.
pub fn run(file: Option<&str>, port: u16, no_open: bool) -> Result<()> {
    let fragment = match file {
        Some(file) => {
            let document = input::load_document(file)?;
            let answer = api::call(&json!({"op": "normalize", "document": document}))?;
            let normalized = api::result(&answer)["document"].to_string();
            format!("/#doc={}", base64url(normalized.as_bytes()))
        }
        None => "/".to_string(),
    };
    let server = Server::http(("127.0.0.1", port)).map_err(serve_error)?;
    let bound = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| serve_error("no IP address"))?;
    let url = format!("http://127.0.0.1:{}{fragment}", bound.port());
    println!("Rflap editor: {url}");
    std::io::stdout().flush().ok();
    if !no_open {
        open_browser(&url);
    }
    let html = Header::from_bytes("Content-Type", "text/html; charset=utf-8")
        .map_err(|_| serve_error("bad header"))?;
    for request in server.incoming_requests() {
        let path = request
            .url()
            .split(['?', '#'])
            .next()
            .unwrap_or("/")
            .to_string();
        let answer = if path == "/" || path == "/index.html" {
            request.respond(Response::from_string(EDITOR).with_header(html.clone()))
        } else {
            request.respond(Response::from_string("not found").with_status_code(StatusCode(404)))
        };
        answer.ok();
    }
    Ok(())
}
