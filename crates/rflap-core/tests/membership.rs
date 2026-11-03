//! Membership by the sets Tk (CLASE 41): T0 = {S}, Tk = Tk-1 ∪ {β : |β| ≤ |w|, α ⇒ β}.

use rflap_core::grammar::{Grammar, MembershipError, membership_tk, membership_tk_limited};

fn course_grammar() -> Grammar {
    Grammar::parse("S -> T\nDB -> BD\nD -> c\nT -> aTBD\nT -> abD\nbB -> bb").unwrap()
}

fn forms(set: &[&str]) -> Vec<String> {
    set.iter().map(|s| s.to_string()).collect()
}

#[test]
fn abbc_is_not_in_the_course_language() {
    let r = membership_tk(&course_grammar(), "abbc").unwrap();
    assert_eq!(r.bound, 4);
    assert!(!r.member);
    assert_eq!(
        r.sets,
        vec![
            forms(&["S"]),
            forms(&["S", "T"]),
            forms(&["S", "T", "aTBD", "abD"]),
            forms(&["S", "T", "aTBD", "abD", "aTBc", "abc"]),
            forms(&["S", "T", "aTBD", "abD", "aTBc", "abc"]),
        ]
    );
    assert_eq!(
        r.to_text("abbc"),
        "T0 = { S }\nT1 = { S, T }\nT2 = { S, T, aTBD, abD }\nT3 = { S, T, aTBD, abD, aTBc, abc }\nT4 = T3\nComo T4 = T3, \"abbc\" no pertenece al lenguaje.\n"
    );
}

#[test]
fn abc_is_accepted_as_soon_as_it_enters_a_set() {
    let r = membership_tk(&course_grammar(), "abc").unwrap();
    assert!(r.member);
    assert_eq!(r.bound, 3);
    assert_eq!(
        r.sets,
        vec![
            forms(&["S"]),
            forms(&["S", "T"]),
            forms(&["S", "T", "abD"]),
            forms(&["S", "T", "abD", "abc"]),
        ]
    );
    assert!(
        r.to_text("abc")
            .ends_with("\"abc\" ∈ T3: la palabra pertenece al lenguaje.\n")
    );
}

#[test]
fn the_empty_word_is_a_member_when_s_derives_lambda() {
    let g = Grammar::parse("S -> aSb | λ").unwrap();
    let r = membership_tk(&g, "").unwrap();
    assert!(r.member);
    assert_eq!(r.sets, vec![forms(&["S"]), forms(&["S", "λ"])]);
}

#[test]
fn contracting_grammars_are_refused() {
    let g = Grammar::parse("S -> aAb\naA -> a\nA -> c").unwrap();
    let e = membership_tk(&g, "ab").unwrap_err();
    assert_eq!(
        e,
        MembershipError::Contracting {
            production: "aA → a".into()
        }
    );
    assert_eq!(
        e.to_string(),
        "La gramática es compresora (la producción aA → a tiene |α| > |β|): la comprobación por los conjuntos Tk solo vale para gramáticas no compresoras."
    );
}

#[test]
fn a_grammar_without_productions_has_no_start_symbol() {
    assert_eq!(
        membership_tk(&Grammar::empty(), "a").unwrap_err(),
        MembershipError::NoStart
    );
}

#[test]
fn a_form_budget_protects_the_caller() {
    let g = Grammar::parse("S -> SS | a | b").unwrap();
    let e = membership_tk_limited(&g, "abababababab", 50).unwrap_err();
    assert_eq!(e, MembershipError::LimitExceeded { limit: 50 });
}
