//! `.jff` read/write for grammars and regular expressions, and the single entry point
//! `read_structure` / `write_structure` (JFLAP 7.1 `TransducerFactory`).

use rflap_core::grammar::{Grammar, Production};
use rflap_core::jff::{
    JffError, Structure, read_grammar, read_regex, read_structure, write_grammar, write_regex,
    write_structure,
};
use rflap_core::regex::Regex;

const GRAMMAR: &str = include_str!("fixtures/jff/jflap/grammar.jff");
const RE: &str = include_str!("fixtures/jff/jflap/re.jff");
const FA: &str = include_str!("fixtures/jff/sample.jff");
const PDA: &str = include_str!("fixtures/jff/jflap/pda.jff");
const TURING: &str = include_str!("fixtures/jff/jflap/turing2.jff");
const MEALY: &str = include_str!("fixtures/jff/jflap/mealy.jff");
const MOORE: &str = include_str!("fixtures/jff/jflap/moore.jff");

fn p(l: &str, r: &str) -> Production {
    Production::new(l, r)
}

// ---------- grammar ----------

#[test]
fn reads_grammar_written_by_jflap() {
    let g = read_grammar(GRAMMAR).unwrap();
    assert_eq!(g.productions(), &[p("S", "aSb"), p("S", ""), p("aS", "Ab")]);
    assert_eq!(g.start(), "S");
}

#[test]
fn grammar_roundtrips() {
    let g = read_grammar(GRAMMAR).unwrap();
    let text = write_grammar(&g);
    assert!(text.contains("<type>grammar</type>"));
    assert!(text.contains("<!--The list of productions.-->"));
    assert!(text.contains("<left>S</left>"));
    assert!(text.contains("<right/>"));
    assert_eq!(read_grammar(&text).unwrap(), g);
}

#[test]
fn grammar_without_productions_roundtrips() {
    let text = write_grammar(&Grammar::empty());
    assert!(!text.contains("<production>"));
    assert_eq!(read_grammar(&text).unwrap(), Grammar::empty());
}

#[test]
fn grammar_rejects_malformed_productions() {
    let doc = |body: &str| format!("<structure><type>grammar</type>{body}</structure>");
    // no left side at all
    assert!(matches!(
        read_grammar(&doc("<production><right>a</right></production>")),
        Err(JffError::BadProduction(_))
    ));
    // empty left side
    assert!(matches!(
        read_grammar(&doc("<production><left/><right>a</right></production>")),
        Err(JffError::BadProduction(_))
    ));
    // the first production must be restricted (a start variable is needed)
    assert!(matches!(
        read_grammar(&doc(
            "<production><left>a</left><right>b</right></production>"
        )),
        Err(JffError::BadProduction(_))
    ));
    // a missing right side is λ
    let g = read_grammar(&doc("<production><left>S</left></production>")).unwrap();
    assert_eq!(g.productions(), &[p("S", "")]);
}

// ---------- regular expression ----------

#[test]
fn reads_regex_written_by_jflap() {
    let re = read_regex(RE).unwrap();
    assert_eq!(re, Regex::parse("(a+b)*abb").unwrap());
    assert_eq!(re.to_jflap_string(), "(a+b)*abb");
}

#[test]
fn regex_roundtrips() {
    let re = read_regex(RE).unwrap();
    let text = write_regex(&re);
    assert!(text.contains("<type>re</type>"));
    assert!(text.contains("<!--The regular expression.-->"));
    assert!(text.contains("<expression>(a+b)*abb</expression>"));
    assert_eq!(read_regex(&text).unwrap(), re);
}

#[test]
fn regex_errors() {
    let doc = |body: &str| format!("<structure><type>re</type>{body}</structure>");
    assert!(matches!(
        read_regex(&doc("<expression>(a+b</expression>")),
        Err(JffError::BadExpression(_))
    ));
    assert!(matches!(
        read_regex(GRAMMAR),
        Err(JffError::WrongType { .. })
    ));
    // special symbols survive the XML escaping
    let re = Regex::parse("a+!").unwrap();
    assert_eq!(read_regex(&write_regex(&re)).unwrap(), re);
}

// ---------- entry point ----------

#[test]
fn read_structure_dispatches_every_supported_type() {
    assert!(matches!(read_structure(FA), Ok(Structure::Fa(_))));
    assert!(matches!(read_structure(PDA), Ok(Structure::Pda(_))));
    assert!(matches!(read_structure(TURING), Ok(Structure::Turing(t)) if t.tapes() == 2));
    assert!(matches!(read_structure(MEALY), Ok(Structure::Mealy(_))));
    assert!(matches!(read_structure(MOORE), Ok(Structure::Moore(_))));
    assert!(matches!(read_structure(GRAMMAR), Ok(Structure::Grammar(_))));
    assert!(matches!(read_structure(RE), Ok(Structure::Regex(_))));
}

#[test]
fn structures_roundtrip_through_write_structure() {
    for fixture in [FA, PDA, TURING, MEALY, MOORE, GRAMMAR, RE] {
        let s = read_structure(fixture).unwrap();
        let text = write_structure(&s);
        assert_eq!(read_structure(&text).unwrap(), s, "{text}");
    }
}

#[test]
fn structure_is_serializable() {
    let s = read_structure(RE).unwrap();
    let json = serde_json::to_string(&s).unwrap();
    let back: Structure = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
    let g = read_structure(GRAMMAR).unwrap();
    let back: Structure = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
    assert_eq!(back, g);
}

#[test]
fn read_structure_reports_unsupported_types() {
    let lsystem = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\
        <structure><type>lsystem</type><axiom>F</axiom>\
        <production><left>F</left><right>F + F</right></production></structure>";
    assert_eq!(
        read_structure(lsystem),
        Err(JffError::Unsupported {
            structure_type: "lsystem".to_string()
        })
    );
    for ty in [
        "regular pumping lemma",
        "context-free pumping lemma",
        "something else",
    ] {
        let xml = format!("<structure><type>{ty}</type></structure>");
        assert_eq!(
            read_structure(&xml),
            Err(JffError::Unsupported {
                structure_type: ty.to_string()
            })
        );
    }
    assert!(matches!(
        read_structure("<structure><automaton/></structure>"),
        Err(JffError::MissingElement { .. })
    ));
    assert!(matches!(
        read_structure("not xml at all <"),
        Err(JffError::BadXml(_))
    ));
}

/// With `RFLAP_JFF_EXPORT=<dir>` the files we write are saved there so the JFLAP harness can
/// load them (`HarnessAll.java load`).
#[test]
fn exports_written_files_for_the_jflap_harness() {
    let files = [
        (
            "grammar.jff",
            write_grammar(&read_grammar(GRAMMAR).unwrap()),
        ),
        ("re.jff", write_regex(&read_regex(RE).unwrap())),
    ];
    for (name, text) in &files {
        assert!(!text.is_empty(), "{name}");
        if let Ok(dir) = std::env::var("RFLAP_JFF_EXPORT") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(std::path::Path::new(&dir).join(name), text).unwrap();
        }
    }
}
