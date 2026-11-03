//! Rflap API: the single JSON-in/JSON-out layer over [`rflap_core`].
//!
//! The CLI, the WASM web editor and the MCP server all call [`call`] (or [`handle`]) and
//! nothing else: this crate only adapts `rflap-core`, it implements no algorithm. It has no
//! threads, files or clocks, so it compiles for `wasm32-unknown-unknown`, and its output is
//! deterministic (object keys are sorted, layouts are seeded).
//!
//! ```
//! let answer = rflap_api::call(
//!     r#"{"op": "simulate", "input": "ab", "document": {
//!         "type": "fa", "initial": "q0", "finals": ["q2"],
//!         "transitions": [["q0", "a", "q1"], ["q1", "b", "q2"]]}}"#,
//! );
//! let answer: serde_json::Value = serde_json::from_str(&answer).unwrap();
//! assert_eq!(answer["ok"], true);
//! assert_eq!(answer["result"]["outcome"], "accepted");
//! ```
//!
//! # Requests and responses
//!
//! A request is one JSON object tagged by `"op"` ([`Request`]); unknown arguments are
//! rejected. The answer is `{"ok": true, "op": "<op>", "result": {...}}` ([`Response`]) or
//! `{"ok": false, "error": {"code", "message", "details"?}}` ([`ApiError`]). [`call`] never
//! panics: bad JSON, unknown operations, bad documents and even a panic inside an operation
//! come back as `ok: false`. Error codes ([`code`]): `invalid_json`, `invalid_request`,
//! `unknown_op`, `invalid_document`, `unknown_document_type`, `unknown_state`,
//! `invalid_label`, `invalid_grammar`, `invalid_regex`, `wrong_document_type`,
//! `unsupported_option`, `invalid_argument`, `no_initial_state`, `jff_error`,
//! `conversion_failed`, `design_error`, `parse_error`, `invalid_move`, `limit_exceeded`,
//! `contracting_grammar`, `internal`.
//!
//! `{"op": "capabilities"}` describes everything below (operations with an example each,
//! document types with their compact syntax, conversions): it is what an AI reads to learn
//! the tool.
//!
//! # Documents
//!
//! A [`Document`] is a JSON object with a `"type"`: `fa`, `pda`, `turing`, `mealy`, `moore`,
//! `grammar`, `regex`, `lsystem`, `regular_pumping_lemma` or `context_free_pumping_lemma`.
//! Wherever a document is expected two forms are accepted:
//!
//! * the **full form**, the serde of the `rflap-core` models next to `"type"` (states by id with
//!   their coordinates, transitions with their labels); every operation returns it, and
//! * the **compact form**, short enough for an AI to write by hand. States are created by
//!   name in order of first use (initial, then transitions, then finals) and laid out
//!   automatically with [`rflap_core::layout`] unless `"points"` gives coordinates.
//!
//! The common fields of the compact machines:
//!
//! | field | meaning |
//! |---|---|
//! | `initial` | name of the initial state |
//! | `finals` | names of the final states |
//! | `transitions` | `[from, label, to]` (or `{"from", "label", "to"}`); the label syntax depends on the type |
//! | `states` | optional list of names; when present every name used elsewhere must be in it, otherwise `unknown_state` |
//! | `points` | optional `{"q0": [x, y]}` (state name to coordinates); states without a point are laid out around the fixed ones |
//! | `layout` | optional algorithm for the states without a point: `layered` (default: the initial state on the far left, the others in columns by BFS distance from it, unreachable ones last), `circle`, `gem`, `random`, `spiral`, `tree`, `tree-degree`, `two-circle` |
//! | `title` | optional string (alias `name`), accepted by every compact type; it is metadata that the operations ignore |
//!
//! Other unknown fields are errors. `λ` and `ε` mean the empty string in every label.
//!
//! ## Finite automaton
//!
//! ```json
//! {"type": "fa", "initial": "q0", "finals": ["q2"],
//!  "transitions": [["q0", "a", "q1"], ["q1", "", "q2"], ["q2", "[0-9]", "q2"]]}
//! ```
//!
//! A label is the string read (`""` is λ); the exact form `[x-y]` reads a character range.
//!
//! ## Pushdown automaton
//!
//! ```json
//! {"type": "pda", "initial": "q0", "finals": ["q2"],
//!  "transitions": [["q0", "a,Z;AZ", "q0"], ["q0", "b,A;λ", "q1"], ["q1", "λ,Z;Z", "q2"]]}
//! ```
//!
//! A label is `input,pop;push` in JFLAP text; the stack starts with `Z` and the first symbol
//! of `pop`/`push` is the top.
//!
//! ## Turing machine
//!
//! ```json
//! {"type": "turing", "initial": "q0", "finals": ["q1"],
//!  "transitions": [["q0", "a;b,R", "q0"], ["q0", "_;_,S", "q1"]]}
//! ```
//!
//! A label is `read;write,direction` per tape joined by `|` (`a;a,R|_;x,S` for two tapes);
//! the direction is `L`, `R` or `S` and `_` is the blank. A read can also be `~` (any
//! symbol), `!x` (anything but `x`) or `a,b}v` (remember in variable `v`). The number of tapes
//! is read from the first label (or `"tapes"`). `"blocks": {"state": <compact turing>}` turns
//! a state into a building block that runs an inner machine.
//!
//! The notation of the course "Sintaxis y Semántica de los Lenguajes" is also read: a one-tape label
//! can be `(read,write,move)` or `read,write,move` (move `I`, `N`, `D`; `Δ` is the blank), several
//! labels of one edge are joined with ` / `, and `"input_alphabet": "ab"` (or `["a", "b"]`) declares
//! Σ_E. See the `catedra_*` operations below.
//!
//! ## Mealy and Moore machines
//!
//! ```json
//! {"type": "mealy", "initial": "q0", "transitions": [["q0", "a;1", "q0"], ["q0", "b;0", "q0"]]}
//! {"type": "moore", "initial": "q0", "outputs": {"q0": "x", "q1": "y"},
//!  "transitions": [["q0", "a", "q1"], ["q1", "a", "q0"]]}
//! ```
//!
//! A Mealy label is `input;output`; a Moore label is the string read and `outputs` gives the
//! output of each state.
//!
//! ## Grammar and regular expression
//!
//! ```json
//! {"type": "grammar", "text": "S -> aSb | λ"}
//! {"type": "grammar", "rules": ["S -> aSb", "S -> λ"], "start": "S"}
//! {"type": "regex", "expression": "(a+b)*abb"}
//! ```
//!
//! An uppercase letter is a variable and anything else a terminal; the start variable is the
//! left side of the first production unless `"start"` says otherwise. In a regular
//! expression `+` is union, `*` star, juxtaposition concatenation, `λ` the empty string and
//! `ø` the empty language. Operations that take a `"grammar"` also accept the text itself:
//! `{"op": "parse_tables", "grammar": "S -> aSb | λ"}`. The full form of a grammar is
//! `{"type": "grammar", "start": "S", "productions": [{"lhs": "S", "rhs": "aSb"}]}` and that of
//! a regex `{"type": "regex", "expression": "..."}` (the expression is the model).
//!
//! ## L-system and pumping lemma games
//!
//! ```json
//! {"type": "lsystem", "axiom": "F", "rules": ["F -> F + F - F - F + F"],
//!  "parameters": {"angle": "90", "distance": "4"}}
//! {"type": "regular_pumping_lemma", "language": 0, "mode": "computer_first", "seed": 1}
//! ```
//!
//! L-system symbols are separated by spaces (`"single_char_symbols": true` reads `F+F` as
//! three symbols; `rules` may also be an object `{"F": "F + F"}`). A pumping lemma `language`
//! is an index or a title from `pumping_languages`; the document is the whole game and
//! `pumping_move` plays it.
//!
//! # Operations
//!
//! Every result is a JSON object; the keys listed are the stable top-level ones. Nested
//! structures that come straight from the core (`steps` of conversions, `derivation`, `tree`,
//! the SLR automaton) keep the serde of `rflap-core`.
//!
//! | op | request | result |
//! |---|---|---|
//! | `capabilities` | `{"op": "capabilities"}` | `ops`, `document_types`, `convert`, `parsers`, `layouts`, `formats`, `error_codes`, `notes` |
//! | `open_jff` | `{"op": "open_jff", "xml": "<?xml ...>"}` | `document` (any `.jff` type, L-systems and pumping games included) |
//! | `save_jff` | `{"op": "save_jff", "document": {...}}` | `xml`, `type` |
//! | `normalize` | `{"op": "normalize", "document": {...}}` | `document` (full form) |
//! | `simulate` | `{"op": "simulate", "document": {...}, "input": "abb", "mode": "closure", "acceptance": "empty_stack", "max_configs": 10000}` | `outcome` (`accepted`/`rejected`/`undecided`), `accepted`, `trace`, `output` (Mealy/Moore), `tape_outputs` (Turing: the final content of each tape, blanks around it trimmed) |
//! | `step` | `{"op": "step", "document": {...}, "input": "abb", "steps": 2}` | `steps_done`, `finished`, `truncated`, `frontier`, `accepted` (`config`, `trace`), `rejected` |
//! | `multiple_run` | `{"op": "multiple_run", "document": {...}, "inputs": ["ab", "ba"]}` | `results`: `input`, `outcome`, `accepted`, `output`/`tape_outputs` |
//! | `analyze` | `{"op": "analyze", "document": {...}}` | `type`, `kind` (`dfa`, `nfa`, `dpda`, `npda`, `turing`, `mealy`, `moore`), `alphabet`, `states`, `initial`, `finals`, `deterministic`, `nondeterministic_states`, `lambda_transitions`, `unreachable_states`, `useless_states`, `complete` (fa), `multipista` (`tapes`, `applicable`, `is_multipista`, `offending`) and `catedra_warnings` (list of messages) for a turing machine |
//! | `convert` | `{"op": "convert", "document": {...}, "to": "minimal_dfa"}` | `to`, `document`, `steps`, plus conversion-specific keys |
//! | `equivalent` | `{"op": "equivalent", "a": {...}, "b": {...}}` | `equivalent`, `counterexample` |
//! | `combine` | `{"op": "combine", "a": {...}, "b": {...}}` | `document` |
//! | `grammar_type` | `{"op": "grammar_type", "grammar": "S -> aSb \| λ"}` | `classification`, `message`, `report` |
//! | `parse` | `{"op": "parse", "grammar": "...", "inputs": ["ab"], "parser": "ll1", "budget": 20000}` | `parser`, `results`: `input`, `outcome` (also `error`), `derivation_forms`, `derivation`, `tree`, `steps`/`table` |
//! | `parse_tables` | `{"op": "parse_tables", "grammar": "..."}` | `first`, `follow`, `ll1`, `slr1` |
//! | `user_parse` | `{"op": "user_parse", "grammar": "...", "target": "aabb", "moves": [{"production": 0}, "undo"]}` | `sentential`, `finished`, `derivation_forms`, `derivation`, `tree`, `applicable` |
//! | `layout` | `{"op": "layout", "document": {...}, "algorithm": "tree", "options": {"seed": 7}}` | `document` |
//! | `render` | `{"op": "render", "document": {...}, "format": "svg", "highlight_states": ["q1"], "highlight_transitions": [0], "dark": false}` | `format`, `content` |
//! | `design` | `{"op": "design", "spec": {"alphabet": "ab", "language": {"constraint_text": "ends_with(\"ab\")"}, "examples": {...}}}` | `document`, `kind`, `description`, `regex`, `states`, `stats`, `accepted`, `verification` |
//! | `explain` | `{"op": "explain", "document": {...}}` | `markdown` |
//! | `lsystem_render` | `{"op": "lsystem_render", "document": {...}, "depth": 3}` | `svg`, `depth`, `expansion_length`, `expansion` |
//! | `pumping_languages` | `{"op": "pumping_languages"}` | `languages`: `kind`, `index`, `title`, `explanation`, ... |
//! | `pumping_new` | `{"op": "pumping_new", "kind": "regular", "language": 0, "mode": "computer_first"}` | `document`, `view`, `lemma`, `result`, `conclusion` |
//! | `pumping_move` | `{"op": "pumping_move", "game": {...}, "move": {"move": "choose_w", "w": "aabb"}}` | like `pumping_new` plus `reply` |
//! | `catedra_definition` | `{"op": "catedra_definition", "document": {...}, "model": "mt"}` | `model`, `states`, `input_alphabet`, `auxiliary_alphabet`, `initial`, `finals`, `input_alphabet_inferred`, `deterministic`, `issues` (`kind`, `message`, `transitions`), `text` |
//! | `catedra_table` | `{"op": "catedra_table", "document": {...}}` | `header`, `rows`, `markdown`, `text` |
//! | `catedra_run` | `{"op": "catedra_run", "document": {...}, "inputs": ["aba"], "model": "mt", "max_steps": 1000}` | `model`, `max_steps`, `results`: `input`, `model`, `verdict` (`accepted`/`rejected`/`no_halt`/`violation`), `accepted`, `steps`, `configurations`, `sequence`, `violation`? |
//! | `all_check` | `{"op": "all_check", "document": {...}, "inputs": ["ab"]}` | `ok`, `violations` (`kind`, `message`, `transition`, `transition_text`, `step`?, `input`?), `runs` |
//! | `compose_tm` | `{"op": "compose_tm", "a": {...}, "b": {...}}` | `document`, `definition` |
//! | `membership_tk` | `{"op": "membership_tk", "grammar": "S -> T", "input": "abbc", "max_forms": 100000}` | `member`, `bound`, `sets`, `text` |
//!
//! `convert` targets (`to`): `dfa`, `minimal_dfa` (from a fa, a regex or a regular grammar),
//! `regex` and `regular_grammar` (from a fa), `nfa` (from a regex), `fa` (from a right-linear
//! grammar), `pda_ll` and `pda_lr` (from a grammar), `cfg` (from a pda; one that is not in JFLAP's form is
//! rewritten first and `steps` starts with a `pda_to_jflap_form` entry), `pda_jflap_form` (that rewriting
//! alone),
//! `unrestricted_grammar` (from a one-tape Turing machine), `remove_lambda`, `remove_unit`,
//! `remove_useless`, `cnf` (grammars), `complete` (adds the trap state), `split_labels` and
//! `remove_useless_states`.
//!
//! Parsers: `brute`, `brute_unrestricted`, `ll1`, `slr1`, `cyk`. A parse that cannot be tried
//! (CYK on a grammar that is not in Chomsky normal form, LL(1) on a grammar that is not LL(1),
//! a symbol outside the grammar) is reported inside `results` with `outcome: "error"`.
//!
//! Operations that stop on a budget (`max_configs`, `budget`) answer `undecided` instead of
//! running forever; `depth` of an L-system and sizes of runs are capped with `limit_exceeded`.

mod compact;
mod document;
mod error;
mod ops;
mod request;

use std::panic::{AssertUnwindSafe, catch_unwind};

pub use document::{Document, RegexDoc, TYPES};
pub use error::{ApiError, code};
pub use request::{
    Acceptance, AllCheck, CatedraDefinition, CatedraRun, Convert, ConvertTo, DesignArgs, Format,
    GrammarArg, InputSpec, LanguageRef, LayoutArgs, LsystemRender, MembershipTk, MultipleRun, OPS,
    OneDocument, OpenJff, Pair, ParseArgs, Parser, PumpKind, PumpingMove, PumpingNew, RenderArgs,
    Request, SimMode, Simulate, StateRef, Step, UndoMove, UserMove, UserParse,
};

use serde::Serialize;
use serde_json::Value;

/// A successful answer: `{"ok": true, "op": "<op>", "result": {...}}`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Response {
    pub ok: bool,
    pub op: String,
    pub result: Value,
}

/// Runs a request.
pub fn handle(request: Request) -> Result<Response, ApiError> {
    ops::dispatch(request)
}

fn error_json(error: &ApiError) -> String {
    serde_json::json!({"ok": false, "error": error}).to_string()
}

/// The JSON entry point: a request as text in, the response as text out. Never panics.
pub fn call(request_json: &str) -> String {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        Request::from_json(request_json).and_then(handle)
    }));
    match outcome {
        Ok(Ok(response)) => serde_json::to_string(&response)
            .unwrap_or_else(|e| error_json(&ApiError::new(code::INTERNAL, e.to_string()))),
        Ok(Err(error)) => error_json(&error),
        Err(panic) => {
            let message = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            error_json(&ApiError::new(
                code::INTERNAL,
                format!("the operation panicked: {message}"),
            ))
        }
    }
}
