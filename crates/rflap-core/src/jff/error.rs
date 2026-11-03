//! Errors of the `.jff` reader.

use std::fmt;

/// Why a `.jff` file could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JffError {
    /// The text is not well-formed XML.
    BadXml(String),
    /// The file holds another kind of structure than the reader handles.
    WrongType { expected: String, found: String },
    /// A required element is absent.
    MissingElement { element: String, context: String },
    /// A number could not be parsed.
    BadNumber { what: String, text: String },
    /// Two states share one id.
    DuplicateState(u32),
    /// A transition names a state that does not exist.
    UnknownState { id: u32, context: String },
    /// A transition label could not be read.
    BadLabel(String),
    /// A production has no left side, or the first one is not restricted.
    BadProduction(String),
    /// A regular expression could not be parsed.
    BadExpression(String),
    /// A Turing machine transition does not describe exactly the tapes of the machine.
    TapeCount { expected: usize, found: usize },
    /// A `tape` attribute names a tape the machine does not have.
    TapeOutOfRange { tape: usize, tapes: usize },
    /// A building block names an inner machine that is not in the file.
    UnknownBlock { tag: String },
    /// A building block contains itself.
    BlockCycle { tag: String },
    /// The file holds a structure this reader does not handle (L-systems, pumping lemma
    /// games, unknown types).
    Unsupported { structure_type: String },
}

impl JffError {
    pub(crate) fn missing(element: &str, context: &str) -> Self {
        JffError::MissingElement {
            element: element.to_string(),
            context: context.to_string(),
        }
    }

    pub(crate) fn bad_number(what: impl Into<String>, text: &str) -> Self {
        JffError::BadNumber {
            what: what.into(),
            text: text.to_string(),
        }
    }
}

impl fmt::Display for JffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JffError::BadXml(msg) => write!(f, "could not parse XML: {msg}"),
            JffError::WrongType { expected, found } => write!(
                f,
                "expected a structure of type \"{expected}\" but the file has type \"{found}\""
            ),
            JffError::MissingElement { element, context } => {
                write!(f, "missing <{element}> in <{context}>")
            }
            JffError::BadNumber { what, text } => {
                write!(f, "could not read \"{text}\" as a number ({what})")
            }
            JffError::DuplicateState(id) => write!(f, "the state id {id} appears twice"),
            JffError::UnknownState { id, context } => {
                write!(f, "{context} refers to the non-existent state {id}")
            }
            JffError::BadLabel(msg) => write!(f, "bad transition label: {msg}"),
            JffError::BadProduction(msg) => write!(f, "bad production: {msg}"),
            JffError::BadExpression(msg) => write!(f, "bad regular expression: {msg}"),
            JffError::TapeCount { expected, found } => write!(
                f,
                "a transition describes {found} tapes but the machine has {expected}"
            ),
            JffError::TapeOutOfRange { tape, tapes } => {
                write!(f, "tape {tape} is out of range: the machine has {tapes}")
            }
            JffError::UnknownBlock { tag } => {
                write!(f, "the building block \"{tag}\" is not defined in the file")
            }
            JffError::BlockCycle { tag } => {
                write!(f, "the building block \"{tag}\" contains itself")
            }
            JffError::Unsupported { structure_type } => {
                write!(f, "structure type \"{structure_type}\" is not supported")
            }
        }
    }
}

impl std::error::Error for JffError {}
