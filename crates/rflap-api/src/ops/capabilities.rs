//! `capabilities`: what an AI reads to learn the tool.

use rflap_core::layout::Algorithm;
use serde_json::{Value, json};

use crate::document::{Document, TYPES};
use crate::request::{ConvertTo, OPS};

fn nfa() -> Value {
    json!({"type": "fa", "initial": "q0", "finals": ["q3"],
           "transitions": [["q0", "a", "q0"], ["q0", "b", "q0"], ["q0", "a", "q1"],
                           ["q1", "b", "q2"], ["q2", "b", "q3"]]})
}

fn pda() -> Value {
    json!({"type": "pda", "initial": "q0", "finals": ["q2"],
           "transitions": [["q0", "a,Z;AZ", "q0"], ["q0", "a,A;AA", "q0"],
                           ["q0", "b,A;λ", "q1"], ["q1", "b,A;λ", "q1"],
                           ["q1", "λ,Z;Z", "q2"], ["q0", "λ,Z;Z", "q2"]]})
}

fn turing() -> Value {
    json!({"type": "turing", "initial": "q0", "finals": ["q2"],
           "transitions": [["q0", "a;b,R", "q0"], ["q0", "_;_,L", "q1"],
                           ["q1", "b;b,L", "q1"], ["q1", "_;_,R", "q2"]]})
}

/// A one-tape machine in the cátedra notation: accepts the words that start with a.
fn cat_tm() -> Value {
    json!({"type": "turing", "states": ["1", "2", "3"], "initial": "1", "finals": ["3"],
           "transitions": [["1", "(a,a,D)", "2"], ["2", "(a,a,D) / (b,b,D)", "2"],
                           ["2", "(Δ,Δ,N)", "3"]]})
}

/// The ALL that accepts words ending in b: a cátedra example in compact form.
fn cat_all() -> Value {
    json!({"type": "turing", "states": ["I", "D", "F"], "initial": "I", "finals": ["F"],
           "transitions": [["I", "(a,a,D) / (b,b,D)", "I"], ["I", "(⟩,⟩,I)", "D"],
                           ["D", "(b,b,I)", "F"]]})
}

fn grammar() -> Value {
    json!({"type": "grammar", "text": "S -> aSb | λ"})
}

fn lsystem() -> Value {
    json!({"type": "lsystem", "axiom": "F",
           "rules": ["F -> F + F - F - F + F"],
           "parameters": {"angle": "90", "distance": "4"}})
}

fn dfa_xml() -> String {
    let doc = Document::from_value(nfa()).expect("the example is valid");
    super::jff::save(&doc).expect("the example saves")["xml"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn op_entry(op: &str) -> Value {
    let (summary, params, example) = match op {
        "capabilities" => (
            "This description: operations, document types with their compact syntax, conversions.",
            "",
            json!({"op": "capabilities"}),
        ),
        "open_jff" => (
            "Reads the text of a JFLAP .jff file (any type, including L-systems and pumping lemma games) into a document.",
            "xml",
            json!({"op": "open_jff", "xml": dfa_xml()}),
        ),
        "save_jff" => (
            "Writes a document as .jff text that JFLAP 7.1 opens. Returns {xml, type}.",
            "document",
            json!({"op": "save_jff", "document": nfa()}),
        ),
        "normalize" => (
            "Expands a compact document into the full form (states created and laid out). Returns {document}.",
            "document",
            json!({"op": "normalize", "document": nfa()}),
        ),
        "simulate" => (
            "Runs a machine on an input. Returns {outcome: accepted|rejected|undecided, accepted, trace, output?, tape_outputs? (final content of each tape)}. Options: mode step_by_state|closure (fa, pda), acceptance final_state|empty_stack (pda) or final_state|halting|both (turing), max_configs, inputs (one string per tape).",
            "document, input | inputs, mode?, acceptance?, max_configs?",
            json!({"op": "simulate", "document": nfa(), "input": "abb"}),
        ),
        "step" => (
            "Stateless step-by-step run: re-runs the input for `steps` steps and returns {steps_done, finished, truncated, frontier, accepted, rejected}.",
            "document, input | inputs, steps?, mode?, acceptance?, max_configs?",
            json!({"op": "step", "document": nfa(), "input": "abb", "steps": 2}),
        ),
        "multiple_run" => (
            "Runs many inputs. Returns {results: [{input, outcome, accepted, output?, tape_outputs?}]}.",
            "document, inputs, mode?, acceptance?, max_configs?",
            json!({"op": "multiple_run", "document": nfa(), "inputs": ["abb", "ab", "aabb"]}),
        ),
        "analyze" => (
            "Structure of a document: kind (dfa, nfa, dpda, npda, ...), alphabet, nondeterministic states, λ transitions, unreachable and useless states.",
            "document",
            json!({"op": "analyze", "document": nfa()}),
        ),
        "convert" => (
            "Converts a document; see `convert` for the targets. Returns {to, document, steps, ...}.",
            "document, to",
            json!({"op": "convert", "document": nfa(), "to": "dfa"}),
        ),
        "equivalent" => (
            "Whether two finite automata (fa, regex or right-linear grammar documents) accept the same language, with a counterexample string when they do not.",
            "a, b",
            json!({"op": "equivalent", "a": nfa(),
                   "b": {"type": "regex", "expression": "(a+b)*abb"}}),
        ),
        "combine" => (
            "Appends the states and transitions of the second machine below the first (same type).",
            "a, b",
            json!({"op": "combine", "a": nfa(), "b": nfa()}),
        ),
        "grammar_type" => (
            "Classifies a grammar (right_linear, left_linear, cnf, gnf, context_free, context_sensitive, unrestricted, ...). `grammar` may be the grammar text.",
            "grammar",
            json!({"op": "grammar_type", "grammar": "S -> aSb | λ"}),
        ),
        "parse" => (
            "Parses strings with brute, brute_unrestricted, ll1, slr1 or cyk. Returns {parser, results: [{input, outcome, derivation_forms, derivation, tree, ...}]}; an input a parser cannot handle has outcome `error`.",
            "grammar, input | inputs, parser?, budget?",
            json!({"op": "parse", "grammar": "S -> aSb | λ", "inputs": ["aabb", "aab"],
                   "parser": "ll1"}),
        ),
        "parse_tables" => (
            "FIRST and FOLLOW sets, the LL(1) table and the SLR(1) automaton, ACTION and GOTO tables, with conflicts.",
            "grammar",
            json!({"op": "parse_tables", "grammar": "E -> E+T | T\nT -> T*F | F\nF -> (E) | i"}),
        ),
        "user_parse" => (
            "User-controlled derivation: applies the moves in order from the start variable and returns the sentential form, the derivation, the tree and which productions apply where. Moves: {\"production\": 0}, {\"production\": 0, \"position\": 2}, {\"production\": 0, \"positions\": [0, 3]} or \"undo\".",
            "grammar, target, moves",
            json!({"op": "user_parse", "grammar": "S -> aSb | λ", "target": "aabb",
                   "moves": [{"production": 0}, {"production": 0}, {"production": 1}]}),
        ),
        "layout" => (
            "Positions the states with a JFLAP layout algorithm. Returns {document}.",
            "document, algorithm?, options?",
            json!({"op": "layout", "document": nfa(), "algorithm": "circle"}),
        ),
        "render" => (
            "Draws a document. format svg (machines, lsystems), markdown (transition table), dot (Graphviz) or text. Highlights states (names or ids) and transitions (indices).",
            "document, format?, highlight_states?, highlight_transitions?, dark?, depth?",
            json!({"op": "render", "document": nfa(), "format": "svg",
                   "highlight_states": ["q1"], "highlight_transitions": [2]}),
        ),
        "design" => (
            "Builds, minimizes and verifies an automaton from a language specification (regex, regular grammar, constraint, or a template for non-regular languages) with examples.",
            "spec, options?",
            json!({"op": "design", "spec": {
                "alphabet": "ab",
                "language": {"constraint_text": "ends_with(\"ab\")"},
                "examples": {"accept": ["ab", "aab"], "reject": ["", "ba"]}}}),
        ),
        "explain" => (
            "A plain-language Markdown summary of a document.",
            "document",
            json!({"op": "explain", "document": nfa()}),
        ),
        "lsystem_render" => (
            "Expands an L-system `depth` levels and draws it with the turtle. Returns {svg, depth, expansion_length, expansion}.",
            "document, depth?, options?",
            json!({"op": "lsystem_render", "document": lsystem(), "depth": 2}),
        ),
        "pumping_languages" => (
            "The 26 predefined pumping lemma languages of JFLAP (13 regular, 13 context-free).",
            "",
            json!({"op": "pumping_languages"}),
        ),
        "pumping_new" => (
            "Starts a pumping lemma game. mode human_first (you pick m...) or computer_first. Returns {document, view, lemma, result, conclusion}.",
            "kind, language, mode?, seed?",
            json!({"op": "pumping_new", "kind": "regular", "language": 0,
                   "mode": "computer_first", "seed": 1}),
        ),
        "pumping_move" => (
            "Plays one move: choose_m, choose_w, choose_decomposition, choose_i, clear_all, add_case, replace_case, show_case, delete_case, list_cases, clear_cases, cases_done. Returns the new game state and the reply.",
            "game, move",
            json!({"op": "pumping_move",
                   "game": {"type": "regular_pumping_lemma", "language": 0},
                   "move": {"move": "choose_w", "w": "ab"}}),
        ),
        "catedra_definition" => (
            "Cátedra notation (Sintaxis y Semántica de los Lenguajes, UTN FRT): the formal definition MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩ (or ALL) of a one-tape machine. Σ_E is the document's \"input_alphabet\" or is inferred (uppercase letters, Δ, ⟨, ⟩ and symbols the machine writes that are not letters or digits are auxiliary). Returns {model: mt|all, states, input_alphabet, auxiliary_alphabet, initial, finals, input_alphabet_inferred, deterministic, issues: [{kind: no_determinista|dominio_incluye_final|sin_estado_inicial|alfabeto_entrada_invalido, message, transitions}], text}.",
            "document, model? (mt|all; default detected: all when ⟨ or ⟩ appear)",
            json!({"op": "catedra_definition", "document": cat_tm()}),
        ),
        "catedra_table" => (
            "Cátedra transition table: columns Estado Actual | Símbolo Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal, → on the initial state, * on finals (one row of dashes each), rows grouped by state in state order and symbols ordered input, auxiliary, ⟨, ⟩, Δ. Returns {header, rows, markdown, text}.",
            "document",
            json!({"op": "catedra_table", "document": cat_tm()}),
        ),
        "catedra_run" => (
            "Deterministic run in the cátedra's configuration notation, e.g. Δ1abaΔ ├─ ΔΔ2baΔ ... final/acepta. Entering a final state is final/acepta; no transition is bloqueo/rechazo (rechaza for an ALL); max_steps transitions without halting is no se detiene (límite de N pasos). model mt: tape of Δ, head on the first symbol, window of cells -1..n plus those visited. model all: tape ⟨w⟩, whole tape shown, a violation stops the run. Returns {model, max_steps, results: [{input, model, verdict: accepted|rejected|no_halt|violation, accepted, steps, configurations, sequence, violation?}]}.",
            "document, input | inputs, model? (mt|all), max_steps? (default 1000, at most 100000)",
            json!({"op": "catedra_run", "document": cat_tm(), "inputs": ["ab", "ba", ""], "model": "mt"}),
        ),
        "all_check" => (
            "Checks that a one-tape machine follows the rules of an ALL: statically (reading ⟨ writes ⟨ and moves D; reading ⟩ writes ⟩ and moves I; no Δ read or written; ⟨ and ⟩ only written over themselves) and, for each input, dynamically (the head never leaves ⟨…⟩). Returns {ok, violations: [{kind, message, transition, transition_text, step?, input?}], runs}.",
            "document, inputs?, max_steps?",
            json!({"op": "all_check", "document": cat_all(), "inputs": ["ab"]}),
        ),
        "compose_tm" => (
            "Composition of Turing machines MT3 = ⟨Q1∪Q2, Σ_E, Σ_A, q01, F2, f3⟩: transitions of MT1 into F1 go to the initial state of MT2 (its states are renamed with primes when names clash) and MT2 is laid out to the right. Returns {document, definition}.",
            "a, b",
            json!({"op": "compose_tm", "a": cat_tm(), "b": cat_tm()}),
        ),
        "membership_tk" => (
            "Membership of a word by the sets Tk (T0 = {S}, Tk = Tk-1 ∪ {β : |β| ≤ |w|, α ⇒ β}) for non-contracting grammars; a contracting one is refused with contracting_grammar. Returns {member, bound, sets (T0, T1, ... as lists of forms, λ is the empty form), text}.",
            "grammar, input, max_forms?",
            json!({"op": "membership_tk", "input": "abbc",
                   "grammar": "S -> T
DB -> BD
D -> c
T -> aTBD
T -> abD
bB -> bb"}),
        ),
        _ => ("", "", Value::Null),
    };
    json!({"op": op, "summary": summary, "params": params, "example": example})
}

fn type_entry(kind: &str) -> Value {
    let (summary, compact, example) = match kind {
        "fa" => (
            "Finite automaton.",
            "{\"type\":\"fa\", \"initial\":\"q0\", \"finals\":[\"q2\"], \"transitions\":[[from, label, to], ...]}. A label is the string read; \"\" or \"λ\" is the empty string; [0-9] reads a character range. States are created by name in order of first use; add \"states\":[...] to declare them (then an unknown name is an error); \"points\":{\"q0\":[x,y]} fixes coordinates and \"layout\" picks the algorithm for the rest.",
            json!({"type": "fa", "initial": "q0", "finals": ["q2"],
                   "transitions": [["q0", "a", "q1"], ["q1", "", "q2"]]}),
        ),
        "pda" => (
            "Pushdown automaton (the stack starts with Z).",
            "Like fa; labels are \"input,pop;push\" in JFLAP text, e.g. \"a,Z;aZ\" reads a, pops Z, pushes aZ (first symbol on top); use λ or nothing for an empty part.",
            pda(),
        ),
        "turing" => (
            "Turing machine, one or more tapes, with building blocks.",
            "Like fa; a one-tape label can be written in the cátedra notation \"(read,write,move)\", e.g. \"(a,X,D)\" or \"a,X,D\" (move I left, N stay, D right; Δ is the blank), and several labels of one edge are joined with \" / \", e.g. \"(a,a,D) / (b,b,D)\". The JFLAP form \"read;write,direction\" per tape joined by \"|\" is always accepted, e.g. \"a;b,R\" or \"a;a,R|_;x,S\" (direction L, R or S; _ or Δ is the blank; a read can be ~ any, !x not x, or a,b}v to remember in variable v). \"input_alphabet\" (\"ab\" or [\"a\",\"b\"]) optionally declares Σ_E. \"tapes\" is inferred from the first label. \"blocks\":{\"state\": <compact turing document>} makes a state run an inner machine.",
            turing(),
        ),
        "mealy" => (
            "Mealy machine (output on transitions).",
            "Like fa; labels are \"input;output\", e.g. \"a;1\".",
            json!({"type": "mealy", "initial": "q0",
                   "transitions": [["q0", "a;1", "q0"], ["q0", "b;0", "q0"]]}),
        ),
        "moore" => (
            "Moore machine (output on states).",
            "Like fa (label = the string read) plus \"outputs\":{\"q0\":\"x\"}.",
            json!({"type": "moore", "initial": "q0", "outputs": {"q0": "x", "q1": "y"},
                   "transitions": [["q0", "a", "q1"], ["q1", "a", "q0"]]}),
        ),
        "grammar" => (
            "Grammar. An uppercase letter is a variable, anything else is a terminal; the start variable is the left side of the first production.",
            "{\"type\":\"grammar\", \"text\":\"S -> aSb | λ\"} or \"rules\":[\"S -> aSb\", \"S -> λ\"]; optional \"start\".",
            grammar(),
        ),
        "regex" => (
            "Regular expression in JFLAP syntax: + is union, * is star, juxtaposition is concatenation, λ the empty string, ø the empty language.",
            "{\"type\":\"regex\", \"expression\":\"(a+b)*abb\"}",
            json!({"type": "regex", "expression": "(a+b)*abb"}),
        ),
        "lsystem" => (
            "L-system with turtle drawing.",
            "{\"type\":\"lsystem\", \"axiom\":\"F\", \"rules\":[\"F -> F + F - F - F + F\"], \"parameters\":{\"angle\":\"90\",\"distance\":\"4\"}}. Symbols are separated by spaces (or set \"single_char_symbols\":true to read F+F as three symbols). Rules may also be {\"F\": \"F + F\"}.",
            lsystem(),
        ),
        "regular_pumping_lemma" => (
            "Pumping lemma game for regular languages.",
            "{\"type\":\"regular_pumping_lemma\", \"language\":0, \"mode\":\"computer_first\", \"seed\":1}; language is an index or a title from `pumping_languages`. Play it with `pumping_move`.",
            json!({"type": "regular_pumping_lemma", "language": 0}),
        ),
        _ => (
            "Pumping lemma game for context-free languages.",
            "{\"type\":\"context_free_pumping_lemma\", \"language\":0, \"mode\":\"human_first\"}; see regular_pumping_lemma.",
            json!({"type": "context_free_pumping_lemma", "language": 0, "mode": "human_first"}),
        ),
    };
    json!({"type": kind, "summary": summary, "compact": compact, "example": example})
}

fn convert_entry(to: ConvertTo) -> Value {
    let name = to.name();
    let (from, summary, document) = match to {
        ConvertTo::Dfa => (
            "fa, regex, grammar",
            "NFA to DFA (subset construction).",
            nfa(),
        ),
        ConvertTo::MinimalDfa => ("fa, regex, grammar", "Minimal DFA.", nfa()),
        ConvertTo::Regex => ("fa", "FA to regular expression (state elimination).", nfa()),
        ConvertTo::Nfa => (
            "regex",
            "Regular expression to NFA.",
            json!({"type": "regex", "expression": "(a+b)*abb"}),
        ),
        ConvertTo::RegularGrammar => ("fa", "FA to right-linear grammar.", nfa()),
        ConvertTo::Fa => (
            "grammar",
            "Right-linear grammar to FA.",
            json!({"type": "grammar", "text": "S -> aS | bA\nA -> bA | λ"}),
        ),
        ConvertTo::PdaLl => ("grammar", "CFG to PDA, LL construction.", grammar()),
        ConvertTo::PdaLr => ("grammar", "CFG to PDA, LR construction.", grammar()),
        ConvertTo::Cfg => (
            "pda",
            "PDA to CFG. JFLAP needs one final state, transitions that pop one symbol and push none or two, and those into the final state popping only Z; a PDA that is not in that form is rewritten first (see pda_jflap_form) and the rewriting is reported in the steps.",
            json!({"type": "pda", "initial": "q0", "finals": ["qf"],
                   "transitions": [["q0", "a,Z;AZ", "q0"], ["q0", "a,A;AA", "q0"],
                                   ["q0", "b,A;λ", "q1"], ["q1", "b,A;λ", "q1"],
                                   ["q1", "λ,Z;λ", "qf"]]}),
        ),
        ConvertTo::PdaJflapForm => (
            "pda",
            "Rflap addition: rewrites a PDA that accepts by final state (a designed one, say) into the form `cfg` needs: one final state reached on an empty stack, every transition pops one symbol and pushes none or two. `cfg` applies it by itself when it is needed.",
            json!({"type": "pda", "initial": "q0", "finals": ["q1"],
                   "transitions": [["q0", "a,Z;AZ", "q0"], ["q0", "a,A;AA", "q0"],
                                   ["q0", "b,A;λ", "q1"], ["q1", "b,A;λ", "q1"]]}),
        ),
        ConvertTo::UnrestrictedGrammar => (
            "turing",
            "One-tape Turing machine to unrestricted grammar.",
            json!({"type": "turing", "initial": "q0", "finals": ["q1"],
                   "transitions": [["q0", "a;a,R", "q0"], ["q0", "_;_,S", "q1"]]}),
        ),
        ConvertTo::RemoveLambda => ("grammar", "Removes λ-productions.", grammar()),
        ConvertTo::RemoveUnit => (
            "grammar",
            "Removes unit productions.",
            json!({"type": "grammar", "text": "S -> aSb | A\nA -> a"}),
        ),
        ConvertTo::RemoveUseless => (
            "grammar",
            "Removes useless productions.",
            json!({"type": "grammar", "text": "S -> aS | a\nB -> b"}),
        ),
        ConvertTo::Cnf => (
            "grammar",
            "Chomsky normal form (λ, unit and useless removal first).",
            grammar(),
        ),
        ConvertTo::Complete => ("fa", "Adds the trap state.", nfa()),
        ConvertTo::SplitLabels => (
            "fa",
            "Replaces multi-character labels by chains of single-character transitions.",
            json!({"type": "fa", "initial": "a", "finals": ["b"],
                   "transitions": [["a", "xyz", "b"]]}),
        ),
        ConvertTo::RemoveUselessStates => (
            "fa, pda, turing, mealy, moore",
            "Drops unreachable and useless states.",
            nfa(),
        ),
    };
    json!({
        "to": name,
        "from": from,
        "summary": summary,
        "example": {"op": "convert", "document": document, "to": name},
    })
}

pub fn capabilities() -> Value {
    json!({
        "name": "rflap",
        "version": env!("CARGO_PKG_VERSION"),
        "summary": "JFLAP 7.1 in Rust: build, simulate, convert, analyze, draw and design automata, grammars, regular expressions, L-systems and pumping lemma games. Every call is one JSON object {\"op\": ..., ...}; the answer is {\"ok\": true, \"op\": ..., \"result\": ...} or {\"ok\": false, \"error\": {\"code\", \"message\", \"details\"?}}.",
        "ops": OPS.iter().map(|op| op_entry(op)).collect::<Vec<_>>(),
        "document_types": TYPES.iter().map(|t| type_entry(t)).collect::<Vec<_>>(),
        "convert": ConvertTo::ALL.iter().map(|t| convert_entry(*t)).collect::<Vec<_>>(),
        "parsers": ["brute", "brute_unrestricted", "ll1", "slr1", "cyk"],
        "layouts": Algorithm::ALL.iter().map(|a| a.name()).collect::<Vec<_>>(),
        "formats": ["svg", "markdown", "dot", "text"],
        "modes": ["step_by_state", "closure"],
        "acceptance": ["final_state", "empty_stack", "halting", "both"],
        "error_codes": [
            "invalid_json", "invalid_request", "unknown_op", "invalid_document",
            "unknown_document_type", "unknown_state", "invalid_label", "invalid_grammar",
            "invalid_regex", "wrong_document_type", "unsupported_option", "invalid_argument",
            "no_initial_state", "jff_error", "conversion_failed", "design_error", "parse_error",
            "invalid_move", "limit_exceeded", "contracting_grammar", "internal"
        ],
        "notes": [
            "A document is accepted in compact form (see document_types) or in the full form that operations return; send either wherever a document is expected.",
            "Name states yourself in compact documents; ids are assigned in order of first use. Results use the names back.",
            "Start with `design` when you only have a language description: it returns a verified, minimal automaton with its regex and the meaning of each state.",
            "Use `render` with format svg to show an automaton and `explain` for a text summary; highlight the current states of a `step` to animate a run.",
            "Nested JSON of conversions and parsers (steps, derivation, tree) is the serde of the core crate; the top-level keys of every result are documented here and stable."
        ],
    })
}
