//! The stderr line written for every `tools/call`. Hosts keep a server's stderr in their log
//! (Claude Desktop: `mcp-server-rflap.log`), so this is how a run can be reviewed afterwards.

use std::time::Duration;

use serde_json::Value;

/// Arguments longer than this many characters are cut (documents can be large).
const MAX_ARGS: usize = 2000;
/// The first line of the answer is cut at this many characters.
const MAX_ANSWER: usize = 200;

/// `rflap-mcp: tools/call <tool> ok|error <ms>ms args=<json> -> <first line of the answer>`.
pub fn call_log_line(tool: &str, arguments: &Value, output: &Value, elapsed: Duration) -> String {
    let outcome = if output["isError"].as_bool().unwrap_or(false) {
        "error"
    } else {
        "ok"
    };
    let answer = output["content"][0]["text"]
        .as_str()
        .and_then(|text| text.lines().map(str::trim).find(|l| !l.is_empty()))
        .unwrap_or("");
    format!(
        "rflap-mcp: tools/call {tool} {outcome} {}ms args={} -> {}",
        elapsed.as_millis(),
        cut(&arguments.to_string(), MAX_ARGS),
        cut(answer, MAX_ANSWER),
    )
}

fn cut(text: &str, max: usize) -> String {
    let total = text.chars().count();
    if total <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max).collect();
    format!("{kept}… (+{} chars)", total - max)
}
