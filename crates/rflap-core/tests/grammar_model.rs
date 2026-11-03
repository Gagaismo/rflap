//! Grammar model, text parsing and JFLAP type checks (T5, S2.6).

use rflap_core::grammar::{Classification, Grammar, GrammarError, Production};
use std::collections::BTreeSet;

fn g(text: &str) -> Grammar {
    Grammar::parse(text).unwrap()
}

fn show(g: &Grammar) -> String {
    g.productions()
        .iter()
        .map(|p| format!("{}->{}", p.lhs, p.rhs))
        .collect::<Vec<_>>()
        .join(" ; ")
}

fn chars(s: &str) -> BTreeSet<char> {
    s.chars().collect()
}

#[test]
fn parses_alternatives_and_lambda() {
    let gr = g("S -> aSb | λ");
    assert_eq!(show(&gr), "S->aSb ; S->");
    assert_eq!(gr.start(), "S");
    assert_eq!(gr.variables(), chars("S"));
    assert_eq!(gr.terminals(), chars("ab"));
}

#[test]
fn start_is_lhs_of_first_production_and_symbols_follow_case() {
    let gr = g("A -> bB\nB -> c | ε\n");
    assert_eq!(gr.start(), "A");
    assert_eq!(gr.variables(), chars("AB"));
    assert_eq!(gr.terminals(), chars("bc"));
    assert_eq!(show(&gr), "A->bB ; B->c ; B->");
}

#[test]
fn accepts_arrow_variants_and_ignores_blanks() {
    let a = g("S → a S b | ");
    assert_eq!(show(&a), "S->aSb ; S->");
    let b = g("\n  S -> ab\n\n");
    assert_eq!(show(&b), "S->ab");
}

#[test]
fn duplicate_productions_are_stored_once() {
    let gr = g("S -> a | a\nS -> a");
    assert_eq!(show(&gr), "S->a");
}

#[test]
fn from_pairs_builds_the_same_grammar_as_text() {
    let gr = Grammar::from_pairs(&[("S", "aA"), ("A", "b"), ("A", "")]).unwrap();
    assert_eq!(gr, g("S -> aA\nA -> b | λ"));
}

#[test]
fn rejects_bad_lines_and_first_unrestricted_production() {
    assert!(matches!(
        Grammar::parse("S a"),
        Err(GrammarError::Syntax { line: 1, .. })
    ));
    assert!(matches!(
        Grammar::parse("S -> a\n -> b"),
        Err(GrammarError::EmptyLhs { line: 2 })
    ));
    assert!(matches!(
        Grammar::parse("aS -> b"),
        Err(GrammarError::FirstProductionUnrestricted)
    ));
    assert!(Grammar::from_pairs(&[("S", "a"), ("", "b")]).is_err());
}

#[test]
fn empty_text_is_the_empty_grammar() {
    let gr = g("");
    assert!(gr.productions().is_empty());
    assert_eq!(gr.start(), "");
}

#[test]
fn text_round_trips_and_serde_round_trips() {
    let gr = g("S -> aSb | λ\nT -> a");
    assert_eq!(g(&gr.to_text()), gr);
    let json = serde_json::to_string(&gr).unwrap();
    let back: Grammar = serde_json::from_str(&json).unwrap();
    assert_eq!(back, gr);
    // deserialization keeps the invariants (no empty lhs)
    assert!(
        serde_json::from_str::<Grammar>(r#"{"start":"S","productions":[{"lhs":"","rhs":"a"}]}"#)
            .is_err()
    );
}

#[test]
fn production_predicates_follow_jflap_production_checker() {
    let p = |l: &str, r: &str| Production::new(l, r);
    assert!(p("S", "aA").is_right_linear());
    assert!(p("S", "abA").is_right_linear());
    assert!(!p("S", "Aa").is_right_linear());
    assert!(p("S", "Aa").is_left_linear());
    assert!(p("S", "ab").is_right_linear() && p("S", "ab").is_left_linear());
    assert!(p("S", "").is_right_linear() && p("S", "").is_left_linear());
    assert!(!p("S", "aAb").is_linear());
    assert!(!p("S", "AB").is_linear());
    assert!(p("S", "A").is_unit());
    assert!(!p("S", "a").is_unit());
    assert!(!p("S", "AB").is_unit());
    assert!(p("S", "").is_lambda());
    assert!(!p("aS", "").is_lambda());
    assert!(!p("aS", "a").is_restricted_on_lhs());
}

#[test]
fn context_free_but_not_regular() {
    let r = g("S -> aSb | λ").type_report();
    assert!(r.context_free);
    assert!(!r.regular && !r.right_linear && !r.left_linear);
    assert_eq!(r.classification, Classification::ContextFree);
    assert_eq!(r.classification.message(), "This is a Context-Free Grammar");
}

#[test]
fn right_linear_grammar_is_regular_and_context_free() {
    let r = g("S -> aA\nA -> b | λ").type_report();
    assert!(r.right_linear && !r.left_linear && r.regular && r.context_free);
    assert_eq!(r.classification, Classification::RightLinear);
    assert_eq!(
        r.classification.message(),
        "This is a right-linear Grammar (Regular Grammar and Context-Free Grammar)"
    );
}

#[test]
fn left_linear_grammar() {
    let r = g("S -> Ab\nA -> Aa | λ").type_report();
    assert!(r.left_linear && !r.right_linear && r.regular && r.context_free);
    assert_eq!(r.classification, Classification::LeftLinear);
}

#[test]
fn chomsky_and_greibach_normal_forms() {
    let cnf = g("S -> AB\nA -> BB | a\nB -> b").type_report();
    assert!(cnf.cnf && cnf.context_free && !cnf.regular);
    assert_eq!(cnf.classification, Classification::Cnf);
    assert_eq!(
        cnf.classification.message(),
        "This is a CNF Grammar (also Context-Free Grammar)"
    );

    let gnf = g("S -> aSB | b\nB -> b").type_report();
    assert!(gnf.gnf && !gnf.cnf);
    assert_eq!(gnf.classification, Classification::Gnf);
}

#[test]
fn context_sensitive_unrestricted_and_tm_converted() {
    let cs = g("S -> AB\nAB -> ab").type_report();
    assert!(!cs.context_free && cs.context_sensitive);
    assert_eq!(cs.classification, Classification::ContextSensitive);

    let un = g("S -> aA\naA -> b").type_report();
    assert!(!un.context_free && !un.context_sensitive);
    assert_eq!(un.classification, Classification::Unrestricted);
    assert_eq!(
        un.classification.message(),
        "This is an Unrestricted Grammar"
    );

    let tm = g("S -> V(==)S\nS -> SV(==)\nS -> T\naT -> b").type_report();
    assert_eq!(tm.classification, Classification::TuringConverted);
    assert!(tm.turing_converted);
}

#[test]
fn production_lookup_helpers() {
    let gr = g("S -> aSb | A\nA -> a");
    assert_eq!(gr.productions_for("S").count(), 2);
    assert!(gr.contains(&Production::new("A", "a")));
    assert!(gr.is_variable('A') && gr.is_terminal('a') && !gr.is_terminal('A'));
}
