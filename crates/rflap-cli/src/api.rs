//! The only door to `rflap-api`: send a request, get the answer or a [`CliError`].

use serde_json::Value;

use crate::error::{CliError, Result};

/// Runs a request. The answer is the whole `{"ok": true, "op", "result"}` object.
pub fn call(request: &Value) -> Result<Value> {
    let (raw, answer) = call_raw(&request.to_string())?;
    if answer["ok"] == true {
        Ok(answer)
    } else {
        Err(CliError::from_api(&raw, &answer))
    }
}

/// Runs request text; returns the raw answer text and its parsed form, whatever `ok` says.
pub fn call_raw(request: &str) -> Result<(String, Value)> {
    let raw = rflap_api::call(request);
    let answer: Value = serde_json::from_str(&raw)
        .map_err(|e| CliError::new("internal", format!("the API answered invalid JSON: {e}")))?;
    Ok((raw, answer))
}

/// The `result` of an answer.
pub fn result(answer: &Value) -> &Value {
    &answer["result"]
}
