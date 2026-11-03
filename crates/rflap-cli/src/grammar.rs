//! Grammar commands: grammar-type, parse, tables.

use serde_json::{Value, json};

use crate::api;
use crate::cli::{GrammarArgs, InlineGrammar, ParserArg};
use crate::error::Result;
use crate::input;
use crate::machine::split_doc;
use crate::text::{self, Reply, exit, plain, quote, table};

fn grammar_doc(args: &GrammarArgs) -> Result<Value> {
    input::resolve_document(args.file.as_deref(), None, args.inline.grammar.as_deref())
}

pub fn grammar_type(args: &GrammarArgs) -> Result<Reply> {
    let grammar = grammar_doc(args)?;
    let answer = api::call(&json!({"op": "grammar_type", "grammar": grammar}))?;
    let r = api::result(&answer);
    let out = format!(
        "{}\nclassification: {}\n",
        plain(&r["message"]),
        plain(&r["classification"])
    );
    Ok(Reply::new(out, &answer, exit::OK))
}

pub fn parse(
    inline: &InlineGrammar,
    args: &[String],
    parser: ParserArg,
    tree: bool,
    budget: Option<usize>,
) -> Result<Reply> {
    let (grammar, positional) = split_doc(None, inline.grammar.as_deref(), args, "parse")?;
    let inputs: Vec<String> = positional.iter().map(|s| input::input_string(s)).collect();
    if inputs.is_empty() {
        return Err(crate::error::CliError::arg(
            "give at least one INPUT to parse",
        ));
    }
    let parser = match parser {
        ParserArg::Brute => "brute",
        ParserArg::BruteUnrestricted => "brute_unrestricted",
        ParserArg::Ll1 => "ll1",
        ParserArg::Slr1 => "slr1",
        ParserArg::Cyk => "cyk",
    };
    let mut request = json!({
        "op": "parse", "grammar": grammar, "inputs": inputs, "parser": parser
    });
    if let Some(budget) = budget {
        request["budget"] = json!(budget);
    }
    let answer = api::call(&request)?;
    let mut out = String::new();
    let (mut errors, mut undecided, mut rejected) = (false, false, false);
    for r in api::result(&answer)["results"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let label = quote(r["input"].as_str().unwrap_or(""));
        match r["outcome"].as_str().unwrap_or("") {
            "accepted" => {
                out.push_str(&format!("{label}: accepted\n"));
                if let Some(forms) = r["derivation_forms"].as_array() {
                    let forms: Vec<String> = forms.iter().map(plain).collect();
                    out.push_str(&format!("  {}\n", forms.join(" => ")));
                }
                if tree && r["tree"].is_object() {
                    out.push_str(&text::tree(&r["tree"], "  "));
                }
            }
            "undecided" => {
                undecided = true;
                out.push_str(&format!("{label}: undecided\n"));
            }
            "error" => {
                errors = true;
                out.push_str(&format!(
                    "{label}: error: {}\n",
                    plain(&r["error"]["message"])
                ));
            }
            _ => {
                rejected = true;
                out.push_str(&format!("{label}: rejected\n"));
            }
        }
    }
    let code = if errors {
        exit::USAGE
    } else if undecided {
        exit::UNDECIDED
    } else if rejected {
        exit::REJECTED
    } else {
        exit::OK
    };
    Ok(Reply::new(out, &answer, code))
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|a| a.iter().map(plain).collect())
        .unwrap_or_default()
}

fn sets(title: &str, map: &Value) -> String {
    let mut out = format!("{title}\n");
    for (variable, set) in map.as_object().into_iter().flatten() {
        out.push_str(&format!("  {variable}: {}\n", strings(set).join(", ")));
    }
    out
}

fn ll1_section(ll1: &Value) -> String {
    let yes = if ll1["is_ll1"] == true { "yes" } else { "no" };
    let mut out = format!("LL(1): {yes}\n");
    let terminals = strings(&ll1["terminals"]);
    let mut header = vec![String::new()];
    header.extend(terminals.iter().cloned());
    let rows: Vec<Vec<String>> = strings(&ll1["variables"])
        .into_iter()
        .map(|variable| {
            let mut cells = vec![variable.clone()];
            for terminal in &terminals {
                cells.push(strings(&ll1["table"][&variable][terminal]).join(" / "));
            }
            cells
        })
        .collect();
    out.push_str(&table(&header, &rows));
    let conflicts = ll1["conflicts"].as_array().cloned().unwrap_or_default();
    if !conflicts.is_empty() {
        out.push_str("conflicts:\n");
        for c in &conflicts {
            out.push_str(&format!(
                "  {} on {}: {}\n",
                plain(&c["variable"]),
                plain(&c["terminal"]),
                strings(&c["entries"]).join(" / ")
            ));
        }
    }
    out
}

fn slr1_section(slr1: &Value) -> String {
    let yes = if slr1["is_slr1"] == true { "yes" } else { "no" };
    let mut out = format!("SLR(1): {yes}\nProductions:\n");
    for (i, production) in strings(&slr1["productions"]).iter().enumerate() {
        out.push_str(&format!("  {i}: {production}\n"));
    }
    out.push_str("States:\n");
    for state in slr1["states"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "  {}: {}\n",
            plain(&state["id"]),
            strings(&state["items"]).join(", ")
        ));
    }
    let terminals = strings(&slr1["terminals"]);
    let variables = strings(&slr1["variables"]);
    let mut header = vec!["State".to_string()];
    header.extend(terminals.iter().cloned());
    header.extend(variables.iter().cloned());
    let count = slr1["states"].as_array().map_or(0, Vec::len);
    let rows: Vec<Vec<String>> = (0..count)
        .map(|i| {
            let mut cells = vec![i.to_string()];
            for terminal in &terminals {
                cells.push(strings(&slr1["actions"][i][terminal]).join(" / "));
            }
            for variable in &variables {
                cells.push(plain(&slr1["gotos"][i][variable]));
            }
            cells
        })
        .collect();
    out.push_str(&table(&header, &rows));
    let conflicts = slr1["conflicts"].as_array().cloned().unwrap_or_default();
    if !conflicts.is_empty() {
        out.push_str("conflicts:\n");
        for c in &conflicts {
            out.push_str(&format!(
                "  state {} on {}: {}\n",
                plain(&c["state"]),
                plain(&c["symbol"]),
                strings(&c["actions"]).join(" / ")
            ));
        }
    }
    out
}

pub fn tables(args: &GrammarArgs) -> Result<Reply> {
    let grammar = grammar_doc(args)?;
    let answer = api::call(&json!({"op": "parse_tables", "grammar": grammar}))?;
    let r = api::result(&answer);
    let out = [
        sets("FIRST", &r["first"]),
        sets("FOLLOW", &r["follow"]),
        ll1_section(&r["ll1"]),
        slr1_section(&r["slr1"]),
    ]
    .join("\n");
    Ok(Reply::new(out, &answer, exit::OK))
}
