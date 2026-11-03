mod common;
use common::*;

const TARGETS: &str = "dfa, minimal-dfa, regex, nfa, regular-grammar, fa, pda-ll, pda-lr, cfg, pda-jflap-form, unrestricted-grammar, remove-lambda, remove-unit, remove-useless, cnf, complete, split-labels, remove-useless-states";

#[test]
fn info_of_a_dfa_is_exact() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["info", &f]);
    assert_eq!(
        out.stdout,
        "type: fa (dfa)\nstates: 3 (q0, q1, q2)\ninitial: q0\nfinal: q2\nalphabet: a, b\ntransitions: 3\ndeterministic: yes\ncomplete: no\nlambda transitions: none\nunreachable states: none\nuseless states: none\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn info_of_an_nfa_with_lambda_and_dead_states() {
    let t = Temp::new();
    let f = t.write(
        "nfa.json",
        r#"{"type":"fa","initial":"q0","finals":["q1"],"transitions":[["q0","","q1"],["q0","a","q1"],["q0","a","q2"],["q9","b","q9"]]}"#,
    );
    let out = rflap(&["info", &f]);
    assert_eq!(
        out.stdout,
        "type: fa (nfa)\nstates: 4 (q0, q1, q2, q9)\ninitial: q0\nfinal: q1\nalphabet: a, b\ntransitions: 4\ndeterministic: no (nondeterministic states: q0)\ncomplete: no\nlambda transitions: 1 (q0 -λ-> q1)\nunreachable states: q9\nuseless states: q2, q9\n"
    );
}

#[test]
fn info_of_a_grammar_given_inline() {
    let out = rflap(&["info", "--grammar", "S -> aSb | λ"]);
    assert_eq!(
        out.stdout,
        "type: grammar\nThis is a Context-Free Grammar\nstart: S\nvariables: S\nterminals: a, b\nproductions: 2\n"
    );
}

#[test]
fn info_of_a_regex_given_inline() {
    let out = rflap(&["info", "--regex", "(a+b)*abb"]);
    assert_eq!(
        out.stdout,
        "type: regex\nexpression: (a+b)*abb\nalphabet: a, b\nnullable: no\n"
    );
}

#[test]
fn info_reads_a_grammar_text_file() {
    let t = Temp::new();
    let f = t.write("g.txt", "S -> aSb\nS -> λ\n");
    let out = rflap(&["info", &f]);
    assert!(out.stdout.starts_with("type: grammar\n"), "{}", out.stdout);
    assert_eq!(out.code, 0);
}

#[test]
fn convert_to_minimal_dfa_then_info_shows_the_state_count() {
    let t = Temp::new();
    let nfa = t.write("nfa.json", NFA);
    let out_path = t.path("min.jff");
    let out = rflap(&["convert", &nfa, "--to", "minimal-dfa", "-o", &out_path]);
    assert_eq!(out.stdout, format!("wrote {out_path}\n"));
    assert_eq!(out.code, 0);
    let info = rflap(&["info", &out_path]);
    assert!(
        info.stdout.contains("\nstates: 4 (q0, q1, q2, q3)\n"),
        "{}",
        info.stdout
    );
    assert!(
        info.stdout.starts_with("type: fa (dfa)\n"),
        "{}",
        info.stdout
    );
    let run = rflap(&["run", &out_path, "aabb", "abb"]);
    assert_eq!(run.stdout, "\"aabb\": accepted\n\"abb\": accepted\n");
    assert_eq!(run.code, 0);
}

#[test]
fn convert_to_regex_prints_the_expression() {
    let t = Temp::new();
    let nfa = t.write("nfa.json", NFA);
    let out = rflap(&["convert", &nfa, "--to", "regex"]);
    assert_eq!(out.stdout, "(a+b)*abb\n");
    assert_eq!(out.code, 0);
}

#[test]
fn convert_writes_normalized_json() {
    let t = Temp::new();
    let nfa = t.write("nfa.json", NFA);
    let out_path = t.path("dfa.json");
    let out = rflap(&["convert", &nfa, "--to", "dfa", "-o", &out_path]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let doc: serde_json::Value = serde_json::from_str(&t.read("dfa.json")).unwrap();
    assert_eq!(doc["type"], "fa");
    assert!(doc["states"].is_object());
}

#[test]
fn convert_steps_prints_the_steps() {
    let t = Temp::new();
    let nfa = t.write("nfa.json", NFA);
    let out = rflap(&["convert", &nfa, "--to", "minimal-dfa", "--steps"]);
    assert_eq!(out.code, 0);
    assert!(
        out.stdout.contains("\nsteps:\n  1. {\"groups\""),
        "{}",
        out.stdout
    );
}

#[test]
fn convert_unknown_target_is_a_usage_error() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["convert", &f, "--to", "bogus"]);
    assert_eq!(out.stdout, "");
    assert_eq!(
        out.stderr,
        format!("error: invalid_argument: unknown target `bogus`; the targets are {TARGETS}\n")
    );
    assert_eq!(out.code, 2);
}

#[test]
fn convert_accepts_every_target_of_capabilities() {
    let caps = rflap(&["capabilities", "--json"]);
    let caps: serde_json::Value = serde_json::from_str(&caps.stdout).unwrap();
    let t = Temp::new();
    let entries = caps["result"]["convert"].as_array().unwrap();
    assert_eq!(entries.len(), 18);
    for entry in entries {
        let to = entry["to"].as_str().unwrap().replace('_', "-");
        let doc = entry["example"]["document"].to_string();
        let f = t.write("example.json", &doc);
        let out = rflap(&["convert", &f, "--to", &to]);
        assert_eq!(out.code, 0, "{to}: {}", out.stderr);
        assert!(!out.stdout.is_empty(), "{to}");
    }
}

#[test]
fn missing_file_is_an_input_error() {
    let t = Temp::new();
    let missing = t.path("nope.jff");
    let out = rflap(&["info", &missing]);
    assert_eq!(out.stdout, "");
    assert_eq!(
        out.stderr,
        format!("error: io_error: cannot read `{missing}`: file not found\n")
    );
    assert_eq!(out.code, 2);
}

#[test]
fn bad_json_file_is_an_input_error() {
    let t = Temp::new();
    let f = t.write("bad.json", "{not json");
    let out = rflap(&["info", &f]);
    assert_eq!(
        out.stderr,
        format!("error: invalid_json: {f}: key must be a string at line 1 column 2\n")
    );
    assert_eq!(out.code, 2);
}

#[test]
fn bad_jff_file_reports_the_api_error() {
    let t = Temp::new();
    let f = t.write("bad.jff", "nope");
    let out = rflap(&["info", &f]);
    assert_eq!(
        out.stderr,
        "error: jff_error: could not parse XML: the document has no root element\n"
    );
    assert_eq!(out.code, 2);
}

#[test]
fn json_mode_prints_the_error_as_json_too() {
    let t = Temp::new();
    let f = t.write("bad.jff", "nope");
    let out = rflap(&["info", &f, "--json"]);
    assert_eq!(
        out.stdout,
        "{\"error\":{\"code\":\"jff_error\",\"message\":\"could not parse XML: the document has no root element\"},\"ok\":false}\n"
    );
    assert_eq!(out.code, 2);
}

#[test]
fn equiv_equivalent_exits_0() {
    let t = Temp::new();
    let nfa = t.write("nfa.json", NFA);
    let out = rflap(&["equiv", &nfa, "regex:(a+b)*abb"]);
    assert_eq!(out.stdout, "equivalent\n");
    assert_eq!(out.code, 0);
}

#[test]
fn equiv_different_prints_a_counterexample_and_exits_1() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["equiv", &f, "regex:ab"]);
    assert_eq!(out.stdout, "not equivalent (counterexample: \"abab\")\n");
    assert_eq!(out.code, 1);
}

#[test]
fn combine_writes_the_union_machine() {
    let t = Temp::new();
    let a = t.write("a.json", DFA);
    let b = t.write("b.json", NFA);
    let out_path = t.path("c.jff");
    let out = rflap(&["combine", &a, &b, "-o", &out_path]);
    assert_eq!(out.stdout, format!("wrote {out_path}\n"));
    assert_eq!(out.code, 0, "{}", out.stderr);
    let info = rflap(&["info", &out_path]);
    assert!(info.stdout.contains("\nstates: 7 ("), "{}", info.stdout);
}

#[test]
fn layout_writes_the_repositioned_document() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out_path = t.path("laid.json");
    let out = rflap(&["layout", &f, "--algorithm", "tree", "-o", &out_path]);
    assert_eq!(out.stdout, format!("wrote {out_path}\n"));
    assert_eq!(out.code, 0, "{}", out.stderr);
    let doc: serde_json::Value = serde_json::from_str(&t.read("laid.json")).unwrap();
    assert_eq!(doc["type"], "fa");
}

#[test]
fn explain_prints_the_markdown_summary() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["explain", &f]);
    assert!(
        out.stdout
            .starts_with("Finite automaton with 3 states and 3 transitions.\n"),
        "{}",
        out.stdout
    );
    assert_eq!(out.code, 0);
}

#[test]
fn convert_help_lists_every_target() {
    let help = rflap(&["convert", "--help"]).stdout.replace('\n', " ");
    for target in TARGETS.split(", ") {
        assert!(
            help.contains(target),
            "`convert --help` does not list `{target}`"
        );
    }
}
