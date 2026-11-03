//! The `Document`: every kind of structure Rflap handles, as JSON.
//!
//! A document is a JSON object with a `"type"`. Two forms are accepted wherever a document is:
//!
//! * the **full form**, the serde of the core models (what the operations return), and
//! * the **compact form**, written by hand or by an AI (see [`crate::compact`] and the
//!   crate docs). A compact document is expanded to the full form (states created by name,
//!   laid out automatically) before anything else happens.

use rflap_core::automaton::{Automaton, Label};
use rflap_core::fa::Fa;
use rflap_core::grammar::Grammar;
use rflap_core::lsystem::LSystem;
use rflap_core::mealy::{Mealy, Moore};
use rflap_core::pda::Pda;
use rflap_core::pumping::Game;
use rflap_core::regex::Regex;
use rflap_core::turing::TuringMachine;
use rflap_core::turing::catedra::with_delimiter_aliases;
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::{Map, Value};

use crate::compact;
use crate::error::{ApiError, code};

/// The document types, in the order `capabilities` lists them.
pub const TYPES: [&str; 10] = [
    "fa",
    "pda",
    "turing",
    "mealy",
    "moore",
    "grammar",
    "regex",
    "lsystem",
    "regular_pumping_lemma",
    "context_free_pumping_lemma",
];

const MAX_REGEX_NESTING: usize = 200;

/// A regular expression in JFLAP syntax (`+` union, `*` star, juxtaposition, `λ`, `ø`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegexDoc {
    pub expression: String,
}

impl RegexDoc {
    /// Parses and validates the expression.
    pub fn new(expression: &str) -> Result<Self, ApiError> {
        let doc = RegexDoc {
            expression: expression.trim().to_string(),
        };
        doc.parse()?;
        Ok(doc)
    }

    /// The expression of an already parsed regex (always parseable).
    pub fn of(regex: &Regex) -> Self {
        RegexDoc {
            expression: regex.to_string(),
        }
    }

    pub fn parse(&self) -> Result<Regex, ApiError> {
        // the parser is recursive: refuse nesting that could exhaust the stack
        let (mut depth, mut deepest) = (0usize, 0usize);
        for c in self.expression.chars() {
            match c {
                '(' => {
                    depth += 1;
                    deepest = deepest.max(depth);
                }
                ')' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if deepest > MAX_REGEX_NESTING {
            return Err(ApiError::new(
                code::INVALID_REGEX,
                format!("regular expression nested more than {MAX_REGEX_NESTING} parentheses deep"),
            ));
        }
        Regex::parse(&self.expression).map_err(|e| {
            ApiError::new(
                code::INVALID_REGEX,
                format!(
                    "regular expression `{}`: {} (at position {})",
                    self.expression, e.message, e.position
                ),
            )
            .with_details(serde_json::json!({"position": e.position}))
        })
    }
}

/// Any structure Rflap can hold. Serialized with `"type"` next to the fields of the model.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Document {
    Fa(Fa),
    Pda(Pda),
    Turing(TuringMachine),
    Mealy(Mealy),
    Moore(Moore),
    Grammar(Grammar),
    Regex(RegexDoc),
    Lsystem(LSystem),
    RegularPumpingLemma(Game),
    ContextFreePumpingLemma(Game),
}

impl Document {
    /// The `"type"` string.
    pub fn type_name(&self) -> &'static str {
        match self {
            Document::Fa(_) => "fa",
            Document::Pda(_) => "pda",
            Document::Turing(_) => "turing",
            Document::Mealy(_) => "mealy",
            Document::Moore(_) => "moore",
            Document::Grammar(_) => "grammar",
            Document::Regex(_) => "regex",
            Document::Lsystem(_) => "lsystem",
            Document::RegularPumpingLemma(_) => "regular_pumping_lemma",
            Document::ContextFreePumpingLemma(_) => "context_free_pumping_lemma",
        }
    }

    /// Whether the document is a machine drawn as a graph (fa, pda, turing, mealy, moore).
    pub fn is_automaton(&self) -> bool {
        matches!(
            self,
            Document::Fa(_)
                | Document::Pda(_)
                | Document::Turing(_)
                | Document::Mealy(_)
                | Document::Moore(_)
        )
    }

    /// The full-form JSON.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("documents always serialize")
    }

    /// Reads a document in either form and checks that it is consistent.
    pub fn from_value(value: Value) -> Result<Document, ApiError> {
        let Value::Object(mut object) = value else {
            return Err(ApiError::invalid_document(format!(
                "a document must be a JSON object with a \"type\" (one of {})",
                TYPES.join(", ")
            )));
        };
        let kind = match object.get("type") {
            Some(Value::String(kind)) => kind.clone(),
            _ => {
                return Err(ApiError::invalid_document(format!(
                    "a document needs a \"type\" string (one of {})",
                    TYPES.join(", ")
                )));
            }
        };
        if !TYPES.contains(&kind.as_str()) {
            return Err(ApiError::new(
                code::UNKNOWN_DOCUMENT_TYPE,
                format!(
                    "unknown document type `{kind}`; the types are {}",
                    TYPES.join(", ")
                ),
            ));
        }
        if compact::is_compact(&kind, &object) {
            return compact::build(&kind, &object).map(Document::with_delimiters);
        }
        object.remove("type");
        let doc = decode_full(&kind, object)?;
        doc.check()?;
        Ok(doc.with_delimiters())
    }

    /// A Turing machine with no `⟨` `⟩` whose labels use both `<` and `>` gets them turned into
    /// `⟨` and `⟩`, so every view and file shows the delimiters of an ALL.
    pub fn with_delimiters(self) -> Document {
        match self {
            Document::Turing(t) => Document::Turing(with_delimiter_aliases(&t)),
            other => other,
        }
    }

    /// Reads a document from JSON text.
    pub fn from_json(text: &str) -> Result<Document, ApiError> {
        let value = serde_json::from_str(text)
            .map_err(|e| ApiError::new(code::INVALID_JSON, e.to_string()))?;
        Document::from_value(value)
    }

    /// Integrity of the full form: every reference points to an existing state.
    fn check(&self) -> Result<(), ApiError> {
        match self {
            Document::Fa(a) => check_automaton(a, "fa"),
            Document::Pda(a) => check_automaton(a, "pda"),
            Document::Mealy(a) => check_automaton(a, "mealy"),
            Document::Moore(a) => check_automaton(a, "moore"),
            Document::Turing(t) => check_turing(t, "turing"),
            Document::Regex(r) => r.parse().map(|_| ()),
            Document::RegularPumpingLemma(_)
            | Document::ContextFreePumpingLemma(_)
            | Document::Grammar(_)
            | Document::Lsystem(_) => Ok(()),
        }
    }
}

fn decode_full(kind: &str, object: Map<String, Value>) -> Result<Document, ApiError> {
    fn model<T: serde::de::DeserializeOwned>(
        kind: &str,
        object: Map<String, Value>,
    ) -> Result<T, ApiError> {
        serde_json::from_value(Value::Object(object)).map_err(|e| {
            ApiError::invalid_document(format!(
                "the full form of a `{kind}` document is malformed ({e}); \
                 for the compact form see `capabilities`"
            ))
        })
    }
    Ok(match kind {
        "fa" => Document::Fa(model(kind, object)?),
        "pda" => Document::Pda(model(kind, object)?),
        "turing" => Document::Turing(model(kind, object)?),
        "mealy" => Document::Mealy(model(kind, object)?),
        "moore" => Document::Moore(model(kind, object)?),
        "grammar" => Document::Grammar(model(kind, object)?),
        "regex" => Document::Regex(model(kind, object)?),
        "lsystem" => Document::Lsystem(model(kind, object)?),
        "regular_pumping_lemma" => Document::RegularPumpingLemma(model(kind, object)?),
        _ => Document::ContextFreePumpingLemma(model(kind, object)?),
    })
}

fn check_automaton<L: Label>(a: &Automaton<L>, kind: &str) -> Result<(), ApiError> {
    let known = |id| a.state(id).is_some();
    for (i, t) in a.transitions().iter().enumerate() {
        if !known(t.from) || !known(t.to) {
            return Err(ApiError::new(
                code::UNKNOWN_STATE,
                format!(
                    "{kind}: transition #{i} goes from state {} to state {} but one of them does not exist",
                    t.from, t.to
                ),
            ));
        }
    }
    if let Some(i) = a.initial()
        && !known(i)
    {
        return Err(ApiError::new(
            code::UNKNOWN_STATE,
            format!("{kind}: the initial state {i} does not exist"),
        ));
    }
    if let Some(f) = a.finals().iter().find(|f| !known(**f)) {
        return Err(ApiError::new(
            code::UNKNOWN_STATE,
            format!("{kind}: the final state {f} does not exist"),
        ));
    }
    Ok(())
}

fn check_turing(t: &TuringMachine, kind: &str) -> Result<(), ApiError> {
    check_automaton(t.automaton(), kind)?;
    if let Some(bad) = t
        .automaton()
        .transitions()
        .iter()
        .find(|tr| tr.label.tapes() != t.tapes())
    {
        return Err(ApiError::invalid_document(format!(
            "{kind}: a transition has {} tape actions but the machine has {} tapes",
            bad.label.tapes(),
            t.tapes()
        )));
    }
    for (state, inner) in t.blocks() {
        if t.automaton().state(*state).is_none() {
            return Err(ApiError::new(
                code::UNKNOWN_STATE,
                format!("{kind}: the building block is on the missing state {state}"),
            ));
        }
        if inner.tapes() != t.tapes() {
            return Err(ApiError::invalid_document(format!(
                "{kind}: a building block has {} tapes but the machine has {}",
                inner.tapes(),
                t.tapes()
            )));
        }
        check_turing(inner, kind)?;
    }
    Ok(())
}

impl<'de> Deserialize<'de> for Document {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Document::from_value(value).map_err(|e| de::Error::custom(e.to_string()))
    }
}
