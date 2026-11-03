//! The error of a command: a stable `code` and a message, printed as `error: <code>: <message>`.

use std::fmt;

use serde_json::{Value, json};

#[derive(Debug)]
pub struct CliError {
    pub code: String,
    pub message: String,
    /// The raw answer of `rflap-api` when the error came from it (printed by `--json`).
    pub raw: Option<String>,
}

impl CliError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        CliError {
            code: code.to_string(),
            message: message.into(),
            raw: None,
        }
    }

    /// A bad argument or option value.
    pub fn arg(message: impl Into<String>) -> Self {
        CliError::new("invalid_argument", message)
    }

    /// An error answer of the API (`{"ok": false, "error": {...}}`).
    pub fn from_api(raw: &str, answer: &Value) -> Self {
        let error = &answer["error"];
        CliError {
            code: error["code"].as_str().unwrap_or("internal").to_string(),
            message: error["message"]
                .as_str()
                .unwrap_or("unknown error")
                .to_string(),
            raw: Some(raw.to_string()),
        }
    }

    /// The error as JSON, for `--json`.
    pub fn json(&self) -> String {
        match &self.raw {
            Some(raw) => raw.clone(),
            None => json!({"ok": false, "error": {"code": self.code, "message": self.message}})
                .to_string(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

pub type Result<T> = std::result::Result<T, CliError>;
