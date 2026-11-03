//! The local viewer: the widget HTML served over HTTP so a terminal agent can hand the user a
//! link (`http://127.0.0.1:P/#doc=<base64url JSON>`). The document travels in the URL fragment,
//! so it never reaches any server.

use std::io;
use std::net::TcpListener;
use std::thread;

use serde_json::Value;

use crate::EDITOR_HTML;

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Longest link put in a result; a bigger document is left out rather than flooding the chat.
const MAX_LINK: usize = 32 * 1024;

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

/// The link that opens `document` in the viewer at `base` (`http://127.0.0.1:8787/`).
pub fn link(base: &str, document: &Value) -> Option<String> {
    let url = format!("{base}#doc={}", base64url(document.to_string().as_bytes()));
    (url.len() <= MAX_LINK).then_some(url)
}

/// Serves the widget at `/` on `127.0.0.1:port` from a background thread and returns its URL.
pub fn spawn(port: u16) -> io::Result<String> {
    // Bind first so a busy port is reported instead of lost in the thread.
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let addr = listener.local_addr()?;
    let server = tiny_http::Server::from_listener(listener, None).map_err(io::Error::other)?;
    thread::spawn(move || {
        for request in server.incoming_requests() {
            let path = request
                .url()
                .split(['?', '#'])
                .next()
                .unwrap_or("/")
                .to_string();
            let response = if request.method() == &tiny_http::Method::Get
                && (path == "/" || path == "/index.html")
            {
                html_response()
            } else {
                tiny_http::Response::from_string("not found")
                    .with_status_code(404)
                    .boxed()
            };
            let _ = request.respond(response);
        }
    });
    Ok(format!("http://127.0.0.1:{}/", addr.port()))
}

/// The widget as an HTTP response.
pub fn html_response() -> tiny_http::ResponseBox {
    tiny_http::Response::from_string(EDITOR_HTML)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                .expect("a valid header"),
        )
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_matches_rfc_4648_vectors() {
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(b"foob"), "Zm9vYg");
        assert_eq!(base64url(&[0xfb, 0xff]), "-_8");
    }
}
