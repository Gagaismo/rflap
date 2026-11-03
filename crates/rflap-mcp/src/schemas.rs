//! The tool catalogue: names, descriptions written for an LLM, and JSON Schemas.

use serde_json::{Value, json};

use crate::EDITOR_URI;

/// The non-visual tool the widget calls as its remote engine when the host blocks wasm.
const APP_CALLABLE: &str = "rflap_call";

/// One tool of the server.
pub struct ToolDef {
    pub name: &'static str,
    pub title: &'static str,
    pub description: String,
    pub input: Value,
    pub output: Option<Value>,
    /// Shows its result in the widget (`_meta.ui.resourceUri`).
    pub visual: bool,
    /// Never touches the file system.
    pub read_only: bool,
    pub invoking: &'static str,
    pub invoked: &'static str,
}

impl ToolDef {
    /// The entry of `tools/list`.
    pub fn to_json(&self) -> Value {
        let mut tool = json!({
            "name": self.name,
            "title": self.title,
            "description": self.description,
            "inputSchema": self.input,
            "annotations": {
                "title": self.title,
                "readOnlyHint": self.read_only,
                "destructiveHint": false,
                "idempotentHint": true,
                "openWorldHint": false,
            },
        });
        if let Some(output) = &self.output {
            tool["outputSchema"] = output.clone();
        }
        if !self.visual && self.name == APP_CALLABLE {
            // The widget calls it as its remote engine: visible to the app (MCP Apps
            // `_meta.ui.visibility`) and to ChatGPT's widgets.
            tool["_meta"] = json!({
                "ui": {"visibility": ["model", "app"]},
                "openai/widgetAccessible": true,
            });
        }
        if self.visual {
            tool["_meta"] = json!({
                "ui": {"resourceUri": EDITOR_URI, "visibility": ["model", "app"]},
                "openai/outputTemplate": EDITOR_URI,
                "openai/widgetAccessible": true,
                "openai/toolInvocation/invoking": self.invoking,
                "openai/toolInvocation/invoked": self.invoked,
            });
        }
        tool
    }
}

fn string(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

fn integer(description: &str) -> Value {
    json!({"type": "integer", "minimum": 0, "description": description})
}

fn boolean(description: &str) -> Value {
    json!({"type": "boolean", "description": description})
}

fn strings(description: &str) -> Value {
    json!({"type": "array", "items": {"type": "string"}, "description": description})
}

fn choice(description: &str, values: &[&str]) -> Value {
    json!({"type": "string", "enum": values, "description": description})
}

fn object(description: &str) -> Value {
    json!({"type": "object", "description": description})
}

/// A document in compact or full form.
fn document(description: &str) -> Value {
    json!({
           "type": "object",
           "description": format!(
    "{description} A JSON object with a \"type\": fa, pda, turing, mealy, moore, grammar, regex, lsystem or a pumping lemma game. Compact form: {{\"type\":\"fa\",\"initial\":\"q0\",\"finals\":[\"q1\"], \"transitions\":[[\"q0\",\"a\",\"q1\"]]}}, {{\"type\":\"grammar\",\"text\":\"S -> aSb | λ\"}}, {{\"type\":\"regex\",\"expression\":\"(a+b)*abb\"}}; the full form other tools return works too. A compact document may also carry `title` (alias `name`, shown as the header of the result and the widget), `layout` (an algorithm name, default `layered`: initial state at the left) and `points` ({{\"q0\": [x, y]}}, fixed coordinates). Other types: rflap_capabilities."
    ),
           "properties": {"type": {"type": "string"}},
           "required": ["type"],
       })
}

/// A grammar as text (one production per line) or as a grammar document.
fn grammar(description: &str) -> Value {
    json!({
        "description": format!(
            "{description} Either the text, one production per line (\"S -> aSb | λ\": an \
             uppercase letter is a variable, anything else a terminal, λ is the empty string), \
             or a grammar document."
        ),
        "anyOf": [{"type": "string"}, {"type": "object"}],
    })
}

/// `model` of the tools that run or show a Turing machine.
fn model_arg() -> Value {
    choice(
        "Turing machines only. all = run it as an ALL (linear bounded automaton): the tape is ⟨w⟩ with the delimiters placed automatically and the head on the first symbol. mt = a Turing machine on a blank-delimited tape. Default: all when a transition mentions ⟨ or ⟩, otherwise mt.",
        &["mt", "all"],
    )
}

/// `notation` of the tools that show a Turing machine.
fn notation_arg() -> Value {
    choice(
        "Turing machines only: catedra (default) writes the labels (e,s,m) with Δ and I/N/D, jflap writes e;s,m with □ and L/S/R, in the table and the drawing.",
        &["catedra", "jflap"],
    )
}

fn schema(properties: Vec<(&str, Value)>, required: &[&str]) -> Value {
    let properties: serde_json::Map<String, Value> = properties
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn output(properties: Value) -> Value {
    json!({"type": "object", "properties": properties})
}

const MODES: &[&str] = &["step_by_state", "closure"];
const ACCEPTANCE: &[&str] = &["final_state", "empty_stack", "halting", "both"];

/// Every tool, in the order `tools/list` returns them.
pub fn tools() -> Vec<ToolDef> {
    vec![
        design_automaton(),
        show(),
        simulate(),
        step(),
        convert(),
        equivalent(),
        combine(),
        analyze(),
        grammar_tool(),
        parse(),
        parse_tables(),
        render(),
        open_jff(),
        save_jff(),
        lsystem_render(),
        pumping(),
        capabilities(),
        call(),
        mt_catedra(),
        compose_tm(),
        membership(),
    ]
}

fn design_automaton() -> ToolDef {
    ToolDef {
        name: "rflap_design_automaton",
        title: "Design an automaton from a language",
        description: "Builds, minimizes and VERIFIES an automaton for a language you have already \
            reasoned out. You translate the student's language into a formal spec; Rflap builds \
            the minimal DFA (or a PDA for a non-regular template), draws it and checks it against \
            your examples and against a brute-force cross-check. Give the alphabet, exactly ONE \
            of constraint_text / constraint / regex / grammar / template, and accept/reject \
            examples (include the empty string and boundary cases). If the verification reports \
            a failing example, your spec or example is wrong: fix it and call again. \
            constraint_text syntax: starts_with(\"ab\"), ends_with(\"ab\"), contains(\"aa\"), \
            length >= 3, length % 2 == 0, count(a) % 2 == 0, count(a,b) >= 2 (a or b), \
            count(a) == count(b) (NOT regular), regex(\"(a+b)*\"), combined with and / or / \
            xor / not and parentheses. Regex syntax is JFLAP's: + union, \
            * star, juxtaposition concatenation, λ empty string. Non-regular languages need a \
            template. Example: {\"alphabet\":\"ab\",\"constraint_text\":\"ends_with(\\\"ab\\\")\",\
            \"examples\":{\"accept\":[\"ab\",\"aab\"],\"reject\":[\"\",\"ba\",\"abb\"]}}."
            .to_string(),
        input: schema(
            vec![
                (
                    "alphabet",
                    json!({"description": "The symbols, single characters: \"ab\" or [\"a\",\"b\"].",
                           "anyOf": [{"type": "string"}, {"type": "array", "items": {"type": "string"}}]}),
                ),
                (
                    "constraint_text",
                    string(
                        "A condition in text syntax, e.g. ends_with(\"ab\") and count(a) % 2 == 0.",
                    ),
                ),
                (
                    "constraint",
                    object(
                        "A structured condition: {\"type\":\"ends_with\",\"word\":\"ab\"}, starts_with, \
                         contains, length {cmp,n}, length_mod {m,r}, count {symbols,cmp,n}, \
                         count_mod {symbols,m,r}, regex {pattern}, and/or/xor {constraints:[...]}, \
                         not {constraint}. Prefer constraint_text.",
                    ),
                ),
                (
                    "regex",
                    string("The language as a regular expression (JFLAP syntax)."),
                ),
                (
                    "grammar",
                    string(
                        "The language as a right- or left-linear grammar, one production per line: \"S -> aS | bA\".",
                    ),
                ),
                (
                    "template",
                    object(
                        "A non-regular language, built as a PDA: {\"name\":\"an_bn\",\"a\":\"a\",\"b\":\"b\",\"min\":0}, \
                         {\"name\":\"an_bm\",\"a\":\"a\",\"b\":\"b\",\"relation\":\"le\"|\"ge\"|\"ne\"}, \
                         {\"name\":\"equal_count\",\"a\":\"a\",\"b\":\"b\"}, \
                         {\"name\":\"balanced_brackets\",\"pairs\":[[\"(\",\")\"]]}, \
                         {\"name\":\"wcwr\",\"c\":\"c\"}, {\"name\":\"even_palindrome\"}, {\"name\":\"palindrome\"}.",
                    ),
                ),
                (
                    "examples",
                    json!({"type": "object",
                           "description": "Strings the automaton must accept and reject (\"\" is the empty string).",
                           "properties": {"accept": {"type": "array", "items": {"type": "string"}},
                                          "reject": {"type": "array", "items": {"type": "string"}}}}),
                ),
                (
                    "options",
                    object(
                        "Optional: check_len (brute-force length, default 8), layout (circle, gem, \
                         spiral, tree, ...), include_trap_state (default true).",
                    ),
                ),
            ],
            &["alphabet"],
        ),
        output: Some(output(json!({
            "document": {"type": "object"},
            "description": {"type": "string"},
            "states": {"type": "array"},
            "verification": {"type": "object"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Designing the automaton…",
        invoked: "Automaton designed",
    }
}

fn show() -> ToolDef {
    ToolDef {
        name: "rflap_show",
        title: "Show a document",
        description: "Shows any Rflap document (automaton, grammar, regex, L-system…) in the \
            interactive widget and returns its analysis (deterministic?, unreachable states…), \
            a transition table and an explanation. Use it to display something you wrote by \
            hand. Optional fields of a compact document: `title` (alias `name`; shown as the \
            first line and the header of the widget), `layout` (algorithm for the state \
            positions: layered = default, initial state at the left and the rest in columns by \
            distance; circle, gem, random, spiral, tree, tree-degree, two-circle) and `points` \
            ({\"q0\": [x, y]}, fixed coordinates that win over `layout`). For a Turing machine, \
            `notation` catedra (default, the labels (e,s,m)) or jflap. For an ALL (linear \
            bounded automaton) or exercises in the notation of the course (UTN FRT), use \
            rflap_mt_catedra (or model: \"all\"). Example: \
            {\"document\":{\"type\":\"fa\",\"initial\":\"q0\",\"finals\":[\"q1\"],\
            \"transitions\":[[\"q0\",\"a\",\"q1\"],[\"q1\",\"b\",\"q0\"]],\
            \"title\":\"Mi AFD\"}}."
            .to_string(),
        input: schema(
            vec![
                ("document", document("The document to show.")),
                ("model", model_arg()),
                ("notation", notation_arg()),
            ],
            &["document"],
        ),
        output: Some(output(json!({
            "document": {"type": "object"},
            "analysis": {"type": "object"},
            "svg": {"type": "string"},
            "explanation": {"type": "string"},
            "title": {"type": "string"},
            "notation": {"type": "string"},
            "model": {"type": "string"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Drawing…",
        invoked: "Drawn",
    }
}

fn simulate() -> ToolDef {
    ToolDef {
        name: "rflap_simulate",
        title: "Simulate a machine on inputs",
        description: "Runs a finite automaton, PDA, Turing machine, Mealy or Moore machine on one \
            input (`input`) or on many (`inputs`, a batch table). Returns accepted / rejected / \
            undecided and the trace of configurations. Options: mode step_by_state|closure \
            (λ handling), acceptance final_state|empty_stack (PDA) or final_state|halting|both \
            (Turing), `tapes` for one string per tape of a multi-tape machine. For a Turing machine, \
            `model: \"all\"` (or labels with ⟨ ⟩) runs it as an ALL on the tape ⟨w⟩ and \
            `notation` catedra|jflap picks the labels. For an ALL (linear bounded automaton) or \
            exercises in the notation of the course (UTN FRT), use rflap_mt_catedra (or model: \
            \"all\"). Example: \
            {\"document\":{\"type\":\"fa\",\"initial\":\"q0\",\"finals\":[\"q1\"],\
            \"transitions\":[[\"q0\",\"a\",\"q1\"]]},\"input\":\"a\"}."
            .to_string(),
        input: schema(
            vec![
                ("document", document("The machine to run.")),
                (
                    "input",
                    string("One input string (\"\" is the empty string)."),
                ),
                (
                    "inputs",
                    json!({"type": "array", "items": {"anyOf": [{"type": "string"},
                           {"type": "array", "items": {"type": "string"}}]},
                           "description": "Many inputs at once (a batch run); each a string, or an array with one string per tape."}),
                ),
                (
                    "tapes",
                    strings("One input per tape of a multi-tape Turing machine (single run)."),
                ),
                ("mode", choice("How λ transitions are stepped.", MODES)),
                (
                    "acceptance",
                    choice("When a run accepts (PDA, Turing).", ACCEPTANCE),
                ),
                (
                    "max_configs",
                    integer("Configurations explored before answering undecided (default 10000)."),
                ),
                ("model", model_arg()),
                ("notation", notation_arg()),
                (
                    "max_steps",
                    integer(
                        "ALL only: steps before a run is reported as not halting (default 1000, at most 100000).",
                    ),
                ),
            ],
            &["document"],
        ),
        output: Some(output(json!({
            "outcome": {"type": "string"},
            "accepted": {"type": "boolean"},
            "trace": {"type": "array"},
            "results": {"type": "array"},
            "document": {"type": "object"},
            "title": {"type": "string"},
            "notation": {"type": "string"},
            "model": {"type": "string"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Simulating…",
        invoked: "Simulation done",
    }
}

fn step() -> ToolDef {
    ToolDef {
        name: "rflap_step",
        title: "Step through a run",
        description: "Runs an input for exactly `steps` steps (stateless: it replays from the \
            start) and returns the live configurations (`frontier`), those already accepted or \
            rejected, and whether the run finished. Call it with steps 1, 2, 3… to walk a \
            student through a run; the widget highlights the current states. With `model: \"all\"` \
            (or labels with ⟨ ⟩) a Turing machine runs as an ALL and the reply is the first \
            `steps` configurations on the tape ⟨w⟩. For an ALL (linear bounded automaton) or \
            exercises in the notation of the course (UTN FRT), use rflap_mt_catedra (or model: \
            \"all\"). Example: \
            {\"document\":{...},\"input\":\"aab\",\"steps\":2}."
            .to_string(),
        input: schema(
            vec![
                ("document", document("The machine to run.")),
                ("input", string("The input string.")),
                (
                    "tapes",
                    strings("One input per tape of a multi-tape Turing machine."),
                ),
                (
                    "steps",
                    integer("Steps to take from the start (default 1)."),
                ),
                ("mode", choice("How λ transitions are stepped.", MODES)),
                (
                    "acceptance",
                    choice("When a run accepts (PDA, Turing).", ACCEPTANCE),
                ),
                (
                    "max_configs",
                    integer("Frontier size at which the run stops as truncated (default 10000)."),
                ),
                ("model", model_arg()),
            ],
            &["document", "input"],
        ),
        output: Some(output(json!({
            "steps_done": {"type": "integer"},
            "finished": {"type": "boolean"},
            "frontier": {"type": "array"},
            "configurations": {"type": "array"},
            "document": {"type": "object"},
            "title": {"type": "string"},
            "model": {"type": "string"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Stepping…",
        invoked: "Step done",
    }
}

fn convert() -> ToolDef {
    ToolDef {
        name: "rflap_convert",
        title: "Convert between models",
        description: "Converts a document. Targets (`to`): dfa, minimal_dfa (from fa, regex or \
            regular grammar); regex, regular_grammar (from fa); nfa (from regex); fa (from a \
            right-linear grammar); pda_ll, pda_lr (from a CFG); cfg (from a pda; one not in JFLAP \
            form is rewritten first), pda_jflap_form (rewrites a pda into that form); \
            unrestricted_grammar (from a one-tape Turing machine); remove_lambda, remove_unit, \
            remove_useless, cnf (grammar transformations); complete (adds the trap state), \
            split_labels, remove_useless_states. Returns the new document and the construction \
            steps. Example: {\"document\":{\"type\":\"regex\",\"expression\":\"(a+b)*abb\"},\
            \"to\":\"minimal_dfa\"}."
            .to_string(),
        input: schema(
            vec![
                ("document", document("The document to convert.")),
                (
                    "to",
                    choice(
                        "The target.",
                        &[
                            "dfa",
                            "minimal_dfa",
                            "regex",
                            "nfa",
                            "regular_grammar",
                            "fa",
                            "pda_ll",
                            "pda_lr",
                            "cfg",
                            "pda_jflap_form",
                            "unrestricted_grammar",
                            "remove_lambda",
                            "remove_unit",
                            "remove_useless",
                            "cnf",
                            "complete",
                            "split_labels",
                            "remove_useless_states",
                        ],
                    ),
                ),
            ],
            &["document", "to"],
        ),
        output: Some(output(json!({
            "to": {"type": "string"},
            "document": {"type": "object"},
            "steps": {},
        }))),
        visual: true,
        read_only: true,
        invoking: "Converting…",
        invoked: "Converted",
    }
}

fn equivalent() -> ToolDef {
    ToolDef {
        name: "rflap_equivalent",
        title: "Compare two languages",
        description: "Whether two finite automata, regular expressions or right-linear grammars \
            accept the same language; when they do not, a shortest counterexample string. Use \
            it to check a student's automaton against a regex. Example: {\"a\":{\"type\":\"fa\",\
            ...},\"b\":{\"type\":\"regex\",\"expression\":\"(a+b)*ab\"}}."
            .to_string(),
        input: schema(
            vec![
                ("a", document("The first language.")),
                ("b", document("The second language.")),
            ],
            &["a", "b"],
        ),
        output: Some(output(json!({
            "equivalent": {"type": "boolean"},
        }))),
        visual: false,
        read_only: true,
        invoking: "Comparing…",
        invoked: "Compared",
    }
}

fn combine() -> ToolDef {
    ToolDef {
        name: "rflap_combine",
        title: "Combine two machines",
        description: "Puts two machines of the same type side by side in one document (JFLAP's \
            \"Combine Automata\"), ready to add λ-transitions for a union or concatenation. \
            Example: {\"a\":{...fa...},\"b\":{...fa...}}."
            .to_string(),
        input: schema(
            vec![
                ("a", document("The first machine.")),
                ("b", document("The second machine.")),
            ],
            &["a", "b"],
        ),
        output: Some(output(json!({"document": {"type": "object"}}))),
        visual: true,
        read_only: true,
        invoking: "Combining…",
        invoked: "Combined",
    }
}

fn analyze() -> ToolDef {
    ToolDef {
        name: "rflap_analyze",
        title: "Analyze a machine",
        description: "Structure of an automaton: kind (dfa, nfa, dpda, npda, turing, mealy, \
            moore), alphabet, deterministic?, nondeterministic states, λ transitions, \
            unreachable and useless states, completeness. Example: {\"document\":{\"type\":\"fa\",...}}."
            .to_string(),
        input: schema(vec![("document", document("The machine to analyze."))], &["document"]),
        output: Some(output(json!({
            "kind": {"type": "string"},
            "alphabet": {"type": "array"},
            "deterministic": {"type": "boolean"},
        }))),
        visual: false,
        read_only: true,
        invoking: "Analyzing…",
        invoked: "Analyzed",
    }
}

fn grammar_tool() -> ToolDef {
    ToolDef {
        name: "rflap_grammar",
        title: "Check or transform a grammar",
        description: "With action `type` (default) classifies a grammar (regular / right_linear / \
            left_linear / context_free / context_sensitive / unrestricted, CNF, GNF). Other \
            actions transform it and return the new grammar: remove_lambda, remove_unit, \
            remove_useless, cnf (Chomsky normal form), to_fa (right-linear grammar → automaton), \
            to_pda_ll, to_pda_lr (CFG → PDA). Example: {\"grammar\":\"S -> aSb | λ\",\"action\":\"cnf\"}."
            .to_string(),
        input: schema(
            vec![
                ("grammar", grammar("The grammar.")),
                (
                    "action",
                    choice(
                        "What to do (default type).",
                        &[
                            "type", "remove_lambda", "remove_unit", "remove_useless", "cnf",
                            "to_fa", "to_pda_ll", "to_pda_lr",
                        ],
                    ),
                ),
            ],
            &["grammar"],
        ),
        output: Some(output(json!({
            "classification": {"type": "string"},
            "document": {"type": "object"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Working on the grammar…",
        invoked: "Grammar done",
    }
}

fn parse() -> ToolDef {
    ToolDef {
        name: "rflap_parse",
        title: "Parse strings with a grammar",
        description: "Parses one or many strings: parser brute (default, any grammar), \
            brute_unrestricted, ll1 (needs an LL(1) grammar), slr1, cyk (needs Chomsky normal \
            form). Returns, per input, accepted / rejected, the leftmost derivation, the parse \
            tree and the parser steps. A parser that cannot handle the grammar reports it per \
            input. Example: {\"grammar\":\"S -> aSb | λ\",\"inputs\":[\"aabb\",\"aab\"],\"parser\":\"ll1\"}."
            .to_string(),
        input: schema(
            vec![
                ("grammar", grammar("The grammar.")),
                ("input", string("One string to parse.")),
                ("inputs", strings("Many strings to parse.")),
                (
                    "parser",
                    choice("The parser (default brute).", &["brute", "brute_unrestricted", "ll1", "slr1", "cyk"]),
                ),
                ("budget", integer("Candidate forms a brute-force search may examine per input (default 20000).")),
            ],
            &["grammar"],
        ),
        output: Some(output(json!({
            "parser": {"type": "string"},
            "results": {"type": "array"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Parsing…",
        invoked: "Parsed",
    }
}

fn parse_tables() -> ToolDef {
    ToolDef {
        name: "rflap_parse_tables",
        title: "FIRST/FOLLOW and parse tables",
        description: "FIRST and FOLLOW sets, the LL(1) table and the SLR(1) automaton with its \
            ACTION/GOTO tables, with conflicts marked. The tool for \"build the LL(1) table of \
            this grammar\". Example: {\"grammar\":\"E -> TA\\nA -> +TA | λ\\nT -> i\"}."
            .to_string(),
        input: schema(vec![("grammar", grammar("The grammar."))], &["grammar"]),
        output: Some(output(json!({
            "first": {"type": "object"},
            "follow": {"type": "object"},
            "ll1": {"type": "object"},
            "slr1": {"type": "object"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Building the tables…",
        invoked: "Tables built",
    }
}

fn render() -> ToolDef {
    ToolDef {
        name: "rflap_render",
        title: "Render a document",
        description: "Draws a document as svg (machines and L-systems), markdown (transition \
            table), dot (Graphviz) or text. `highlight_states` (names) and `highlight_transitions` \
            (indices from rflap_analyze) mark what you are explaining. Example: \
            {\"document\":{...},\"format\":\"markdown\",\"highlight_states\":[\"q1\"]}."
            .to_string(),
        input: schema(
            vec![
                ("document", document("The document to draw.")),
                (
                    "format",
                    choice(
                        "Output format (default svg).",
                        &["svg", "markdown", "dot", "text"],
                    ),
                ),
                ("highlight_states", strings("State names to highlight.")),
                (
                    "highlight_transitions",
                    json!({"type": "array", "items": {"type": "integer", "minimum": 0},
                           "description": "Transition indices to highlight."}),
                ),
                ("dark", boolean("Dark theme.")),
                (
                    "notation",
                    choice(
                        "Turing machine labels: catedra `(e,s,m)` with Δ and I/N/D (default) or jflap `e;s,m` with □ and L/S/R.",
                        &["catedra", "jflap"],
                    ),
                ),
                (
                    "join_labels",
                    boolean("Join parallel labels with \" / \" instead of stacking them."),
                ),
                (
                    "depth",
                    integer("Rewrite level when drawing an L-system (default 3)."),
                ),
            ],
            &["document"],
        ),
        output: Some(output(json!({
            "format": {"type": "string"},
            "content": {"type": "string"},
            "document": {"type": "object"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Rendering…",
        invoked: "Rendered",
    }
}

fn open_jff() -> ToolDef {
    ToolDef {
        name: "rflap_open_jff",
        title: "Open a JFLAP file",
        description: "Reads a JFLAP .jff file (any type) into a document and shows it. Pass the \
            file's text as `xml`, or `path` to a file on the machine where this server runs \
            (local use only)."
            .to_string(),
        input: schema(
            vec![
                ("xml", string("The text of the .jff file.")),
                (
                    "path",
                    string("Path of a .jff file to read (local servers only)."),
                ),
            ],
            &[],
        ),
        output: Some(output(json!({"document": {"type": "object"}}))),
        visual: true,
        read_only: false,
        invoking: "Opening…",
        invoked: "Opened",
    }
}

fn save_jff() -> ToolDef {
    ToolDef {
        name: "rflap_save_jff",
        title: "Save as a JFLAP file",
        description: "Writes a document as .jff text that JFLAP 7.1 opens. Returns the XML; with \
            `path` (a .jff file on the machine where this server runs, local use only) it also \
            writes the file, refusing to overwrite unless `overwrite` is true."
            .to_string(),
        input: schema(
            vec![
                ("document", document("The document to save.")),
                (
                    "path",
                    string("Where to write the file; must end in .jff (local servers only)."),
                ),
                ("overwrite", boolean("Allow replacing an existing file.")),
            ],
            &["document"],
        ),
        output: Some(output(
            json!({"xml": {"type": "string"}, "type": {"type": "string"}}),
        )),
        visual: false,
        read_only: false,
        invoking: "Saving…",
        invoked: "Saved",
    }
}

fn lsystem_render() -> ToolDef {
    ToolDef {
        name: "rflap_lsystem_render",
        title: "Draw an L-system",
        description: "Expands an L-system `depth` levels (max 12) and draws it with the turtle. \
            Example: {\"document\":{\"type\":\"lsystem\",\"axiom\":\"F\",\"rules\":[\"F -> F + F - F - F + F\"],\
            \"parameters\":{\"angle\":\"90\",\"distance\":\"4\"}},\"depth\":2}."
            .to_string(),
        input: schema(
            vec![
                ("document", document("An lsystem document.")),
                ("depth", integer("Rewrite level (default 3, at most 12).")),
                ("options", object("Drawing options: seed, ... (see rflap_capabilities).")),
                ("include_svg", boolean("Put the SVG text in the reply as well (default false).")),
            ],
            &["document"],
        ),
        output: Some(output(json!({
            "svg": {"type": "string"},
            "depth": {"type": "integer"},
            "expansion_length": {"type": "integer"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Drawing the L-system…",
        invoked: "L-system drawn",
    }
}

fn pumping() -> ToolDef {
    ToolDef {
        name: "rflap_pumping",
        title: "Pumping lemma game",
        description: "JFLAP's pumping lemma games. action `languages` lists the 26 predefined \
            languages; `new` starts a game (kind regular|context_free, language = index or title, \
            mode human_first|computer_first); `move` plays one move on the `game` document that \
            `new` returned: {\"move\":\"choose_m\",\"m\":3}, {\"move\":\"choose_w\",\"w\":\"aabb\"}, \
            choose_decomposition, choose_i, add_case, ... Example: \
            {\"action\":\"new\",\"kind\":\"regular\",\"language\":0,\"mode\":\"computer_first\"}."
            .to_string(),
        input: schema(
            vec![
                ("action", choice("What to do.", &["languages", "new", "move"])),
                ("kind", choice("Which lemma (new).", &["regular", "context_free"])),
                (
                    "language",
                    json!({"description": "Index or title from `languages` (new).",
                           "anyOf": [{"type": "integer", "minimum": 0}, {"type": "string"}]}),
                ),
                ("mode", choice("Who moves first (new).", &["human_first", "computer_first"])),
                ("seed", json!({"type": "integer", "description": "Seed of the computer's choices (new)."})),
                ("game", document("The game document returned by `new` or `move` (move).")),
                ("move", object("The move, e.g. {\"move\":\"choose_w\",\"w\":\"aabb\"} (move).")),
            ],
            &["action"],
        ),
        output: None,
        visual: true,
        read_only: true,
        invoking: "Setting up the game…",
        invoked: "Game ready",
    }
}

fn capabilities() -> ToolDef {
    ToolDef {
        name: "rflap_capabilities",
        title: "What Rflap can do",
        description:
            "The full description of the Rflap API: every operation with an example, every \
            document type with its compact syntax, every conversion. Read it once when you need \
            the exact syntax of a PDA, Turing machine, L-system… `section` narrows the reply; `layout` \
            lists the layout algorithms and the compact `layout`, `points` and `title` fields."
                .to_string(),
        input: schema(
            vec![(
                "section",
                choice(
                    "Which part (default overview).",
                    &["overview", "operations", "document_types", "convert", "layout", "all"],
                ),
            )],
            &[],
        ),
        output: None,
        visual: false,
        read_only: true,
        invoking: "Reading the capabilities…",
        invoked: "Capabilities read",
    }
}

fn call() -> ToolDef {
    ToolDef {
        name: "rflap_call",
        title: "Raw Rflap API call",
        description: "Escape hatch: sends one raw Rflap API request {\"op\": ..., ...} (see \
            rflap_capabilities) and returns the raw envelope {\"ok\": true, \"result\": ...} or {\"ok\": false, \"error\": {...}} (the widget uses it as its remote engine). Use it for operations without a \
            dedicated tool: normalize, layout, multiple_run, user_parse, explain, … Example: \
            {\"request\":{\"op\":\"layout\",\"document\":{...},\"algorithm\":\"tree\"}}."
            .to_string(),
        input: schema(
            vec![(
                "request",
                json!({"anyOf": [{"type": "string"}, {"type": "object"}],
                    "description": "The request: a JSON object (or the same JSON as text) whose `op` names the operation."}),
            )],
            &["request"],
        ),
        output: None,
        visual: false,
        read_only: true,
        invoking: "Calling Rflap…",
        invoked: "Rflap answered",
    }
}

fn mt_catedra() -> ToolDef {
    ToolDef {
        name: "rflap_mt_catedra",
        title: "Turing machine / ALL in the course notation",
        description: "ALL (autómata linealmente acotado, linear bounded automaton) or one-tape Turing \
            machine (MT) in the notation of the course \"Sintaxis y Semántica de los Lenguajes\" (UTN FRT). `action`: \
            \"definition\" = the formal definition MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩ with its sets; \
            \"table\" = the transition table (Estado Actual | Símbolo Entrada | Estado Nuevo | \
            Símbolo Salida | Movimiento Cabezal); \"run\" = the configuration sequences \
            (Δ1abaΔ ├─ ΔΔ2baΔ … final/acepta or bloqueo/rechazo) for `inputs`; \"all_check\" = \
            whether the machine obeys the rules of an ALL (reads ⟨ moving D, reads ⟩ moving I, \
            never reads or writes Δ, the head never leaves ⟨…⟩) and lists every violation. Write \
            the machine as a compact turing document with cátedra labels (e,s,m): move I left, N \
            stay, D right, Δ the blank, several labels on one edge joined with \" / \"; final \
            states halt. Example: {\"document\":{\"type\":\"turing\",\"initial\":\"1\",\
            \"finals\":[\"8\"],\"transitions\":[[\"1\",\"(a,Δ,D)\",\"2\"],\
            [\"2\",\"(a,a,D) / (b,b,D)\",\"2\"],[\"2\",\"(Δ,Δ,D)\",\"8\"]]},\"action\":\"run\",\
            \"inputs\":[\"aa\"]}."
            .to_string(),
        input: schema(
            vec![
                ("document", document("The one-tape Turing machine or ALL.")),
                (
                    "action",
                    choice(
                        "definition, table, run (needs `inputs`) or all_check (static check, plus \
                         a dynamic one on `inputs`).",
                        &["definition", "table", "run", "all_check"],
                    ),
                ),
                (
                    "inputs",
                    strings(
                        "Words to run (\"\" is the empty word). Needed by `run`; optional for `all_check`.",
                    ),
                ),
                (
                    "model",
                    choice(
                        "mt (Turing machine) or all (linear bounded automaton, the tape is delimited by ⟨ and ⟩). Default: all when a transition mentions ⟨ or ⟩, otherwise mt.",
                        &["mt", "all"],
                    ),
                ),
                (
                    "max_steps",
                    integer(
                        "Steps before a run is reported as not halting (default 1000, at most 100000).",
                    ),
                ),
                (
                    "input_alphabet",
                    json!({"anyOf": [{"type": "string"}, {"type": "array", "items": {"type": "string"}}],
                           "description": "Declares Σ_E, such as \"ab\" or [\"a\",\"b\"]; if missing it is inferred."}),
                ),
            ],
            &["document", "action"],
        ),
        output: Some(output(json!({
            "document": {"type": "object"},
            "action": {"type": "string"},
            "model": {"type": "string"},
            "text": {"type": "string"},
            "results": {"type": "array"},
            "ok": {"type": "boolean"},
            "violations": {"type": "array"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Applying the course notation…",
        invoked: "Course notation ready",
    }
}

fn compose_tm() -> ToolDef {
    ToolDef {
        name: "rflap_compose_tm",
        title: "Compose two Turing machines",
        description: "Composition of two one-tape Turing machines as in the course (CLASE 41): \
            MT3 = ⟨Q1∪Q2, Σ_E, Σ_A, q01, F2, f3⟩; f3 follows f1 in Q1 except that a transition of \
            f1 into a state of F1 goes to q02 instead, and follows f2 in Q2 (states of the second \
            machine get a prime). Returns the composite machine, its definition and its table. \
            Example: {\"a\":{\"type\":\"turing\",\"initial\":\"s0\",\"finals\":[\"s1\"],\
            \"transitions\":[[\"s0\",\"(a,a,D)\",\"s1\"]]},\"b\":{\"type\":\"turing\",\
            \"initial\":\"s0\",\"finals\":[\"s1\"],\"transitions\":[[\"s0\",\"(b,b,D)\",\"s1\"]]}}."
            .to_string(),
        input: schema(
            vec![
                ("a", document("The first machine, MT1.")),
                (
                    "b",
                    document("The second machine, MT2, run after MT1 halts in a final state."),
                ),
            ],
            &["a", "b"],
        ),
        output: Some(output(json!({
            "document": {"type": "object"},
            "definition": {"type": "object"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Composing…",
        invoked: "Composed",
    }
}

fn membership() -> ToolDef {
    ToolDef {
        name: "rflap_membership",
        title: "Membership by the sets Tk",
        description: "Membership of a word in the language of a non-contracting (non-compressing) \
            grammar by the algorithm of the course: T0 = { S }, Tk = Tk-1 ∪ { β / |β| ≤ N ∧ \
            α ∈ Tk-1 ∧ α ⇒ β } with N = |word|, repeated until Tk = Tk-1 or the word is in Tk. \
            Returns every set and the verdict. A grammar with contracting productions is \
            rejected. Example: {\"grammar\":\"S -> T\nDB -> BD\nD -> c\nT -> aTBD\nT -> abD\nbB -> bb\",\
            \"input\":\"abbc\"}."
            .to_string(),
        input: schema(
            vec![
                ("grammar", grammar("The grammar (it may have several symbols on the left).")),
                ("input", string("The word to test (\"\" is the empty word).")),
                (
                    "max_forms",
                    integer("Sentential forms the sets may hold before giving up (default 100000)."),
                ),
            ],
            &["grammar", "input"],
        ),
        output: Some(output(json!({
            "document": {"type": "object"},
            "member": {"type": "boolean"},
            "bound": {"type": "integer"},
            "sets": {"type": "array"},
            "text": {"type": "string"},
        }))),
        visual: true,
        read_only: true,
        invoking: "Building the sets Tk…",
        invoked: "Membership decided",
    }
}
