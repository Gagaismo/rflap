//! `grammar_type`, `parse`, `parse_tables` and `user_parse`.

use std::collections::BTreeSet;

use rflap_core::grammar::{
    Derivation, Grammar, LAMBDA, Production, UserParseError, UserParser, brute_parse, cyk_parse,
    first_sets, follow_sets, ll1_parse, ll1_table, slr_parse, slr_table,
};
use serde_json::{Map, Value, json};

use super::{Object, object, to_value, wrong_type};
use crate::document::Document;
use crate::error::{ApiError, code};
use crate::request::{ParseArgs, Parser, UndoMove, UserMove, UserParse};

const DEFAULT_BUDGET: usize = 20_000;

fn grammar_of<'a>(doc: &'a Document, op: &str) -> Result<&'a Grammar, ApiError> {
    match doc {
        Document::Grammar(g) => Ok(g),
        other => Err(wrong_type(op, "a grammar document", other)),
    }
}

/// `A -> aB`, with `λ` for the empty string.
fn production_text(lhs: &str, rhs: &str) -> String {
    format!("{lhs} -> {}", if rhs.is_empty() { LAMBDA } else { rhs })
}

pub fn grammar_type(doc: &Document) -> Result<Value, ApiError> {
    let g = grammar_of(doc, "grammar_type")?;
    let report = g.type_report();
    let mut details = object(&report);
    details.insert("classification".into(), to_value(&report.classification));
    Ok(json!({
        "classification": report.classification,
        "message": report.classification.message(),
        "report": details,
    }))
}

// ---------------------------------------------------------------------------------------------
// parse
// ---------------------------------------------------------------------------------------------

fn forms_of(derivation: &Value) -> Value {
    let Some(start) = derivation.get("start").and_then(Value::as_str) else {
        return Value::Null;
    };
    let mut forms = vec![json!(start)];
    for step in derivation["steps"].as_array().into_iter().flatten() {
        forms.push(step["result"].clone());
    }
    Value::Array(forms)
}

fn finish(mut result: Object, input: &str) -> Value {
    result.insert("input".into(), json!(input));
    if let Some(derivation) = result.get("derivation") {
        let forms = forms_of(derivation);
        result.insert("derivation_forms".into(), forms);
    }
    Value::Object(result)
}

fn parse_failure(input: &str, e: &rflap_core::grammar::ParseError) -> Value {
    json!({
        "input": input,
        "outcome": "error",
        "error": {"code": "parse_error", "message": e.to_string(), "details": to_value(e)},
    })
}

pub fn parse(r: ParseArgs) -> Result<Value, ApiError> {
    let g = grammar_of(&r.grammar, "parse")?;
    let inputs: Vec<String> = match (&r.input, &r.inputs) {
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
                "give `input` (a string, possibly empty) or `inputs`",
            ));
        }
    };
    let parser = r.parser.unwrap_or(Parser::Brute);
    let budget = r.budget.unwrap_or(DEFAULT_BUDGET);
    let results: Vec<Value> = inputs
        .iter()
        .map(|input| {
            let outcome = match parser {
                Parser::Brute | Parser::BruteUnrestricted => {
                    brute_parse(g, input, budget).map(|x| object(&x))
                }
                Parser::Ll1 => ll1_parse(g, input).map(|x| object(&x)),
                Parser::Slr1 => slr_parse(g, input).map(|x| object(&x)),
                Parser::Cyk => cyk_parse(g, input).map(|x| object(&x)),
            };
            match outcome {
                Ok(result) => finish(result, input),
                Err(e) => parse_failure(input, &e),
            }
        })
        .collect();
    Ok(json!({"parser": to_value(&parser), "results": results}))
}

// ---------------------------------------------------------------------------------------------
// parse_tables
// ---------------------------------------------------------------------------------------------

fn symbol_set(set: &BTreeSet<String>) -> Vec<String> {
    let mut out: Vec<String> = set
        .iter()
        .map(|s| {
            if s.is_empty() {
                LAMBDA.to_string()
            } else {
                s.clone()
            }
        })
        .collect();
    out.sort();
    out
}

pub fn parse_tables(doc: &Document) -> Result<Value, ApiError> {
    let g = grammar_of(doc, "parse_tables")?;
    let variables: Vec<String> = g.variables().iter().map(char::to_string).collect();

    let first = first_sets(g);
    let follow = follow_sets(g);
    let mut first_json = Map::new();
    let mut follow_json = Map::new();
    for v in &variables {
        first_json.insert(
            v.clone(),
            json!(first.get(v).map(symbol_set).unwrap_or_default()),
        );
        follow_json.insert(
            v.clone(),
            json!(follow.get(v).map(symbol_set).unwrap_or_default()),
        );
    }

    let ll = ll1_table(g);
    let mut table = Map::new();
    for (r, variable) in ll.variables.iter().enumerate() {
        let mut row = Map::new();
        for (c, terminal) in ll.terminals.iter().enumerate() {
            let cell = &ll.cells[r][c];
            if !cell.is_empty() {
                let entries: Vec<String> = cell
                    .iter()
                    .map(|rhs| production_text(variable, rhs))
                    .collect();
                row.insert(terminal.clone(), json!(entries));
            }
        }
        table.insert(variable.clone(), Value::Object(row));
    }
    let ll_conflicts: Vec<Value> = ll
        .conflicts()
        .iter()
        .map(|c| {
            json!({
                "variable": c.variable,
                "terminal": c.terminal,
                "entries": c.entries.iter()
                    .map(|rhs| production_text(&c.variable, rhs)).collect::<Vec<_>>(),
            })
        })
        .collect();

    let lr = slr_table(g);
    let productions: Vec<String> = lr
        .productions
        .iter()
        .map(|p: &Production| production_text(&p.lhs, &p.rhs))
        .collect();
    let states: Vec<Value> = lr
        .automaton
        .states
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "items": s.items.iter().map(|i| lr.automaton.item_text(i)).collect::<Vec<_>>(),
                "transitions": s.transitions,
                "is_final": s.is_final,
            })
        })
        .collect();
    let actions: Vec<Value> = lr
        .actions
        .iter()
        .map(|row| {
            let cells: Map<String, Value> = row
                .iter()
                .map(|(symbol, acts)| {
                    (
                        symbol.clone(),
                        json!(acts.iter().map(|a| a.text()).collect::<Vec<_>>()),
                    )
                })
                .collect();
            Value::Object(cells)
        })
        .collect();
    let lr_conflicts: Vec<Value> = lr
        .conflicts()
        .iter()
        .map(|c| {
            json!({
                "state": c.state,
                "symbol": c.symbol,
                "actions": c.actions.iter().map(|a| a.text()).collect::<Vec<_>>(),
            })
        })
        .collect();

    Ok(json!({
        "start": g.start(),
        "first": first_json,
        "follow": follow_json,
        "ll1": {
            "is_ll1": ll.is_ll1(),
            "variables": ll.variables,
            "terminals": ll.terminals,
            "table": table,
            "conflicts": ll_conflicts,
        },
        "slr1": {
            "is_slr1": lr.is_slr1(),
            "productions": productions,
            "states": states,
            "terminals": lr.terminals,
            "variables": lr.variables,
            "actions": actions,
            "gotos": lr.gotos,
            "conflicts": lr_conflicts,
        },
    }))
}

// ---------------------------------------------------------------------------------------------
// user_parse
// ---------------------------------------------------------------------------------------------

fn derivation_json(d: &Derivation) -> (Value, Value) {
    (to_value(d), json!(d.sentential_forms()))
}

pub fn user_parse(r: UserParse) -> Result<Value, ApiError> {
    let g = grammar_of(&r.grammar, "user_parse")?;
    let mut parser = UserParser::new(g, &r.target)
        .map_err(|e| ApiError::new(code::PARSE_ERROR, e.to_string()).with_details(to_value(&e)))?;
    for (index, mv) in r.moves.iter().enumerate() {
        let applied: Result<(), UserParseError> = match mv {
            UserMove::Undo(UndoMove::Undo) => parser.undo(),
            UserMove::Apply {
                production,
                positions: Some(positions),
                ..
            } => parser.apply_many(*production, positions),
            UserMove::Apply {
                production,
                position,
                ..
            } => parser.apply(*production, *position),
        };
        if let Err(e) = applied {
            return Err(
                ApiError::new(code::INVALID_MOVE, format!("move #{index}: {e}"))
                    .with_details(json!({"move_index": index, "error": to_value(&e)})),
            );
        }
    }
    let applicable: Vec<Value> = g
        .productions()
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let positions = parser.positions(i);
            (!positions.is_empty()).then(|| {
                json!({
                    "production": i,
                    "text": production_text(&p.lhs, &p.rhs),
                    "positions": positions,
                })
            })
        })
        .collect();
    let (derivation, forms) = derivation_json(&parser.derivation());
    Ok(json!({
        "target": parser.target(),
        "sentential": parser.sentential(),
        "finished": parser.is_finished(),
        "derivation": derivation,
        "derivation_forms": forms,
        "tree": parser.tree().map(|t| to_value(&t)),
        "applicable": applicable,
    }))
}
