//! The words the server says to the AI: the `instructions` of `initialize` and the prompts.

use serde_json::{Value, json};

/// A guide for the model, sent once at `initialize`.
pub const INSTRUCTIONS: &str = "\
Rflap is JFLAP 7.1 in Rust. You are the reasoning; Rflap designs, draws, runs and verifies. \
Use it to help a student of \"Sintaxis y Semántica de los Lenguajes\" (finite automata, regular \
expressions, regular and context-free grammars, pushdown automata, Turing machines, LL(1)/SLR(1) \
parsing, pumping lemma).

Workflow for an exercise:
1. Reason about the language first: what must the machine remember? Write the alphabet and a formal \
spec (a constraint such as ends_with(\"ab\") and count(a) % 2 == 0, a regular expression, a regular \
grammar, or a template for a^n b^n and other non-regular languages). Pick 4-8 accept and 4-8 reject \
examples, always including the empty string and the boundary cases.
2. Call rflap_design_automaton with that spec. It builds the minimal automaton and verifies it \
against your examples and a brute-force cross-check. If a failing example is reported, your spec \
or the example is wrong: fix it and call again. Never present an automaton whose verification failed.
3. Show the result: visual tools open an interactive widget in hosts that support it; otherwise the \
reply already carries the transition table, and a viewer link when one is configured. Do not paste \
raw JSON or SVG at the student.
4. Explain it in the student's language (Spanish if they write Spanish): what each state remembers, \
why the transitions are what they are. rflap_simulate / rflap_step walk through a run.
5. For conversions and checks use rflap_convert (NFA→DFA, minimization, regex↔FA, grammar↔FA, CFG↔PDA, \
CNF…), rflap_equivalent (is the student's automaton equal to a regex?), rflap_grammar, rflap_parse and \
rflap_parse_tables (FIRST/FOLLOW, LL(1), SLR(1)).

Write documents in the compact form, e.g. {\"type\":\"fa\",\"initial\":\"q0\",\"finals\":[\"q2\"],\
\"transitions\":[[\"q0\",\"a\",\"q1\"],[\"q1\",\"b\",\"q2\"]]}; \"\" or λ is the empty string. A grammar \
is plain text, one production per line (\"S -> aSb | λ\"), uppercase letters are variables. Call \
rflap_capabilities when you need the exact syntax of a PDA, Turing machine or L-system. Tool errors \
carry a code and a precise message: read it, fix the input and retry.";

/// `prompts/list`.
pub fn prompt_list() -> Value {
    json!([
        {
            "name": "ssl_exercise",
            "title": "Solve a Sintaxis y Semántica exercise",
            "description": "Helps with an exercise on automata, grammars or regular expressions: reason, design with Rflap, verify and explain.",
            "arguments": [{"name": "exercise", "description": "The exercise statement, as the student wrote it.", "required": true}],
        },
        {
            "name": "grammar_tables",
            "title": "FIRST/FOLLOW, LL(1) and SLR(1) of a grammar",
            "description": "Analyzes a context-free grammar with Rflap and explains the tables and conflicts.",
            "arguments": [{"name": "grammar", "description": "The grammar, one production per line (S -> aSb | λ).", "required": true}],
        },
    ])
}

fn required<'a>(arguments: &'a Value, name: &str) -> Result<&'a str, String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("the prompt needs the argument `{name}`"))
}

/// `prompts/get`.
pub fn prompt(name: &str, arguments: &Value) -> Result<Value, String> {
    let (description, text) = match name {
        "ssl_exercise" => {
            let exercise = required(arguments, "exercise")?;
            (
                "Solve a Sintaxis y Semántica de los Lenguajes exercise with Rflap",
                format!(
                    "Help me with this exercise for \"Sintaxis y Semántica de los Lenguajes\":\n\n{exercise}\n\n\
                     First reason about the language and say what the machine must remember. Then write the \
                     alphabet, a formal spec and accept/reject examples (empty string included), and call \
                     rflap_design_automaton (or rflap_convert / rflap_grammar / rflap_parse_tables if the \
                     exercise is about conversions or grammars). If the verification reports a failing \
                     example, fix the spec and design again. Show the result with the Rflap widget and \
                     finish by explaining what each state or production does, in the language I wrote in."
                ),
            )
        }
        "grammar_tables" => {
            let grammar = required(arguments, "grammar")?;
            (
                "Analyze a grammar with Rflap",
                format!(
                    "Analyze this grammar with Rflap:\n\n{grammar}\n\nCall rflap_grammar to classify it and \
                     rflap_parse_tables for FIRST/FOLLOW, the LL(1) table and the SLR(1) tables. Explain any \
                     conflict, say whether the grammar is LL(1) and SLR(1), and, if it is not, suggest the \
                     transformation (left recursion removal, left factoring) and check the new grammar the same way."
                ),
            )
        }
        other => {
            return Err(format!(
                "unknown prompt `{other}`; the prompts are ssl_exercise and grammar_tables"
            ));
        }
    };
    Ok(json!({
        "description": description,
        "messages": [{"role": "user", "content": {"type": "text", "text": text}}],
    }))
}
