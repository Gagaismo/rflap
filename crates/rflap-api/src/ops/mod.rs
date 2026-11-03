//! The operations: each one adapts `rflap-core` and never re-implements an algorithm.

use rflap_core::automaton::{Automaton, Label, StateId};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::Response;
use crate::document::Document;
use crate::error::{ApiError, code};
use crate::request::Request;

mod capabilities;
mod catedra;
mod convert;
mod grammar;
mod inspect;
mod jff;
mod misc;
mod sim;

type Object = Map<String, Value>;

/// Runs a request.
pub fn dispatch(request: Request) -> Result<Response, ApiError> {
    let op = request.op();
    let result = match request {
        Request::Capabilities => capabilities::capabilities(),
        Request::OpenJff(r) => jff::open(&r.xml)?,
        Request::SaveJff(r) => jff::save(&r.document)?,
        Request::Normalize(r) => json!({"document": r.document}),
        Request::Simulate(r) => sim::simulate(r)?,
        Request::Step(r) => sim::step(r)?,
        Request::MultipleRun(r) => sim::multiple_run(r)?,
        Request::Analyze(r) => inspect::analyze(&r.document)?,
        Request::Convert(r) => convert::convert(&r.document, r.to)?,
        Request::Equivalent(r) => convert::equivalent(&r.a, &r.b)?,
        Request::Combine(r) => inspect::combine(&r.a, &r.b)?,
        Request::GrammarType(r) => grammar::grammar_type(&r.grammar)?,
        Request::Parse(r) => grammar::parse(r)?,
        Request::ParseTables(r) => grammar::parse_tables(&r.grammar)?,
        Request::UserParse(r) => grammar::user_parse(r)?,
        Request::Layout(r) => inspect::layout(r)?,
        Request::Render(r) => inspect::render(r)?,
        Request::Design(r) => misc::design(r)?,
        Request::Explain(r) => inspect::explain(&r.document)?,
        Request::LsystemRender(r) => misc::lsystem_render(r)?,
        Request::PumpingLanguages => misc::pumping_languages(),
        Request::PumpingNew(r) => misc::pumping_new(r)?,
        Request::PumpingMove(r) => misc::pumping_move(r)?,
        Request::CatedraDefinition(r) => catedra::definition_op(r)?,
        Request::CatedraTable(r) => catedra::table(r)?,
        Request::CatedraRun(r) => catedra::run(r)?,
        Request::AllCheck(r) => catedra::all_check(r)?,
        Request::ComposeTm(r) => catedra::compose_tm(r)?,
        Request::MembershipTk(r) => catedra::membership_tk(r)?,
    };
    Ok(Response {
        ok: true,
        op: op.to_string(),
        result,
    })
}

/// Runs `$body` on the automaton of a graph-like document, `$other` on any other document.
macro_rules! with_automaton {
    ($doc:expr, $a:ident => $body:expr, _ => $other:expr) => {
        match $doc {
            $crate::document::Document::Fa($a) => $body,
            $crate::document::Document::Pda($a) => $body,
            $crate::document::Document::Mealy($a) => $body,
            $crate::document::Document::Moore($a) => $body,
            $crate::document::Document::Turing(machine) => {
                let $a = machine.automaton();
                $body
            }
            _ => $other,
        }
    };
}
pub(crate) use with_automaton;

/// The value of a core result as a JSON object.
pub(crate) fn object<T: Serialize>(value: &T) -> Object {
    match serde_json::to_value(value) {
        Ok(Value::Object(map)) => map,
        Ok(other) => {
            let mut map = Map::new();
            map.insert("value".to_string(), other);
            map
        }
        Err(_) => Map::new(),
    }
}

pub(crate) fn to_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// The name of a state, `q<id>` when it is not there.
pub(crate) fn state_name<L: Label>(a: &Automaton<L>, id: StateId) -> String {
    a.state(id)
        .map_or_else(|| format!("q{id}"), |s| s.name.clone())
}

pub(crate) fn no_initial_state() -> ApiError {
    ApiError::new(
        code::NO_INITIAL_STATE,
        "the automaton has no initial state: add \"initial\": \"<state>\"",
    )
}

pub(crate) fn wrong_type(op: &str, wanted: &str, doc: &Document) -> ApiError {
    ApiError::wrong_type(op, wanted, doc.type_name())
}

pub(crate) fn core_error<E: Serialize + std::fmt::Display>(error_code: &str, e: &E) -> ApiError {
    ApiError::new(error_code, e.to_string()).with_details(to_value(e))
}
