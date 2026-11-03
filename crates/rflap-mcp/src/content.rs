//! Markdown for `content`: what a chat without the widget (Claude Code, Codex) shows.
//!
//! Everything here is a pure function of the API's JSON, so it is cheap to keep in step with
//! the API: a field that is missing is skipped, never an error.

use std::fmt::Write;

use serde_json::Value;

/// Longest text put in a result before it is cut (the full data stays in `structuredContent`).
const MAX_TEXT: usize = 24_000;

pub fn fence(lang: &str, body: &str) -> String {
    format!("```{lang}\n{}\n```", body.trim_end())
}

/// Cuts `text` to `MAX_TEXT` bytes on a character boundary.
pub fn clip(text: &str) -> String {
    if text.len() <= MAX_TEXT {
        return text.to_string();
    }
    let mut end = MAX_TEXT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n… (cut, {} more bytes; the whole result is in structuredContent)",
        &text[..end],
        text.len() - end
    )
}

fn str_of(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

fn compact(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// A string as the user would type it: the empty string shows as λ.
pub fn word(s: &str) -> String {
    if s.is_empty() {
        "λ".into()
    } else {
        s.to_string()
    }
}

fn names(list: &Value) -> String {
    let items: Vec<&str> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if items.is_empty() {
        "none".into()
    } else {
        items.join(", ")
    }
}

fn cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn row(cells: &[String]) -> String {
    format!(
        "| {} |",
        cells
            .iter()
            .map(|c| cell(c))
            .collect::<Vec<_>>()
            .join(" | ")
    )
}

fn table(header: &[String], rows: &[Vec<String>]) -> String {
    let mut out = row(header);
    out.push('\n');
    out.push_str(&format!("|{}|", vec!["---"; header.len()].join("|")));
    for r in rows {
        out.push('\n');
        out.push_str(&row(r));
    }
    out
}

/// One configuration of a trace, e.g. `q1 [remaining b, stack AZ]`.
fn configuration(entry: &Value) -> String {
    let Some(object) = entry.as_object() else {
        return compact(entry);
    };
    let state = object.get("state").map(compact).unwrap_or_default();
    let extras: Vec<String> = object
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "state" | "state_id"))
        .map(|(k, v)| match v {
            Value::String(s) => format!("{k} {}", word(s)),
            other => format!("{k} {other}"),
        })
        .collect();
    if extras.is_empty() {
        state
    } else {
        format!("{state} [{}]", extras.join(", "))
    }
}

pub fn trace(trace: &Value) -> String {
    let steps: Vec<String> = trace
        .as_array()
        .into_iter()
        .flatten()
        .map(configuration)
        .collect();
    if steps.is_empty() {
        "no configuration reached".into()
    } else {
        steps.join(" → ")
    }
}

fn outcome_line(result: &Value) -> String {
    match str_of(&result["outcome"]) {
        "accepted" => "✅ accepted".into(),
        "rejected" => "❌ rejected".into(),
        "undecided" => "❓ undecided (the configuration budget ran out)".into(),
        other => other.to_string(),
    }
}

pub fn simulate(result: &Value, input: &str) -> String {
    let mut out = format!("Input `{}`: {}", word(input), outcome_line(result));
    if let Some(output) = result.get("output").filter(|v| !v.is_null()) {
        let _ = write!(out, "\n\nOutput: `{}`", word(str_of(output)));
    }
    if let Some(tapes) = result.get("tape_outputs").filter(|v| !v.is_null()) {
        let _ = write!(out, "\n\nTape contents: `{tapes}`");
    }
    let _ = write!(out, "\n\nTrace: {}", trace(&result["trace"]));
    out
}

pub fn batch(result: &Value) -> String {
    let rows: Vec<Vec<String>> = result["results"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            let shown = match &r["input"] {
                Value::Array(tapes) => tapes
                    .iter()
                    .map(|t| word(str_of(t)))
                    .collect::<Vec<_>>()
                    .join(" | "),
                other => word(str_of(other)),
            };
            let mut cells = vec![shown, outcome_line(r)];
            if let Some(output) = r.get("output").filter(|v| !v.is_null()) {
                cells.push(word(str_of(output)));
            }
            cells
        })
        .collect();
    let with_output = rows.iter().any(|r| r.len() == 3);
    let rows: Vec<Vec<String>> = rows
        .into_iter()
        .map(|mut r| {
            if with_output && r.len() == 2 {
                r.push(String::new());
            }
            r
        })
        .collect();
    let mut header = vec!["input".to_string(), "result".to_string()];
    if with_output {
        header.push("output".into());
    }
    table(&header, &rows)
}

pub fn step(result: &Value, input: &str) -> String {
    let steps = result["steps_done"].as_u64().unwrap_or(0);
    let mut out = format!("After {steps} step(s) on `{}`:", word(input));
    let frontier = result["frontier"].as_array().cloned().unwrap_or_default();
    if frontier.is_empty() {
        out.push_str("\n- no live configuration");
    }
    for c in &frontier {
        let _ = write!(out, "\n- live: {}", configuration(c));
    }
    for c in result["accepted"].as_array().into_iter().flatten() {
        let _ = write!(out, "\n- ✅ accepted: {}", configuration(c));
    }
    for c in result["rejected"].as_array().into_iter().flatten() {
        let _ = write!(out, "\n- ❌ dead end: {}", configuration(c));
    }
    if result["finished"] == true {
        out.push_str("\n\nThe run is finished.");
    }
    if result["truncated"] == true {
        out.push_str("\n\n⚠ The frontier was cut at the configuration limit.");
    }
    out
}

pub fn analysis(result: &Value) -> String {
    let kind = str_of(&result["kind"]);
    let mut out = format!(
        "**{}** ({} states, {} transitions)",
        kind.to_uppercase(),
        result["states"].as_array().map_or(0, Vec::len),
        result["transitions"]
    );
    let _ = write!(out, "\n- alphabet: {}", names(&result["alphabet"]));
    let _ = write!(out, "\n- initial: {}", compact(&result["initial"]));
    let _ = write!(out, "\n- final states: {}", names(&result["finals"]));
    if let Some(deterministic) = result["deterministic"].as_bool() {
        let _ = write!(
            out,
            "\n- deterministic: {}",
            if deterministic { "yes" } else { "no" }
        );
    }
    if result["nondeterministic_states"]
        .as_array()
        .is_some_and(|a| !a.is_empty())
    {
        let _ = write!(
            out,
            "\n- nondeterministic states: {}",
            names(&result["nondeterministic_states"])
        );
    }
    if result["lambda_transitions"]
        .as_array()
        .is_some_and(|a| !a.is_empty())
    {
        let _ = write!(out, "\n- λ transitions: {}", result["lambda_transitions"]);
    }
    if let Some(complete) = result["complete"].as_bool() {
        let _ = write!(
            out,
            "\n- complete (every state has a move on every symbol): {}",
            if complete { "yes" } else { "no" }
        );
    }
    let _ = write!(
        out,
        "\n- unreachable states: {}",
        names(&result["unreachable_states"])
    );
    let _ = write!(
        out,
        "\n- useless states: {}",
        names(&result["useless_states"])
    );
    out
}

pub fn equivalent(result: &Value) -> String {
    if result["equivalent"] == true {
        "✅ Equivalent: both accept exactly the same language.".into()
    } else {
        match result.get("counterexample").filter(|v| !v.is_null()) {
            Some(c) => format!(
                "❌ Not equivalent. counterexample: `{}` (accepted by exactly one of them).",
                word(str_of(c))
            ),
            None => "❌ Not equivalent.".into(),
        }
    }
}

pub fn grammar_type(result: &Value) -> String {
    let mut out = format!(
        "**{}**: {}",
        str_of(&result["classification"]),
        str_of(&result["message"])
    );
    if let Some(report) = result["report"].as_object() {
        let yes: Vec<&str> = report
            .iter()
            .filter(|(k, v)| **v == true && k.as_str() != "turing_converted")
            .map(|(k, _)| k.as_str())
            .collect();
        let _ = write!(out, "\n\nIt is: {}.", yes.join(", "));
    }
    out
}

fn derivation(result: &Value) -> Option<String> {
    let forms = result["derivation_forms"].as_array()?;
    if forms.is_empty() {
        return None;
    }
    Some(
        forms
            .iter()
            .map(|f| word(str_of(f)))
            .collect::<Vec<_>>()
            .join(" ⇒ "),
    )
}

pub fn parse(result: &Value) -> String {
    let mut out = format!("Parser: `{}`", str_of(&result["parser"]));
    for r in result["results"].as_array().into_iter().flatten() {
        let _ = write!(
            out,
            "\n\n- `{}`: {}",
            word(str_of(&r["input"])),
            outcome_line(r)
        );
        if let Some(error) = r.get("error").filter(|v| !v.is_null()) {
            let _ = write!(out, " — {}", compact(error));
        }
        if r["outcome"] == "accepted"
            && let Some(d) = derivation(r)
        {
            let _ = write!(out, "\n  derivation: {d}");
        }
    }
    out
}

fn list(v: &Value) -> String {
    let items: Vec<String> = v
        .as_array()
        .into_iter()
        .flatten()
        .map(|x| word(str_of(x)))
        .collect();
    format!("{{{}}}", items.join(", "))
}

pub fn parse_tables(result: &Value) -> String {
    let mut out = String::new();
    if let (Some(first), Some(follow)) = (result["first"].as_object(), result["follow"].as_object())
    {
        out.push_str("### FIRST and FOLLOW\n\n");
        let rows: Vec<Vec<String>> = first
            .iter()
            .map(|(variable, f)| vec![variable.clone(), list(f), list(&follow[variable])])
            .collect();
        out.push_str(&table(
            &["variable".into(), "FIRST".into(), "FOLLOW".into()],
            &rows,
        ));
    }
    let ll1 = &result["ll1"];
    if let (Some(variables), Some(terminals)) =
        (ll1["variables"].as_array(), ll1["terminals"].as_array())
    {
        let verdict = if ll1["is_ll1"] == true {
            "the grammar is LL(1)"
        } else {
            "NOT LL(1): conflicts"
        };
        let _ = write!(out, "\n\n### LL(1) table ({verdict})\n\n");
        let mut header = vec!["".to_string()];
        header.extend(terminals.iter().map(|t| str_of(t).to_string()));
        let rows: Vec<Vec<String>> = variables
            .iter()
            .map(|v| {
                let variable = str_of(v);
                let mut cells = vec![variable.to_string()];
                for t in terminals {
                    let entries = &ll1["table"][variable][str_of(t)];
                    cells.push(
                        entries
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .map(|e| str_of(e).to_string())
                                    .collect::<Vec<_>>()
                                    .join("; ")
                            })
                            .unwrap_or_default(),
                    );
                }
                cells
            })
            .collect();
        out.push_str(&table(&header, &rows));
    }
    let slr = &result["slr1"];
    if let (Some(actions), Some(gotos), Some(terminals), Some(variables)) = (
        slr["actions"].as_array(),
        slr["gotos"].as_array(),
        slr["terminals"].as_array(),
        slr["variables"].as_array(),
    ) {
        let verdict = if slr["is_slr1"] == true {
            "the grammar is SLR(1)"
        } else {
            "NOT SLR(1): conflicts"
        };
        let _ = write!(out, "\n\n### SLR(1) ACTION and GOTO ({verdict})\n\n");
        let mut header = vec!["state".to_string()];
        header.extend(terminals.iter().map(|t| str_of(t).to_string()));
        header.extend(variables.iter().map(|v| str_of(v).to_string()));
        let rows: Vec<Vec<String>> = actions
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let mut cells = vec![i.to_string()];
                for t in terminals {
                    cells.push(
                        action[str_of(t)]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .map(|e| str_of(e).to_string())
                                    .collect::<Vec<_>>()
                                    .join("/")
                            })
                            .unwrap_or_default(),
                    );
                }
                for v in variables {
                    cells.push(
                        gotos[i][str_of(v)]
                            .as_u64()
                            .map(|n| n.to_string())
                            .unwrap_or_default(),
                    );
                }
                cells
            })
            .collect();
        out.push_str(&table(&header, &rows));
        if let Some(states) = slr["states"].as_array() {
            out.push_str("\n\nSLR(1) item sets:\n");
            for s in states {
                let items: Vec<&str> = s["items"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                let _ = write!(out, "\n- I{}: {}", s["id"], items.join("  "));
            }
        }
    }
    out
}

/// One line about a document, plus its table when `table` is given.
pub fn document(doc: &Value, table: Option<&str>) -> String {
    let kind = str_of(&doc["type"]);
    let mut out = match kind {
        "regex" => format!("Regular expression: `{}`", str_of(&doc["expression"])),
        "lsystem" => format!("L-system, axiom `{}`", compact(&doc["axiom"])),
        "grammar" => format!(
            "Grammar with {} productions, start `{}`",
            doc["productions"].as_array().map_or(0, Vec::len),
            str_of(&doc["start"])
        ),
        "fa" | "pda" | "turing" | "mealy" | "moore" => {
            // The full form of a Turing machine nests its graph under `automaton`.
            let graph = if doc["automaton"].is_object() {
                &doc["automaton"]
            } else {
                doc
            };
            let states = match &graph["states"] {
                Value::Object(map) => map.len(),
                Value::Array(list) => list.len(),
                _ => 0,
            };
            let transitions = graph["transitions"].as_array().map_or(0, Vec::len);
            format!(
                "{} with {states} states and {transitions} transitions",
                machine_name(kind)
            )
        }
        other => format!("Document of type `{other}`"),
    };
    if let Some(table) = table {
        let _ = write!(out, "\n\n{table}");
    }
    out
}

pub fn machine_name(kind: &str) -> &'static str {
    match kind {
        "fa" => "Finite automaton",
        "pda" => "Pushdown automaton",
        "turing" => "Turing machine",
        "mealy" => "Mealy machine",
        "moore" => "Moore machine",
        _ => "Document",
    }
}

/// The report of `design`.
pub fn design(result: &Value, table: Option<&str>) -> String {
    let kind = str_of(&result["kind"]).to_uppercase();
    let stats = &result["stats"];
    let mut out = format!(
        "**Designed {kind}** for `{}`: {} states, {} transitions, {} final.",
        str_of(&result["description"]),
        stats["states"],
        stats["transitions"],
        stats["final_states"]
    );
    if let Some(before) = stats["states_before_minimization"].as_u64()
        && Some(before) != stats["states"].as_u64()
    {
        let _ = write!(out, " Minimized from {before} states.");
    }
    if let Some(regex) = result.get("regex").and_then(Value::as_str) {
        let _ = write!(out, "\n\nEquivalent regex: `{regex}`");
    }
    if let Some(states) = result["states"].as_array() {
        out.push_str("\n\nWhat each state remembers:\n");
        for s in states {
            let marks = format!(
                "{}{}",
                if s["initial"] == true { "→ " } else { "" },
                if s["accepting"] == true { "* " } else { "" }
            );
            let _ = write!(
                out,
                "\n- {marks}{} (reached by `{}`): {}",
                str_of(&s["name"]),
                word(str_of(&s["access_word"])),
                str_of(&s["meaning"])
            );
        }
    }
    if let Some(table) = table {
        let _ = write!(out, "\n\nTransition table (→ start, * final):\n\n{table}");
    }
    out.push_str("\n\n");
    out.push_str(&verification(&result["verification"]));
    if let Some(accepted) = result["accepted"].as_array().filter(|a| !a.is_empty()) {
        let shown: Vec<String> = accepted
            .iter()
            .map(|w| format!("`{}`", word(str_of(w))))
            .collect();
        let _ = write!(out, "\n\nShortest accepted strings: {}", shown.join(", "));
    }
    out
}

fn verification(v: &Value) -> String {
    let mut out = String::from("**Verification**: ");
    let examples = v["examples"].as_array().cloned().unwrap_or_default();
    let failed: Vec<&Value> = examples.iter().filter(|e| e["passed"] == false).collect();
    if examples.is_empty() {
        out.push_str("no examples were given (add accept/reject examples to check the spec). ");
    } else if failed.is_empty() {
        let _ = write!(
            out,
            "✅ all {} examples behave as expected. ",
            examples.len()
        );
    } else {
        let _ = write!(
            out,
            "❌ {} of {} examples FAIL — your spec or an example is wrong, fix it and design again:",
            failed.len(),
            examples.len()
        );
        for e in &failed {
            let _ = write!(
                out,
                "\n- `{}` expected {}, the automaton {}{}",
                word(str_of(&e["word"])),
                if e["expected_accept"] == true {
                    "accepted"
                } else {
                    "rejected"
                },
                if e["actual"] == true {
                    "accepts it"
                } else {
                    "rejects it"
                },
                e["detail"]
                    .as_str()
                    .map(|d| format!(" ({d})"))
                    .unwrap_or_default()
            );
        }
        out.push('\n');
    }
    let cross = &v["cross_check"];
    if cross["ok"] == true {
        let _ = write!(
            out,
            "✅ brute-force cross-check: {} strings up to length {} agree with the spec{}.",
            cross["strings_checked"],
            cross["max_len"],
            if cross["regex_agrees"] == true {
                " and with the regex"
            } else {
                ""
            }
        );
    } else if !cross.is_null() {
        let _ = write!(
            out,
            "❌ brute-force cross-check found {} mismatches",
            cross["mismatch_count"]
        );
        for m in cross["mismatches"].as_array().into_iter().flatten().take(5) {
            let _ = write!(out, "\n- {m}");
        }
    }
    out
}

pub fn pumping(result: &Value) -> String {
    let view = &result["view"];
    let mut out = String::new();
    if let Some(title) = view["title"].as_str() {
        let _ = write!(out, "**{title}**");
    }
    if let Some(objective) = view["objective"].as_str() {
        let _ = write!(out, "\n\nGoal: {objective}");
    }
    for b in view["boxes"].as_array().into_iter().flatten() {
        if b["visible"] == true {
            let content = str_of(&b["content"]);
            let _ = write!(
                out,
                "\n- {}{}",
                str_of(&b["caption"]),
                if content.is_empty() {
                    String::new()
                } else {
                    format!(" `{content}`")
                }
            );
        }
    }
    if let Some(reply) = result.get("reply").filter(|v| !v.is_null()) {
        let _ = write!(out, "\n\nReply: {}", compact(reply));
    }
    if let Some(prompt) = view["prompt"].as_str().filter(|p| !p.is_empty()) {
        let _ = write!(out, "\n\nNext: {prompt}");
    }
    if let Some(conclusion) = result["conclusion"].as_str().filter(|c| !c.is_empty()) {
        let _ = write!(out, "\n\nConclusion: {conclusion}");
    }
    out
}

pub fn pumping_languages(result: &Value) -> String {
    let rows: Vec<Vec<String>> = result["languages"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|l| {
            vec![
                compact(&l["kind"]),
                compact(&l["index"]),
                compact(&l["title"]),
            ]
        })
        .collect();
    table(&["kind".into(), "index".into(), "language".into()], &rows)
}

pub fn lsystem(result: &Value) -> String {
    format!(
        "L-system expanded {} level(s): {} symbols.",
        result["depth"], result["expansion_length"]
    )
}

/// `rflap_capabilities`: the overview or one section as Markdown.
pub fn capabilities(caps: &Value, section: &str) -> String {
    let mut out = String::new();
    let all = section == "all";
    if section == "overview" || all {
        let _ = write!(out, "{}\n\n", str_of(&caps["summary"]));
        let ops: Vec<&str> = caps["ops"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|o| o["op"].as_str())
            .collect();
        let _ = write!(out, "Operations: {}.\n\n", ops.join(", "));
        let types: Vec<&str> = caps["document_types"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|o| o["type"].as_str())
            .collect();
        let _ = write!(out, "Document types: {}.\n\n", types.join(", "));
        out.push_str(
            "Ask for section `operations`, `document_types` or `convert` for the details.\n",
        );
    }
    if section == "operations" || all {
        out.push_str("\n## Operations\n");
        for o in caps["ops"].as_array().into_iter().flatten() {
            let _ = write!(
                out,
                "\n- `{}` ({}): {}",
                str_of(&o["op"]),
                str_of(&o["params"]),
                str_of(&o["summary"])
            );
        }
        out.push('\n');
    }
    if section == "document_types" || all {
        out.push_str("\n## Document types\n");
        for t in caps["document_types"].as_array().into_iter().flatten() {
            let _ = write!(
                out,
                "\n### {}\n{}\n\n{}\n\n{}\n",
                str_of(&t["type"]),
                str_of(&t["summary"]),
                str_of(&t["compact"]),
                fence("json", &t["example"].to_string())
            );
        }
    }
    if section == "convert" || all {
        out.push_str("\n## Conversions (`to`)\n");
        for c in caps["convert"].as_array().into_iter().flatten() {
            let _ = write!(
                out,
                "\n- `{}` (from {}): {}",
                str_of(&c["to"]),
                str_of(&c["from"]),
                str_of(&c["summary"])
            );
        }
        out.push('\n');
    }
    out
}
