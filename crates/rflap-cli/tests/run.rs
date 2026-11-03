mod common;
use common::*;

#[test]
fn run_accepting_input_exits_0() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["run", &f, "ab"]);
    assert_eq!(out.stdout, "\"ab\": accepted\n");
    assert_eq!(out.stderr, "");
    assert_eq!(out.code, 0);
}

#[test]
fn run_rejected_input_exits_1() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["run", &f, "ab", "b"]);
    assert_eq!(out.stdout, "\"ab\": accepted\n\"b\": rejected\n");
    assert_eq!(out.code, 1);
}

#[test]
fn run_reads_a_jff_file() {
    let out = rflap(&["run", &fixture("sample.jff"), "a", "b"]);
    assert_eq!(out.stdout, "\"a\": accepted\n\"b\": rejected\n");
    assert_eq!(out.code, 1);
}

#[test]
fn run_trace_lists_the_configurations() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["run", &f, "ab", "--trace"]);
    assert_eq!(
        out.stdout,
        "\"ab\": accepted\n  0. q0 remaining=\"ab\"\n  1. q1 remaining=\"b\"\n  2. q2 remaining=\"\"\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn run_lambda_stands_for_the_empty_input() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["run", &f, "λ"]);
    assert_eq!(out.stdout, "\"\": rejected\n");
    assert_eq!(out.code, 1);
}

#[test]
fn run_reads_inputs_from_a_file() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let inputs = t.write("inputs.txt", "ab\nabab\nba\n");
    let out = rflap(&["run", &f, "--inputs-file", &inputs]);
    assert_eq!(
        out.stdout,
        "\"ab\": accepted\n\"abab\": accepted\n\"ba\": rejected\n"
    );
    assert_eq!(out.code, 1);
}

#[test]
fn run_reads_the_document_from_stdin() {
    let out = rflap_stdin(&["run", "-", "ab"], DFA);
    assert_eq!(out.stdout, "\"ab\": accepted\n");
    assert_eq!(out.code, 0);
}

#[test]
fn run_json_prints_the_raw_api_answer() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["run", &f, "ab", "--json"]);
    assert_eq!(
        out.stdout,
        "{\"ok\":true,\"op\":\"simulate\",\"result\":{\"accepted\":true,\"outcome\":\"accepted\",\"trace\":[{\"remaining\":\"ab\",\"state\":\"q0\",\"state_id\":0},{\"remaining\":\"b\",\"state\":\"q1\",\"state_id\":1},{\"remaining\":\"\",\"state\":\"q2\",\"state_id\":2}]}}\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn run_json_with_many_inputs_is_one_multiple_run() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["run", &f, "ab", "b", "--json"]);
    assert_eq!(
        out.stdout,
        "{\"ok\":true,\"op\":\"multiple_run\",\"result\":{\"results\":[{\"accepted\":true,\"input\":\"ab\",\"outcome\":\"accepted\"},{\"accepted\":false,\"input\":\"b\",\"outcome\":\"rejected\"}]}}\n"
    );
    assert_eq!(out.code, 1);
}

#[test]
fn run_on_a_regex_is_the_api_wrong_type_error() {
    let out = rflap(&["run", "--regex", "ab", "ab"]);
    assert_eq!(out.stdout, "");
    assert_eq!(
        out.stderr,
        "error: wrong_document_type: `simulate` needs a fa, pda, turing, mealy or moore document, got a document of type `regex`\n"
    );
    assert_eq!(out.code, 2);
}

#[test]
fn run_undecided_exits_3() {
    let t = Temp::new();
    let f = t.write(
        "loop.json",
        r#"{"type":"turing","initial":"q0","finals":["q1"],"transitions":[["q0","_;a,R","q0"]]}"#,
    );
    let out = rflap(&["run", &f, "λ", "--max-configs", "5"]);
    assert_eq!(out.stdout, "\"\": undecided\n");
    assert_eq!(out.code, 3);
}

#[test]
fn run_turing_machine_with_one_input_per_tape() {
    let t = Temp::new();
    let f = t.write(
        "tm2.json",
        r#"{"type":"turing","initial":"q0","finals":["q1"],"transitions":[["q0","a;a,R|_;x,R","q0"],["q0","_;_,S|_;_,S","q1"]]}"#,
    );
    let out = rflap(&["run", &f, "--tape", "aa", "--tape", "λ"]);
    assert_eq!(
        out.stdout,
        "\"aa\" | \"\": accepted, tape 1 \"\", tape 2 \"\"\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn run_mealy_machine_prints_the_output() {
    let t = Temp::new();
    let f = t.write(
        "mealy.json",
        r#"{"type":"mealy","initial":"q0","transitions":[["q0","a;1","q0"],["q0","b;0","q0"]]}"#,
    );
    let out = rflap(&["run", &f, "abab"]);
    assert_eq!(out.stdout, "\"abab\": accepted, output \"1010\"\n");
    assert_eq!(out.code, 0);
}

#[test]
fn step_shows_the_frontier() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["step", &f, "ab", "--steps", "1"]);
    assert_eq!(
        out.stdout,
        "steps done: 1 (not finished)\nfrontier:\n  q1 remaining=\"b\"\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn step_finished_without_acceptance_exits_1() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["step", &f, "ba", "--steps", "5"]);
    assert_eq!(
        out.stdout,
        "steps done: 1 (finished)\nrejected:\n  q0 remaining=\"ba\"\n"
    );
    assert_eq!(out.code, 1);
}

#[test]
fn step_finished_with_acceptance_exits_0() {
    let t = Temp::new();
    let f = t.write("dfa.json", DFA);
    let out = rflap(&["step", &f, "ab", "--steps", "9"]);
    assert_eq!(
        out.stdout,
        "steps done: 3 (finished)\naccepted:\n  q2 remaining=\"\"\n"
    );
    assert_eq!(out.code, 0);
}
