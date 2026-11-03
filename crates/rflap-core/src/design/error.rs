//! Errors of the design engine. A failed example is not an error: see `Verification`.

use std::fmt;

use serde::Serialize;

/// A syntax error in the constraint text syntax; `position` counts characters from 0.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConstraintSyntaxError {
    pub position: usize,
    pub expected: String,
    pub found: String,
    pub message: String,
}

impl fmt::Display for ConstraintSyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConstraintSyntaxError {}

/// A condition that no finite automaton can check, with the template that does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NotRegularReport {
    /// The offending condition in text syntax.
    pub constraint: String,
    pub reason: String,
    /// Name of the matching template, if one fits.
    pub template: Option<String>,
    /// What to put in the spec instead (a JSON `language` value).
    pub suggestion: String,
}

/// Why a spec could not be turned into an automaton.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum DesignError {
    InvalidAlphabet {
        message: String,
    },
    /// A symbol outside the alphabet; `context` says where (a constraint or an example).
    AlphabetViolation {
        context: String,
        symbol: char,
        /// Index (in characters) inside the offending word, pattern or symbol list.
        position: usize,
        message: String,
    },
    ConstraintSyntax(ConstraintSyntaxError),
    RegexSyntax {
        position: usize,
        message: String,
    },
    GrammarSyntax {
        message: String,
    },
    /// The grammar is not right-linear or left-linear.
    NotRegularGrammar {
        message: String,
    },
    InvalidParameter {
        context: String,
        message: String,
    },
    NotRegular(NotRegularReport),
    TooLarge {
        message: String,
    },
}

impl fmt::Display for DesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DesignError::InvalidAlphabet { message }
            | DesignError::AlphabetViolation { message, .. }
            | DesignError::GrammarSyntax { message }
            | DesignError::NotRegularGrammar { message }
            | DesignError::TooLarge { message } => f.write_str(message),
            DesignError::ConstraintSyntax(e) => e.fmt(f),
            DesignError::RegexSyntax { position, message } => {
                write!(f, "regular expression, position {position}: {message}")
            }
            DesignError::InvalidParameter { context, message } => {
                write!(f, "{context}: {message}")
            }
            DesignError::NotRegular(r) => {
                write!(f, "not regular: {} {}", r.reason, r.suggestion)
            }
        }
    }
}

impl std::error::Error for DesignError {}
