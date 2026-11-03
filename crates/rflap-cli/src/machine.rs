//! Commands on documents and machines: call, info, run, step, convert, equiv, combine,
//! render, layout, explain, lsystem.

use std::path::Path;

use rflap_api::ConvertTo;
use serde_json::{Map, Value, json};

use crate::api;
use crate::cli::{AcceptArg, DocArgs, FormatArg, Inline, ModeArg, RunOptions};
use crate::error::{CliError, Result};
use crate::input;
use crate::text::{self, Reply, exit, list_or_none, plain, quote, with_newline};

/// `wrote <path>` line.
pub fn wrote(path: &str) -> String {
    format!("wrote {path}\n")
}

pub fn doc_of(args: &DocArgs) -> Result<Value> {
    input::resolve_document(
        args.file.as_deref(),
        args.inline.regex.as_deref(),
        args.inline.grammar.as_deref(),
    )
}

/// The document of `run`/`step`/`parse`: the first positional argument, or the inline form
/// (then every positional argument is an input).
pub fn split_doc(
    regex: Option<&str>,
    grammar: Option<&str>,
    args: &[String],
    command: &str,
) -> Result<(Value, Vec<String>)> {
    if regex.is_some() || grammar.is_some() {
        if regex.is_some() && grammar.is_some() {
            return Err(CliError::arg("give only one of --regex and --grammar"));
        }
        let doc = input::resolve_document(None, regex, grammar)?;
        return Ok((doc, args.to_vec()));
    }
    let Some((file, rest)) = args.split_first() else {
        return Err(CliError::arg(format!(
            "`{command}` needs a FILE (`-` reads standard input), --regex or --grammar"
        )));
    };
    Ok((input::load_document(file)?, rest.to_vec()))
}

// ---------------------------------------------------------------------------------------------
// call
// ---------------------------------------------------------------------------------------------

pub fn call(request: Option<&str>) -> Result<Reply> {
    let text = match request {
        None | Some("-") => input::read_text("-")?,
        Some(text) => text.to_string(),
    };
    let (raw, answer) = api::call_raw(&text)?;
    if answer["ok"] != true {
        return Err(CliError::from_api(&raw, &answer));
    }
    Ok(Reply {
        text: with_newline(raw.clone()),
        json: raw,
        code: exit::OK,
    })
}

// ---------------------------------------------------------------------------------------------
// info
// ---------------------------------------------------------------------------------------------

pub fn info(args: &DocArgs) -> Result<Reply> {
    let doc = doc_of(args)?;
    let answer = api::call(&json!({"op": "analyze", "document": doc}))?;
    let r = api::result(&answer);
    let kind = r["type"].as_str().unwrap_or("");
    let mut out = String::new();
    match kind {
        "grammar" => {
            out.push_str("type: grammar\n");
            out.push_str(&format!("{}\n", plain(&r["message"])));
            out.push_str(&format!("start: {}\n", plain(&r["start"])));
            out.push_str(&format!("variables: {}\n", list_or_none(&r["variables"])));
            out.push_str(&format!("terminals: {}\n", list_or_none(&r["terminals"])));
            out.push_str(&format!("productions: {}\n", plain(&r["productions"])));
        }
        "regex" => {
            out.push_str("type: regex\n");
            out.push_str(&format!("expression: {}\n", plain(&r["expression"])));
            out.push_str(&format!("alphabet: {}\n", list_or_none(&r["alphabet"])));
            let nullable = if r["nullable"] == true { "yes" } else { "no" };
            out.push_str(&format!("nullable: {nullable}\n"));
        }
        _ if r["states"].is_array() => machine_info(r, &mut out),
        _ => {
            let mut rest = r.clone();
            if let Some(map) = rest.as_object_mut() {
                map.remove("type");
            }
            out.push_str(&format!("type: {kind}\n"));
            out.push_str(&serde_json::to_string_pretty(&rest).unwrap_or_default());
            out.push('\n');
        }
    }
    Ok(Reply::new(out, &answer, exit::OK))
}

fn machine_info(r: &Value, out: &mut String) {
    let kind = plain(&r["type"]);
    match r["kind"].as_str() {
        Some(k) => out.push_str(&format!("type: {kind} ({k})\n")),
        None => out.push_str(&format!("type: {kind}\n")),
    }
    let names: Vec<Value> = r["states"]
        .as_array()
        .map(|a| a.iter().map(|s| s["name"].clone()).collect())
        .unwrap_or_default();
    out.push_str(&format!(
        "states: {} ({})\n",
        names.len(),
        list_or_none(&Value::Array(names))
    ));
    let initial = r["initial"].as_str().unwrap_or("none");
    out.push_str(&format!("initial: {initial}\n"));
    if !r["finals"].is_null() {
        out.push_str(&format!("final: {}\n", list_or_none(&r["finals"])));
    }
    if !r["alphabet"].is_null() {
        out.push_str(&format!("alphabet: {}\n", list_or_none(&r["alphabet"])));
    }
    if !r["stack_alphabet"].is_null() {
        out.push_str(&format!(
            "stack alphabet: {}\n",
            list_or_none(&r["stack_alphabet"])
        ));
    }
    if !r["tapes"].is_null() {
        out.push_str(&format!("tapes: {}\n", plain(&r["tapes"])));
    }
    out.push_str(&format!("transitions: {}\n", plain(&r["transitions"])));
    if let Some(deterministic) = r["deterministic"].as_bool() {
        if deterministic {
            out.push_str("deterministic: yes\n");
        } else {
            out.push_str(&format!(
                "deterministic: no (nondeterministic states: {})\n",
                list_or_none(&r["nondeterministic_states"])
            ));
        }
    }
    if let Some(complete) = r["complete"].as_bool() {
        out.push_str(&format!(
            "complete: {}\n",
            if complete { "yes" } else { "no" }
        ));
    }
    let lambdas = r["lambda_transitions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if lambdas.is_empty() {
        out.push_str("lambda transitions: none\n");
    } else {
        let shown: Vec<String> = lambdas
            .iter()
            .map(|t| {
                format!(
                    "{} -{}-> {}",
                    plain(&t["from"]),
                    plain(&t["label"]),
                    plain(&t["to"])
                )
            })
            .collect();
        out.push_str(&format!(
            "lambda transitions: {} ({})\n",
            lambdas.len(),
            shown.join(", ")
        ));
    }
    out.push_str(&format!(
        "unreachable states: {}\n",
        list_or_none(&r["unreachable_states"])
    ));
    out.push_str(&format!(
        "useless states: {}\n",
        list_or_none(&r["useless_states"])
    ));
}

// ---------------------------------------------------------------------------------------------
// run and step
// ---------------------------------------------------------------------------------------------

fn sim_options(request: &mut Map<String, Value>, options: &RunOptions) {
    if let Some(mode) = options.mode {
        let mode = match mode {
            ModeArg::Step => "step_by_state",
            ModeArg::Closure => "closure",
        };
        request.insert("mode".into(), json!(mode));
    }
    if let Some(acceptance) = options.acceptance {
        let acceptance = match acceptance {
            AcceptArg::Final => "final_state",
            AcceptArg::EmptyStack => "empty_stack",
            AcceptArg::Halting => "halting",
            AcceptArg::Both => "both",
        };
        request.insert("acceptance".into(), json!(acceptance));
    }
    if let Some(max) = options.max_configs {
        request.insert("max_configs".into(), json!(max));
    }
}

fn run_line(label: &str, r: &Value) -> String {
    let mut line = format!("{label}: {}", plain(&r["outcome"]));
    if let Some(output) = r["output"].as_str() {
        line.push_str(&format!(", output {}", quote(output)));
    }
    if let Some(tapes) = r["tape_outputs"].as_array() {
        for (k, tape) in tapes.iter().enumerate() {
            line.push_str(&format!(", tape {} {}", k + 1, quote(&plain(tape))));
        }
    }
    line
}

/// Exit code of a set of outcomes: undecided beats rejected.
fn outcome_code(outcomes: &[&str]) -> i32 {
    if outcomes.contains(&"undecided") {
        exit::UNDECIDED
    } else if outcomes.iter().any(|o| *o != "accepted") {
        exit::REJECTED
    } else {
        exit::OK
    }
}

struct Inputs {
    /// The strings to run, one run each (empty when the tapes form a single run).
    strings: Vec<String>,
    /// One string per tape, for a single multi-tape run.
    tapes: Vec<String>,
}

fn collect_inputs(
    positional: &[String],
    inputs_file: Option<&str>,
    options: &RunOptions,
) -> Result<Inputs> {
    let mut strings: Vec<String> = positional.iter().map(|s| input::input_string(s)).collect();
    if let Some(file) = inputs_file {
        strings.extend(input::inputs_from_file(file)?);
    }
    let tapes: Vec<String> = options
        .tape
        .iter()
        .map(|s| input::input_string(s))
        .collect();
    if !tapes.is_empty() && !strings.is_empty() {
        return Err(CliError::arg(
            "give the inputs either as arguments or with --tape, not both",
        ));
    }
    if tapes.is_empty() && strings.is_empty() {
        return Err(CliError::arg(
            "give at least one input (arguments, --inputs-file or --tape)",
        ));
    }
    Ok(Inputs { strings, tapes })
}

pub fn run(
    inline: &Inline,
    args: &[String],
    inputs_file: Option<&str>,
    trace: bool,
    options: &RunOptions,
) -> Result<Reply> {
    let (doc, positional) = split_doc(
        inline.regex.as_deref(),
        inline.grammar.as_deref(),
        args,
        "run",
    )?;
    let inputs = collect_inputs(&positional, inputs_file, options)?;
    // (label, answer) per request
    let mut answers: Vec<(String, Value)> = Vec::new();
    let mut lines: Vec<(String, Value)> = Vec::new(); // label and result of each run
    let simulate = |input: Option<&str>, tapes: Option<&[String]>| -> Result<Value> {
        let mut request = Map::new();
        request.insert("op".into(), json!("simulate"));
        request.insert("document".into(), doc.clone());
        if let Some(input) = input {
            request.insert("input".into(), json!(input));
        }
        if let Some(tapes) = tapes {
            request.insert("inputs".into(), json!(tapes));
        }
        sim_options(&mut request, options);
        api::call(&Value::Object(request))
    };
    if !inputs.tapes.is_empty() {
        let label = inputs
            .tapes
            .iter()
            .map(|t| quote(t))
            .collect::<Vec<_>>()
            .join(" | ");
        let answer = simulate(None, Some(&inputs.tapes))?;
        lines.push((label.clone(), api::result(&answer).clone()));
        answers.push((label, answer));
    } else if inputs.strings.len() == 1 || trace {
        for s in &inputs.strings {
            let answer = simulate(Some(s), None)?;
            lines.push((quote(s), api::result(&answer).clone()));
            answers.push((quote(s), answer));
        }
    } else {
        let mut request = Map::new();
        request.insert("op".into(), json!("multiple_run"));
        request.insert("document".into(), doc.clone());
        request.insert("inputs".into(), json!(inputs.strings));
        sim_options(&mut request, options);
        let answer = api::call(&Value::Object(request))?;
        for r in api::result(&answer)["results"]
            .as_array()
            .into_iter()
            .flatten()
        {
            lines.push((quote(r["input"].as_str().unwrap_or("")), r.clone()));
        }
        answers.push((String::new(), answer));
    }
    let mut out = String::new();
    let mut outcomes = Vec::new();
    for (label, r) in &lines {
        out.push_str(&run_line(label, r));
        out.push('\n');
        if trace {
            for (i, entry) in r["trace"].as_array().into_iter().flatten().enumerate() {
                out.push_str(&format!("  {i}. {}\n", text::config(entry)));
            }
        }
        outcomes.push(r["outcome"].as_str().unwrap_or("rejected"));
    }
    let json = if answers.len() == 1 {
        answers[0].1.clone()
    } else {
        Value::Array(answers.into_iter().map(|(_, a)| a).collect())
    };
    Ok(Reply::new(out, &json, outcome_code(&outcomes)))
}

pub fn step(inline: &Inline, args: &[String], steps: usize, options: &RunOptions) -> Result<Reply> {
    let (doc, positional) = split_doc(
        inline.regex.as_deref(),
        inline.grammar.as_deref(),
        args,
        "step",
    )?;
    let mut request = Map::new();
    request.insert("op".into(), json!("step"));
    request.insert("document".into(), doc);
    request.insert("steps".into(), json!(steps));
    if options.tape.is_empty() {
        match positional.as_slice() {
            [one] => {
                request.insert("input".into(), json!(input::input_string(one)));
            }
            _ => {
                return Err(CliError::arg(
                    "`step` needs exactly one INPUT (or one --tape per tape)",
                ));
            }
        }
    } else {
        if !positional.is_empty() {
            return Err(CliError::arg(
                "give the input either as an argument or with --tape, not both",
            ));
        }
        let tapes: Vec<String> = options
            .tape
            .iter()
            .map(|s| input::input_string(s))
            .collect();
        request.insert("inputs".into(), json!(tapes));
    }
    sim_options(&mut request, options);
    let answer = api::call(&Value::Object(request))?;
    let r = api::result(&answer);
    let finished = r["finished"] == true;
    let truncated = r["truncated"] == true;
    let mut out = format!(
        "steps done: {} ({}{})\n",
        plain(&r["steps_done"]),
        if finished { "finished" } else { "not finished" },
        if truncated { ", truncated" } else { "" }
    );
    let frontier = r["frontier"].as_array().cloned().unwrap_or_default();
    if !frontier.is_empty() {
        out.push_str("frontier:\n");
        for c in &frontier {
            out.push_str(&format!("  {}\n", text::config(c)));
        }
    }
    let accepted = r["accepted"].as_array().cloned().unwrap_or_default();
    if !accepted.is_empty() {
        out.push_str("accepted:\n");
        for a in &accepted {
            out.push_str(&format!("  {}\n", text::config(&a["config"])));
        }
    }
    let rejected = r["rejected"].as_array().cloned().unwrap_or_default();
    if !rejected.is_empty() {
        out.push_str("rejected:\n");
        for c in &rejected {
            out.push_str(&format!("  {}\n", text::config(c)));
        }
    }
    let code = if truncated {
        exit::UNDECIDED
    } else if finished && accepted.is_empty() {
        exit::REJECTED
    } else {
        exit::OK
    };
    Ok(Reply::new(out, &answer, code))
}

// ---------------------------------------------------------------------------------------------
// convert, equiv, combine, layout
// ---------------------------------------------------------------------------------------------

fn target_names() -> Vec<String> {
    ConvertTo::ALL
        .iter()
        .map(|t| t.name().replace('_', "-"))
        .collect()
}

pub fn convert(doc: &DocArgs, to: &str, output: Option<&str>, steps: bool) -> Result<Reply> {
    let wanted = to.trim().to_ascii_lowercase().replace('_', "-");
    if !target_names().contains(&wanted) {
        return Err(CliError::arg(format!(
            "unknown target `{to}`; the targets are {}",
            target_names().join(", ")
        )));
    }
    if let Some(path) = output {
        input::check_output(path)?;
    }
    let document = doc_of(doc)?;
    let answer = api::call(&json!({
        "op": "convert", "document": document, "to": wanted.replace('-', "_")
    }))?;
    let r = api::result(&answer);
    let mut out = String::new();
    if let Some(path) = output {
        input::write_document(&r["document"], path)?;
        out.push_str(&wrote(path));
    } else {
        out.push_str(&render_text(&r["document"])?);
    }
    if steps {
        out.push_str("steps:\n");
        for (i, step) in r["steps"].as_array().into_iter().flatten().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, step));
        }
    }
    Ok(Reply::new(out, &answer, exit::OK))
}

/// The plain-text drawing of a document.
fn render_text(document: &Value) -> Result<String> {
    let answer = api::call(&json!({"op": "render", "document": document, "format": "text"}))?;
    Ok(with_newline(plain(&api::result(&answer)["content"])))
}

pub fn equiv(a: &str, b: &str) -> Result<Reply> {
    let (a, b) = (input::load_document(a)?, input::load_document(b)?);
    let answer = api::call(&json!({"op": "equivalent", "a": a, "b": b}))?;
    let r = api::result(&answer);
    if r["equivalent"] == true {
        Ok(Reply::new("equivalent\n".into(), &answer, exit::OK))
    } else {
        let text = match r["counterexample"].as_str() {
            Some(w) => format!("not equivalent (counterexample: {})\n", quote(w)),
            None => "not equivalent\n".to_string(),
        };
        Ok(Reply::new(text, &answer, exit::REJECTED))
    }
}

pub fn combine(a: &str, b: &str, output: &str) -> Result<Reply> {
    input::check_output(output)?;
    let (a, b) = (input::load_document(a)?, input::load_document(b)?);
    let answer = api::call(&json!({"op": "combine", "a": a, "b": b}))?;
    input::write_document(&api::result(&answer)["document"], output)?;
    Ok(Reply::new(wrote(output), &answer, exit::OK))
}

pub fn layout(doc: &DocArgs, algorithm: &str, output: &str) -> Result<Reply> {
    input::check_output(output)?;
    let document = doc_of(doc)?;
    let answer = api::call(&json!({
        "op": "layout", "document": document, "algorithm": algorithm
    }))?;
    input::write_document(&api::result(&answer)["document"], output)?;
    Ok(Reply::new(wrote(output), &answer, exit::OK))
}

// ---------------------------------------------------------------------------------------------
// render, explain, lsystem
// ---------------------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn render(
    doc: &DocArgs,
    format: Option<FormatArg>,
    output: Option<&str>,
    dark: bool,
    highlight: &[String],
    highlight_transitions: &[usize],
    depth: Option<usize>,
) -> Result<Reply> {
    let document = doc_of(doc)?;
    let by_extension = output.and_then(|p| {
        Path::new(p)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
    });
    let format = match (format, by_extension.as_deref()) {
        (Some(FormatArg::Svg), _) => "svg",
        (Some(FormatArg::Md), _) => "markdown",
        (Some(FormatArg::Dot), _) => "dot",
        (Some(FormatArg::Text), _) => "text",
        (None, Some("svg")) => "svg",
        (None, Some("md" | "markdown")) => "markdown",
        (None, Some("dot" | "gv")) => "dot",
        (None, _) => "text",
    };
    let mut request = json!({
        "op": "render", "document": document, "format": format, "dark": dark,
        "highlight_states": highlight, "highlight_transitions": highlight_transitions,
    });
    if let Some(depth) = depth {
        request["depth"] = json!(depth);
    }
    let answer = api::call(&request)?;
    let content = plain(&api::result(&answer)["content"]);
    match output {
        Some(path) => {
            input::write_file(path, &content)?;
            Ok(Reply::new(wrote(path), &answer, exit::OK))
        }
        None => Ok(Reply::new(with_newline(content), &answer, exit::OK)),
    }
}

pub fn explain(doc: &DocArgs) -> Result<Reply> {
    let document = doc_of(doc)?;
    let answer = api::call(&json!({"op": "explain", "document": document}))?;
    let markdown = plain(&api::result(&answer)["markdown"]);
    Ok(Reply::new(with_newline(markdown), &answer, exit::OK))
}

pub fn lsystem(file: &str, depth: usize, output: Option<&str>) -> Result<Reply> {
    let document = input::load_document(file)?;
    let answer = api::call(&json!({"op": "lsystem_render", "document": document, "depth": depth}))?;
    let r = api::result(&answer);
    let symbols = plain(&r["expansion_length"]);
    let out = match output {
        Some(path) => {
            input::write_file(path, &plain(&r["svg"]))?;
            format!("wrote {path} (depth {depth}, {symbols} symbols)\n")
        }
        None => match r["expansion"].as_str() {
            Some(expansion) => format!("depth {depth}: {symbols} symbols\n{expansion}\n"),
            None => format!("depth {depth}: {symbols} symbols (too long to show; use -o)\n"),
        },
    };
    Ok(Reply::new(out, &answer, exit::OK))
}
