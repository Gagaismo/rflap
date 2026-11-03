//! The error every operation reports: a stable machine-readable `code` and a message.

use std::fmt;

use serde::Serialize;
use serde_json::Value;

/// Stable error codes (the `error.code` of a failed response).
pub mod code {
    /// The request text is not valid JSON.
    pub const INVALID_JSON: &str = "invalid_json";
    /// The request is JSON but not a well-formed request (missing or unknown field, wrong type).
    pub const INVALID_REQUEST: &str = "invalid_request";
    /// The `op` is not one of the operations in `capabilities`.
    pub const UNKNOWN_OP: &str = "unknown_op";
    /// A document is malformed.
    pub const INVALID_DOCUMENT: &str = "invalid_document";
    /// The `type` of a document is not one of the known types.
    pub const UNKNOWN_DOCUMENT_TYPE: &str = "unknown_document_type";
    /// A compact document names a state that was not declared.
    pub const UNKNOWN_STATE: &str = "unknown_state";
    /// A transition label cannot be read.
    pub const INVALID_LABEL: &str = "invalid_label";
    /// A grammar cannot be read.
    pub const INVALID_GRAMMAR: &str = "invalid_grammar";
    /// A regular expression cannot be read.
    pub const INVALID_REGEX: &str = "invalid_regex";
    /// The operation does not work on this type of document.
    pub const WRONG_DOCUMENT_TYPE: &str = "wrong_document_type";
    /// An option is not available for this type of document.
    pub const UNSUPPORTED_OPTION: &str = "unsupported_option";
    /// An argument has a value the operation cannot use.
    pub const INVALID_ARGUMENT: &str = "invalid_argument";
    /// The automaton needs an initial state.
    pub const NO_INITIAL_STATE: &str = "no_initial_state";
    /// A `.jff` file could not be read.
    pub const JFF_ERROR: &str = "jff_error";
    /// A conversion refused its input.
    pub const CONVERSION_FAILED: &str = "conversion_failed";
    /// The design engine refused a specification.
    pub const DESIGN_ERROR: &str = "design_error";
    /// An input cannot be parsed (a symbol outside the grammar, ...).
    pub const PARSE_ERROR: &str = "parse_error";
    /// A user-controlled derivation move is not valid.
    pub const INVALID_MOVE: &str = "invalid_move";
    /// A size limit protects the caller.
    pub const LIMIT_EXCEEDED: &str = "limit_exceeded";
    /// A bug: the operation panicked.
    pub const INTERNAL: &str = "internal";
}

/// Why an operation failed. Serialized as `{"code", "message", "details"?}`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    /// Structured context (the offending transition, the core error, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        ApiError {
            code: code.to_string(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub(crate) fn invalid_document(message: impl Into<String>) -> Self {
        ApiError::new(code::INVALID_DOCUMENT, message)
    }

    pub(crate) fn wrong_type(op: &str, wanted: &str, got: &str) -> Self {
        ApiError::new(
            code::WRONG_DOCUMENT_TYPE,
            format!("`{op}` needs {wanted}, got a document of type `{got}`"),
        )
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}
