//! `.jff` read/write for L-systems (JFLAP `LSystemTransducer`).

use rflap_core::jff::{JffError, structure_type};
use rflap_core::lsystem::{LSystem, jff};

const KOCH: &str = include_str!("fixtures/t8/lsystem_koch.jff");

fn sorted(l: &LSystem) -> Vec<(String, String)> {
    let mut p = l.parameters().to_vec();
    p.sort();
    p
}

#[test]
fn reads_a_file_jflap_wrote() {
    let l = jff::read(KOCH).unwrap();
    assert_eq!(structure_type(KOCH).unwrap(), "lsystem");
    assert_eq!(l.axiom(), ["g", "-", "-", "g", "-", "-", "g"]);
    assert_eq!(l.symbols_with_replacements().collect::<Vec<_>>(), ["g"]);
    assert_eq!(l.replacements("g").len(), 1);
    assert_eq!(l.replacements("g")[0].join(" "), "g + g - - g + g");
    assert_eq!(
        sorted(&l),
        [
            ("angle".to_string(), "60".to_string()),
            ("distance".to_string(), "5".to_string())
        ]
    );
}

#[test]
fn writes_the_layout_jflap_writes() {
    let l = LSystem::new(
        "g - - g - - g",
        &[("g", "g + g - - g + g")],
        &[("angle", "60"), ("distance", "5")],
    );
    // JFLAP keeps parameters in a HashMap: `distance` comes out before `angle`
    assert_eq!(jff::write(&l), KOCH);
}

#[test]
fn round_trips_every_part() {
    let l = LSystem::new(
        "x",
        &[
            ("x", "g [ + x ] [ - x ] g x"),
            ("g", "g g"),
            ("g", "g"),
            ("1 a b", ""),
        ],
        &[("angle", "25"), ("distance", "4"), ("color", "red")],
    );
    let back = jff::read(&jff::write(&l)).unwrap();
    assert_eq!(back.axiom(), l.axiom());
    for s in l.symbols_with_replacements() {
        assert_eq!(back.replacements(s), l.replacements(s), "{s}");
    }
    assert_eq!(
        back.symbols_with_replacements().count(),
        l.symbols_with_replacements().count()
    );
    assert_eq!(sorted(&back), sorted(&l));
    assert_eq!(back.is_nondeterministic(), l.is_nondeterministic());
    // a second round does not move anything
    assert_eq!(jff::write(&back), jff::write(&l));
}

#[test]
fn an_empty_l_system_is_an_axiom_element_only() {
    let text = jff::write(&LSystem::default());
    assert!(text.ends_with(
        "<structure>\n\t<type>lsystem</type>\n\t<!--The L-system axiom.-->\n\t<axiom/>\n</structure>"
    ));
    let back = jff::read(&text).unwrap();
    assert!(back.axiom().is_empty());
    assert_eq!(back.symbols_with_replacements().count(), 0);
    assert!(back.parameters().is_empty());
}

#[test]
fn the_last_axiom_wins_and_odd_elements_are_tolerated() {
    let text = "<structure><type>lsystem</type>\
        <axiom>a</axiom><axiom>b c</axiom>\
        <production><left>b</left></production>\
        <production><left>c</left><right>x</right><right></right><right>y  z</right></production>\
        <production><right>q</right></production>\
        <parameter><name>angle</name></parameter>\
        <parameter><value>7</value></parameter>\
        <parameter><name>distance</name><value>3</value></parameter>\
        </structure>";
    let l = jff::read(text).unwrap();
    assert_eq!(l.axiom(), ["b", "c"]);
    // a production with no `right` adds nothing
    assert_eq!(l.replacements("b").len(), 0);
    assert_eq!(
        l.replacements("c"),
        [
            vec!["x".to_string()],
            vec![],
            vec!["y".to_string(), "z".to_string()]
        ]
    );
    // a missing `left` is the empty symbol
    assert_eq!(l.replacements(""), [vec!["q".to_string()]]);
    // a nameless parameter is skipped, a valueless one is empty
    assert_eq!(
        sorted(&l),
        [
            ("angle".to_string(), String::new()),
            ("distance".to_string(), "3".to_string())
        ]
    );
}

#[test]
fn rejects_other_types_and_a_missing_axiom() {
    let fa = "<structure><type>fa</type><automaton/></structure>";
    assert_eq!(
        jff::read(fa),
        Err(JffError::WrongType {
            expected: "lsystem".to_string(),
            found: "fa".to_string()
        })
    );
    let no_axiom = "<structure><type>lsystem</type></structure>";
    assert!(matches!(
        jff::read(no_axiom),
        Err(JffError::MissingElement { .. })
    ));
    assert!(matches!(jff::read("<structure>"), Err(JffError::BadXml(_))));
}

/// Writes samples for `LHarness load` (JFLAP's own reader):
/// `RFLAP_T8_OUT=<dir> cargo test -p rflap-core --test lsystem_jff -- --ignored`.
#[test]
#[ignore = "writes files for the JFLAP jar to read"]
fn write_samples_for_the_jflap_jar() {
    let dir = std::env::var("RFLAP_T8_OUT").expect("set RFLAP_T8_OUT to a directory");
    let l = LSystem::new(
        "x",
        &[
            ("x", "g [ + x ] [ - x ] g x"),
            ("g", "g g"),
            ("g", "g"),
            ("1 a b", ""),
        ],
        &[("angle", "25"), ("distance", "4"), ("color", "red")],
    );
    std::fs::write(format!("{dir}/rust_lsystem.jff"), jff::write(&l)).unwrap();
}
