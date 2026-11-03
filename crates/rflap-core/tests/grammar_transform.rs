//! λ / unit / useless removal and Chomsky normal form (T5, S2.6).

use rflap_core::grammar::{
    CnfKind, CnfStep, Grammar, LambdaStep, Production, TransformError, UnitStep, UselessStep,
    chomsky_pipeline, remove_lambda_productions, remove_unit_productions,
    remove_useless_productions, to_chomsky_normal_form,
};
use std::collections::BTreeSet;

fn g(text: &str) -> Grammar {
    Grammar::parse(text).unwrap()
}

fn show(g: &Grammar) -> String {
    show_all(g.productions())
}

fn show_all(ps: &[Production]) -> String {
    ps.iter()
        .map(|p| format!("{}->{}", p.lhs, p.rhs))
        .collect::<Vec<_>>()
        .join(" ; ")
}

fn p(l: &str, r: &str) -> Production {
    Production::new(l, r)
}

fn chars(s: &str) -> BTreeSet<char> {
    s.chars().collect()
}

// ---- λ-production removal ------------------------------------------------

const LAMBDA_TEXTBOOK: &str = "S -> ABC\nA -> aA | λ\nB -> bB | λ\nC -> c | λ";

#[test]
fn lambda_removal_exact_production_set_on_textbook_grammar() {
    let r = remove_lambda_productions(&g(LAMBDA_TEXTBOOK)).unwrap();
    assert_eq!(r.lambda_set, chars("ABCS"));
    assert!(r.start_derives_lambda);
    assert_eq!(
        show(&r.grammar),
        "S->A ; S->AB ; S->ABC ; S->AC ; S->B ; S->BC ; S->C ; A->a ; A->aA ; B->b ; B->bB ; C->c"
    );
    assert_eq!(r.grammar.start(), "S");
}

#[test]
fn lambda_removal_steps_replay_in_jflap_order() {
    let r = remove_lambda_productions(&g(LAMBDA_TEXTBOOK)).unwrap();
    // step 1: the derives-λ set, one variable at a time with the production that proves it
    assert_eq!(
        &r.steps[..4],
        &[
            LambdaStep::DerivesLambda {
                variable: 'A',
                production: p("A", "")
            },
            LambdaStep::DerivesLambda {
                variable: 'B',
                production: p("B", "")
            },
            LambdaStep::DerivesLambda {
                variable: 'C',
                production: p("C", "")
            },
            LambdaStep::DerivesLambda {
                variable: 'S',
                production: p("S", "ABC")
            },
        ]
    );
    // step 2: λ-productions are deleted, then every other one is expanded
    assert_eq!(
        &r.steps[4..7],
        &[
            LambdaStep::RemoveLambda {
                production: p("A", "")
            },
            LambdaStep::RemoveLambda {
                production: p("B", "")
            },
            LambdaStep::RemoveLambda {
                production: p("C", "")
            },
        ]
    );
    assert_eq!(
        r.steps[7],
        LambdaStep::Expand {
            production: p("S", "ABC"),
            added: ["A", "AB", "ABC", "AC", "B", "BC", "C"]
                .iter()
                .map(|s| p("S", s))
                .collect(),
        }
    );
    assert_eq!(r.steps.len(), 4 + 3 + 4);
}

#[test]
fn lambda_removal_drops_lambda_from_the_language_when_start_is_nullable() {
    let r = remove_lambda_productions(&g("S -> aSb | λ")).unwrap();
    assert_eq!(show(&r.grammar), "S->aSb ; S->ab");
    assert!(r.start_derives_lambda);
}

#[test]
fn lambda_removal_with_no_lambda_changes_nothing() {
    let gr = g("S -> aS | b");
    let r = remove_lambda_productions(&gr).unwrap();
    assert!(r.lambda_set.is_empty());
    assert!(!r.start_derives_lambda);
    assert_eq!(r.grammar, gr);
}

#[test]
fn transformations_reject_unrestricted_grammars() {
    let un = g("S -> AB\nAB -> ab");
    assert!(matches!(
        remove_lambda_productions(&un),
        Err(TransformError::NotContextFree(_))
    ));
    assert!(remove_unit_productions(&un).is_err());
    assert!(remove_useless_productions(&un).is_err());
    assert!(to_chomsky_normal_form(&un).is_err());
}

// ---- unit-production removal ----------------------------------------------

const UNIT_CYCLE: &str = "S -> A | bb\nA -> B | a\nB -> C | c\nC -> A";

#[test]
fn unit_removal_exact_production_set_with_a_cycle() {
    let r = remove_unit_productions(&g(UNIT_CYCLE)).unwrap();
    assert_eq!(
        show(&r.grammar),
        "S->bb ; A->a ; B->c ; A->c ; B->a ; S->a ; S->c ; C->a ; C->c"
    );
    assert!(!r.grammar.productions().iter().any(|p| p.is_unit()));
}

#[test]
fn unit_removal_exposes_graph_and_unit_pairs() {
    let r = remove_unit_productions(&g(UNIT_CYCLE)).unwrap();
    assert_eq!(
        r.edges,
        vec![('S', 'A'), ('A', 'B'), ('B', 'C'), ('C', 'A')]
    );
    assert_eq!(
        r.unit_pairs,
        vec![
            ('A', 'B'),
            ('A', 'C'),
            ('B', 'A'),
            ('B', 'C'),
            ('S', 'A'),
            ('S', 'B'),
            ('S', 'C'),
            ('C', 'A'),
            ('C', 'B'),
        ]
    );
}

#[test]
fn unit_removal_steps_replay() {
    let r = remove_unit_productions(&g(UNIT_CYCLE)).unwrap();
    assert_eq!(
        &r.steps[..4],
        &[
            UnitStep::Edge {
                from: 'S',
                to: 'A',
                production: p("S", "A")
            },
            UnitStep::Edge {
                from: 'A',
                to: 'B',
                production: p("A", "B")
            },
            UnitStep::Edge {
                from: 'B',
                to: 'C',
                production: p("B", "C")
            },
            UnitStep::Edge {
                from: 'C',
                to: 'A',
                production: p("C", "A")
            },
        ]
    );
    assert!(r.steps.contains(&UnitStep::Reach {
        variable: 'S',
        reachable: vec!['A', 'B', 'C']
    }));
    assert!(r.steps.contains(&UnitStep::RemoveUnit {
        production: p("B", "C")
    }));
    assert!(r.steps.contains(&UnitStep::Add {
        production: p("S", "c"),
        via: 'B'
    }));
}

#[test]
fn unit_removal_without_units_changes_nothing() {
    let gr = g("S -> aSb | ab");
    let r = remove_unit_productions(&gr).unwrap();
    assert_eq!(r.grammar, gr);
    assert!(r.steps.is_empty() && r.unit_pairs.is_empty());
}

// ---- useless-production removal --------------------------------------------

const USELESS: &str = "S -> AB | a\nA -> aA\nB -> b\nC -> c\nD -> E\nE -> e";

#[test]
fn useless_removal_exact_production_set() {
    let r = remove_useless_productions(&g(USELESS)).unwrap();
    assert_eq!(show(&r.grammar), "S->a");
    assert_eq!(r.grammar.start(), "S");
}

#[test]
fn useless_removal_exposes_derives_terminal_and_reachable_sets() {
    let r = remove_useless_productions(&g(USELESS)).unwrap();
    assert_eq!(r.derives_terminals, chars("BSCED"));
    assert_eq!(r.reachable, chars("S"));
    assert!(!r.start_useless);
    let derive_order: Vec<char> = r
        .steps
        .iter()
        .filter_map(|s| match s {
            UselessStep::DerivesTerminals { variable, .. } => Some(*variable),
            _ => None,
        })
        .collect();
    assert_eq!(derive_order, vec!['B', 'S', 'C', 'E', 'D']);
    assert!(r.steps.contains(&UselessStep::RemoveNonDeriving {
        production: p("S", "AB")
    }));
    assert!(r.steps.contains(&UselessStep::RemoveNonDeriving {
        production: p("A", "aA")
    }));
    assert!(r.steps.contains(&UselessStep::Reachable {
        variable: 'S',
        from: None
    }));
    assert!(r.steps.contains(&UselessStep::RemoveUnreachable {
        production: p("D", "E")
    }));
}

#[test]
fn useless_removal_keeps_reachable_chains() {
    let r = remove_useless_productions(&g("S -> AB\nA -> a\nB -> C | b\nC -> c")).unwrap();
    assert_eq!(show(&r.grammar), "S->AB ; A->a ; B->C ; B->b ; C->c");
    assert_eq!(r.reachable, chars("SABC"));
}

#[test]
fn useless_removal_when_start_derives_no_terminal_string_leaves_no_productions() {
    let r = remove_useless_productions(&g("S -> aS\nA -> a")).unwrap();
    assert!(r.start_useless);
    assert!(r.grammar.productions().is_empty());
    assert_eq!(r.grammar.start(), "S");
}

// ---- Chomsky normal form ------------------------------------------------------

#[test]
fn cnf_of_a_to_the_n_b_to_the_n_matches_jflap() {
    let r = to_chomsky_normal_form(&g("S -> aSb | ab")).unwrap();
    assert_eq!(show(&r.grammar), "S->AC ; C->SB ; S->AB ; A->a ; B->b");
    assert_eq!(r.grammar.start(), "S");
    // JFLAP's working names before the final renaming to unused capital letters
    assert_eq!(
        show_all(&r.intermediate),
        "S->B(a)D(1) ; D(1)->SB(b) ; S->B(a)B(b) ; B(a)->a ; B(b)->b"
    );
    assert_eq!(
        r.renames,
        vec![
            ("B(a)".to_string(), 'A'),
            ("B(b)".to_string(), 'B'),
            ("D(1)".to_string(), 'C'),
        ]
    );
}

#[test]
fn cnf_steps_follow_the_chomsky_pane_round_order() {
    let r = to_chomsky_normal_form(&g("S -> aSb | ab")).unwrap();
    // round 1 converts the last production first
    assert_eq!(
        r.steps[0],
        CnfStep {
            round: 1,
            production: p("S", "ab"),
            replacement: vec![p("S", "B(a)B(b)"), p("B(a)", "a"), p("B(b)", "b")],
            kind: CnfKind::Terminals,
        }
    );
    assert_eq!(
        r.steps[1],
        CnfStep {
            round: 1,
            production: p("S", "aSb"),
            replacement: vec![p("S", "B(a)SB(b)")],
            kind: CnfKind::Terminals,
        }
    );
    assert_eq!(
        r.steps[2],
        CnfStep {
            round: 2,
            production: p("S", "B(a)SB(b)"),
            replacement: vec![p("S", "B(a)D(1)"), p("D(1)", "SB(b)")],
            kind: CnfKind::Split,
        }
    );
    assert_eq!(r.steps.len(), 3);
}

#[test]
fn cnf_reuses_a_variable_whose_only_production_is_the_symbol() {
    let r = to_chomsky_normal_form(&g("S -> aSb | ab | ASBc\nA -> a\nB -> bb")).unwrap();
    assert_eq!(
        show_all(&r.intermediate),
        "S->AD(2) ; D(2)->SB(b) ; S->AB(b) ; S->AD(1) ; D(1)->SD(3) ; D(3)->BB(c) ; B(c)->c ; A->a ; B->B(b)B(b) ; B(b)->b"
    );
    assert_eq!(
        show(&r.grammar),
        "S->AF ; F->SC ; S->AC ; S->AE ; E->SG ; G->BD ; D->c ; A->a ; B->CC ; C->b"
    );
}

#[test]
fn cnf_of_a_grammar_already_in_cnf_is_a_no_op() {
    let gr = g("S -> AB\nA -> a\nB -> b");
    let r = to_chomsky_normal_form(&gr).unwrap();
    assert!(r.steps.is_empty());
    assert_eq!(r.grammar, gr);
}

#[test]
fn cnf_refuses_lambda_unit_and_reserved_parentheses() {
    assert!(matches!(
        to_chomsky_normal_form(&g("S -> aS | λ")),
        Err(TransformError::NotApplicable(_))
    ));
    assert!(matches!(
        to_chomsky_normal_form(&g("S -> A\nA -> a")),
        Err(TransformError::NotApplicable(_))
    ));
    assert!(matches!(
        to_chomsky_normal_form(&g("S -> (S) | ab")),
        Err(TransformError::ReservedSymbol('('))
    ));
}

#[test]
fn cnf_runs_out_of_capital_letters_like_jflap() {
    // 26 variables already in use: a new working variable has no letter left
    let mut lines = vec!["A -> aAb | ab".to_string()];
    for c in 'B'..='Z' {
        lines.push(format!("{c} -> a"));
    }
    let err = to_chomsky_normal_form(&g(&lines.join("\n"))).unwrap_err();
    assert!(matches!(err, TransformError::TooManyVariables { .. }));
}

// ---- λ → unit → useless → CNF pipeline (JFLAP "Transform Grammar") -------------

#[test]
fn pipeline_matches_the_transform_grammar_action() {
    let r = chomsky_pipeline(&g("S -> aSb | λ")).unwrap();
    assert!(r.lambda.as_ref().unwrap().start_derives_lambda);
    assert!(r.unit.is_none() && r.useless.is_none());
    assert_eq!(show(&r.grammar), "S->AC ; C->SB ; S->AB ; A->a ; B->b");
    assert_eq!(r.grammar, r.cnf.as_ref().unwrap().grammar);
}

#[test]
fn pipeline_runs_every_stage_that_applies() {
    let r = chomsky_pipeline(&g("S -> A | aSb | λ\nA -> a | D\nD -> d\nE -> e")).unwrap();
    assert!(r.lambda.is_some() && r.unit.is_some() && r.useless.is_some() && r.cnf.is_some());
    // λ removed, units removed, E->e useless, then CNF
    let cnf_input = r.cnf.as_ref().unwrap().input.clone();
    assert!(
        !cnf_input
            .productions()
            .iter()
            .any(|p| p.rhs.is_empty() || p.is_unit())
    );
    assert!(!cnf_input.productions().iter().any(|p| p.lhs == "E"));
    // every final production is in Chomsky form
    for pr in r.grammar.productions() {
        let n = pr.rhs.chars().count();
        assert!(
            (n == 1 && !pr.rhs.chars().next().unwrap().is_uppercase())
                || (n == 2 && pr.rhs.chars().all(|c| c.is_uppercase())),
            "{pr} is not in CNF"
        );
    }
}

#[test]
fn pipeline_leaves_a_clean_cnf_grammar_alone() {
    let gr = g("S -> AB\nA -> a\nB -> b");
    let r = chomsky_pipeline(&gr).unwrap();
    assert!(r.lambda.is_none() && r.unit.is_none() && r.useless.is_none() && r.cnf.is_none());
    assert_eq!(r.grammar, gr);
}

#[test]
fn steps_are_serde_serializable() {
    let r = remove_lambda_productions(&g(LAMBDA_TEXTBOOK)).unwrap();
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("DerivesLambda") || json.contains("derives_lambda"));
    let c = to_chomsky_normal_form(&g("S -> aSb | ab")).unwrap();
    assert!(serde_json::to_string(&c).is_ok());
    assert!(serde_json::to_string(&remove_unit_productions(&g(UNIT_CYCLE)).unwrap()).is_ok());
    assert!(serde_json::to_string(&remove_useless_productions(&g(USELESS)).unwrap()).is_ok());
}
