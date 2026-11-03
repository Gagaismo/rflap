//! Reading documents and inputs, writing output files.

use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::api;
use crate::error::{CliError, Result};

static STDIN: OnceLock<String> = OnceLock::new();

fn stdin_text() -> Result<String> {
    if let Some(text) = STDIN.get() {
        return Ok(text.clone());
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| CliError::new("io_error", format!("cannot read standard input: {e}")))?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(STDIN.get_or_init(|| text).clone())
}

/// The text of a file, or of standard input for `-`.
pub fn read_text(path: &str) -> Result<String> {
    let text = if path == "-" {
        stdin_text()?
    } else {
        match std::fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(CliError::new(
                    "io_error",
                    format!("cannot read `{path}`: file not found"),
                ));
            }
            Err(e) => {
                return Err(CliError::new(
                    "io_error",
                    format!("cannot read `{path}`: {e}"),
                ));
            }
        }
    };
    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Jff,
    Json,
    Grammar,
}

fn sniff(text: &str) -> Kind {
    let trimmed = text.trim_start();
    if trimmed.starts_with('<') {
        Kind::Jff
    } else if trimmed.starts_with('{') {
        Kind::Json
    } else {
        Kind::Grammar
    }
}

fn kind_of(path: &str, text: &str) -> Kind {
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jff") => Kind::Jff,
        Some("json") => Kind::Json,
        Some("txt" | "grammar") => Kind::Grammar,
        _ => sniff(text),
    }
}

fn from_text(origin: &str, kind: Kind, text: &str) -> Result<Value> {
    match kind {
        Kind::Jff => {
            let answer = api::call(&json!({"op": "open_jff", "xml": text}))?;
            Ok(api::result(&answer)["document"].clone())
        }
        Kind::Json => {
            let value: Value = serde_json::from_str(text)
                .map_err(|e| CliError::new("invalid_json", format!("{origin}: {e}")))?;
            // an answer of the API (`rflap ... --json > x.json`) is accepted as a document
            if value["ok"] == true && value["result"]["document"].is_object() {
                return Ok(value["result"]["document"].clone());
            }
            Ok(value)
        }
        Kind::Grammar => Ok(json!({"type": "grammar", "text": text})),
    }
}

/// A document from a path (`.jff`, `.json`, grammar text, `-` for stdin) or from the inline
/// forms `regex:EXPR` and `grammar:TEXT`.
pub fn load_document(spec: &str) -> Result<Value> {
    if let Some(expression) = spec.strip_prefix("regex:") {
        return Ok(regex_document(expression));
    }
    if let Some(text) = spec.strip_prefix("grammar:") {
        return Ok(grammar_document(text));
    }
    let text = read_text(spec)?;
    let origin = if spec == "-" { "<stdin>" } else { spec };
    from_text(origin, kind_of(spec, &text), &text)
}

pub fn regex_document(expression: &str) -> Value {
    json!({"type": "regex", "expression": expression})
}

/// Inline grammar text; a literal `\n` separates productions.
pub fn grammar_document(text: &str) -> Value {
    json!({"type": "grammar", "text": text.replace("\\n", "\n")})
}

/// The text of a `--grammar` value: the contents of the file when it names one, else itself.
pub fn grammar_text(value: &str) -> Result<String> {
    if Path::new(value).is_file() {
        read_text(value)
    } else {
        Ok(value.replace("\\n", "\n"))
    }
}

/// The document named by a FILE argument or by `--regex` / `--grammar` (exactly one).
pub fn resolve_document(
    file: Option<&str>,
    regex: Option<&str>,
    grammar: Option<&str>,
) -> Result<Value> {
    let given = [file.is_some(), regex.is_some(), grammar.is_some()]
        .iter()
        .filter(|g| **g)
        .count();
    match (file, regex, grammar) {
        _ if given > 1 => Err(CliError::arg(
            "give only one of FILE, --regex and --grammar",
        )),
        (Some(file), _, _) => load_document(file),
        (_, Some(expression), _) => Ok(regex_document(expression)),
        (_, _, Some(grammar)) => {
            let text = grammar_text(grammar)?;
            Ok(json!({"type": "grammar", "text": text}))
        }
        _ => Err(CliError::arg(
            "give a FILE (`-` reads standard input), --regex or --grammar",
        )),
    }
}

/// `λ` and `ε` stand for the empty string (an empty argument is easy to lose in a shell).
pub fn input_string(raw: &str) -> String {
    match raw {
        "λ" | "ε" => String::new(),
        other => other.to_string(),
    }
}

/// The lines of an inputs file, one input per line.
pub fn inputs_from_file(path: &str) -> Result<Vec<String>> {
    Ok(read_text(path)?
        .lines()
        .map(|line| input_string(line.trim_end_matches('\r')))
        .collect())
}

/// Fails early on an output path the CLI cannot write.
pub fn check_output(path: &str) -> Result<()> {
    match extension(path).as_deref() {
        Some("jff" | "json" | "svg") => Ok(()),
        _ => Err(CliError::arg(format!(
            "cannot write `{path}`: use a .jff, .json or .svg output file"
        ))),
    }
}

fn extension(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
}

pub fn write_file(path: &str, content: &str) -> Result<()> {
    std::fs::write(path, content)
        .map_err(|e| CliError::new("io_error", format!("cannot write `{path}`: {e}")))
}

/// Saves a document: `.jff` as JFLAP text, `.json` as the normalized document, `.svg` drawn.
pub fn write_document(document: &Value, path: &str) -> Result<()> {
    check_output(path)?;
    match extension(path).as_deref() {
        Some("jff") => {
            let answer = api::call(&json!({"op": "save_jff", "document": document}))?;
            write_file(path, api::result(&answer)["xml"].as_str().unwrap_or(""))
        }
        Some("json") => {
            let answer = api::call(&json!({"op": "normalize", "document": document}))?;
            let normalized = &api::result(&answer)["document"];
            let mut text = serde_json::to_string_pretty(normalized)
                .map_err(|e| CliError::new("internal", e.to_string()))?;
            text.push('\n');
            write_file(path, &text)
        }
        _ => write_svg(document, path),
    }
}

/// Draws a document into an SVG file.
pub fn write_svg(document: &Value, path: &str) -> Result<()> {
    let answer = api::call(&json!({"op": "render", "document": document, "format": "svg"}))?;
    write_file(path, api::result(&answer)["content"].as_str().unwrap_or(""))
}
