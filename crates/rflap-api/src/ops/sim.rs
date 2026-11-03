//! `simulate`, `step` and `multiple_run`: one adapter turns every core machine into the same
//! JSON shapes.

use rflap_core::fa::{FaConfig, FaStepByState, FaWithClosure};
use rflap_core::mealy::{MealyConfig, MealyStepByState, MooreStepByState};
use rflap_core::pda::{Acceptance as PdaAcceptance, PdaConfig, PdaStepByState, PdaWithClosure};
use rflap_core::sim::{Machine, RunOutcome, StepRun, fast_run};
use rflap_core::turing::{AcceptanceFilters, TmConfig, TmSimulator, TuringMachine, tape_contents};
use serde_json::{Map, Value, json};

use super::{Object, no_initial_state, state_name};
use crate::document::Document;
use crate::error::{ApiError, code};
use crate::request::{Acceptance, InputSpec, MultipleRun, SimMode, Simulate, Step};

const DEFAULT_MAX_CONFIGS: usize = 10_000;
const HARD_MAX_CONFIGS: usize = 1_000_000;

/// What a run is given: one string, or one string per tape.
enum Input {
    One(String),
    Tapes(Vec<String>),
}

impl Input {
    fn resolve(input: &Option<String>, inputs: &Option<Vec<String>>) -> Result<Input, ApiError> {
        match (input, inputs) {
            (Some(_), Some(_)) => Err(ApiError::new(
                code::INVALID_ARGUMENT,
                "give either `input` or `inputs`, not both",
            )),
            (Some(s), None) => Ok(Input::One(s.clone())),
            (None, Some(v)) => Ok(Input::Tapes(v.clone())),
            (None, None) => Err(ApiError::new(
                code::INVALID_ARGUMENT,
                "give `input` (a string, possibly empty) or `inputs` (one string per tape)",
            )),
        }
    }
}

fn limit(max_configs: Option<usize>, what: &str) -> Result<usize, ApiError> {
    let n = max_configs.unwrap_or(DEFAULT_MAX_CONFIGS);
    if n > HARD_MAX_CONFIGS {
        return Err(ApiError::new(
            code::LIMIT_EXCEEDED,
            format!("{what} may be at most {HARD_MAX_CONFIGS}"),
        ));
    }
    Ok(n)
}

/// A machine plus the way its configurations are shown.
trait Runner {
    fn run(&self, input: &str, max_configs: usize) -> Object;
    fn step(&self, input: &str, steps: usize, cap: usize) -> Value;
}

struct Adapter<M: Machine, V, X> {
    machine: M,
    view: V,
    extra: X,
}

impl<M, V, X> Runner for Adapter<M, V, X>
where
    M: Machine,
    V: Fn(&M::Config) -> Value,
    X: Fn(&RunOutcome<M::Config>, &mut Object),
{
    fn run(&self, input: &str, max_configs: usize) -> Object {
        let outcome = fast_run(&self.machine, input, max_configs);
        let mut out = Map::new();
        let (name, trace) = match &outcome {
            RunOutcome::Accepted { trace } => ("accepted", trace.iter().map(&self.view).collect()),
            RunOutcome::Rejected => ("rejected", Vec::new()),
            RunOutcome::Undecided => ("undecided", Vec::new()),
        };
        out.insert("outcome".into(), json!(name));
        out.insert("accepted".into(), json!(outcome.is_accepted()));
        out.insert("trace".into(), Value::Array(trace));
        (self.extra)(&outcome, &mut out);
        out
    }

    fn step(&self, input: &str, steps: usize, cap: usize) -> Value {
        let mut run = StepRun::new(&self.machine, input);
        let (mut done, mut truncated) = (0, false);
        while done < steps && !run.frontier().is_empty() {
            run.step();
            done += 1;
            if run.frontier().len() > cap {
                truncated = true;
                break;
            }
        }
        let configs = |ids: &[usize]| -> Vec<Value> {
            ids.iter().map(|&id| (self.view)(run.config(id))).collect()
        };
        let accepted: Vec<Value> = run
            .accepted()
            .iter()
            .map(|&id| {
                json!({
                    "config": (self.view)(run.config(id)),
                    "trace": run.trace(id).iter().map(&self.view).collect::<Vec<_>>(),
                })
            })
            .collect();
        json!({
            "steps_done": done,
            "finished": run.frontier().is_empty(),
            "truncated": truncated,
            "frontier": configs(run.frontier()),
            "accepted": accepted,
            "rejected": configs(run.rejected()),
        })
    }
}

fn boxed<'a, M, V, X>(machine: M, view: V, extra: X) -> Box<dyn Runner + 'a>
where
    M: Machine + 'a,
    V: Fn(&M::Config) -> Value + 'a,
    X: Fn(&RunOutcome<M::Config>, &mut Object) + 'a,
{
    Box::new(Adapter {
        machine,
        view,
        extra,
    })
}

fn unsupported(message: String) -> ApiError {
    ApiError::new(code::UNSUPPORTED_OPTION, message)
}

/// The name of the innermost state of a Turing machine configuration, `block/inner` inside
/// building blocks.
fn tm_state_name(tm: &TuringMachine, path: &[u32]) -> String {
    let mut names = Vec::new();
    let mut machine = Some(tm);
    for id in path {
        let Some(m) = machine else { break };
        names.push(state_name(m.automaton(), *id));
        machine = m.block(*id);
    }
    names.join("/")
}

fn check_mode(doc: &Document, mode: Option<SimMode>) -> Result<(), ApiError> {
    if mode == Some(SimMode::Closure) && !matches!(doc, Document::Fa(_) | Document::Pda(_)) {
        return Err(unsupported(format!(
            "mode `closure` applies to fa and pda documents, not to `{}`",
            doc.type_name()
        )));
    }
    Ok(())
}

fn build<'a>(
    doc: &'a Document,
    mode: Option<SimMode>,
    acceptance: Option<Acceptance>,
    tape_inputs: Option<&[String]>,
) -> Result<Box<dyn Runner + 'a>, ApiError> {
    check_mode(doc, mode)?;
    let closure = mode == Some(SimMode::Closure);
    if !matches!(doc, Document::Pda(_) | Document::Turing(_)) && acceptance.is_some() {
        return Err(unsupported(format!(
            "`acceptance` applies to pda and turing documents, not to `{}`",
            doc.type_name()
        )));
    }
    if tape_inputs.is_some() && !matches!(doc, Document::Turing(_)) {
        return Err(ApiError::new(
            code::INVALID_ARGUMENT,
            format!(
                "`inputs` (one string per tape) only applies to turing documents, not to `{}`",
                doc.type_name()
            ),
        ));
    }
    match doc {
        Document::Fa(fa) => {
            if fa.initial().is_none() {
                return Err(no_initial_state());
            }
            let view = move |c: &FaConfig| {
                json!({"state": state_name(fa, c.state), "state_id": c.state,
                       "remaining": c.remaining})
            };
            let none = |_: &RunOutcome<FaConfig>, _: &mut Object| {};
            Ok(if closure {
                boxed(FaWithClosure::new(fa), view, none)
            } else {
                boxed(FaStepByState::new(fa), view, none)
            })
        }
        Document::Pda(pda) => {
            if pda.initial().is_none() {
                return Err(no_initial_state());
            }
            let accept = match acceptance {
                None | Some(Acceptance::FinalState) => PdaAcceptance::FinalState,
                Some(Acceptance::EmptyStack) => PdaAcceptance::EmptyStack,
                Some(other) => {
                    return Err(unsupported(format!(
                        "pda acceptance is `final_state` or `empty_stack`, not `{}`",
                        super::to_value(&other).as_str().unwrap_or("?")
                    )));
                }
            };
            let view = move |c: &PdaConfig| {
                json!({"state": state_name(pda, c.state), "state_id": c.state,
                       "remaining": c.remaining, "stack": c.stack})
            };
            let none = |_: &RunOutcome<PdaConfig>, _: &mut Object| {};
            Ok(if closure {
                boxed(PdaWithClosure::new(pda, accept), view, none)
            } else {
                boxed(PdaStepByState::new(pda, accept), view, none)
            })
        }
        Document::Mealy(m) => {
            if m.initial().is_none() {
                return Err(no_initial_state());
            }
            let view = move |c: &MealyConfig| {
                json!({"state": state_name(m, c.state), "state_id": c.state,
                       "remaining": c.remaining, "output": c.output})
            };
            Ok(boxed(MealyStepByState::new(m), view, output_of))
        }
        Document::Moore(m) => {
            if m.initial().is_none() {
                return Err(no_initial_state());
            }
            let view = move |c: &MealyConfig| {
                json!({"state": state_name(m, c.state), "state_id": c.state,
                       "remaining": c.remaining, "output": c.output})
            };
            Ok(boxed(MooreStepByState::new(m), view, output_of))
        }
        Document::Turing(tm) => {
            if tm.automaton().initial().is_none() {
                return Err(no_initial_state());
            }
            let filters = match acceptance {
                None | Some(Acceptance::FinalState) => AcceptanceFilters::by_final_state(),
                Some(Acceptance::Halting) => AcceptanceFilters::by_halting(),
                Some(Acceptance::Both) => AcceptanceFilters {
                    final_state: true,
                    halting: true,
                },
                Some(Acceptance::EmptyStack) => {
                    return Err(unsupported(
                        "turing acceptance is `final_state`, `halting` or `both`, not `empty_stack`"
                            .to_string(),
                    ));
                }
            };
            let simulator = match tape_inputs {
                None => TmSimulator::new(tm, filters),
                Some(inputs) => TmSimulator::with_inputs(tm, filters, inputs.to_vec())
                    .map_err(|e| ApiError::new(code::INVALID_ARGUMENT, e.to_string()))?,
            };
            let view = move |c: &TmConfig| {
                json!({
                    "state": tm_state_name(tm, &c.path),
                    "state_id": c.state(),
                    "path": c.path,
                    "halted": c.halted,
                    "tapes": c.tapes.iter().map(|t| json!({
                        "content": t.contents(),
                        "head": t.head(),
                    })).collect::<Vec<_>>(),
                })
            };
            let extra = |outcome: &RunOutcome<TmConfig>, out: &mut Object| {
                if let Some(outputs) = tape_contents(outcome) {
                    out.insert("tape_outputs".into(), json!(outputs));
                }
            };
            Ok(boxed(simulator, view, extra))
        }
        other => Err(ApiError::wrong_type(
            "simulate",
            "a fa, pda, turing, mealy or moore document",
            other.type_name(),
        )),
    }
}

fn output_of(outcome: &RunOutcome<MealyConfig>, out: &mut Object) {
    if let RunOutcome::Accepted { trace } = outcome
        && let Some(last) = trace.last()
    {
        out.insert("output".into(), json!(last.output));
    }
}

/// Runs the input with a runner built for it.
fn run_one(
    doc: &Document,
    input: &Input,
    mode: Option<SimMode>,
    acceptance: Option<Acceptance>,
    max_configs: usize,
) -> Result<Object, ApiError> {
    Ok(match input {
        Input::One(s) => build(doc, mode, acceptance, None)?.run(s, max_configs),
        Input::Tapes(tapes) => build(doc, mode, acceptance, Some(tapes))?.run("", max_configs),
    })
}

pub fn simulate(r: Simulate) -> Result<Value, ApiError> {
    let input = Input::resolve(&r.input, &r.inputs)?;
    let max = limit(r.max_configs, "`max_configs`")?;
    Ok(Value::Object(run_one(
        &r.document,
        &input,
        r.mode,
        r.acceptance,
        max,
    )?))
}

pub fn step(r: Step) -> Result<Value, ApiError> {
    let input = Input::resolve(&r.input, &r.inputs)?;
    let cap = limit(r.max_configs, "`max_configs`")?;
    let steps = r.steps.unwrap_or(1);
    Ok(match &input {
        Input::One(s) => build(&r.document, r.mode, r.acceptance, None)?.step(s, steps, cap),
        Input::Tapes(t) => build(&r.document, r.mode, r.acceptance, Some(t))?.step("", steps, cap),
    })
}

pub fn multiple_run(r: MultipleRun) -> Result<Value, ApiError> {
    let max = limit(r.max_configs, "`max_configs`")?;
    let mut results = Vec::new();
    for spec in &r.inputs {
        let (shown, input) = match spec {
            InputSpec::One(s) => (json!(s), Input::One(s.clone())),
            InputSpec::Tapes(t) => (json!(t), Input::Tapes(t.clone())),
        };
        let mut run = run_one(&r.document, &input, r.mode, r.acceptance, max)?;
        run.remove("trace");
        run.insert("input".into(), shown);
        results.push(Value::Object(run));
    }
    Ok(json!({"results": results}))
}
