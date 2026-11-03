//! `rflap catedra ...` and `rflap membership`: the notation of the course.

use serde_json::{Value, json};

use crate::api;
use crate::cli::{InlineGrammar, ModelArg};
use crate::error::{CliError, Result};
use crate::input;
use crate::machine::{split_doc, wrote};
use crate::text::{self, Reply, exit, plain, quote};

fn model_name(model: ModelArg) -> &'static str {
    match model {
        ModelArg::Mt => "mt",
        ModelArg::All => "all",
    }
}

pub fn definition(file: &str, model: Option<ModelArg>) -> Result<Reply> {
    let mut request = json!({"op": "catedra_definition", "document": input::load_document(file)?});
    if let Some(model) = model {
        request["model"] = json!(model_name(model));
    }
    let answer = api::call(&request)?;
    let r = api::result(&answer);
    let mut out = plain(&r["text"]);
    let issues = r["issues"].as_array().cloned().unwrap_or_default();
    if !issues.is_empty() {
        out.push_str("Avisos:\n");
        for issue in &issues {
            out.push_str(&format!("- {}\n", plain(&issue["message"])));
        }
    }
    Ok(Reply::new(out, &answer, exit::OK))
}

pub fn table(file: &str, markdown: bool) -> Result<Reply> {
    let answer =
        api::call(&json!({"op": "catedra_table", "document": input::load_document(file)?}))?;
    let r = api::result(&answer);
    let out = plain(if markdown { &r["markdown"] } else { &r["text"] });
    Ok(Reply::new(out, &answer, exit::OK))
}

/// `"input": sequence` per run.
fn run_lines(results: &Value) -> String {
    let mut out = String::new();
    for r in results.as_array().into_iter().flatten() {
        out.push_str(&format!(
            "{}: {}\n",
            quote(r["input"].as_str().unwrap_or("")),
            plain(&r["sequence"])
        ));
    }
    out
}

/// 3 when some run does not stop, 1 when some is not accepted, otherwise 0.
fn verdict_code(results: &Value) -> i32 {
    let verdicts: Vec<&str> = results
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["verdict"].as_str())
        .collect();
    if verdicts.contains(&"no_halt") {
        exit::UNDECIDED
    } else if verdicts.iter().any(|v| *v != "accepted") {
        exit::REJECTED
    } else {
        exit::OK
    }
}

fn inputs_of(positional: &[String], inputs_file: Option<&str>) -> Result<Vec<String>> {
    let mut inputs: Vec<String> = positional.iter().map(|s| input::input_string(s)).collect();
    if let Some(file) = inputs_file {
        inputs.extend(input::inputs_from_file(file)?);
    }
    if inputs.is_empty() {
        return Err(CliError::arg(
            "give at least one input (arguments or --inputs-file)",
        ));
    }
    Ok(inputs)
}

pub fn run(
    file: &str,
    positional: &[String],
    inputs_file: Option<&str>,
    model: Option<ModelArg>,
    all: bool,
    max_steps: Option<usize>,
) -> Result<Reply> {
    let inputs = inputs_of(positional, inputs_file)?;
    let mut request = json!({
        "op": "catedra_run", "document": input::load_document(file)?, "inputs": inputs
    });
    if all {
        request["model"] = json!("all");
    } else if let Some(model) = model {
        request["model"] = json!(model_name(model));
    }
    if let Some(max) = max_steps {
        request["max_steps"] = json!(max);
    }
    let answer = api::call(&request)?;
    let results = &api::result(&answer)["results"];
    Ok(Reply::new(
        run_lines(results),
        &answer,
        verdict_code(results),
    ))
}

pub fn all_check(file: &str, positional: &[String], max_steps: Option<usize>) -> Result<Reply> {
    let inputs: Vec<String> = positional.iter().map(|s| input::input_string(s)).collect();
    let mut request = json!({
        "op": "all_check", "document": input::load_document(file)?, "inputs": inputs
    });
    if let Some(max) = max_steps {
        request["max_steps"] = json!(max);
    }
    let answer = api::call(&request)?;
    let r = api::result(&answer);
    let mut out = run_lines(&r["runs"]);
    let ok = r["ok"] == true;
    if ok {
        out.push_str("La máquina cumple las reglas de un ALL.\n");
    } else {
        out.push_str("La máquina NO cumple las reglas de un ALL:\n");
        for v in r["violations"].as_array().into_iter().flatten() {
            out.push_str(&format!("- {}\n", plain(&v["message"])));
        }
    }
    let code = if ok { exit::OK } else { exit::REJECTED };
    Ok(Reply::new(out, &answer, code))
}

pub fn compose(a: &str, b: &str, output: Option<&str>) -> Result<Reply> {
    if let Some(output) = output {
        input::check_output(output)?;
    }
    let (a, b) = (input::load_document(a)?, input::load_document(b)?);
    let answer = api::call(&json!({"op": "compose_tm", "a": a, "b": b}))?;
    let r = api::result(&answer);
    let out = match output {
        Some(path) => {
            input::write_document(&r["document"], path)?;
            wrote(path)
        }
        None => match r["definition"]["text"].as_str() {
            Some(text) => text.to_string(),
            None => text::with_newline(plain(&r["document"])),
        },
    };
    Ok(Reply::new(out, &answer, exit::OK))
}

pub fn membership(
    inline: &InlineGrammar,
    args: &[String],
    max_forms: Option<usize>,
) -> Result<Reply> {
    let (grammar, positional) = split_doc(None, inline.grammar.as_deref(), args, "membership")?;
    let [word] = positional.as_slice() else {
        return Err(CliError::arg(
            "give exactly one WORD after the grammar (`λ` is the empty word)",
        ));
    };
    let mut request = json!({
        "op": "membership_tk", "grammar": grammar, "input": input::input_string(word)
    });
    if let Some(max) = max_forms {
        request["max_forms"] = json!(max);
    }
    let answer = api::call(&request)?;
    let r = api::result(&answer);
    let code = if r["member"] == true {
        exit::OK
    } else {
        exit::REJECTED
    };
    Ok(Reply::new(plain(&r["text"]), &answer, code))
}
