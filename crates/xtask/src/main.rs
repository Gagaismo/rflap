//! Build tasks of Rflap. Usage: `cargo run -p xtask -- build-web [--no-wasm-build]`.
//!
//! `build-web` compiles `rflap-wasm` and writes `web/dist/rflap.html`: ONE file with the CSS, the
//! bundled JavaScript (see [`bundle`]) and the wasm module as base64, with no external URLs, so
//! it works in a browser tab, in the iframe of an MCP Apps host (no network) and in ChatGPT.

mod base64;
mod bundle;

use std::path::{Path, PathBuf};
use std::process::Command;

const WASM_TARGET: &str = "wasm32-unknown-unknown";
const WASM_PROFILE: &str = "wasm-release";
/// XML namespace identifiers: names, not resources the browser would fetch.
const ALLOWED_URLS: [&str; 2] = ["http://www.w3.org/2000/svg", "http://www.w3.org/1999/xlink"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("build-web") => build_web(&args[1..]),
        _ => Err("usage: cargo run -p xtask -- build-web [--no-wasm-build]".to_string()),
    };
    if let Err(message) = result {
        eprintln!("xtask: {message}");
        std::process::exit(1);
    }
}

/// The workspace `cargo run` was called from. `cargo run` sets `CARGO_MANIFEST_DIR` at run time;
/// the compile-time value is only a fallback, because worktrees sharing one target directory
/// share one `xtask` binary, and its baked path may name another worktree.
fn workspace_root() -> PathBuf {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    manifest
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn cargo() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

/// `target_directory` from `cargo metadata` (honours `CARGO_TARGET_DIR` and config files).
fn target_directory(root: &Path) -> Result<PathBuf, String> {
    let output = cargo()
        .current_dir(root)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|e| format!("cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let key = "\"target_directory\":\"";
    let start = text
        .find(key)
        .ok_or("no target_directory in cargo metadata")?
        + key.len();
    let mut path = String::new();
    let mut chars = text[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Ok(PathBuf::from(path)),
            '\\' => path.push(chars.next().ok_or("bad escape in cargo metadata")?),
            c => path.push(c),
        }
    }
    Err("unterminated target_directory in cargo metadata".into())
}

fn build_web(flags: &[String]) -> Result<(), String> {
    let root = workspace_root();
    if !flags.iter().all(|f| f == "--no-wasm-build") {
        return Err("unknown flag (only --no-wasm-build is accepted)".into());
    }
    if !flags.iter().any(|f| f == "--no-wasm-build") {
        let status = cargo()
            .current_dir(&root)
            .args([
                "build",
                "-p",
                "rflap-wasm",
                "--target",
                WASM_TARGET,
                "--profile",
                WASM_PROFILE,
            ])
            .status()
            .map_err(|e| format!("cargo build: {e}"))?;
        if !status.success() {
            return Err("building rflap-wasm failed".into());
        }
    }
    let wasm_path = target_directory(&root)?
        .join(WASM_TARGET)
        .join(WASM_PROFILE)
        .join("rflap_wasm.wasm");
    let wasm = std::fs::read(&wasm_path).map_err(|e| format!("{}: {e}", wasm_path.display()))?;

    let src = root.join("web/src");
    let read = |name: &str| {
        std::fs::read_to_string(src.join(name)).map_err(|e| format!("web/src/{name}: {e}"))
    };
    let html = assemble(
        &read("index.html")?,
        &read("style.css")?,
        &bundle::bundle(&src, "main.js")?,
        &wasm,
    )?;

    let dist = root.join("web/dist");
    std::fs::create_dir_all(&dist).map_err(|e| e.to_string())?;
    let out = dist.join("rflap.html");
    std::fs::write(&out, &html).map_err(|e| format!("{}: {e}", out.display()))?;
    println!(
        "web/dist/rflap.html: {} bytes ({:.2} MiB; wasm {} bytes)",
        html.len(),
        html.len() as f64 / 1_048_576.0,
        wasm.len()
    );
    Ok(())
}

/// Fills the template; fails when the result would load anything from outside.
fn assemble(template: &str, css: &str, js: &str, wasm: &[u8]) -> Result<String, String> {
    for marker in ["<!--RFLAP:CSS-->", "<!--RFLAP:JS-->", "<!--RFLAP:WASM-->"] {
        if template.matches(marker).count() != 1 {
            return Err(format!("index.html must contain {marker} exactly once"));
        }
    }
    let html = template
        .replace("<!--RFLAP:CSS-->", css)
        .replace("<!--RFLAP:JS-->", js)
        .replace("<!--RFLAP:WASM-->", &base64::encode(wasm));
    let external = external_urls(&html);
    if !external.is_empty() {
        return Err(format!(
            "the page would reference external URLs: {external:?}"
        ));
    }
    Ok(html)
}

/// Every `http(s)://` or protocol-relative URL in the text that is not a namespace identifier.
fn external_urls(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for scheme in ["http://", "https://", "ftp://", "ws://", "wss://"] {
        for (at, _) in text.match_indices(scheme) {
            let url: String = text[at..]
                .chars()
                .take_while(|c| !c.is_whitespace() && !"\"'<>)`".contains(*c))
                .collect();
            if !ALLOWED_URLS.contains(&url.as_str()) {
                found.push(url);
            }
        }
    }
    for attr in ["src=\"//", "href=\"//", "url(//", "@import"] {
        if text.contains(attr) {
            found.push(attr.to_string());
        }
    }
    found.sort();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "<style><!--RFLAP:CSS--></style><script><!--RFLAP:JS--></script>\
                            <script id=w><!--RFLAP:WASM--></script>";

    #[test]
    fn assembles_one_file() {
        let html = assemble(TEMPLATE, "a{}", "let x;", b"foo").unwrap();
        assert_eq!(
            html,
            "<style>a{}</style><script>let x;</script><script id=w>Zm9v</script>"
        );
    }

    #[test]
    fn needs_every_marker_once() {
        assert!(assemble("<!--RFLAP:CSS-->", "", "", b"").is_err());
        assert!(assemble(&format!("{TEMPLATE}<!--RFLAP:JS-->"), "", "", b"").is_err());
    }

    #[test]
    fn external_urls_are_refused_but_namespaces_are_not() {
        let ns = "createElementNS('http://www.w3.org/2000/svg','svg')";
        assert!(assemble(TEMPLATE, "", ns, b"").is_ok());
        for bad in [
            "fetch('https://example.com/x')",
            "<link href=\"//cdn.example.com/a.css\">",
            "@import 'x.css';",
            "new WebSocket('ws://localhost:1')",
        ] {
            assert!(assemble(TEMPLATE, "", bad, b"").is_err(), "{bad}");
        }
    }
}
