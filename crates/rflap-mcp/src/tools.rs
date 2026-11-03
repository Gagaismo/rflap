//! `tools/call`: each tool adapts one or more `rflap_api` operations and writes the Markdown
//! for chats without the widget. No algorithm lives here.

use std::path::Path;

use serde_json::{Map, Value, json};

use crate::schemas::{self, ToolDef};
use crate::{Config, content, viewer};

/// What a tool produced; becomes the `CallToolResult`.
pub struct ToolOutput {
    pub text: String,
    pub structured: Option<Value>,
    pub is_error: bool,
}

impl ToolOutput {
    fn error(message: impl Into<String>) -> Self {
        ToolOutput {
            text: message.into(),
            structured: None,
            is_error: true,
        }
    }

    pub fn to_json(&self) -> Value {
        let mut result = json!({
            "content": [{"type": "text", "text": self.text}],
            "isError": self.is_error,
        });
        if let Some(structured) = &self.structured {
            result["structuredContent"] = structured.clone();
        }
        result
    }
}

/// A successful tool: Markdown, the data for the widget, and the document a viewer link opens.
struct Done {
    text: String,
    structured: Value,
}

type Outcome = Result<Done, String>;

/// Runs one tool. Tool failures are results (`isError`), never protocol errors.
pub fn run(config: &Config, name: &str, args: &Value) -> ToolOutput {
    let tools = schemas::tools();
    let Some(def) = tools.iter().find(|t| t.name == name) else {
        let names: Vec<&str> = tools.iter().map(|t| t.name).collect();
        return ToolOutput::error(format!(
            "Unknown tool `{name}`. The tools are: {}.",
            names.join(", ")
        ));
    };
    let args = match check_args(def, args) {
        Ok(args) => args,
        Err(message) => return ToolOutput::error(message),
    };
    let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatch(config, name, &args)
    }))
    .unwrap_or_else(|_| Err("the tool panicked; this is a bug in rflap-mcp".into()));
    match done {
        Ok(mut done) => {
            if def.visual {
                done.structured["tool"] = json!(name);
                if let (Some(base), Some(doc)) =
                    (&config.viewer_url, done.structured.get("document"))
                    && let Some(url) = viewer::link(base, doc)
                {
                    done.text
                        .push_str(&format!("\n\n[Open in the Rflap viewer]({url})"));
                }
            }
            ToolOutput {
                text: content::clip(&done.text),
                structured: Some(done.structured),
                is_error: false,
            }
        }
        Err(message) => ToolOutput::error(message),
    }
}

/// The arguments as an object with no unknown key and every required key present.
fn check_args(def: &ToolDef, args: &Value) -> Result<Map<String, Value>, String> {
    let object = match args {
        Value::Null => Map::new(),
        Value::Object(o) => o.clone(),
        _ => return Err(format!("`{}` takes a JSON object of arguments", def.name)),
    };
    let properties = def.input["properties"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    if let Some(unknown) = object.keys().find(|k| !properties.contains_key(*k)) {
        let known: Vec<&str> = properties.keys().map(String::as_str).collect();
        return Err(format!(
            "`{}` has no argument `{unknown}`; its arguments are {}.",
            def.name,
            known.join(", ")
        ));
    }
    for required in def.input["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if object.get(required).is_none_or(Value::is_null) {
            return Err(format!("`{}` needs the argument `{required}`.", def.name));
        }
    }
    Ok(object)
}

// ---------------------------------------------------------------------------------------------
// The API
// ---------------------------------------------------------------------------------------------

/// Runs one API request; the error is the API's message with its code.
fn api(request: Value) -> Result<Value, String> {
    let answer = rflap_api::call(&request.to_string());
    let answer: Value =
        serde_json::from_str(&answer).map_err(|e| format!("unreadable API answer: {e}"))?;
    if answer["ok"] == true {
        return Ok(answer["result"].clone());
    }
    let error = &answer["error"];
    let mut message = format!(
        "Rflap error `{}`: {}",
        error["code"].as_str().unwrap_or("internal"),
        error["message"].as_str().unwrap_or("unknown error")
    );
    if let Some(details) = error.get("details").filter(|d| !d.is_null()) {
        let details = details.to_string();
        let shown: String = details.chars().take(600).collect();
        message.push_str(&format!("\nDetails: {shown}"));
    }
    Err(message)
}

fn op(name: &str, args: &Map<String, Value>) -> Value {
    let mut request = args.clone();
    request.insert("op".into(), json!(name));
    Value::Object(request)
}

/// Removes arguments the API does not know.
fn drop(request: &mut Value, keys: &[&str]) {
    if let Some(map) = request.as_object_mut() {
        for key in keys {
            map.remove(*key);
        }
    }
}

/// Moves an argument under the name the API uses.
fn take(request: &mut Value, key: &str) -> Option<Value> {
    request.as_object_mut().and_then(|map| map.remove(key))
}

fn object(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(o) => o,
        _ => Map::new(),
    }
}

/// The document of the arguments in full form (an AI may have written the compact form).
fn normalized(args: &Map<String, Value>, key: &str) -> Result<Value, String> {
    let raw = args.get(key).cloned().unwrap_or(Value::Null);
    let raw = match raw {
        Value::String(text) => json!({"type": "grammar", "text": text}),
        other => other,
    };
    Ok(api(json!({"op": "normalize", "document": raw}))?["document"].clone())
}

/// A transition table or production list, when the document has one.
fn table_of(document: &Value) -> Option<String> {
    let rendered = api(json!({"op": "render", "document": document, "format": "markdown"})).ok()?;
    rendered["content"]
        .as_str()
        .map(|s| s.trim_end().to_string())
}

fn svg_of(document: &Value) -> Option<String> {
    let rendered = api(json!({"op": "render", "document": document, "format": "svg"})).ok()?;
    rendered["content"].as_str().map(str::to_string)
}

fn structured(mut result: Value, document: Option<Value>) -> Value {
    if let (Some(doc), Value::Object(map)) = (document, &mut result)
        && !map.contains_key("document")
    {
        map.insert("document".into(), doc);
    }
    result
}

// ---------------------------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------------------------

fn dispatch(config: &Config, name: &str, args: &Map<String, Value>) -> Outcome {
    match name {
        "rflap_design_automaton" => design(args),
        "rflap_show" => show(args),
        "rflap_simulate" => simulate(args),
        "rflap_step" => step(args),
        "rflap_convert" => convert(args),
        "rflap_equivalent" => {
            let result = api(op("equivalent", args))?;
            Ok(Done {
                text: content::equivalent(&result),
                structured: result,
            })
        }
        "rflap_combine" => {
            let result = api(op("combine", args))?;
            let doc = &result["document"];
            let text = content::document(doc, table_of(doc).as_deref());
            Ok(Done {
                text,
                structured: result,
            })
        }
        "rflap_analyze" => {
            let result = api(op("analyze", args))?;
            Ok(Done {
                text: content::analysis(&result),
                structured: result,
            })
        }
        "rflap_grammar" => grammar(args),
        "rflap_parse" => {
            let result = api(op("parse", args))?;
            let text = content::parse(&result);
            Ok(Done {
                structured: structured(result, Some(normalized(args, "grammar")?)),
                text,
            })
        }
        "rflap_parse_tables" => {
            let result = api(op("parse_tables", args))?;
            let text = content::parse_tables(&result);
            Ok(Done {
                structured: structured(result, Some(normalized(args, "grammar")?)),
                text,
            })
        }
        "rflap_render" => render(args),
        "rflap_open_jff" => open_jff(config, args),
        "rflap_save_jff" => save_jff(config, args),
        "rflap_lsystem_render" => lsystem_render(args),
        "rflap_pumping" => pumping(args),
        "rflap_capabilities" => capabilities(args),
        "rflap_call" => raw_call(args),
        other => Err(format!("Unknown tool `{other}`.")),
    }
}

fn design(args: &Map<String, Value>) -> Outcome {
    const SOURCES: [(&str, &str); 5] = [
        ("constraint_text", "constraint_text"),
        ("constraint", "constraint"),
        ("regex", "regex"),
        ("grammar", "regular_grammar"),
        ("template", "template"),
    ];
    let given: Vec<(&str, &Value)> = SOURCES
        .iter()
        .filter_map(|(arg, field)| args.get(*arg).filter(|v| !v.is_null()).map(|v| (*field, v)))
        .collect();
    if given.len() != 1 {
        return Err(format!(
            "Give exactly one of constraint_text, constraint, regex, grammar or template (got {}).",
            given.len()
        ));
    }
    let mut spec = json!({
        "alphabet": args["alphabet"],
        "language": {given[0].0: given[0].1},
    });
    if let Some(examples) = args.get("examples").filter(|v| !v.is_null()) {
        spec["examples"] = examples.clone();
    }
    let mut request = json!({"op": "design", "spec": spec});
    if let Some(options) = args.get("options").filter(|v| !v.is_null()) {
        request["options"] = options.clone();
    }
    let result = api(request)?;
    let table = table_of(&result["document"]);
    Ok(Done {
        text: content::design(&result, table.as_deref()),
        structured: result,
    })
}

fn show(args: &Map<String, Value>) -> Outcome {
    let document = normalized(args, "document")?;
    let mut out = object(json!({"document": document}));
    let mut text = content::document(&document, None);
    if let Ok(analysis) = api(json!({"op": "analyze", "document": document})) {
        text.push_str(&format!(
            "

{}",
            content::analysis(&analysis)
        ));
        out.insert("analysis".into(), analysis);
    }
    if let Ok(explained) = api(json!({"op": "explain", "document": document})) {
        out.insert("explanation".into(), explained["markdown"].clone());
    }
    if let Some(svg) = svg_of(&document) {
        out.insert("svg".into(), json!(svg));
    }
    if let Some(table) = table_of(&document) {
        text.push_str(&format!(
            "

Transition table / productions:

{table}"
        ));
    }
    Ok(Done {
        text,
        structured: Value::Object(out),
    })
}

fn simulate(args: &Map<String, Value>) -> Outcome {
    let document = normalized(args, "document")?;
    let mut request = op("simulate", args);
    request["document"] = document.clone();
    if let Some(batch) = take(&mut request, "inputs") {
        drop(&mut request, &["input", "tapes"]);
        request["op"] = json!("multiple_run");
        request["inputs"] = batch;
        let result = api(request)?;
        return Ok(Done {
            text: content::batch(&result),
            structured: structured(result, Some(document)),
        });
    }
    if let Some(tapes) = take(&mut request, "tapes") {
        request["inputs"] = tapes;
    }
    let result = api(request)?;
    let shown = args
        .get("input")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| args.get("tapes").map(Value::to_string))
        .unwrap_or_default();
    Ok(Done {
        text: content::simulate(&result, &shown),
        structured: structured(result, Some(document)),
    })
}

fn step(args: &Map<String, Value>) -> Outcome {
    let document = normalized(args, "document")?;
    let mut request = op("step", args);
    request["document"] = document.clone();
    if let Some(tapes) = take(&mut request, "tapes") {
        request["inputs"] = tapes;
    }
    let result = api(request)?;
    let shown = args
        .get("input")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Ok(Done {
        text: content::step(&result, shown),
        structured: structured(result, Some(document)),
    })
}

fn convert(args: &Map<String, Value>) -> Outcome {
    let result = api(op("convert", args))?;
    let doc = &result["document"];
    let to = result["to"].as_str().unwrap_or("?");
    let mut text = format!(
        "Converted to `{to}`.\n\n{}",
        content::document(doc, table_of(doc).as_deref())
    );
    if let Some(expression) = doc["expression"].as_str() {
        text.push_str(&format!("\n\nResult: `{expression}`"));
    }
    let steps = result["steps"].as_array().map_or(0, Vec::len);
    if steps > 0 {
        text.push_str(&format!(
            "\n\nThe construction has {steps} recorded steps (structuredContent.steps)."
        ));
    }
    Ok(Done {
        text,
        structured: result,
    })
}

fn grammar(args: &Map<String, Value>) -> Outcome {
    let action = args.get("action").and_then(Value::as_str).unwrap_or("type");
    let to = match action {
        "type" => {
            let result = api(op(
                "grammar_type",
                &object(json!({"grammar": args["grammar"]})),
            ))?;
            let text = content::grammar_type(&result);
            return Ok(Done {
                structured: structured(result, Some(normalized(args, "grammar")?)),
                text,
            });
        }
        "to_fa" => "fa",
        "to_pda_ll" => "pda_ll",
        "to_pda_lr" => "pda_lr",
        other => other,
    };
    let document = normalized(args, "grammar")?;
    let result = api(json!({"op": "convert", "document": document, "to": to}))?;
    let doc = &result["document"];
    let text = format!(
        "Applied `{action}`.\n\n{}",
        content::document(doc, table_of(doc).as_deref())
    );
    Ok(Done {
        text,
        structured: result,
    })
}

fn render(args: &Map<String, Value>) -> Outcome {
    let mut request = op("render", args);
    drop(&mut request, &["include_svg"]);
    let document = normalized(args, "document")?;
    request["document"] = document.clone();
    let result = api(request)?;
    let format = result["format"].as_str().unwrap_or("svg");
    let body = result["content"].as_str().unwrap_or_default();
    let text = match format {
        "markdown" => body.trim_end().to_string(),
        "svg" => {
            let table = table_of(&document);
            let mut text = content::document(&document, table.as_deref());
            text.push_str(&format!("\n\n{}", content::fence("svg", body)));
            text
        }
        other => content::fence(other, body),
    };
    Ok(Done {
        text,
        structured: structured(result, Some(document)),
    })
}

fn open_jff(config: &Config, args: &Map<String, Value>) -> Outcome {
    let xml = match (
        args.get("xml").and_then(Value::as_str),
        args.get("path").and_then(Value::as_str),
    ) {
        (Some(xml), None) => xml.to_string(),
        (None, Some(path)) => {
            files_allowed(config)?;
            std::fs::read_to_string(path).map_err(|e| format!("could not read `{path}`: {e}"))?
        }
        _ => return Err("Give exactly one of `xml` or `path`.".into()),
    };
    let result = api(json!({"op": "open_jff", "xml": xml}))?;
    let doc = &result["document"];
    let text = format!(
        "Opened a .jff file.\n\n{}",
        content::document(doc, table_of(doc).as_deref())
    );
    Ok(Done {
        text,
        structured: result,
    })
}

fn files_allowed(config: &Config) -> Result<(), String> {
    if config.allow_files {
        Ok(())
    } else {
        Err(
            "This server does not access files (it is not running locally); pass the text instead."
                .into(),
        )
    }
}

fn save_jff(config: &Config, args: &Map<String, Value>) -> Outcome {
    let result = api(op(
        "save_jff",
        &object(json!({"document": args["document"]})),
    ))?;
    let xml = result["xml"].as_str().unwrap_or_default();
    let Some(path) = args.get("path").and_then(Value::as_str) else {
        return Ok(Done {
            text: format!(
                "The .jff file for JFLAP 7.1:\n\n{}",
                content::fence("xml", xml)
            ),
            structured: result,
        });
    };
    files_allowed(config)?;
    let target = Path::new(path);
    if target
        .extension()
        .and_then(|e| e.to_str())
        .is_none_or(|e| !e.eq_ignore_ascii_case("jff"))
    {
        return Err(format!("`{path}` must end in .jff."));
    }
    let overwrite = args
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if target.exists() && !overwrite {
        return Err(format!(
            "`{path}` already exists; pass overwrite: true to replace it."
        ));
    }
    std::fs::write(target, xml).map_err(|e| format!("could not write `{path}`: {e}"))?;
    Ok(Done {
        text: format!("Saved `{path}` ({} bytes); JFLAP 7.1 opens it.", xml.len()),
        structured: result,
    })
}

fn lsystem_render(args: &Map<String, Value>) -> Outcome {
    let include = args
        .get("include_svg")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut request = op("lsystem_render", args);
    drop(&mut request, &["include_svg"]);
    let document = normalized(args, "document")?;
    request["document"] = document.clone();
    let result = api(request)?;
    let mut text = content::lsystem(&result);
    if include {
        text.push_str(&format!(
            "\n\n{}",
            content::fence("svg", result["svg"].as_str().unwrap_or_default())
        ));
    }
    Ok(Done {
        text,
        structured: structured(result, Some(document)),
    })
}

fn pumping(args: &Map<String, Value>) -> Outcome {
    let action = args
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match action {
        "languages" => {
            let result = api(json!({"op": "pumping_languages"}))?;
            Ok(Done {
                text: content::pumping_languages(&result),
                structured: result,
            })
        }
        "new" => {
            let mut request = op("pumping_new", args);
            drop(&mut request, &["action"]);
            let result = api(request)?;
            Ok(Done {
                text: content::pumping(&result),
                structured: result,
            })
        }
        "move" => {
            let mut request = op("pumping_move", args);
            drop(&mut request, &["action"]);
            let result = api(request)?;
            Ok(Done {
                text: content::pumping(&result),
                structured: result,
            })
        }
        other => Err(format!(
            "`action` must be languages, new or move (got `{other}`)."
        )),
    }
}

fn capabilities(args: &Map<String, Value>) -> Outcome {
    let section = args
        .get("section")
        .and_then(Value::as_str)
        .unwrap_or("overview");
    let result = api(json!({"op": "capabilities"}))?;
    Ok(Done {
        text: content::capabilities(&result, section),
        structured: result,
    })
}

/// `rflap_call` answers the `rflap_api::call` envelope (`{"ok": ..., "result" | "error"}`) as
/// is, errors included, because the widget's remote fallback (`web/README.md`) reads it from
/// `structuredContent`. `request` is the request as JSON text, or the request object itself.
fn raw_call(args: &Map<String, Value>) -> Outcome {
    let request = match &args["request"] {
        Value::String(text) => text.clone(),
        object @ Value::Object(_) => object.to_string(),
        _ => {
            return Err(
                "`request` must be the request as JSON text (or a JSON object) such as                  {\"op\": \"capabilities\"}."
                    .into(),
            );
        }
    };
    let envelope: Value = serde_json::from_str(&rflap_api::call(&request))
        .map_err(|e| format!("unreadable API answer: {e}"))?;
    let text = content::fence(
        "json",
        &serde_json::to_string_pretty(&envelope).unwrap_or_default(),
    );
    Ok(Done {
        text,
        structured: envelope,
    })
}
