//! The requests: one variant per operation, tagged by `"op"`.

use rflap_core::design::{DesignOptions, LanguageSpec};
use rflap_core::layout::LayoutOptions;
use rflap_core::lsystem::DrawOptions;
use rflap_core::pumping::{Kind, Mode, Move};
use rflap_core::turing::catedra::Notation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::document::Document;
use crate::error::{ApiError, code};

/// Every operation, in the order `capabilities` lists them.
pub const OPS: [&str; 30] = [
    "capabilities",
    "open_jff",
    "save_jff",
    "jflap_check",
    "normalize",
    "simulate",
    "step",
    "multiple_run",
    "analyze",
    "convert",
    "equivalent",
    "combine",
    "grammar_type",
    "parse",
    "parse_tables",
    "user_parse",
    "layout",
    "render",
    "design",
    "explain",
    "lsystem_render",
    "pumping_languages",
    "pumping_new",
    "pumping_move",
    "catedra_definition",
    "catedra_table",
    "catedra_run",
    "all_check",
    "compose_tm",
    "membership_tk",
];

/// An operation with its arguments. In JSON: `{"op": "simulate", "document": {...}, ...}`.
/// Unknown arguments are rejected.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Capabilities,
    OpenJff(OpenJff),
    SaveJff(OneDocument),
    JflapCheck(OneDocument),
    Normalize(OneDocument),
    Simulate(Simulate),
    Step(Step),
    MultipleRun(MultipleRun),
    Analyze(OneDocument),
    Convert(Convert),
    Equivalent(Pair),
    Combine(Pair),
    GrammarType(GrammarArg),
    Parse(ParseArgs),
    ParseTables(GrammarArg),
    UserParse(UserParse),
    Layout(LayoutArgs),
    Render(RenderArgs),
    Design(DesignArgs),
    Explain(OneDocument),
    LsystemRender(LsystemRender),
    PumpingLanguages,
    PumpingNew(PumpingNew),
    PumpingMove(PumpingMove),
    CatedraDefinition(CatedraDefinition),
    CatedraTable(OneDocument),
    CatedraRun(CatedraRun),
    AllCheck(AllCheck),
    ComposeTm(Pair),
    MembershipTk(MembershipTk),
}

impl Request {
    /// The `op` string of the request.
    pub fn op(&self) -> &'static str {
        match self {
            Request::Capabilities => "capabilities",
            Request::OpenJff(_) => "open_jff",
            Request::SaveJff(_) => "save_jff",
            Request::JflapCheck(_) => "jflap_check",
            Request::Normalize(_) => "normalize",
            Request::Simulate(_) => "simulate",
            Request::Step(_) => "step",
            Request::MultipleRun(_) => "multiple_run",
            Request::Analyze(_) => "analyze",
            Request::Convert(_) => "convert",
            Request::Equivalent(_) => "equivalent",
            Request::Combine(_) => "combine",
            Request::GrammarType(_) => "grammar_type",
            Request::Parse(_) => "parse",
            Request::ParseTables(_) => "parse_tables",
            Request::UserParse(_) => "user_parse",
            Request::Layout(_) => "layout",
            Request::Render(_) => "render",
            Request::Design(_) => "design",
            Request::Explain(_) => "explain",
            Request::LsystemRender(_) => "lsystem_render",
            Request::PumpingLanguages => "pumping_languages",
            Request::PumpingNew(_) => "pumping_new",
            Request::PumpingMove(_) => "pumping_move",
            Request::CatedraDefinition(_) => "catedra_definition",
            Request::CatedraTable(_) => "catedra_table",
            Request::CatedraRun(_) => "catedra_run",
            Request::AllCheck(_) => "all_check",
            Request::ComposeTm(_) => "compose_tm",
            Request::MembershipTk(_) => "membership_tk",
        }
    }

    /// Reads a request from JSON text. The error codes are `invalid_json`, `unknown_op`,
    /// `invalid_request` and the document errors (`invalid_document`, `unknown_state`, ...).
    pub fn from_json(text: &str) -> Result<Request, ApiError> {
        let mut value: Value = serde_json::from_str(text).map_err(|e| {
            ApiError::new(
                code::INVALID_JSON,
                format!("the request is not valid JSON: {e}"),
            )
        })?;
        let Some(object) = value.as_object_mut() else {
            return Err(ApiError::new(
                code::INVALID_REQUEST,
                "a request must be a JSON object such as {\"op\": \"capabilities\"}",
            ));
        };
        match object.get("op") {
            Some(Value::String(op)) if OPS.contains(&op.as_str()) => {}
            Some(Value::String(op)) => {
                return Err(ApiError::new(
                    code::UNKNOWN_OP,
                    format!("unknown op `{op}`; the operations are {}", OPS.join(", ")),
                ));
            }
            _ => {
                return Err(ApiError::new(
                    code::INVALID_REQUEST,
                    format!(
                        "a request needs an \"op\" string, one of {}",
                        OPS.join(", ")
                    ),
                ));
            }
        }
        // Documents are expanded first so their errors keep their precise codes.
        for key in ["document", "a", "b", "game"] {
            if let Some(slot) = object.get_mut(key) {
                let doc = Document::from_value(slot.take())?;
                *slot = doc.to_value();
            }
        }
        if let Some(slot) = object.get_mut("grammar") {
            let raw = slot.take();
            let raw = match raw {
                Value::String(text) => serde_json::json!({"type": "grammar", "text": text}),
                other => other,
            };
            *slot = Document::from_value(raw)?.to_value();
        }
        serde_json::from_value(value)
            .map_err(|e| ApiError::new(code::INVALID_REQUEST, e.to_string()))
    }
}

/// `open_jff`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenJff {
    /// The text of a `.jff` file.
    pub xml: String,
}

/// Operations on one document (`save_jff`, `normalize`, `analyze`, `explain`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OneDocument {
    pub document: Document,
}

/// How λ transitions are stepped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimMode {
    /// λ transitions are steps of their own (JFLAP "step by state").
    StepByState,
    /// Every configuration is closed under λ transitions (JFLAP "step with closure").
    Closure,
}

/// When a run accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Acceptance {
    /// PDA and Turing machines: a final state.
    FinalState,
    /// PDA: the stack is empty.
    EmptyStack,
    /// Turing machines: no transition applies any more.
    Halting,
    /// Turing machines: a final state or halting.
    Both,
}

/// `simulate`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Simulate {
    pub document: Document,
    /// The input string (on every tape of a multi-tape machine).
    #[serde(default)]
    pub input: Option<String>,
    /// One input string per tape of a multi-tape Turing machine.
    #[serde(default)]
    pub inputs: Option<Vec<String>>,
    #[serde(default)]
    pub mode: Option<SimMode>,
    #[serde(default)]
    pub acceptance: Option<Acceptance>,
    /// Configurations explored before the answer is `undecided` (default 10000).
    #[serde(default)]
    pub max_configs: Option<usize>,
}

/// `step`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub document: Document,
    pub input: Option<String>,
    #[serde(default)]
    pub inputs: Option<Vec<String>>,
    /// Steps to take from the start (default 1).
    #[serde(default)]
    pub steps: Option<usize>,
    #[serde(default)]
    pub mode: Option<SimMode>,
    #[serde(default)]
    pub acceptance: Option<Acceptance>,
    /// Frontier size at which the run stops with `truncated` (default 10000).
    #[serde(default)]
    pub max_configs: Option<usize>,
}

/// One input of `multiple_run`: a string, or one string per tape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputSpec {
    One(String),
    Tapes(Vec<String>),
}

/// `multiple_run`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultipleRun {
    pub document: Document,
    pub inputs: Vec<InputSpec>,
    #[serde(default)]
    pub mode: Option<SimMode>,
    #[serde(default)]
    pub acceptance: Option<Acceptance>,
    #[serde(default)]
    pub max_configs: Option<usize>,
}

/// The targets of `convert`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConvertTo {
    /// NFA to DFA (subset construction); from a fa, a regex or a regular grammar.
    Dfa,
    /// Minimal DFA; from a fa, a regex or a regular grammar.
    MinimalDfa,
    /// FA to regular expression.
    Regex,
    /// Regular expression to NFA.
    Nfa,
    /// FA to right-linear grammar.
    RegularGrammar,
    /// Right-linear grammar to FA.
    Fa,
    /// CFG to PDA, LL construction.
    PdaLl,
    /// CFG to PDA, LR construction.
    PdaLr,
    /// PDA to CFG. A PDA that is not in JFLAP's form is rewritten first (`pda_jflap_form`).
    Cfg,
    /// Rflap addition: rewrites a PDA into the form `cfg` requires (one final state reached on
    /// an empty stack, every transition pops one symbol and pushes zero or two).
    PdaJflapForm,
    /// One-tape Turing machine to unrestricted grammar.
    UnrestrictedGrammar,
    RemoveLambda,
    RemoveUnit,
    RemoveUseless,
    /// Chomsky normal form (λ, unit and useless removal first, as JFLAP's "Transform Grammar").
    Cnf,
    /// Adds the trap state to a FA.
    Complete,
    /// Replaces multi-character FA labels by chains of single-character ones.
    SplitLabels,
    /// Drops the unreachable and useless states of an automaton.
    RemoveUselessStates,
}

impl ConvertTo {
    pub const ALL: [ConvertTo; 18] = [
        ConvertTo::Dfa,
        ConvertTo::MinimalDfa,
        ConvertTo::Regex,
        ConvertTo::Nfa,
        ConvertTo::RegularGrammar,
        ConvertTo::Fa,
        ConvertTo::PdaLl,
        ConvertTo::PdaLr,
        ConvertTo::Cfg,
        ConvertTo::PdaJflapForm,
        ConvertTo::UnrestrictedGrammar,
        ConvertTo::RemoveLambda,
        ConvertTo::RemoveUnit,
        ConvertTo::RemoveUseless,
        ConvertTo::Cnf,
        ConvertTo::Complete,
        ConvertTo::SplitLabels,
        ConvertTo::RemoveUselessStates,
    ];

    /// The `to` string.
    pub fn name(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default()
    }
}

/// `convert`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Convert {
    pub document: Document,
    pub to: ConvertTo,
}

/// `equivalent` and `combine`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pair {
    pub a: Document,
    pub b: Document,
}

/// `grammar_type` and `parse_tables`: a grammar document (or its text, `S -> aSb | λ`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrammarArg {
    pub grammar: Document,
}

/// The parsers of `parse`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parser {
    /// Brute force; the restricted or unrestricted search is picked from the grammar.
    Brute,
    /// Same search, named for unrestricted grammars.
    BruteUnrestricted,
    Ll1,
    Slr1,
    Cyk,
}

/// `parse`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParseArgs {
    pub grammar: Document,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub inputs: Option<Vec<String>>,
    /// Default `brute`.
    #[serde(default)]
    pub parser: Option<Parser>,
    /// Candidate forms a brute-force search may examine per input (default 20000).
    #[serde(default)]
    pub budget: Option<usize>,
}

/// One move of `user_parse`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserMove {
    /// `"undo"`.
    Undo(UndoMove),
    /// `{"production": 0}` (first occurrence), `{"production": 0, "position": 2}` or
    /// `{"production": 0, "positions": [1, 3]}`.
    Apply {
        production: usize,
        #[serde(default)]
        position: Option<usize>,
        #[serde(default)]
        positions: Option<Vec<usize>>,
    },
}

/// The literal string `"undo"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UndoMove {
    Undo,
}

/// `user_parse`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserParse {
    pub grammar: Document,
    /// The string to derive.
    pub target: String,
    /// Moves applied in order from the start variable.
    #[serde(default)]
    pub moves: Vec<UserMove>,
}

/// `layout`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutArgs {
    pub document: Document,
    /// `circle` (default), `gem`, `random`, `spiral`, `tree`, `tree-degree`, `two-circle`.
    #[serde(default)]
    pub algorithm: Option<String>,
    #[serde(default)]
    pub options: Option<LayoutOptions>,
}

/// The output formats of `render`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Svg,
    Markdown,
    Dot,
    Text,
}

/// A state named by its name or its id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StateRef {
    Name(String),
    Id(u32),
}

/// `render`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderArgs {
    pub document: Document,
    /// Default `svg`.
    #[serde(default)]
    pub format: Option<Format>,
    #[serde(default)]
    pub highlight_states: Vec<StateRef>,
    /// Indices into the transitions of the document (see `analyze`).
    #[serde(default)]
    pub highlight_transitions: Vec<usize>,
    #[serde(default)]
    pub dark: bool,
    /// Rewrite level when rendering an L-system (default 3).
    #[serde(default)]
    pub depth: Option<usize>,
    /// How the labels of a Turing machine are written: `catedra` (default; `(e,s,m)` with `Δ`
    /// and `I`/`N`/`D`) or `jflap` (`a ; b , R` with `□`). Other machines ignore it.
    #[serde(default)]
    pub notation: Option<Notation>,
    /// Write the labels of parallel transitions on one line joined with ` / ` instead of
    /// stacking them (default false).
    #[serde(default)]
    pub join_labels: bool,
}

/// `design`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignArgs {
    pub spec: LanguageSpec,
    #[serde(default)]
    pub options: Option<DesignOptions>,
}

/// `lsystem_render`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LsystemRender {
    pub document: Document,
    /// Rewrite level (default 3, at most 12).
    #[serde(default)]
    pub depth: Option<usize>,
    #[serde(default)]
    pub options: Option<DrawOptions>,
}

/// A predefined pumping lemma language: its title or its index in the chooser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LanguageRef {
    Title(String),
    Index(usize),
}

/// Which pumping lemma.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PumpKind {
    Regular,
    ContextFree,
}

impl From<PumpKind> for Kind {
    fn from(kind: PumpKind) -> Kind {
        match kind {
            PumpKind::Regular => Kind::Regular,
            PumpKind::ContextFree => Kind::ContextFree,
        }
    }
}

/// `pumping_new`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PumpingNew {
    pub kind: PumpKind,
    pub language: LanguageRef,
    /// Default `computer_first`.
    #[serde(default)]
    pub mode: Option<Mode>,
    /// Seed of the computer's random choices (default 1).
    #[serde(default)]
    pub seed: Option<i64>,
}

/// `pumping_move`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PumpingMove {
    pub game: Document,
    #[serde(rename = "move")]
    pub mv: Move,
}

/// `catedra_definition`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatedraDefinition {
    pub document: Document,
    /// `mt` or `all`; default: `all` when a transition mentions `⟨` or `⟩`, otherwise `mt`.
    #[serde(default)]
    pub model: Option<String>,
}

/// `catedra_run`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatedraRun {
    pub document: Document,
    /// One input (possibly empty).
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub inputs: Option<Vec<String>>,
    /// `mt` or `all`; default: detected from the transitions.
    #[serde(default)]
    pub model: Option<String>,
    /// Transitions applied before the run is reported as not halting (default 1000).
    #[serde(default)]
    pub max_steps: Option<usize>,
    /// Writing `⟨` over another symbol moving right (or `⟩` moving left) is valid in an ALL
    /// (default `true`); `false` restores the strict rule.
    #[serde(default)]
    pub allow_shrink: Option<bool>,
}

/// `all_check`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllCheck {
    pub document: Document,
    /// Inputs to run as an ALL for the dynamic check.
    #[serde(default)]
    pub inputs: Option<Vec<String>>,
    #[serde(default)]
    pub max_steps: Option<usize>,
    /// See `catedra_run`.
    #[serde(default)]
    pub allow_shrink: Option<bool>,
}

/// `membership_tk`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipTk {
    pub grammar: Document,
    pub input: String,
    /// Forms the sets may hold before the search gives up (default 100000).
    #[serde(default)]
    pub max_forms: Option<usize>,
}
