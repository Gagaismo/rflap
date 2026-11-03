//! The command line: arguments, help texts, exit codes.

use clap::{Args, Parser, Subcommand, ValueEnum};

const EXIT_CODES: &str = "\
Exit codes:
  0  success; every input was accepted
  1  some input was rejected, or the automata are not equivalent
  2  usage or input error (stderr: `error: <code>: <message>`)
  3  undecided: a configuration or search budget ran out

Files: .jff (any JFLAP type), .json (compact or full Rflap document), .txt/.grammar (one
production per line); `-` reads standard input. `regex:EXPR` and `grammar:TEXT` stand for
inline documents where a FILE is expected. `λ` (or `ε`) stands for the empty input.

Example:
  rflap design --constraint 'ends_with(\"ab\")' --alphabet ab -o ends_ab.jff
  rflap run ends_ab.jff aab abb      # exit 1: abb is rejected
  rflap render ends_ab.jff -o ends_ab.svg";

#[derive(Parser)]
#[command(
    name = "rflap",
    version,
    about = "Rflap: JFLAP 7.1 in Rust. Build, run, convert, draw and design automata, grammars and regular expressions.",
    after_help = EXIT_CODES,
    arg_required_else_help = true
)]
pub struct Cli {
    /// Print the raw JSON answer of the API instead of text (errors too)
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// A document written inline instead of in a file.
#[derive(Args, Clone, Default)]
pub struct Inline {
    /// Use this regular expression, e.g. "(a+b)*abb", instead of a FILE
    #[arg(long, value_name = "EXPR")]
    pub regex: Option<String>,
    /// Use this grammar (text such as "S -> aSb | λ", or a grammar file) instead of a FILE
    #[arg(long, value_name = "TEXT|FILE")]
    pub grammar: Option<String>,
}

/// A grammar written inline instead of in a file.
#[derive(Args, Clone, Default)]
pub struct InlineGrammar {
    /// Use this grammar (text such as "S -> aSb | λ", or a grammar file) instead of a FILE
    #[arg(long, value_name = "TEXT|FILE")]
    pub grammar: Option<String>,
}

/// One document: a FILE or an inline form.
#[derive(Args, Clone)]
pub struct DocArgs {
    /// .jff, .json or grammar text file; `-` reads standard input
    pub file: Option<String>,
    #[command(flatten)]
    pub inline: Inline,
}

/// One grammar: a FILE or inline text.
#[derive(Args, Clone)]
pub struct GrammarArgs {
    /// Grammar text file, .json or .jff; `-` reads standard input
    pub file: Option<String>,
    #[command(flatten)]
    pub inline: InlineGrammar,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ModeArg {
    /// λ transitions are steps of their own
    Step,
    /// every configuration is closed under λ transitions
    Closure,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum AcceptArg {
    Final,
    EmptyStack,
    Halting,
    Both,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum FormatArg {
    Svg,
    Md,
    Dot,
    Text,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ParserArg {
    Brute,
    BruteUnrestricted,
    Ll1,
    Slr1,
    Cyk,
}

/// The model of a machine of the course.
#[derive(Clone, Copy, ValueEnum)]
pub enum ModelArg {
    /// Turing machine
    Mt,
    /// linear bounded automaton
    All,
}

/// What a machine run needs besides the document.
#[derive(Args, Clone)]
pub struct RunOptions {
    /// How λ transitions are stepped (default: the API default)
    #[arg(long, value_enum)]
    pub mode: Option<ModeArg>,
    /// When a PDA or Turing machine accepts
    #[arg(long, value_enum)]
    pub acceptance: Option<AcceptArg>,
    /// Configurations explored before the answer is `undecided` (exit 3)
    #[arg(long, value_name = "N")]
    pub max_configs: Option<usize>,
    /// Input of one tape of a multi-tape Turing machine; repeat once per tape
    #[arg(long, value_name = "TEXT")]
    pub tape: Vec<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Send one raw JSON request to the API and print the raw JSON answer
    #[command(after_help = "\
The request is JSON text, or `-` (or nothing) to read it from standard input. Exit code 2
when the API answers an error. `rflap capabilities` lists every operation with an example.

Example:
  rflap call '{\"op\":\"grammar_type\",\"grammar\":\"S -> aSb | λ\"}'
  echo '{\"op\":\"capabilities\"}' | rflap call -")]
    Call {
        /// The request JSON, or `-` for standard input
        #[arg(value_name = "JSON")]
        request: Option<String>,
    },

    /// Show type, states, alphabet, determinism, λ transitions, unreachable and useless states
    #[command(after_help = "\
Example:
  rflap info machine.jff
  rflap info --grammar 'S -> aSb | λ'
  rflap info --regex '(a+b)*abb'")]
    Info(DocArgs),

    /// Run a machine on one or more inputs (exit 0 all accepted, 1 some rejected, 3 undecided)
    #[command(after_help = "\
Inputs are positional, from --inputs-file (one per line) or, for a multi-tape Turing machine,
one --tape per tape. `λ` is the empty input.

Example:
  rflap run machine.jff ab aab abb
  rflap run machine.jff abb --trace
  rflap run tm.jff --tape aab --tape λ --acceptance both")]
    Run {
        #[command(flatten)]
        inline: Inline,
        /// FILE followed by the INPUTs (with --regex/--grammar only inputs)
        #[arg(value_name = "FILE INPUT...")]
        args: Vec<String>,
        /// Read more inputs from this file, one per line
        #[arg(long, value_name = "FILE")]
        inputs_file: Option<String>,
        /// Print every configuration of the run
        #[arg(long)]
        trace: bool,
        #[command(flatten)]
        options: RunOptions,
    },

    /// Run the first N steps of a machine and show the configurations reached
    #[command(after_help = "\
Example:
  rflap step machine.jff abb --steps 2
  rflap step machine.jff abb --steps 2 --mode closure")]
    Step {
        #[command(flatten)]
        inline: Inline,
        /// FILE followed by the INPUT
        #[arg(value_name = "FILE INPUT")]
        args: Vec<String>,
        /// Steps to take from the start
        #[arg(long, default_value_t = 1, value_name = "N")]
        steps: usize,
        #[command(flatten)]
        options: RunOptions,
    },

    /// Convert a document: NFA to DFA, minimize, FA to regex, regex to FA, grammars, PDA, ...
    #[command(after_help = "\
Targets: dfa, minimal-dfa, regex, nfa, regular-grammar, fa, pda-ll, pda-lr, cfg,
pda-jflap-form, unrestricted-grammar, remove-lambda, remove-unit, remove-useless, cnf,
complete, split-labels, remove-useless-states. `cfg` rewrites a PDA that is not in JFLAP's
form first (`pda-jflap-form` does only that). See `rflap capabilities` for what each accepts.
-o writes a .jff, .json (normalized) or .svg file; without it the result is printed.

Example:
  rflap convert nfa.jff --to minimal-dfa -o min.jff
  rflap convert nfa.jff --to regex
  rflap convert --grammar 'S -> aSb | λ' --to cnf --steps")]
    Convert {
        #[command(flatten)]
        doc: DocArgs,
        /// The target (see below)
        #[arg(long, value_name = "TARGET")]
        to: String,
        /// Write the result to this .jff, .json or .svg file
        #[arg(short, long, value_name = "OUT")]
        output: Option<String>,
        /// Also print the steps of the conversion (one JSON object per line)
        #[arg(long)]
        steps: bool,
    },

    /// Tell whether two finite automata (or regexes, or regular grammars) accept the same language
    #[command(after_help = "\
Exit 0 when equivalent, 1 with a counterexample string otherwise.

Example:
  rflap equiv a.jff b.jff
  rflap equiv machine.jff 'regex:(a+b)*abb'")]
    Equiv {
        /// First document (FILE, `-`, `regex:EXPR` or `grammar:TEXT`)
        a: String,
        /// Second document
        b: String,
    },

    /// Append the states and transitions of B below A (same type) and save the result
    #[command(after_help = "\
Example:
  rflap combine a.jff b.jff -o both.jff")]
    Combine {
        a: String,
        b: String,
        /// Output .jff, .json or .svg file
        #[arg(short, long, value_name = "OUT")]
        output: String,
    },

    /// Draw a document: svg, markdown transition table (md), Graphviz dot or text
    #[command(after_help = "\
Without --format the output file extension decides (.svg, .md, .dot); otherwise text.

Example:
  rflap render machine.jff --format md
  rflap render machine.jff -o machine.svg --dark
  rflap render machine.jff -o step2.svg --highlight q1,q2")]
    Render {
        #[command(flatten)]
        doc: DocArgs,
        #[arg(long, value_enum)]
        format: Option<FormatArg>,
        /// Write the drawing to this file instead of printing it
        #[arg(short, long, value_name = "OUT")]
        output: Option<String>,
        /// Dark theme (svg)
        #[arg(long)]
        dark: bool,
        /// States to highlight, by name: q1,q2
        #[arg(long, value_delimiter = ',', value_name = "STATES")]
        highlight: Vec<String>,
        /// Transitions to highlight, by index (see `info`): 0,2
        #[arg(long, value_delimiter = ',', value_name = "INDICES")]
        highlight_transitions: Vec<usize>,
        /// Rewrite level when drawing an L-system
        #[arg(long, value_name = "N")]
        depth: Option<usize>,
    },

    /// Position the states with a JFLAP layout algorithm and save the result
    #[command(after_help = "\
Algorithms: circle, gem, random, spiral, tree, tree-degree, two-circle.

Example:
  rflap layout machine.jff --algorithm tree -o tidy.jff")]
    Layout {
        #[command(flatten)]
        doc: DocArgs,
        #[arg(long, default_value = "circle", value_name = "ALGORITHM")]
        algorithm: String,
        /// Output .jff, .json or .svg file
        #[arg(short, long, value_name = "OUT")]
        output: String,
    },

    /// Classify a grammar: regular, context-free, context-sensitive, unrestricted, CNF, GNF
    #[command(after_help = "\
Example:
  rflap grammar-type grammar.txt
  rflap grammar-type --grammar 'S -> aSb | λ'")]
    GrammarType(GrammarArgs),

    /// Parse strings with a grammar (brute force, LL(1), SLR(1), CYK)
    #[command(after_help = "\
Exit 0 all accepted, 1 some rejected, 3 search budget exhausted, 2 an input cannot be parsed
(a symbol outside the grammar, CYK on a grammar not in CNF, ...).

Example:
  rflap parse grammar.txt aabb aab --parser ll1
  rflap parse --grammar 'S -> aSb | λ' ab --parser brute --tree")]
    Parse {
        #[command(flatten)]
        inline: InlineGrammar,
        /// FILE followed by the INPUTs (with --grammar only inputs)
        #[arg(value_name = "GRAMMAR INPUT...")]
        args: Vec<String>,
        #[arg(long, value_enum, default_value = "brute")]
        parser: ParserArg,
        /// Print the parse tree
        #[arg(long)]
        tree: bool,
        /// Candidate forms a brute-force search may examine per input
        #[arg(long, value_name = "N")]
        budget: Option<usize>,
    },

    /// Print FIRST/FOLLOW sets, the LL(1) table and the SLR(1) tables of a grammar
    #[command(after_help = "\
Example:
  rflap tables grammar.txt
  rflap tables --grammar 'E -> E+T | T\\nT -> T*F | F\\nF -> (E) | i'")]
    Tables(GrammarArgs),

    /// Build a minimal, verified automaton from a language description
    #[command(after_help = "\
Give exactly one of --constraint, --regex, --grammar or --template. Prints the states with
their meaning, the regex and the verification report (exit 1 if an example fails).
Constraints: starts_with(\"ab\"), ends_with(\"ab\"), contains(\"ab\"), length >= 3,
length % 2 == 0, count(a) % 2 == 0, regex(\"a*b\"), joined by and / or / xor / not.
Templates (not regular, build a PDA): an_bn, an_bm (--param relation=le|ge|ne),
equal_count, balanced_brackets (--param pairs='()[]'), wcwr (--param c=c), even_palindrome,
palindrome. --param a=x / b=y choose the symbols (default: the first two of the alphabet).

Example:
  rflap design --constraint 'ends_with(\"ab\")' --alphabet ab -o ends_ab.jff
  rflap design --regex '(a+b)*abb' --alphabet ab --accept abb,aabb --reject ab -o r.json
  rflap design --template an_bn --alphabet ab --svg anbn.svg")]
    Design {
        /// A constraint in text form
        #[arg(long, value_name = "TEXT")]
        constraint: Option<String>,
        /// A regular expression
        #[arg(long, value_name = "EXPR")]
        regex: Option<String>,
        /// A regular grammar (text or file)
        #[arg(long, value_name = "TEXT|FILE")]
        grammar: Option<String>,
        /// A ready-made non-regular language
        #[arg(long, value_name = "NAME")]
        template: Option<String>,
        /// Template parameter, KEY=VALUE (repeatable)
        #[arg(long, value_name = "KEY=VALUE")]
        param: Vec<String>,
        /// The symbols of the language, e.g. ab
        #[arg(long, value_name = "SYMBOLS")]
        alphabet: String,
        /// Strings that must be accepted, comma separated (`λ` is the empty string)
        #[arg(long, value_delimiter = ',', value_name = "W,...")]
        accept: Vec<String>,
        /// Strings that must be rejected, comma separated
        #[arg(long, value_delimiter = ',', value_name = "W,...")]
        reject: Vec<String>,
        /// Save the automaton to this .jff, .json or .svg file
        #[arg(short, long, value_name = "OUT")]
        output: Option<String>,
        /// Also save a drawing
        #[arg(long, value_name = "OUT.svg")]
        svg: Option<String>,
    },

    /// Explain a document in plain language (Markdown)
    #[command(after_help = "\
Example:
  rflap explain machine.jff")]
    Explain(DocArgs),

    /// Expand an L-system and draw it with the turtle
    #[command(after_help = "\
Example:
  rflap lsystem koch.jff --depth 3 -o koch.svg
  rflap lsystem koch.jff --depth 2")]
    Lsystem {
        /// .jff or .json L-system
        file: String,
        #[arg(long, default_value_t = 3, value_name = "N")]
        depth: usize,
        /// Write the drawing to this .svg file; without it the expansion is printed
        #[arg(short, long, value_name = "OUT.svg")]
        output: Option<String>,
    },

    /// The pumping lemma games of JFLAP (regular and context-free)
    #[command(after_help = "\nExample:
  rflap pumping list
  rflap pumping play regular 0 --mode human_first --move m=4")]
    Pumping {
        #[command(subcommand)]
        command: PumpingCommand,
    },

    /// Turing machines and linear bounded automata in the notation of the course (Sintaxis y Semántica de los Lenguajes)
    #[command(after_help = "\
One-tape machines only. A label is `(e,s,m)` or `e,s,m` (move I left, N stay, D right; Δ is the
blank), several labels of one edge are joined with ` / `, and JFLAP's `e;s,m` is read too.
A document may declare Σ_E with \"input_alphabet\": \"ab\"; otherwise it is inferred.

Example:
  rflap catedra definition palindrome.json
  rflap catedra table palindrome.json --markdown
  rflap catedra run palindrome.json aba ab λ
  rflap catedra run igual.json ab --all
  rflap catedra all-check igual.json ab b")]
    Catedra {
        #[command(subcommand)]
        command: CatedraCommand,
    },

    /// Membership of a word by the sets Tk (CLASE 41), for non-contracting grammars
    #[command(after_help = "\
Prints T0, T1, ... until Tk == Tk-1 or the word is in Tk. Exit 0 when the word belongs to the
language, 1 when it does not, 2 on errors (a contracting grammar is refused with
`contracting_grammar`).

Example:
  rflap membership grammar.txt abbc
  rflap membership --grammar 'S -> aSb | λ' aabb")]
    Membership {
        #[command(flatten)]
        inline: InlineGrammar,
        /// FILE followed by the WORD (with --grammar only the word); `λ` is the empty word
        #[arg(value_name = "GRAMMAR WORD")]
        args: Vec<String>,
        /// Forms the sets may hold before the search gives up
        #[arg(long, value_name = "N")]
        max_forms: Option<usize>,
    },

    /// List every operation, document type, conversion, parser and layout
    #[command(after_help = "\
Example:
  rflap capabilities
  rflap capabilities --json")]
    Capabilities,

    /// Open the web editor in the browser (served on 127.0.0.1)
    #[command(after_help = "\
Serves the editor until you press Ctrl-C and prints its URL; the document travels in the
URL fragment (#doc=<base64url JSON>), so it never leaves your machine.

Example:
  rflap gui machine.jff
  rflap gui --no-open --port 8080")]
    Gui {
        /// Document to open (.jff, .json, grammar text)
        file: Option<String>,
        /// Port to listen on (0 picks a free one)
        #[arg(long, default_value_t = 0, value_name = "P")]
        port: u16,
        /// Print the URL but do not open the browser
        #[arg(long)]
        no_open: bool,
    },
}

#[derive(Subcommand)]
pub enum CatedraCommand {
    /// The formal definition ⟨Q, Σ_E, Σ_A, q0, F, f⟩ of a machine, with its warnings
    #[command(after_help = "\
Warnings: a transition leaving a final state (\"El dominio no incluye a F\") and nondeterminism.

Example:
  rflap catedra definition palindrome.json
  rflap catedra definition igual.json --model all")]
    Definition {
        /// .json or .jff machine; `-` reads standard input
        file: String,
        /// The model (default: all when a transition mentions ⟨ or ⟩)
        #[arg(long, value_enum)]
        model: Option<ModelArg>,
    },

    /// The transition table: Estado Actual | Símbolo Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal
    #[command(after_help = "\
→ marks the initial state and * the finals (one row of dashes each).

Example:
  rflap catedra table palindrome.json
  rflap catedra table palindrome.json --markdown")]
    Table {
        /// .json or .jff machine; `-` reads standard input
        file: String,
        /// Print a Markdown table instead of aligned text
        #[arg(long)]
        markdown: bool,
    },

    /// Run a machine and print the configuration sequences (c1 ├─ c2 ├─ ... final/acepta)
    #[command(after_help = "\
Exit 0 all accepted, 1 some rejected (bloqueo/rechazo, or a violation of an ALL), 3 some run does
not stop within --max-steps, 2 errors. `λ` is the empty input.

Example:
  rflap catedra run palindrome.json aba bb ab λ
  rflap catedra run igual.json ab b --all")]
    Run {
        /// .json or .jff machine; `-` reads standard input
        file: String,
        /// Inputs (`λ` is the empty word)
        inputs: Vec<String>,
        /// Read more inputs from this file, one per line
        #[arg(long, value_name = "FILE")]
        inputs_file: Option<String>,
        /// Run as a linear bounded automaton (tape ⟨w⟩); same as --model all
        #[arg(long, conflicts_with = "model")]
        all: bool,
        /// The model (default: all when a transition mentions ⟨ or ⟩)
        #[arg(long, value_enum)]
        model: Option<ModelArg>,
        /// Transitions before a run is reported as not stopping (default 1000)
        #[arg(long, value_name = "N")]
        max_steps: Option<usize>,
    },

    /// Check that a machine follows the rules of an ALL (static rules, and a run per input)
    #[command(after_help = "\
Exit 0 when it does, 1 when there are violations.

Example:
  rflap catedra all-check igual.json ab b")]
    AllCheck {
        /// .json or .jff machine; `-` reads standard input
        file: String,
        /// Inputs to run for the dynamic check
        inputs: Vec<String>,
        #[arg(long, value_name = "N")]
        max_steps: Option<usize>,
    },

    /// Compose two machines: MT1 followed by MT2
    #[command(after_help = "\
Transitions of A into its final states go to the initial state of B; the final states are
those of B. Without -o the definition of the composite is printed.

Example:
  rflap catedra compose a.json b.json -o ab.json")]
    Compose {
        a: String,
        b: String,
        /// Output .jff, .json or .svg file
        #[arg(short, long, value_name = "OUT")]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PumpingCommand {
    /// List the 26 predefined languages
    #[command(after_help = "\
Example:
  rflap pumping list")]
    List,

    /// Play a pumping lemma game (scripted with --move, or interactive on standard input)
    #[command(after_help = "\
Moves: m=N, w=TEXT, decomposition=N,N (lengths |x|,|y|; |u|,|v|,|x|,|y| for context-free),
i=N, clear_all, add_case, replace_case=ROW, show_case=ROW, delete_case=ROW, list_cases,
clear_cases, cases_done, or a JSON move. Without --move, moves are read from standard input,
one per line, until `quit` or the end of input.

Example:
  rflap pumping play regular 0 --mode human_first --move m=4 --move w=aabb
  rflap pumping play context-free 0 --mode computer_first --seed 7")]
    Play {
        /// `regular` or `context-free`
        kind: String,
        /// Language index or title (see `pumping list`)
        language: String,
        /// `human_first` (you pick first) or `computer_first`
        #[arg(long, default_value = "computer_first")]
        mode: String,
        #[arg(long, value_name = "N")]
        seed: Option<i64>,
        /// A move to play (repeatable)
        #[arg(long = "move", value_name = "MOVE")]
        moves: Vec<String>,
    },
}
