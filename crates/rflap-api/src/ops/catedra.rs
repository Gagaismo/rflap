//! The notation of the course "Sintaxis y Semántica de los Lenguajes": definition, transition
//! table, runs as configuration sequences, the ALL check, composition and membership by the
//! sets Tk. Each operation adapts `rflap_core::turing::catedra` or `rflap_core::grammar`.

use rflap_core::grammar::{MembershipError, membership_tk_limited};
use rflap_core::turing::TuringMachine;
use rflap_core::turing::catedra::{
    self, CatedraError, Model, RunReport, Verdict, all_check as core_all_check, compose,
    definition, run as core_run, transition_table,
};
use serde_json::{Value, json};

use super::{Object, object, to_value, wrong_type};
use crate::document::Document;
use crate::error::{ApiError, code};
use crate::request::{AllCheck, CatedraDefinition, CatedraRun, MembershipTk, OneDocument, Pair};

const DEFAULT_MAX_STEPS: usize = 1_000;
const HARD_MAX_STEPS: usize = 100_000;

fn machine<'a>(op: &str, doc: &'a Document) -> Result<&'a TuringMachine, ApiError> {
    match doc {
        Document::Turing(t) => Ok(t),
        other => Err(wrong_type(op, "a turing document", other)),
    }
}

fn catedra_error(e: CatedraError) -> ApiError {
    let error_code = match e {
        CatedraError::MultiTape { .. } | CatedraError::HasBlocks => code::UNSUPPORTED_OPTION,
        CatedraError::NoInitialState => code::NO_INITIAL_STATE,
        CatedraError::InvalidInput { .. } | CatedraError::TapeMismatch { .. } => {
            code::INVALID_ARGUMENT
        }
    };
    ApiError::new(error_code, e.to_string()).with_details(to_value(&e))
}

fn model_arg(model: Option<&str>) -> Result<Option<Model>, ApiError> {
    model
        .map(|m| {
            Model::parse(m).ok_or_else(|| {
                ApiError::new(
                    code::INVALID_ARGUMENT,
                    format!("unknown model `{m}`; the models are `mt` (Turing machine) and `all` (linear bounded automaton)"),
                )
            })
        })
        .transpose()
}

fn max_steps(requested: Option<usize>) -> Result<usize, ApiError> {
    let n = requested.unwrap_or(DEFAULT_MAX_STEPS);
    if n > HARD_MAX_STEPS {
        return Err(ApiError::new(
            code::LIMIT_EXCEEDED,
            format!("`max_steps` may be at most {HARD_MAX_STEPS}"),
        ));
    }
    Ok(n)
}

/// A run with its `accepted` flag.
fn run_value(report: &RunReport) -> Value {
    let mut out: Object = object(report);
    out.insert(
        "accepted".into(),
        json!(report.verdict == Verdict::Accepted),
    );
    Value::Object(out)
}

pub fn definition_op(r: CatedraDefinition) -> Result<Value, ApiError> {
    let tm = machine("catedra_definition", &r.document)?;
    let model = model_arg(r.model.as_deref())?;
    let d = definition(tm, model).map_err(catedra_error)?;
    Ok(to_value(&d))
}

pub fn table(r: OneDocument) -> Result<Value, ApiError> {
    let tm = machine("catedra_table", &r.document)?;
    let t = transition_table(tm).map_err(catedra_error)?;
    let mut out = object(&t);
    out.insert("markdown".into(), json!(t.to_markdown()));
    out.insert("text".into(), json!(t.to_text()));
    Ok(Value::Object(out))
}

pub fn run(r: CatedraRun) -> Result<Value, ApiError> {
    let tm = machine("catedra_run", &r.document)?;
    let model = model_arg(r.model.as_deref())?.unwrap_or_else(|| Model::detect(tm));
    let steps = max_steps(r.max_steps)?;
    let inputs = match (&r.input, &r.inputs) {
        (Some(_), Some(_)) => {
            return Err(ApiError::new(
                code::INVALID_ARGUMENT,
                "give either `input` or `inputs`, not both",
            ));
        }
        (Some(s), None) => vec![s.clone()],
        (None, Some(v)) => v.clone(),
        (None, None) => {
            return Err(ApiError::new(
                code::INVALID_ARGUMENT,
                "give `input` (a string, possibly empty) or `inputs` (a list of strings)",
            ));
        }
    };
    let mut results = Vec::new();
    for input in &inputs {
        let report = core_run(tm, input, model, steps).map_err(catedra_error)?;
        results.push(run_value(&report));
    }
    Ok(json!({"model": model, "max_steps": steps, "results": results}))
}

pub fn all_check(r: AllCheck) -> Result<Value, ApiError> {
    let tm = machine("all_check", &r.document)?;
    let steps = max_steps(r.max_steps)?;
    let inputs = r.inputs.unwrap_or_default();
    let report = core_all_check(tm, &inputs, steps).map_err(catedra_error)?;
    let runs: Vec<Value> = report.runs.iter().map(run_value).collect();
    Ok(json!({"ok": report.ok, "violations": report.violations, "runs": runs}))
}

pub fn compose_tm(r: Pair) -> Result<Value, ApiError> {
    let a = machine("compose_tm", &r.a)?;
    let b = machine("compose_tm", &r.b)?;
    let composed = compose(a, b).map_err(catedra_error)?;
    let definition = definition(&composed, None).ok();
    Ok(json!({"document": Document::Turing(composed), "definition": definition}))
}

pub fn membership_tk(r: MembershipTk) -> Result<Value, ApiError> {
    let Document::Grammar(g) = &r.grammar else {
        return Err(wrong_type("membership_tk", "a grammar", &r.grammar));
    };
    let limit = r
        .max_forms
        .unwrap_or(rflap_core::grammar::DEFAULT_MAX_FORMS);
    let report = membership_tk_limited(g, &r.input, limit).map_err(|e| {
        let error_code = match e {
            MembershipError::NoStart => code::INVALID_GRAMMAR,
            MembershipError::Contracting { .. } => code::CONTRACTING_GRAMMAR,
            MembershipError::LimitExceeded { .. } => code::LIMIT_EXCEEDED,
        };
        ApiError::new(error_code, e.to_string()).with_details(to_value(&e))
    })?;
    let mut out = object(&report);
    out.insert("text".into(), json!(report.to_text(&r.input)));
    Ok(Value::Object(out))
}

/// What `analyze` adds to a Turing machine: the multipista report, the cátedra warnings and,
/// for a one-tape machine without blocks, Σ_E (`alphabet`) and Σ_A (`tape_alphabet`) as
/// `catedra_definition` infers them.
pub fn analysis(tm: &TuringMachine) -> Object {
    let mut out = Object::new();
    out.insert("multipista".into(), to_value(&catedra::multipista(tm)));
    let definition = definition(tm, None).ok();
    let warnings: Vec<String> = definition
        .iter()
        .flat_map(|d| d.issues.iter().map(|i| i.message.clone()))
        .collect();
    out.insert("catedra_warnings".into(), json!(warnings));
    if let Some(d) = definition {
        out.insert("alphabet".into(), json!(d.input_alphabet));
        out.insert("tape_alphabet".into(), json!(d.auxiliary_alphabet));
    }
    out
}
