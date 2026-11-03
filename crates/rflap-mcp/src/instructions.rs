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

Turing machines (MT) and linear bounded automata (ALL) follow the notation of the course \
bibliography (CLASE 39 and 41), not JFLAP's. Reason first about what the machine does to the tape. \
Write it as a compact turing document with cátedra labels (e,s,m): move I left, N stay, D right, Δ \
is the blank, several labels on one edge joined with \" / \", e.g. [\"2\",\"(a,a,D) / (b,b,D)\",\"2\"], \
and name the states as the student expects (1, 2, … or I, A, B, …). Then call rflap_mt_catedra with \
action \"definition\" (MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩ with its sets), \"table\" (Estado Actual | Símbolo \
Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal) and \"run\" on accepted and rejected \
sample words (configuration sequences with ├─ ending in final/acepta or bloqueo/rechazo). For an ALL \
(the tape is ⟨w⟩, the head starts on the first symbol, f(q,⟨) = (q′,⟨,D), f(q,⟩) = (q′,⟩,I), Δ is \
never read or written) always call action \"all_check\" with sample inputs (static + dynamic) before \
presenting it. Present the 6-tuple, the table, the graph (widget) and the sequences. Remember: final \
states halt (f is defined on Q-F); a missing transition is a bloqueo, which rejects; an MT may never \
halt, an ALL always accepts or rejects. Multitape machines use JFLAP notation (multipista: all heads \
move together). rflap_compose_tm composes MT1 and MT2; rflap_membership builds the sets T0..Tk of a \
non-contracting grammar. rflap_simulate, rflap_step and rflap_show also run a Turing machine as an ALL \
with model \"all\" (or when its labels use ⟨ ⟩) and rflap_show / rflap_simulate take notation \
\"catedra\" (default) or \"jflap\"; give each machine a title (alias name) so the answers and the widgets \
tell them apart.

Compatibility with classic JFLAP 7.1 is an internal rule: every automaton you design, show or save \
must open in classic JFLAP 7.1 with the same states, initial state, finals, labels and symbols, and \
FA, PDA, Mealy, Moore and Turing machines must run there with the same verdicts. Avoid Rflap-only \
constructions (more than 5 tapes, a range label [x-y] into a state with a λ exit, anything the \
reply flags) unless the student asks for them. The tools check it: a \"⚠️ JFLAP 7.1\" line in a reply \
is a construction JFLAP cannot represent or runs differently (change the design; rflap_save_jff \
still saves it, unchanged, and warns), and a \"ℹ️ JFLAP 7.1\" line is a note to pass on \
(structuredContent.jflap has both). An ALL opens the same in JFLAP but JFLAP has no ALL model: say \
how to run it there (add an initial state with (⟨,⟨,D) to the old one and type the word as ⟨w⟩) and \
that the cátedra semantics (tape ⟨w⟩, shrinking tape) only hold in Rflap.

Write documents in the compact form, e.g. {\"type\":\"fa\",\"initial\":\"q0\",\"finals\":[\"q2\"],\
\"transitions\":[[\"q0\",\"a\",\"q1\"],[\"q1\",\"b\",\"q2\"]]}; \"\" or λ is the empty string. A grammar \
is plain text, one production per line (\"S -> aSb | λ\"), uppercase letters are variables. Call \
rflap_capabilities when you need the exact syntax of a PDA, Turing machine or L-system (section \"layout\" \
lists the layout algorithms and the compact layout, points and title fields). Tool errors \
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
            "name": "mt_catedra",
            "title": "Turing machine or ALL in the notation of the course",
            "description": "Solves an exercise on Turing machines or linear bounded automata (ALL) with the notation of the course bibliography: definition, table, graph and configuration sequences, verified with Rflap.",
            "arguments": [{"name": "enunciado", "description": "The exercise statement, as the student wrote it.", "required": true}],
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
                     finish by explaining what each state or production does, in the language I wrote in. \
                     If the exercise is about a Turing machine or an ALL, use rflap_mt_catedra and the \
                     notation of the course (see the mt_catedra prompt)."
                ),
            )
        }
        "mt_catedra" => {
            let enunciado = required(arguments, "enunciado")?;
            (
                "Solve a Turing machine / ALL exercise in the notation of the course",
                format!(
                    "Resolvé este ejercicio de Máquinas de Turing / ALL de \"Sintaxis y Semántica de los \
                     Lenguajes\" con la notación de la bibliografía de la cátedra:\n\n{enunciado}\n\n\
                     Primero razoná qué hace la máquina sobre la cinta (¿MT o ALL?). Escribila como documento \
                     turing compacto con etiquetas (e,s,m) (movimientos I, N, D; blanco Δ; varias etiquetas de \
                     una arista unidas con \" / \") y los estados nombrados como en la cátedra. Llamá a \
                     rflap_mt_catedra con action \"definition\", \"table\" y \"run\" sobre palabras que acepta y \
                     que rechaza; si es un ALL, llamá siempre a action \"all_check\" (estático y dinámico) antes \
                     de presentarlo y corregí lo que informe. Presentá la definición \
                     MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩, la tabla de transición, el grafo (widget) y las secuencias de \
                     configuraciones con ├─ terminadas en final/acepta o bloqueo/rechazo. Recordá que los \
                     estados finales son de parada y que una MT puede no detenerse mientras que un ALL siempre \
                     decide. Respondé en español."
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
                "unknown prompt `{other}`; the prompts are ssl_exercise, mt_catedra and grammar_tables"
            ));
        }
    };
    Ok(json!({
        "description": description,
        "messages": [{"role": "user", "content": {"type": "text", "text": text}}],
    }))
}
