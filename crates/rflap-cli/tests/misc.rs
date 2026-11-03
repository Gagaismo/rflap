mod common;
use common::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const SUBCOMMANDS: [&str; 18] = [
    "call",
    "info",
    "run",
    "step",
    "convert",
    "equiv",
    "combine",
    "render",
    "layout",
    "grammar-type",
    "parse",
    "tables",
    "design",
    "explain",
    "lsystem",
    "pumping",
    "capabilities",
    "gui",
];

#[test]
fn help_documents_the_exit_codes() {
    let out = rflap(&["--help"]);
    assert_eq!(out.code, 0);
    for needle in [
        "Exit codes:",
        "0  success",
        "1  some input was rejected",
        "2  usage or input error",
        "3  undecided",
        "--json",
    ] {
        assert!(
            out.stdout.contains(needle),
            "missing {needle:?}:\n{}",
            out.stdout
        );
    }
}

#[test]
fn every_subcommand_has_help_with_an_example() {
    for sub in SUBCOMMANDS.iter() {
        let out = rflap(&[sub, "--help"]);
        assert_eq!(out.code, 0, "{sub}");
        assert!(out.stdout.contains("Example"), "{sub}:\n{}", out.stdout);
    }
    let pumping_play = rflap(&["pumping", "play", "--help"]);
    assert!(pumping_play.stdout.contains("Example"));
}

#[test]
fn unknown_subcommand_is_a_usage_error() {
    let out = rflap(&["frobnicate"]);
    assert_eq!(out.code, 2);
    assert_eq!(out.stdout, "");
    assert!(out.stderr.starts_with("error: "), "{}", out.stderr);
}

#[test]
fn missing_arguments_are_a_usage_error() {
    let out = rflap(&["run"]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.starts_with("error: "), "{}", out.stderr);
}

#[test]
fn call_passes_the_request_through() {
    let out = rflap(&["call", r#"{"op":"grammar_type","grammar":"S -> aSb | λ"}"#]);
    assert_eq!(
        out.stdout,
        "{\"ok\":true,\"op\":\"grammar_type\",\"result\":{\"classification\":\"context_free\",\"message\":\"This is a Context-Free Grammar\",\"report\":{\"classification\":\"context_free\",\"cnf\":false,\"context_free\":true,\"context_sensitive\":false,\"gnf\":false,\"left_linear\":false,\"regular\":false,\"right_linear\":false,\"turing_converted\":false}}}\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn call_reads_the_request_from_stdin() {
    let out = rflap_stdin(&["call", "-"], r#"{"op":"pumping_languages"}"#);
    assert!(
        out.stdout
            .starts_with("{\"ok\":true,\"op\":\"pumping_languages\"")
    );
    assert_eq!(out.code, 0);
    let again = rflap_stdin(&["call"], r#"{"op":"pumping_languages"}"#);
    assert_eq!(again.stdout, out.stdout);
}

#[test]
fn call_error_prints_the_raw_error_and_exits_2() {
    let out = rflap(&["call", r#"{"op":"nope"}"#]);
    assert!(
        out.stdout
            .starts_with("{\"error\":{\"code\":\"unknown_op\""),
        "{}",
        out.stdout
    );
    assert!(
        out.stderr
            .starts_with("error: unknown_op: unknown op `nope`; the operations are "),
        "{}",
        out.stderr
    );
    assert_eq!(out.code, 2);
}

#[test]
fn call_with_bad_json_exits_2() {
    let out = rflap(&["call", "{oops"]);
    assert!(
        out.stderr
            .starts_with("error: invalid_json: the request is not valid JSON"),
        "{}",
        out.stderr
    );
    assert_eq!(out.code, 2);
}

#[test]
fn render_markdown_table_is_exact() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["render", &f, "--format", "md"]);
    assert_eq!(
        out.stdout,
        "| State | a | b |\n|---|---|---|\n| → q0 | q1 | ∅ |\n| q1 | ∅ | q2 |\n| * q2 | q1 | ∅ |\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn render_text_is_the_default() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["render", &f]);
    assert_eq!(
        out.stdout,
        "Finite automaton: 3 states, 3 transitions\ninitial: q0\nfinal: q2\nq0 --a--> q1\nq1 --b--> q2\nq2 --a--> q1\n"
    );
}

#[test]
fn render_svg_infers_the_format_from_the_output_file() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let svg = t.path("dfa.svg");
    let out = rflap(&["render", &f, "-o", &svg, "--dark", "--highlight", "q1,q2"]);
    assert_eq!(out.stdout, format!("wrote {svg}\n"));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(t.read("dfa.svg").starts_with("<svg"));
}

#[test]
fn render_dot() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["render", &f, "--format", "dot"]);
    assert!(out.stdout.starts_with("digraph"), "{}", out.stdout);
}

#[test]
fn convert_to_svg_output_renders_the_result() {
    let t = Temp::new();
    let nfa = t.write("nfa.json", NFA);
    let svg = t.path("dfa.svg");
    let out = rflap(&["convert", &nfa, "--to", "dfa", "-o", &svg]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(t.read("dfa.svg").starts_with("<svg"));
}

#[test]
fn unsupported_output_extension_is_an_error() {
    let t = Temp::new();
    let nfa = t.write("nfa.json", NFA);
    let bad = t.path("x.doc");
    let out = rflap(&["convert", &nfa, "--to", "dfa", "-o", &bad]);
    assert_eq!(
        out.stderr,
        format!(
            "error: invalid_argument: cannot write `{bad}`: use a .jff, .json or .svg output file\n"
        )
    );
    assert_eq!(out.code, 2);
}

#[test]
fn lsystem_writes_an_svg() {
    let t = Temp::new();
    let l = t.write(
        "l.json",
        r#"{"type":"lsystem","axiom":"F","rules":["F -> F + F - F - F + F"],"parameters":{"angle":"90","distance":"4"}}"#,
    );
    let svg = t.path("l.svg");
    let out = rflap(&["lsystem", &l, "--depth", "1", "-o", &svg]);
    assert_eq!(out.stdout, format!("wrote {svg} (depth 1, 9 symbols)\n"));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(t.read("l.svg").starts_with("<svg"));
}

#[test]
fn lsystem_without_output_prints_the_expansion() {
    let t = Temp::new();
    let l = t.write(
        "l.json",
        r#"{"type":"lsystem","axiom":"F","rules":["F -> F + F - F - F + F"],"parameters":{"angle":"90","distance":"4"}}"#,
    );
    let out = rflap(&["lsystem", &l, "--depth", "1"]);
    assert_eq!(out.stdout, "depth 1: 9 symbols\nF + F - F - F + F\n");
}

#[test]
fn pumping_list_shows_the_26_languages() {
    let out = rflap(&["pumping", "list"]);
    let lines: Vec<&str> = out.stdout.lines().collect();
    assert_eq!(lines.len(), 26);
    assert_eq!(lines[0], " 0  regular       a^n b^n : n >= 0");
    assert_eq!(out.code, 0);
}

#[test]
fn pumping_play_scripted_moves() {
    let out = rflap(&[
        "pumping",
        "play",
        "regular",
        "0",
        "--mode",
        "human_first",
        "--move",
        "m=100",
    ]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stdout
            .contains("L = {a^n b^n : n >= 0} Regular Pumping Lemma\n"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("Prompt: Please select a value for m in Box 1 and press \"Enter\".\n"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains(
            "> m=100\nreply: Please enter a positive integer in range [4, 18] for best results.\n"
        ),
        "{}",
        out.stdout
    );
}

#[test]
fn pumping_play_interactive_reads_moves_from_stdin() {
    let out = rflap_stdin(
        &["pumping", "play", "regular", "0", "--mode", "human_first"],
        "m=100\nquit\n",
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("> m=100\nreply: "), "{}", out.stdout);
}

#[test]
fn pumping_play_unknown_move_is_an_input_error() {
    let out = rflap(&["pumping", "play", "regular", "0", "--move", "zzz=1"]);
    assert_eq!(
        out.stderr,
        "error: invalid_argument: unknown move `zzz=1`; moves are m=N, w=TEXT, decomposition=N,N, i=N, clear_all, add_case, replace_case=ROW, show_case=ROW, delete_case=ROW, list_cases, clear_cases, cases_done or a JSON move\n"
    );
    assert_eq!(out.code, 2);
}

#[test]
fn capabilities_lists_operations() {
    let out = rflap(&["capabilities"]);
    assert_eq!(out.code, 0);
    for needle in ["simulate", "minimal-dfa", "ll1", "circle"] {
        assert!(out.stdout.contains(needle), "{needle}:\n{}", out.stdout);
    }
    let json = rflap(&["capabilities", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&json.stdout).unwrap();
    assert_eq!(json["ok"], true);
}

struct Gui(Child);

impl Drop for Gui {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

fn get(port: &str, path: &str) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).expect("connect");
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n").unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("answer");
    answer
}

#[test]
fn gui_serves_the_editor_and_prints_the_url() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let child = Command::new(env!("CARGO_BIN_EXE_rflap"))
        .args(["gui", &f, "--no-open", "--port", "0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("starts");
    let mut gui = Gui(child);
    let mut line = String::new();
    BufReader::new(gui.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let url = line
        .trim()
        .strip_prefix("Rflap editor: ")
        .expect("url line")
        .to_string();
    let rest = url.strip_prefix("http://127.0.0.1:").expect("loopback url");
    let (port, fragment) = rest.split_once("/#doc=").expect("document fragment");
    assert!(!fragment.is_empty());
    assert!(
        fragment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "{fragment}"
    );
    let page = get(port, "/");
    assert!(
        page.starts_with("HTTP/1.0 200") || page.starts_with("HTTP/1.1 200"),
        "{page}"
    );
    assert!(page.contains("<html"), "{page}");
    assert!(page.contains("Rflap"), "{page}");
    let missing = get(port, "/nope");
    assert!(missing.contains(" 404 "), "{missing}");
}
