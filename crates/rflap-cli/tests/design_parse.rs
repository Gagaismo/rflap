mod common;
use common::*;

const ENDS_WITH_AB: &str = "Designed a DFA with 3 states for: ends_with(\"ab\")\n  q0 (initial): ends with no start of \"ab\"\n  q1: ends with \"a\", the start of \"ab\"\n  q2 (accepting): ends with \"ab\"\nregex: (b*aa*b(aa*b)*b)*b*aa*b(aa*b)*\n";

#[test]
fn design_constraint_prints_states_regex_and_verification_and_saves_a_jff() {
    let t = Temp::new();
    let path = t.path("d.jff");
    let out = rflap(&[
        "design",
        "--constraint",
        "ends_with(\"ab\")",
        "--alphabet",
        "ab",
        "-o",
        &path,
    ]);
    assert_eq!(
        out.stdout,
        format!(
            "{ENDS_WITH_AB}verification: ok (511 strings up to length 8 checked)\nwrote {path}\n"
        )
    );
    assert_eq!(out.stderr, "");
    assert_eq!(out.code, 0);
    let run = rflap(&["run", &path, "aab"]);
    assert_eq!(run.stdout, "\"aab\": accepted\n");
    assert_eq!(run.code, 0);
}

#[test]
fn design_reports_the_examples() {
    let out = rflap(&[
        "design",
        "--constraint",
        "ends_with(\"ab\")",
        "--alphabet",
        "ab",
        "--accept",
        "aab",
        "--reject",
        "ba,a",
    ]);
    assert_eq!(
        out.stdout,
        format!(
            "{ENDS_WITH_AB}verification: ok (511 strings up to length 8 checked; examples 3/3 passed)\n"
        )
    );
    assert_eq!(out.code, 0);
}

#[test]
fn design_with_a_failing_example_exits_1() {
    let out = rflap(&[
        "design",
        "--constraint",
        "ends_with(\"ab\")",
        "--alphabet",
        "ab",
        "--accept",
        "ba",
    ]);
    assert_eq!(
        out.stdout,
        format!(
            "{ENDS_WITH_AB}verification: FAILED (511 strings up to length 8 checked; examples 0/1 passed)\n  expected accept but rejected: \"ba\"\n"
        )
    );
    assert_eq!(out.code, 1);
}

#[test]
fn design_from_a_regex() {
    let out = rflap(&["design", "--regex", "(a+b)*abb", "--alphabet", "ab"]);
    assert!(
        out.stdout
            .starts_with("Designed a DFA with 4 states for: strings matching (a+b)*abb\n"),
        "{}",
        out.stdout
    );
    assert_eq!(out.code, 0);
}

#[test]
fn design_from_a_regular_grammar() {
    let out = rflap(&[
        "design",
        "--grammar",
        "S -> aS | bA\nA -> bA | λ",
        "--alphabet",
        "ab",
    ]);
    assert!(
        out.stdout.starts_with("Designed a DFA with "),
        "{}",
        out.stdout
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
}

#[test]
fn design_from_a_template_builds_a_pda() {
    let t = Temp::new();
    let svg = t.path("pda.svg");
    let out = rflap(&[
        "design",
        "--template",
        "an_bn",
        "--alphabet",
        "ab",
        "--svg",
        &svg,
    ]);
    assert!(
        out.stdout.starts_with("Designed a PDA with "),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.ends_with(&format!("wrote {svg}\n")),
        "{}",
        out.stdout
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(t.read("pda.svg").starts_with("<svg"));
}

#[test]
fn design_template_parameters() {
    let out = rflap(&[
        "design",
        "--template",
        "an_bm",
        "--param",
        "relation=le",
        "--alphabet",
        "ab",
    ]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stdout.starts_with("Designed a PDA with "),
        "{}",
        out.stdout
    );
}

#[test]
fn design_json_prints_the_raw_answer() {
    let out = rflap(&[
        "design",
        "--constraint",
        "ends_with(\"ab\")",
        "--alphabet",
        "ab",
        "--json",
    ]);
    let json: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    assert_eq!(json["op"], "design");
    assert_eq!(json["result"]["stats"]["states"], 3);
}

#[test]
fn design_rejects_a_bad_constraint_with_the_design_error() {
    let out = rflap(&["design", "--constraint", "frobnicate(", "--alphabet", "ab"]);
    assert!(
        out.stderr.starts_with("error: design_error: "),
        "{}",
        out.stderr
    );
    assert_eq!(out.code, 2);
}

#[test]
fn design_needs_exactly_one_language() {
    let out = rflap(&["design", "--alphabet", "ab"]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.starts_with("error: "), "{}", out.stderr);
}

const G: &str = "S -> aSb | λ";

#[test]
fn parse_ll1_accepts_and_rejects() {
    let out = rflap(&["parse", "--grammar", G, "aabb", "aab", "--parser", "ll1"]);
    assert_eq!(
        out.stdout,
        "\"aabb\": accepted\n  S => aSb => aaSbb => aabb\n\"aab\": rejected\n"
    );
    assert_eq!(out.code, 1);
}

#[test]
fn parse_slr1_reads_a_grammar_file() {
    let t = Temp::new();
    let f = t.write("expr.txt", "E -> E+T | T\nT -> T*F | F\nF -> (E) | i\n");
    let out = rflap(&["parse", &f, "i+i", "--parser", "slr1"]);
    assert_eq!(
        out.stdout,
        "\"i+i\": accepted\n  E => E+T => E+F => E+i => T+i => F+i => i+i\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn parse_tree_is_drawn_in_text() {
    let out = rflap(&["parse", "--grammar", G, "ab", "--parser", "brute", "--tree"]);
    assert_eq!(
        out.stdout,
        "\"ab\": accepted\n  S => aSb => ab\n  S\n  ├─ a\n  ├─ S\n  │  └─ λ\n  └─ b\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn parse_cyk_on_a_grammar_not_in_cnf_is_an_error() {
    let out = rflap(&["parse", "--grammar", G, "ab", "--parser", "cyk"]);
    assert_eq!(
        out.stdout,
        "\"ab\": error: the production S → aSb is not in Chomsky normal form\n"
    );
    assert_eq!(out.code, 2);
}

#[test]
fn parse_brute_budget_exhausted_exits_3() {
    let out = rflap(&[
        "parse",
        "--grammar",
        "S -> SS | a",
        "aaaaaaaa",
        "--parser",
        "brute",
        "--budget",
        "3",
    ]);
    assert_eq!(out.stdout, "\"aaaaaaaa\": undecided\n");
    assert_eq!(out.code, 3);
}

#[test]
fn tables_prints_first_follow_ll1_and_slr1() {
    let out = rflap(&["tables", "--grammar", G]);
    let expected = "\
FIRST
  S: a, λ

FOLLOW
  S: $, b

LL(1): yes
|  | a | b | $ |
|---|---|---|---|
| S | S -> aSb | S -> λ | S -> λ |

SLR(1): yes
Productions:
  0: S' -> S
  1: S -> aSb
  2: S -> λ
States:
  0: S'→·S, S→·aSb, S→·
  1: S'→S·
  2: S→·aSb, S→a·Sb, S→·
  3: S→aS·b
  4: S→aSb·
| State | a | b | $ | S |
|---|---|---|---|---|
| 0 | s2 | r2 | r2 | 1 |
| 1 |  |  | acc |  |
| 2 | s2 | r2 | r2 | 3 |
| 3 |  | s4 |  |  |
| 4 |  | r1 | r1 |  |
";
    assert_eq!(out.stdout, expected);
    assert_eq!(out.code, 0);
}

#[test]
fn tables_lists_conflicts() {
    let out = rflap(&["tables", "--grammar", "S -> S+S | a"]);
    assert!(out.stdout.contains("LL(1): no\n"), "{}", out.stdout);
    assert!(
        out.stdout
            .contains("conflicts:\n  S on a: S -> S+S / S -> a\n"),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains("SLR(1): no\n"), "{}", out.stdout);
    assert!(
        out.stdout.contains("conflicts:\n  state 4 on +: r1 / s3\n"),
        "{}",
        out.stdout
    );
}

#[test]
fn grammar_type_names_the_classification() {
    let out = rflap(&["grammar-type", "--grammar", G]);
    assert_eq!(
        out.stdout,
        "This is a Context-Free Grammar\nclassification: context_free\n"
    );
    assert_eq!(out.code, 0);
}
