//! Markdown in the notation of the course "Sintaxis y Semántica de los Lenguajes" (UTN FRT):
//! the formal definition, the transition table, configuration sequences with `├─`, the ALL
//! check, composition and the sets Tk. Pure functions of the API's JSON, like `content`.

use std::fmt::Write;

use serde_json::Value;

use crate::content::{fence, word};

fn str_of(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

/// The formal definition `MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩` with its sets, and the issues found.
pub fn definition(result: &Value) -> String {
    let model = if str_of(&result["model"]) == "all" {
        "ALL"
    } else {
        "MT"
    };
    let mut out = format!("**Definición formal ({model})**\n\n");
    out.push_str(&fence("text", str_of(&result["text"])));
    let issues = result["issues"].as_array().cloned().unwrap_or_default();
    if issues.is_empty() {
        let kind = if result["deterministic"] == true {
            "determinista"
        } else {
            "no determinista"
        };
        let _ = write!(out, "\n\nLa máquina es {kind}.");
    } else {
        out.push_str("\n\nObservaciones:");
        for issue in &issues {
            let _ = write!(out, "\n- {}", str_of(&issue["message"]));
        }
    }
    out.push_str("\n\nLos estados finales son de parada: el dominio de f es (Q-F) x (Σ_E∪Σ_A).");
    out
}

/// What `analyze` adds for a Turing machine: the multipista report and the warnings of the course
/// definition, as Markdown bullets (empty when there is nothing to say).
pub fn analysis_notes(result: &Value) -> String {
    let mut out = String::new();
    let multipista = &result["multipista"];
    if multipista["applicable"] == true {
        if multipista["is_multipista"] == true {
            out.push_str(
                "
- multipista: sí (todos los cabezales se mueven juntos)",
            );
        } else {
            let _ = write!(
                out,
                "
- multipista: no (los cabezales se mueven distinto en las transiciones {}); es una MT multicinta",
                multipista["offending"]
            );
        }
    }
    for warning in result["catedra_warnings"].as_array().into_iter().flatten() {
        let _ = write!(
            out,
            "
- {}",
            str_of(warning)
        );
    }
    out
}

/// The transition table in the course format.
pub fn table(result: &Value) -> String {
    format!(
        "**Tabla de transición** (→ estado inicial, * estado final)\n\n{}",
        str_of(&result["markdown"]).trim_end()
    )
}

fn verdict_line(run: &Value) -> String {
    let steps = run["steps"].as_u64().unwrap_or(0);
    match str_of(&run["verdict"]) {
        "accepted" => format!("✅ final/acepta ({steps} pasos)"),
        "rejected" => format!("❌ bloqueo/rechazo ({steps} pasos)"),
        "no_halt" => format!("⏳ no se detiene dentro del límite ({steps} pasos)"),
        "violation" => "⚠️ viola las reglas del ALL".to_string(),
        other => other.to_string(),
    }
}

/// One run: its input, the verdict and the configuration sequence in a code block.
fn run_block(run: &Value, skip: &Value) -> String {
    let mut out = format!(
        "**Entrada `{}`**: {}\n\n{}",
        word(str_of(&run["input"])),
        verdict_line(run),
        fence("text", str_of(&run["sequence"]))
    );
    if let Some(message) = run["violation"]["message"].as_str() {
        let _ = write!(out, "\n\n⚠️ {message}");
    }
    out.push_str(&notes(&run["notes"], skip));
    out
}

/// `ℹ️ note` per line, each after a blank line; empty without notes.
fn notes(notes: &Value, skip: &Value) -> String {
    let skip = skip.as_array().cloned().unwrap_or_default();
    notes
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| !skip.contains(n))
        .filter_map(Value::as_str)
        .map(|n| {
            format!(
                "

ℹ️ {n}"
            )
        })
        .collect()
}

/// The configuration sequences of `catedra_run`.
pub fn run(result: &Value) -> String {
    let model = if str_of(&result["model"]) == "all" {
        "ALL"
    } else {
        "MT"
    };
    let blocks: Vec<String> = result["results"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| run_block(r, &Value::Null))
        .collect();
    format!(
        "**Secuencias de configuraciones ({model})**, `├─` es un paso de la máquina:\n\n{}",
        blocks.join("\n\n")
    )
}

/// The ALL check: violations (static and dynamic) and the sequences of the sample inputs.
pub fn all_check(result: &Value) -> String {
    let violations = result["violations"].as_array().cloned().unwrap_or_default();
    let runs = result["runs"].as_array().cloned().unwrap_or_default();
    let mut out = String::new();
    if result["ok"] == true {
        out.push_str("✅ La máquina cumple las reglas de un ALL: ");
        out.push_str("f(q,⟨) = (q′,⟨,D), f(q,⟩) = (q′,⟩,I), no lee ni escribe Δ");
        if runs.is_empty() {
            out.push_str(" (solo chequeo estático; pasá `inputs` para el chequeo dinámico).");
        } else if runs.iter().any(|r| r["verdict"] == "no_halt") {
            out.push_str(
                ", pero alguna entrada no se detiene dentro del límite (⏳): un ALL siempre \
                 decide, así que la máquina entra en un ciclo dentro de ⟨…⟩ o `max_steps` es \
                 chico.",
            );
        } else {
            out.push_str(" y el cabezal nunca sale de ⟨…⟩ en las entradas probadas.");
        }
    } else {
        let _ = write!(
            out,
            "❌ La máquina no es un ALL válido: {} violación(es).",
            violations.len()
        );
        out.push('\n');
        for v in &violations {
            let _ = write!(out, "\n- {}", str_of(&v["message"]));
        }
    }
    out.push_str(&notes(&result["notes"], &Value::Null));
    if !runs.is_empty() {
        let blocks: Vec<String> = runs
            .iter()
            .map(|r| run_block(r, &result["notes"]))
            .collect();
        let _ = write!(out, "\n\n{}", blocks.join("\n\n"));
    }
    out
}

/// The static violations of the ALL rules as a warning list; empty when there are none.
pub fn warnings(check: &Value) -> String {
    let violations = check["violations"].as_array().cloned().unwrap_or_default();
    if violations.is_empty() {
        return String::new();
    }
    let mut out = String::from("⚠️ Avisos del ALL (reglas que la máquina no cumple):");
    for v in &violations {
        let _ = write!(out, "\n- {}", str_of(&v["message"]));
    }
    out
}

/// The first configurations of a run in the ALL model (`rflap_step`).
pub fn step(result: &Value, steps: u64) -> String {
    let run = &result["results"][0];
    let configurations: Vec<&str> = run["configurations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let state = if run["verdict"] == "no_halt" {
        "⏳ la ejecución sigue después de estos pasos".to_string()
    } else {
        verdict_line(run)
    };
    format!(
        "**Entrada `{}`, primeros {steps} paso(s) (ALL)**: {state}\n\n{}{}",
        word(str_of(&run["input"])),
        fence("text", &configurations.join(" ├─ ")),
        notes(&run["notes"], &Value::Null)
    )
}

/// The reminder shown with a Turing machine that is not an ALL.
pub const ALL_HINT: &str = "Para verificarla como ALL (autómata linealmente acotado, cinta ⟨w⟩): \
    `rflap_mt_catedra` con action `all_check`, o `model: \"all\"` en esta herramienta.";

/// The composition `MT3 = ⟨Q1∪Q2, Σ_E, Σ_A, q01, F2, f3⟩`.
pub fn compose(definition_text: Option<&str>, table: Option<&str>) -> String {
    let mut out = String::from(
        "**Composición de MT**: MT3 = ⟨Q1∪Q2, Σ_E, Σ_A, q01, F2, f3⟩. f3 sigue a f1 en Q1, salvo que \
         una transición de f1 hacia un estado de F1 pasa a llevar a q02, y sigue a f2 en Q2 \
         (los estados de la segunda máquina llevan comilla).",
    );
    if let Some(text) = definition_text {
        let _ = write!(out, "\n\n{}", fence("text", text));
    }
    if let Some(table) = table {
        let _ = write!(out, "\n\n{table}");
    }
    out
}

/// Membership by the sets Tk.
pub fn membership(result: &Value) -> String {
    let verdict = if result["member"] == true {
        "✅ La palabra pertenece al lenguaje."
    } else {
        "❌ La palabra no pertenece al lenguaje."
    };
    format!(
        "**Comprobación de pertenencia por conjuntos Tk** (T0 = {{ S }}, Tk = Tk-1 ∪ {{ β / |β| ≤ N ∧ α ∈ Tk-1 ∧ α ⇒ β }}):\n\n{}\n\n{verdict}",
        fence("text", str_of(&result["text"]))
    )
}
