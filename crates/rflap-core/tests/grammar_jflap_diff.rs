//! Differential check against `fixtures/t5/expected.txt`, produced by the real JFLAP 7.1 jar
//! (`fixtures/t5/Harness.java`). Each section rebuilds the harness input and compares the output.

use rflap_core::grammar::{
    Grammar, Production, brute_parse, cyk_parse, first_sets, follow_sets, ll1_table,
    remove_lambda_productions, remove_unit_productions, remove_useless_productions, slr_table,
    to_chomsky_normal_form,
};
use std::collections::BTreeSet;

fn expected() -> Vec<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/t5/expected.txt"
    );
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| l.trim_end().to_string())
        .collect()
}

fn lines_with(prefix: &str) -> Vec<String> {
    expected()
        .into_iter()
        .filter_map(|l| l.strip_prefix(prefix).map(|r| r.trim_start().to_string()))
        .collect()
}

fn one(prefix: &str) -> String {
    let v = lines_with(prefix);
    assert_eq!(v.len(), 1, "{prefix}");
    v.into_iter().next().unwrap()
}

/// The harness builds `ContextFreeGrammar`s from `lhs->rhs` strings.
fn cfg(lines: &[&str]) -> Grammar {
    let pairs: Vec<(&str, &str)> = lines.iter().map(|l| l.split_once("->").unwrap()).collect();
    Grammar::from_pairs(&pairs).unwrap()
}

fn show(ps: &[Production]) -> String {
    let mut s: String = ps
        .iter()
        .map(|p| format!("{}->{} ; ", p.lhs, p.rhs))
        .collect();
    s.truncate(s.trim_end().len());
    s
}

fn sorted(set: &BTreeSet<String>) -> String {
    let items: Vec<String> = set
        .iter()
        .map(|s| {
            if s.is_empty() {
                "!".to_string()
            } else {
                s.clone()
            }
        })
        .collect();
    format!("{{{}}}", items.join(" "))
}

#[test]
fn lambda_unit_useless_match_jflap() {
    let l = cfg(&["S->ABC", "A->aA", "A->", "B->bB", "B->", "C->c", "C->"]);
    let r = remove_lambda_productions(&l).unwrap();
    let set: BTreeSet<String> = r.lambda_set.iter().map(|c| c.to_string()).collect();
    assert_eq!(sorted(&set), one("LAMBDA set"));
    assert_eq!(show(r.grammar.productions()), one("LAMBDA out"));

    let u = cfg(&["S->A", "S->bb", "A->B", "A->a", "B->C", "B->c", "C->A"]);
    assert_eq!(
        show(remove_unit_productions(&u).unwrap().grammar.productions()),
        one("UNIT out")
    );

    let u2 = cfg(&[
        "S->PQ", "S->a", "P->Q", "Q->A", "A->b", "A->P", "Z->Q", "Z->z", "M->Z", "P->p",
    ]);
    assert_eq!(
        show(remove_unit_productions(&u2).unwrap().grammar.productions()),
        one("UNIT2 out")
    );
    let l2 = cfg(&[
        "S->PQZ", "P->", "Q->QP", "Q->q", "Z->ZM", "Z->", "M->m", "M->",
    ]);
    let r2 = remove_lambda_productions(&l2).unwrap();
    let set2: BTreeSet<String> = r2.lambda_set.iter().map(|c| c.to_string()).collect();
    assert_eq!(sorted(&set2), one("LAMBDA2 set"));
    assert_eq!(show(r2.grammar.productions()), one("LAMBDA2 out"));

    let x = cfg(&["S->AB", "S->a", "A->aA", "B->b", "C->c", "D->E", "E->e"]);
    assert_eq!(
        show(
            remove_useless_productions(&x)
                .unwrap()
                .grammar
                .productions()
        ),
        one("USELESS out")
    );
}

#[test]
fn cnf_matches_jflap() {
    let pre = lines_with("CNF pre");
    let out = lines_with("CNF out");
    let inputs: [&[&str]; 2] = [
        &["S->aSb", "S->ab"],
        &["S->aSb", "S->ab", "S->ASBc", "A->a", "B->bb"],
    ];
    for (i, lines) in inputs.iter().enumerate() {
        let r = to_chomsky_normal_form(&cfg(lines)).unwrap();
        assert_eq!(show(&r.intermediate), pre[i], "pre {i}");
        assert_eq!(show(r.grammar.productions()), out[i], "out {i}");
    }
}

#[test]
fn first_follow_and_ll_table_match_jflap() {
    let e = cfg(&[
        "E->TX", "X->+TX", "X->", "T->FY", "Y->*FY", "Y->", "F->(E)", "F->a",
    ]);
    let first = first_sets(&e);
    let follow = follow_sets(&e);
    for v in ["E", "X", "T", "Y", "F"] {
        let want = one(&format!("FIRST {v}"));
        let got = format!("{} FOLLOW {}", sorted(&first[v]), sorted(&follow[v]));
        assert_eq!(got, want, "{v}");
    }
    let table = ll1_table(&e);
    let mut got = Vec::new();
    for v in ["E", "X", "T", "Y", "F"] {
        for c in ["(", ")", "*", "+", "a", "$"] {
            let cell = table.get(v, c).unwrap();
            if !cell.is_empty() {
                got.push(format!("{v} {c} {}", sorted(cell)));
            }
        }
    }
    assert_eq!(got, lines_with("LL "));
    assert!(expected().contains(&"LL1 true".to_string()));
    assert!(table.is_ll1());
    let rec = cfg(&["E->E+T", "E->T", "T->a"]);
    assert_eq!(one("LL1 left-recursive"), "false");
    assert!(!ll1_table(&rec).is_ll1());
}

#[test]
fn slr_table_matches_jflap() {
    let s = cfg(&["E->E+T", "E->T", "T->T*F", "T->F", "F->(E)", "F->a"]);
    let t = slr_table(&s);
    assert_eq!(t.automaton.states.len().to_string(), one("SLR states"));
    let want = lines_with("SLR ")
        .into_iter()
        .filter(|l| !l.starts_with("states"))
        .collect::<Vec<_>>();
    let mut got = Vec::new();
    for (i, _) in t.automaton.states.iter().enumerate() {
        let mut row = format!("{i}:");
        for sym in t.terminals.iter().chain(t.variables.iter()) {
            let cell = t.cell_string(i, sym);
            if !cell.is_empty() {
                row.push_str(&format!(" {sym}={}", cell.replace(' ', "/")));
            }
        }
        got.push(row);
    }
    assert_eq!(got, want);
}

#[test]
fn brute_force_and_cyk_match_jflap() {
    let ab = cfg(&["S->aSb", "S->"]);
    for line in lines_with("BRUTE ") {
        let (input, rest) = line.split_once("' -> ").unwrap();
        let input = input.trim_start_matches('\'');
        let (verdict, chain) = rest.split_once(" | ").map_or((rest, ""), |(a, b)| (a, b));
        let r = brute_parse(&ab, input, 100_000).unwrap();
        assert_eq!(
            r.outcome == rflap_core::grammar::ParseOutcome::Accepted,
            verdict.trim() == "true"
        );
        if verdict.trim() == "true" {
            let mut forms: Vec<String> = chain
                .split(" | ")
                .map(|n| {
                    n.split(" <- ")
                        .next()
                        .unwrap()
                        .trim_end_matches(" <-")
                        .to_string()
                })
                .collect();
            forms.reverse();
            assert_eq!(r.derivation.unwrap().sentential_forms(), forms, "{input:?}");
        }
    }
    let cyk = cfg(&[
        "S->AB", "S->BC", "A->BA", "A->a", "B->CC", "B->b", "C->AB", "C->a",
    ]);
    for line in lines_with("CYK ") {
        let (input, verdict) = line.split_once(' ').unwrap();
        let r = cyk_parse(&cyk, input).unwrap();
        assert_eq!(
            r.outcome == rflap_core::grammar::ParseOutcome::Accepted,
            verdict == "true",
            "{input}"
        );
    }
}
