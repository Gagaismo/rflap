//! `capabilities`: what the API offers, as text.

use serde_json::json;

use crate::api;
use crate::error::Result;
use crate::text::{Reply, exit, list_or_none, plain};

fn first_sentence(text: &str) -> &str {
    match text.find(". ") {
        Some(end) => &text[..=end],
        None => text,
    }
}

pub fn capabilities() -> Result<Reply> {
    let answer = api::call(&json!({"op": "capabilities"}))?;
    let r = api::result(&answer);
    let mut out = format!(
        "rflap {}\n{}\n\n",
        plain(&r["version"]),
        first_sentence(&plain(&r["summary"]))
    );
    out.push_str("Operations (rflap call '{\"op\": ...}'):\n");
    for op in r["ops"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "  {:<18} {}\n",
            plain(&op["op"]),
            first_sentence(&plain(&op["summary"]))
        ));
    }
    out.push_str("\nDocument types:\n");
    for t in r["document_types"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "  {:<28} {}\n",
            plain(&t["type"]),
            plain(&t["summary"])
        ));
    }
    out.push_str("\nConversions (rflap convert --to TARGET):\n");
    for c in r["convert"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "  {:<22} from {:<20} {}\n",
            plain(&c["to"]).replace('_', "-"),
            plain(&c["from"]),
            plain(&c["summary"])
        ));
    }
    out.push_str(&format!(
        "\nParsers: {}\n",
        list_or_none(&r["parsers"]).replace('_', "-")
    ));
    out.push_str(&format!("Layouts: {}\n", list_or_none(&r["layouts"])));
    out.push_str(&format!(
        "Render formats: {}\n",
        list_or_none(&r["formats"])
    ));
    Ok(Reply::new(out, &answer, exit::OK))
}
