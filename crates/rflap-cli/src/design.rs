//! `design`: a language description in, a verified automaton out.

use serde_json::{Map, Value, json};

use crate::api;
use crate::error::{CliError, Result};
use crate::input;
use crate::machine::wrote;
use crate::text::{Reply, count, exit, plain, quote};

pub struct DesignArgs<'a> {
    pub constraint: Option<&'a str>,
    pub regex: Option<&'a str>,
    pub grammar: Option<&'a str>,
    pub template: Option<&'a str>,
    pub params: &'a [String],
    pub alphabet: &'a str,
    pub accept: &'a [String],
    pub reject: &'a [String],
    pub output: Option<&'a str>,
    pub svg: Option<&'a str>,
}

/// `KEY=VALUE` pairs.
fn params(raw: &[String]) -> Result<Vec<(String, String)>> {
    raw.iter()
        .map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.to_string()))
                .ok_or_else(|| CliError::arg(format!("template parameter `{p}` is not KEY=VALUE")))
        })
        .collect()
}

fn template(name: &str, raw: &[String], alphabet: &str) -> Result<Value> {
    let mut template = Map::new();
    template.insert(
        "name".into(),
        json!(name.trim().to_ascii_lowercase().replace('-', "_")),
    );
    let symbols: Vec<char> = alphabet.chars().collect();
    if matches!(name, "an_bn" | "an_bm" | "equal_count") {
        if let Some(a) = symbols.first() {
            template.insert("a".into(), json!(a.to_string()));
        }
        if let Some(b) = symbols.get(1) {
            template.insert("b".into(), json!(b.to_string()));
        }
    }
    for (key, value) in params(raw)? {
        let parsed = match key.as_str() {
            "min" => json!(value.parse::<usize>().map_err(|_| {
                CliError::arg(format!(
                    "template parameter `min` must be a number, got `{value}`"
                ))
            })?),
            "pairs" => {
                let chars: Vec<char> = value.chars().collect();
                if chars.is_empty() || !chars.len().is_multiple_of(2) {
                    return Err(CliError::arg(
                        "template parameter `pairs` needs an even number of characters, e.g. ()[]",
                    ));
                }
                json!(
                    chars
                        .chunks(2)
                        .map(|p| [p[0].to_string(), p[1].to_string()])
                        .collect::<Vec<_>>()
                )
            }
            _ => json!(value),
        };
        template.insert(key, parsed);
    }
    Ok(Value::Object(template))
}

fn words(raw: &[String]) -> Vec<String> {
    raw.iter().map(|w| input::input_string(w)).collect()
}

pub fn design(args: &DesignArgs) -> Result<Reply> {
    let given = [
        args.constraint.is_some(),
        args.regex.is_some(),
        args.grammar.is_some(),
        args.template.is_some(),
    ]
    .iter()
    .filter(|g| **g)
    .count();
    if given != 1 {
        return Err(CliError::arg(
            "give exactly one of --constraint, --regex, --grammar and --template",
        ));
    }
    if let Some(path) = args.output {
        input::check_output(path)?;
    }
    let language = if let Some(c) = args.constraint {
        json!({"constraint_text": c})
    } else if let Some(r) = args.regex {
        json!({"regex": r})
    } else if let Some(g) = args.grammar {
        json!({"regular_grammar": input::grammar_text(g)?})
    } else {
        let name = args.template.unwrap_or_default();
        json!({"template": template(name, args.params, args.alphabet)?})
    };
    let request = json!({"op": "design", "spec": {
        "alphabet": args.alphabet,
        "language": language,
        "examples": {"accept": words(args.accept), "reject": words(args.reject)},
    }});
    let answer = api::call(&request)?;
    let r = api::result(&answer);

    let kind = plain(&r["kind"]).to_uppercase();
    let states = r["stats"]["states"].as_u64().unwrap_or(0);
    let mut out = format!(
        "Designed a {kind} with {} for: {}\n",
        count(states, "state"),
        plain(&r["description"])
    );
    for state in r["states"].as_array().into_iter().flatten() {
        let mut marks = Vec::new();
        if state["initial"] == true {
            marks.push("initial");
        }
        if state["accepting"] == true {
            marks.push("accepting");
        }
        let marks = if marks.is_empty() {
            String::new()
        } else {
            format!(" ({})", marks.join(", "))
        };
        out.push_str(&format!(
            "  {}{marks}: {}\n",
            plain(&state["name"]),
            plain(&state["meaning"])
        ));
    }
    if let Some(regex) = r["regex"].as_str() {
        out.push_str(&format!("regex: {regex}\n"));
    }
    let verification = &r["verification"];
    let ok = verification["ok"] == true;
    let mut details = Vec::new();
    let cross = &verification["cross_check"];
    if cross.is_object() {
        details.push(format!(
            "{} strings up to length {} checked",
            plain(&cross["strings_checked"]),
            plain(&cross["max_len"])
        ));
    }
    let examples = verification["examples"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if !examples.is_empty() {
        let passed = examples.iter().filter(|e| e["passed"] == true).count();
        details.push(format!("examples {passed}/{} passed", examples.len()));
    }
    let status = if ok { "ok" } else { "FAILED" };
    if details.is_empty() {
        out.push_str(&format!("verification: {status}\n"));
    } else {
        out.push_str(&format!(
            "verification: {status} ({})\n",
            details.join("; ")
        ));
    }
    for e in examples.iter().filter(|e| e["passed"] != true) {
        let (expected, got) = if e["expected_accept"] == true {
            ("accept", "rejected")
        } else {
            ("reject", "accepted")
        };
        out.push_str(&format!(
            "  expected {expected} but {got}: {}\n",
            quote(&plain(&e["word"]))
        ));
    }
    for m in cross["mismatches"].as_array().into_iter().flatten().take(5) {
        out.push_str(&format!("  mismatch: {m}\n"));
    }
    if let Some(path) = args.output {
        input::write_document(&r["document"], path)?;
        out.push_str(&wrote(path));
    }
    if let Some(path) = args.svg {
        input::write_svg(&r["document"], path)?;
        out.push_str(&wrote(path));
    }
    Ok(Reply::new(
        out,
        &answer,
        if ok { exit::OK } else { exit::REJECTED },
    ))
}
