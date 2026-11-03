//! FIRST/FOLLOW, LL(1), SLR(1), CYK, brute force and user-controlled parsing (T5, S2.6).

use rflap_core::grammar::{
    BruteKind, Grammar, LlAction, LlStep, LrAction, LrStepAction, MultiOutcome, ParseError,
    ParseMethod, ParseOutcome, Production, UserParseError, UserParser, brute_parse, cyk_parse,
    first_sets, follow_sets, ll1_parse, ll1_table, parse_many, slr_parse, slr_table,
};
use std::collections::BTreeSet;

fn g(text: &str) -> Grammar {
    Grammar::parse(text).unwrap()
}

fn p(l: &str, r: &str) -> Production {
    Production::new(l, r)
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

const LL_EXPR: &str = "E -> TX\nX -> +TX | λ\nT -> FY\nY -> *FY | λ\nF -> (E) | a";
const LR_EXPR: &str = "E -> E+T | T\nT -> T*F | F\nF -> (E) | a";

// ---- FIRST / FOLLOW ---------------------------------------------------------------

#[test]
fn first_and_follow_of_the_ll1_expression_grammar() {
    let gr = g(LL_EXPR);
    let first = first_sets(&gr);
    let follow = follow_sets(&gr);
    // "" is JFLAP's λ marker inside FIRST sets, "$" the end marker in FOLLOW sets
    assert_eq!(first["E"], set(&["(", "a"]));
    assert_eq!(first["X"], set(&["", "+"]));
    assert_eq!(first["T"], set(&["(", "a"]));
    assert_eq!(first["Y"], set(&["", "*"]));
    assert_eq!(first["F"], set(&["(", "a"]));
    assert_eq!(first["a"], set(&["a"]));
    assert_eq!(follow["E"], set(&["$", ")"]));
    assert_eq!(follow["X"], set(&["$", ")"]));
    assert_eq!(follow["T"], set(&["$", ")", "+"]));
    assert_eq!(follow["Y"], set(&["$", ")", "+"]));
    assert_eq!(follow["F"], set(&["$", ")", "*", "+"]));
}

#[test]
fn first_and_follow_of_the_classic_left_recursive_expression_grammar() {
    let gr = g(LR_EXPR);
    let first = first_sets(&gr);
    let follow = follow_sets(&gr);
    for v in ["E", "T", "F"] {
        assert_eq!(first[v], set(&["(", "a"]), "FIRST({v})");
    }
    assert_eq!(follow["E"], set(&["$", ")", "+"]));
    assert_eq!(follow["T"], set(&["$", ")", "*", "+"]));
    assert_eq!(follow["F"], set(&["$", ")", "*", "+"]));
}

// ---- LL(1) -------------------------------------------------------------------------

#[test]
fn ll1_table_of_the_expression_grammar() {
    let t = ll1_table(&g(LL_EXPR));
    assert!(t.is_ll1() && t.conflicts().is_empty());
    assert_eq!(t.get("E", "(").unwrap(), &set(&["TX"]));
    assert_eq!(t.get("E", "a").unwrap(), &set(&["TX"]));
    assert_eq!(t.get("X", "+").unwrap(), &set(&["+TX"]));
    assert_eq!(t.get("X", ")").unwrap(), &set(&[""]));
    assert_eq!(t.get("X", "$").unwrap(), &set(&[""]));
    assert_eq!(t.get("Y", "*").unwrap(), &set(&["*FY"]));
    assert_eq!(t.get("Y", "+").unwrap(), &set(&[""]));
    assert_eq!(t.get("F", "(").unwrap(), &set(&["(E)"]));
    assert_eq!(t.get("F", "a").unwrap(), &set(&["a"]));
    assert!(t.get("E", "+").unwrap().is_empty());
    assert!(t.get("Q", "+").is_none() && t.get("E", "z").is_none());
}

#[test]
fn ll1_table_reports_conflicts_for_left_recursion() {
    let t = ll1_table(&g("E -> E+T | T\nT -> a"));
    assert!(!t.is_ll1());
    let c = t.conflicts();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].variable, "E");
    assert_eq!(c[0].terminal, "a");
    assert_eq!(c[0].entries, vec!["E+T".to_string(), "T".to_string()]);
    assert!(matches!(
        ll1_parse(&g("E -> E+T | T\nT -> a"), "a"),
        Err(ParseError::NotLl1 { .. })
    ));
}

#[test]
fn ll1_parse_trace_derivation_and_tree() {
    let r = ll1_parse(&g(LL_EXPR), "a+a*a").unwrap();
    assert_eq!(r.outcome, ParseOutcome::Accepted);
    assert_eq!(
        r.derivation.sentential_forms(),
        [
            "E", "TX", "FYX", "aYX", "aX", "a+TX", "a+FYX", "a+aYX", "a+a*FYX", "a+a*aYX",
            "a+a*aX", "a+a*a"
        ]
    );
    assert!(r.derivation.is_valid_for(&g(LL_EXPR)));
    assert_eq!(r.steps.len(), 17);
    assert_eq!(
        r.steps[0],
        LlStep {
            stack: "E".into(),
            input: "a+a*a$".into(),
            action: LlAction::Replace {
                production: p("E", "TX")
            },
        }
    );
    assert_eq!(
        r.steps[3],
        LlStep {
            stack: "aYX".into(),
            input: "a+a*a$".into(),
            action: LlAction::Match { symbol: "a".into() },
        }
    );
    assert_eq!(
        r.steps[4],
        LlStep {
            stack: "YX".into(),
            input: "+a*a$".into(),
            action: LlAction::Replace {
                production: p("Y", "")
            },
        }
    );
    let last = r.steps.last().unwrap();
    assert_eq!(last.action, LlAction::Accept);
    assert_eq!((last.stack.as_str(), last.input.as_str()), ("", "$"));
    let tree = r.tree.unwrap();
    assert_eq!(tree.symbol, "E");
    assert_eq!(tree.yield_string(), "a+a*a");
}

#[test]
fn ll1_parse_rejects_with_a_reason() {
    for input in ["a+", "aa", "(a", "+a"] {
        let r = ll1_parse(&g(LL_EXPR), input).unwrap();
        assert_eq!(r.outcome, ParseOutcome::Rejected, "{input}");
        assert!(matches!(
            r.steps.last().unwrap().action,
            LlAction::Reject { .. }
        ));
    }
    let r = ll1_parse(&g(LL_EXPR), "a+").unwrap();
    assert_eq!(r.derivation.sentential_forms().last().unwrap(), "a+TX");
    match &r.steps.last().unwrap().action {
        LlAction::Reject { reason } => assert_eq!(reason, "No rule for T with $ as lookahead."),
        a => panic!("{a:?}"),
    }
}

#[test]
fn ll1_parse_of_the_empty_string_and_unknown_symbols() {
    let gr = g("S -> aS | λ");
    assert_eq!(ll1_parse(&gr, "").unwrap().outcome, ParseOutcome::Accepted);
    assert_eq!(
        ll1_parse(&gr, "aaa").unwrap().outcome,
        ParseOutcome::Accepted
    );
    assert_eq!(ll1_parse(&gr, "b").unwrap().outcome, ParseOutcome::Rejected);
}

// ---- SLR(1) -----------------------------------------------------------------------

#[test]
fn slr_table_of_the_classic_expression_grammar_matches_jflap() {
    let t = slr_table(&g(LR_EXPR));
    assert!(t.conflicts().is_empty());
    assert_eq!(t.automaton.states.len(), 12);
    assert_eq!(t.productions[0], p("E'", "E"));
    assert_eq!(t.productions[6], p("F", "a"));
    // state 0 closure: E'->·E and its six closure items
    assert_eq!(t.automaton.states[0].items.len(), 7);
    assert_eq!(
        t.automaton.item_text(&t.automaton.states[0].items[0]),
        "E'→·E"
    );
    for (state, symbol, cell) in [
        (0, "(", "s1"),
        (0, "a", "s5"),
        (0, "E", "2"),
        (0, "T", "4"),
        (0, "F", "3"),
        (2, "+", "s7"),
        (2, "$", "acc"),
        (3, "*", "r4"),
        (4, "*", "s8"),
        (4, "+", "r2"),
        (5, ")", "r6"),
        (6, ")", "s9"),
        (7, "T", "10"),
        (9, "$", "r5"),
        (11, "$", "r3"),
        (0, "+", ""),
        (2, "*", ""),
    ] {
        assert_eq!(
            t.cell_string(state, symbol),
            cell,
            "state {state} on {symbol}"
        );
    }
}

#[test]
fn slr_parse_of_a_plus_a_times_a_gives_the_exact_rightmost_derivation() {
    let gr = g(LR_EXPR);
    let r = slr_parse(&gr, "a+a*a").unwrap();
    assert_eq!(r.outcome, ParseOutcome::Accepted);
    let d = r.derivation.as_ref().unwrap();
    assert_eq!(
        d.sentential_forms(),
        [
            "E", "E+T", "E+T*F", "E+T*a", "E+F*a", "E+a*a", "T+a*a", "F+a*a", "a+a*a"
        ]
    );
    let steps: Vec<(String, usize)> = d
        .steps
        .iter()
        .map(|s| (format!("{}", s.production), s.position))
        .collect();
    assert_eq!(
        steps,
        [
            ("E → E+T", 0),
            ("T → T*F", 2),
            ("F → a", 4),
            ("T → F", 2),
            ("F → a", 2),
            ("E → T", 0),
            ("T → F", 0),
            ("F → a", 0),
        ]
        .map(|(a, b)| (a.to_string(), b))
    );
    assert!(d.is_valid_for(&gr));
    // trace: 5 shifts, 8 reductions, accept
    assert_eq!(r.steps.len(), 14);
    assert_eq!(r.steps[0].states, vec![0]);
    assert_eq!(r.steps[0].symbols, "");
    assert_eq!(r.steps[0].input, "a+a*a$");
    assert_eq!(r.steps[0].action, LrStepAction::Shift { state: 5 });
    assert_eq!(r.steps[1].symbols, "a");
    assert_eq!(
        r.steps[1].action,
        LrStepAction::Reduce {
            production_index: 6,
            production: p("F", "a")
        }
    );
    let last = r.steps.last().unwrap();
    assert_eq!(last.action, LrStepAction::Accept);
    assert_eq!(
        (
            last.states.clone(),
            last.symbols.as_str(),
            last.input.as_str()
        ),
        (vec![0, 2], "E", "$")
    );
    let tree = r.tree.unwrap();
    assert_eq!(tree.symbol, "E");
    let kids: Vec<&str> = tree.children.iter().map(|c| c.symbol.as_str()).collect();
    assert_eq!(kids, ["E", "+", "T"]);
    assert_eq!(tree.yield_string(), "a+a*a");
}

#[test]
fn slr_parse_rejects_bad_input() {
    for input in ["a+", "aa", "(a", "", "+a", "a#"] {
        let r = slr_parse(&g(LR_EXPR), input).unwrap();
        assert_eq!(r.outcome, ParseOutcome::Rejected, "{input:?}");
        assert!(r.derivation.is_none());
        assert!(matches!(
            r.steps.last().unwrap().action,
            LrStepAction::Reject { .. }
        ));
    }
    assert_eq!(
        slr_parse(&g(LR_EXPR), "((a)+a)*a").unwrap().outcome,
        ParseOutcome::Accepted
    );
}

#[test]
fn slr_parse_with_lambda_productions() {
    let gr = g("S -> aSb | λ");
    let r = slr_parse(&gr, "aabb").unwrap();
    assert_eq!(r.outcome, ParseOutcome::Accepted);
    assert_eq!(
        r.derivation.unwrap().sentential_forms(),
        ["S", "aSb", "aaSbb", "aabb"]
    );
    assert_eq!(slr_parse(&gr, "").unwrap().outcome, ParseOutcome::Accepted);
    assert_eq!(
        slr_parse(&gr, "aab").unwrap().outcome,
        ParseOutcome::Rejected
    );
}

#[test]
fn slr_table_reports_shift_reduce_conflicts() {
    let gr = g("E -> E+E | a");
    let t = slr_table(&gr);
    let c = t.conflicts();
    assert!(!c.is_empty());
    let plus = c.iter().find(|c| c.symbol == "+").unwrap();
    assert!(plus.actions.iter().any(|a| matches!(a, LrAction::Shift(_))));
    assert!(plus.actions.contains(&LrAction::Reduce(1)));
    assert!(t.cell_string(plus.state, "+").contains(' '));
    assert!(matches!(
        slr_parse(&gr, "a+a"),
        Err(ParseError::NotSlr1 { .. })
    ));
}

// ---- CYK ---------------------------------------------------------------------------

#[test]
fn cyk_table_tree_and_derivation() {
    let gr = g("S -> AB\nA -> a\nB -> b");
    let r = cyk_parse(&gr, "ab").unwrap();
    assert_eq!(r.outcome, ParseOutcome::Accepted);
    assert_eq!(r.table.get(0, 0), &set(&["A"]));
    assert_eq!(r.table.get(1, 1), &set(&["B"]));
    assert_eq!(r.table.get(0, 1), &set(&["S"]));
    assert_eq!(
        r.derivation.unwrap().sentential_forms(),
        ["S", "AB", "aB", "ab"]
    );
    let t = r.tree.unwrap();
    assert_eq!(t.symbol, "S");
    assert_eq!(t.yield_string(), "ab");
    for rej in ["ba", "a", "abab", ""] {
        assert_eq!(
            cyk_parse(&gr, rej).unwrap().outcome,
            ParseOutcome::Rejected,
            "{rej:?}"
        );
    }
    assert_eq!(cyk_parse(&gr, "c").unwrap().outcome, ParseOutcome::Rejected);
}

#[test]
fn cyk_on_the_textbook_grammar_with_ambiguous_cells() {
    let gr = g("S -> AB | BC\nA -> BA | a\nB -> CC | b\nC -> AB | a");
    let r = cyk_parse(&gr, "baaba").unwrap();
    assert_eq!(r.outcome, ParseOutcome::Accepted);
    assert_eq!(r.table.get(0, 4), &set(&["A", "C", "S"]));
    assert_eq!(r.table.get(0, 0), &set(&["B"]));
    assert_eq!(r.table.get(1, 2), &set(&["B"]));
    assert_eq!(r.tree.unwrap().yield_string(), "baaba");
    assert_eq!(
        cyk_parse(&gr, "aaa").unwrap().outcome,
        ParseOutcome::Accepted
    );
    assert_eq!(cyk_parse(&gr, "b").unwrap().outcome, ParseOutcome::Rejected);
}

#[test]
fn cyk_requires_chomsky_normal_form() {
    assert!(matches!(
        cyk_parse(&g("S -> aSb | ab"), "ab"),
        Err(ParseError::NotChomsky { .. })
    ));
}

// ---- brute force -------------------------------------------------------------------

#[test]
fn brute_force_accepts_aabb_and_rejects_aab() {
    let gr = g("S -> aSb | λ");
    let ok = brute_parse(&gr, "aabb", 100_000).unwrap();
    assert_eq!(ok.outcome, ParseOutcome::Accepted);
    assert_eq!(ok.kind, BruteKind::Restricted);
    let d = ok.derivation.as_ref().unwrap();
    assert_eq!(d.sentential_forms(), ["S", "aSb", "aaSbb", "aabb"]);
    assert!(d.is_valid_for(&gr));
    assert!(ok.nodes_considered > 0);
    let tree = ok.tree.unwrap();
    assert_eq!(tree.symbol, "S");
    assert_eq!(tree.yield_string(), "aabb");

    let no = brute_parse(&gr, "aab", 100_000).unwrap();
    assert_eq!(no.outcome, ParseOutcome::Rejected);
    assert!(no.derivation.is_none() && no.tree.is_none());

    let empty = brute_parse(&gr, "", 100_000).unwrap();
    assert_eq!(empty.outcome, ParseOutcome::Accepted);
    assert_eq!(empty.derivation.unwrap().sentential_forms(), ["S", ""]);
}

#[test]
fn brute_force_rejects_unknown_symbols() {
    let gr = g("S -> aSb | λ");
    assert!(matches!(
        brute_parse(&gr, "axb", 1000),
        Err(ParseError::UnknownSymbol { symbol: 'x' })
    ));
    assert!(matches!(
        brute_parse(&gr, "aSb", 1000),
        Err(ParseError::UnknownSymbol { symbol: 'S' })
    ));
}

#[test]
fn brute_force_prunes_with_terminal_runs_and_length() {
    // a b-free target can never use the "ab" production
    let gr = g("S -> aSa | bSb | c");
    assert_eq!(
        brute_parse(&gr, "abcba", 100_000).unwrap().outcome,
        ParseOutcome::Accepted
    );
    assert_eq!(
        brute_parse(&gr, "abcab", 100_000).unwrap().outcome,
        ParseOutcome::Rejected
    );
}

#[test]
fn brute_force_applies_productions_in_parallel_like_jflap() {
    let gr = g("S -> AB\nA -> a\nB -> b");
    let r = brute_parse(&gr, "ab", 10_000).unwrap();
    assert_eq!(
        r.derivation.unwrap().sentential_forms(),
        ["S", "AB", "aB", "ab"]
    );
}

#[test]
fn unrestricted_brute_force_decides_and_gives_up_with_a_budget() {
    let un = g("S -> AB\nA -> a\nB -> b\nAB -> c");
    let r = brute_parse(&un, "c", 10_000).unwrap();
    assert_eq!(r.kind, BruteKind::Unrestricted);
    assert_eq!(r.outcome, ParseOutcome::Accepted);
    assert_eq!(
        r.derivation.as_ref().unwrap().sentential_forms(),
        ["S", "AB", "c"]
    );
    assert!(r.derivation.unwrap().is_valid_for(&un));
    assert!(r.tree.is_none());
    assert_eq!(
        brute_parse(&un, "ab", 10_000).unwrap().outcome,
        ParseOutcome::Accepted
    );

    // c is a terminal of the grammar but unreachable from S, and S -> SS | λ grows forever
    let looping = g("S -> SS | λ | a\naS -> a\nD -> c");
    let r = brute_parse(&looping, "c", 200).unwrap();
    assert_eq!(r.kind, BruteKind::Unrestricted);
    assert_eq!(r.outcome, ParseOutcome::Undecided);
    assert!(r.derivation.is_none());
    assert!(r.candidates_examined >= 200);
}

#[test]
fn restricted_brute_force_also_honours_the_budget() {
    let looping = g("S -> SS | λ | a\nD -> c");
    let r = brute_parse(&looping, "c", 200).unwrap();
    assert_eq!(r.kind, BruteKind::Restricted);
    assert_eq!(r.outcome, ParseOutcome::Undecided);
}

#[test]
fn brute_force_without_a_usable_start_production_rejects() {
    let r = brute_parse(&g("S -> S | a"), "a", 1000).unwrap();
    assert_eq!(r.outcome, ParseOutcome::Accepted);
    let r = brute_parse(&g("S -> SA\nA -> a"), "a", 1000).unwrap();
    assert_eq!(r.outcome, ParseOutcome::Rejected);
}

// ---- user-controlled parsing --------------------------------------------------------

#[test]
fn user_parser_applies_chosen_productions_at_chosen_positions() {
    let gr = g("S -> aSb | λ");
    let mut u = UserParser::new(&gr, "aabb").unwrap();
    assert_eq!(u.sentential(), "S");
    assert!(!u.is_finished());
    assert_eq!(u.positions(0), vec![0]);
    u.apply(0, None).unwrap();
    assert_eq!(u.sentential(), "aSb");
    assert_eq!(u.positions(0), vec![1]);
    u.apply(0, Some(1)).unwrap();
    assert_eq!(u.sentential(), "aaSbb");
    assert!(!u.is_finished());
    u.apply(1, None).unwrap();
    assert_eq!(u.sentential(), "aabb");
    assert!(u.is_finished());
    assert_eq!(
        u.derivation().sentential_forms(),
        ["S", "aSb", "aaSbb", "aabb"]
    );
    assert_eq!(u.tree().unwrap().yield_string(), "aabb");
}

#[test]
fn user_parser_errors_and_undo() {
    let gr = g("S -> aSb | λ\nT -> a");
    let mut u = UserParser::new(&gr, "ab").unwrap();
    assert_eq!(
        u.apply(2, None),
        Err(UserParseError::NotApplicable { production: 2 })
    );
    assert_eq!(
        u.apply(0, Some(3)),
        Err(UserParseError::NoMatchAt {
            production: 0,
            position: 3
        })
    );
    assert_eq!(
        u.apply(9, None),
        Err(UserParseError::NoSuchProduction { production: 9 })
    );
    assert_eq!(u.undo(), Err(UserParseError::NothingToUndo));
    u.apply(0, None).unwrap();
    assert_eq!(u.sentential(), "aSb");
    u.undo().unwrap();
    assert_eq!(u.sentential(), "S");
    assert!(UserParser::new(&gr, "aXb").is_err());
}

#[test]
fn user_parser_prunes_hopeless_forms_like_jflap() {
    let gr = g("S -> aSb | λ");
    let u = UserParser::new(&gr, "ab").unwrap();
    assert!(u.is_possible_derivation("aSb"));
    assert!(!u.is_possible_derivation("bSb"));
    assert!(!u.is_possible_derivation("aaaa"));
}

#[test]
fn user_parser_applies_one_production_at_several_positions() {
    let gr = g("S -> AA\nA -> a\nAA -> b");
    let mut u = UserParser::new(&gr, "aa").unwrap();
    u.apply(0, None).unwrap();
    assert_eq!(u.positions(1), vec![0, 1]);
    u.apply_many(1, &[0, 1]).unwrap();
    assert_eq!(u.sentential(), "aa");
    assert!(u.is_finished());
    assert_eq!(u.derivation().steps.len(), 3);
    assert_eq!(u.derivation().sentential_forms(), ["S", "AA", "aA", "aa"]);
}

// ---- multiple run ---------------------------------------------------------------------

#[test]
fn multiple_run_parses_many_inputs() {
    let gr = g("S -> aSb | λ");
    let inputs = ["aabb", "aab", "", "x", "ab"];
    let brute = parse_many(&gr, &inputs, ParseMethod::Brute { max_nodes: 10_000 });
    let got: Vec<&MultiOutcome> = brute.iter().map(|r| &r.outcome).collect();
    assert!(matches!(
        got.as_slice(),
        [
            MultiOutcome::Accepted,
            MultiOutcome::Rejected,
            MultiOutcome::Accepted,
            MultiOutcome::Error(_),
            MultiOutcome::Accepted
        ]
    ));
    assert_eq!(brute[0].input, "aabb");

    let slr = parse_many(&gr, &["aabb", "aab"], ParseMethod::Slr1);
    assert_eq!(slr[0].outcome, MultiOutcome::Accepted);
    assert_eq!(slr[1].outcome, MultiOutcome::Rejected);

    let ll = parse_many(&gr, &["aabb", "aab"], ParseMethod::Ll1);
    assert_eq!(ll[0].outcome, MultiOutcome::Accepted);
    assert_eq!(ll[1].outcome, MultiOutcome::Rejected);

    let cnf = g("S -> AB\nA -> a\nB -> b");
    let cyk = parse_many(&cnf, &["ab", "ba"], ParseMethod::Cyk);
    assert_eq!(cyk[0].outcome, MultiOutcome::Accepted);
    assert_eq!(cyk[1].outcome, MultiOutcome::Rejected);

    // a grammar the method cannot handle is reported on every row
    let bad = parse_many(&g("S -> aSb | ab"), &["ab"], ParseMethod::Cyk);
    assert!(matches!(bad[0].outcome, MultiOutcome::Error(_)));
}

#[test]
fn parse_results_are_serde_serializable() {
    let gr = g(LR_EXPR);
    assert!(serde_json::to_string(&slr_parse(&gr, "a+a").unwrap()).is_ok());
    assert!(serde_json::to_string(&slr_table(&gr)).is_ok());
    assert!(serde_json::to_string(&ll1_table(&g(LL_EXPR))).is_ok());
    assert!(serde_json::to_string(&ll1_parse(&g(LL_EXPR), "a").unwrap()).is_ok());
    assert!(serde_json::to_string(&brute_parse(&g("S -> a"), "a", 10).unwrap()).is_ok());
    assert!(
        serde_json::to_string(&cyk_parse(&g("S -> AB\nA -> a\nB -> b"), "ab").unwrap()).is_ok()
    );
}
