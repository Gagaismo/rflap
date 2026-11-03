//! Small text helpers shared by the commands.

use serde_json::Value;

/// What a command produced: its text, the raw JSON for `--json` and the exit code.
pub struct Reply {
    pub text: String,
    pub json: String,
    pub code: i32,
}

impl Reply {
    pub fn new(text: String, json: &Value, code: i32) -> Reply {
        Reply {
            text,
            json: json.to_string(),
            code,
        }
    }
}

/// Exit codes of the CLI.
pub mod exit {
    pub const OK: i32 = 0;
    pub const REJECTED: i32 = 1;
    pub const USAGE: i32 = 2;
    pub const UNDECIDED: i32 = 3;
}

/// Quotes a string for display: `"ab"`, `""`.
pub fn quote(s: &str) -> String {
    format!("{s:?}")
}

/// The strings of a JSON array joined by `, ` (`none` when empty or missing).
pub fn list_or_none(value: &Value) -> String {
    let items: Vec<String> = value
        .as_array()
        .map(|a| a.iter().map(plain).collect())
        .unwrap_or_default();
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

/// A JSON scalar without quotes.
pub fn plain(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A markdown table row.
pub fn row(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

/// A markdown table.
pub fn table(header: &[String], rows: &[Vec<String>]) -> String {
    let mut out = row(header);
    out.push('\n');
    out.push('|');
    out.push_str(&"---|".repeat(header.len()));
    out.push('\n');
    for r in rows {
        out.push_str(&row(r));
        out.push('\n');
    }
    out
}

/// Makes sure a block of text ends with a newline.
pub fn with_newline(mut s: String) -> String {
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// `1 state`, `3 states`.
pub fn count(n: u64, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// A configuration of a run: `q0 remaining="ab" stack="Z"`.
pub fn config(entry: &Value) -> String {
    let mut out = plain(&entry["state"]);
    if let Some(remaining) = entry["remaining"].as_str() {
        out.push_str(&format!(" remaining={}", quote(remaining)));
    }
    if let Some(stack) = entry["stack"].as_str() {
        out.push_str(&format!(" stack={}", quote(stack)));
    }
    if let Some(output) = entry["output"].as_str() {
        out.push_str(&format!(" output={}", quote(output)));
    }
    if let Some(tapes) = entry["tapes"].as_array() {
        for (k, tape) in tapes.iter().enumerate() {
            out.push_str(&format!(
                " tape{}={} head={}",
                k + 1,
                quote(tape["content"].as_str().unwrap_or("")),
                plain(&tape["head"])
            ));
        }
    }
    out
}

/// A tree `{symbol, children}` drawn with box characters, every line prefixed by `indent`.
pub fn tree(node: &Value, indent: &str) -> String {
    let mut out = format!("{indent}{}\n", plain(&node["symbol"]));
    children(node, indent, "", &mut out);
    out
}

fn children(node: &Value, indent: &str, prefix: &str, out: &mut String) {
    let Some(kids) = node["children"].as_array() else {
        return;
    };
    for (i, kid) in kids.iter().enumerate() {
        let last = i + 1 == kids.len();
        out.push_str(&format!(
            "{indent}{prefix}{} {}\n",
            if last { "└─" } else { "├─" },
            plain(&kid["symbol"])
        ));
        let deeper = format!("{prefix}{}", if last { "   " } else { "│  " });
        children(kid, indent, &deeper, out);
    }
}
