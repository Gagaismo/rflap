//! `pumping list` and `pumping play`.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::api;
use crate::error::{CliError, Result};
use crate::input;
use crate::text::{Reply, exit, plain};

pub fn list() -> Result<Reply> {
    let answer = api::call(&json!({"op": "pumping_languages"}))?;
    let mut out = String::new();
    for language in api::result(&answer)["languages"]
        .as_array()
        .into_iter()
        .flatten()
    {
        out.push_str(&format!(
            "{:>2}  {:<12}  {}\n",
            plain(&language["index"]),
            plain(&language["kind"]),
            plain(&language["title"])
        ));
    }
    Ok(Reply::new(out, &answer, exit::OK))
}

const MOVES: &str = "moves are m=N, w=TEXT, decomposition=N,N, i=N, clear_all, add_case, \
replace_case=ROW, show_case=ROW, delete_case=ROW, list_cases, clear_cases, cases_done or a JSON move";

fn number<T: std::str::FromStr>(text: &str, whole: &str) -> Result<T> {
    text.trim()
        .parse()
        .map_err(|_| CliError::arg(format!("bad number in move `{whole}`; {MOVES}")))
}

/// Reads a move such as `m=4`, `w=aabb`, `add_case` or a JSON object.
pub fn parse_move(text: &str) -> Result<Value> {
    let text = text.trim();
    if text.starts_with('{') {
        return serde_json::from_str(text)
            .map_err(|e| CliError::new("invalid_json", format!("move `{text}`: {e}")));
    }
    let unknown = || CliError::arg(format!("unknown move `{text}`; {MOVES}"));
    let (key, value) = match text.split_once('=') {
        Some((k, v)) => (k.trim(), Some(v)),
        None => (text, None),
    };
    let row = |name: &str| -> Result<Value> {
        let value = value.ok_or_else(unknown)?;
        Ok(json!({"move": name, "row": number::<usize>(value, text)?}))
    };
    match (key, value) {
        ("m", Some(v)) => Ok(json!({"move": "choose_m", "m": number::<i32>(v, text)?})),
        ("w", Some(v)) => Ok(json!({"move": "choose_w", "w": input::input_string(v)})),
        ("i", Some(v)) => Ok(json!({"move": "choose_i", "i": number::<i32>(v, text)?})),
        ("decomposition" | "d", Some(v)) => {
            let lengths: Result<Vec<i32>> = v.split(',').map(|n| number::<i32>(n, text)).collect();
            Ok(json!({"move": "choose_decomposition", "lengths": lengths?}))
        }
        ("replace_case" | "show_case" | "delete_case", _) => row(key),
        ("clear_all" | "add_case" | "list_cases" | "clear_cases" | "cases_done", None) => {
            Ok(json!({"move": key}))
        }
        _ => Err(unknown()),
    }
}

/// The view of a game: title, objective, visible boxes, prompt, result.
fn show(result: &Value) -> String {
    let view = &result["view"];
    let mut out = format!("{}\n", plain(&view["title"]));
    out.push_str(&format!("Objective: {}\n", plain(&view["objective"])));
    for b in view["boxes"].as_array().into_iter().flatten() {
        if b["visible"] != true {
            continue;
        }
        let content = plain(&b["content"]);
        if content.is_empty() {
            out.push_str(&format!(
                "[{}] {}\n",
                plain(&b["number"]),
                plain(&b["caption"])
            ));
        } else {
            out.push_str(&format!(
                "[{}] {content}  ({})\n",
                plain(&b["number"]),
                plain(&b["caption"])
            ));
        }
        let message = plain(&b["message"]);
        if !message.is_empty() {
            out.push_str(&format!("    {message}\n"));
        }
    }
    let prompt = plain(&view["prompt"]);
    if !prompt.is_empty() {
        out.push_str(&format!("Prompt: {prompt}\n"));
    }
    for (label, key) in [("Result", "result"), ("Conclusion", "conclusion")] {
        if let Some(text) = result[key].as_str().filter(|t| !t.is_empty()) {
            out.push_str(&format!("{label}: {text}\n"));
        }
    }
    out
}

pub fn play(
    kind: &str,
    language: &str,
    mode: &str,
    seed: Option<i64>,
    moves: &[String],
    json_mode: bool,
) -> Result<Reply> {
    let kind = kind.trim().to_ascii_lowercase().replace('-', "_");
    let language = match language.trim().parse::<usize>() {
        Ok(index) => json!(index),
        Err(_) => json!(language),
    };
    let mut request = json!({
        "op": "pumping_new", "kind": kind, "language": language,
        "mode": mode.trim().replace('-', "_"),
    });
    if let Some(seed) = seed {
        request["seed"] = json!(seed);
    }
    // read every scripted move before touching the game: a typo should cost nothing
    let scripted: Vec<(String, Value)> = moves
        .iter()
        .map(|m| parse_move(m).map(|parsed| (m.trim().to_string(), parsed)))
        .collect::<Result<_>>()?;
    let mut answer = api::call(&request)?;
    let mut answers = vec![answer.clone()];
    let mut out = show(api::result(&answer));
    let interactive = scripted.is_empty();
    let mut play_move = |label: &str, mv: Value, answer: &mut Value, out: &mut String| {
        let game = api::result(answer)["document"].clone();
        let next = api::call(&json!({"op": "pumping_move", "game": game, "move": mv}))?;
        out.push_str(&format!("> {label}\n"));
        let reply = plain(&api::result(&next)["reply"]["message"]);
        if !reply.is_empty() {
            out.push_str(&format!("reply: {reply}\n"));
        }
        *answer = next.clone();
        answers.push(next);
        Ok::<(), CliError>(())
    };
    if interactive {
        // each move is answered with the new view; the output is streamed
        if json_mode {
            println!("{answer}");
        } else {
            print!("{out}");
        }
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = line.map_err(|e| CliError::new("io_error", e.to_string()))?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if line == "quit" || line == "exit" {
                break;
            }
            match parse_move(line) {
                Ok(mv) => {
                    let mut chunk = String::new();
                    play_move(line, mv, &mut answer, &mut chunk)?;
                    if json_mode {
                        println!("{answer}");
                    } else {
                        chunk.push_str(&show(api::result(&answer)));
                        print!("{chunk}");
                    }
                    std::io::stdout().flush().ok();
                }
                Err(e) => eprintln!("error: {e}"),
            }
        }
    } else {
        for (label, mv) in scripted {
            play_move(&label, mv, &mut answer, &mut out)?;
        }
        out.push_str(&show(api::result(&answer)));
    }
    if interactive {
        return Ok(Reply {
            text: String::new(),
            json: String::new(),
            code: exit::OK,
        });
    }
    let json = if answers.len() == 1 {
        answers[0].clone()
    } else {
        Value::Array(answers)
    };
    Ok(Reply::new(out, &json, exit::OK))
}
