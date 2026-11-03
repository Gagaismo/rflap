//! `.jff` read/write for finite automata (JFLAP 7.1 `file.xml`).

use rflap_core::automaton::{Note, Point, Transition};
use rflap_core::fa::{Fa, FaLabel};
use rflap_core::jff::{
    Element, JffError, parse_document, read_automaton, read_fa, structure_type, write_automaton,
    write_document, write_fa,
};

const SAMPLE: &str = include_str!("fixtures/jff/sample.jff");
const SAMPLE_CRLF: &str = include_str!("fixtures/jff/sample_crlf.jff");
const OLD_LAYOUT: &str = include_str!("fixtures/jff/old_layout.jff");

fn pt(x: f64, y: f64) -> Point {
    Point { x, y }
}

/// Transitions as `(from, to, label)` sorted, since JFLAP writes them in hash order.
fn edges(fa: &Fa) -> Vec<(u32, u32, String)> {
    let mut v: Vec<_> = fa
        .transitions()
        .iter()
        .map(|t| (t.from, t.to, t.label.0.clone()))
        .collect();
    v.sort();
    v
}

fn check_sample(fa: &Fa) {
    let states: Vec<_> = fa.states().collect();
    assert_eq!(states.len(), 3);
    assert_eq!(
        (states[0].id, states[0].name.as_str(), states[0].point),
        (0, "q0", pt(50.0, 100.0))
    );
    assert_eq!(
        (states[1].id, states[1].name.as_str(), states[1].point),
        (1, "middle", pt(150.0, 100.0))
    );
    assert_eq!(
        (states[2].id, states[2].name.as_str(), states[2].point),
        (2, "q2", pt(250.0, 150.0))
    );
    assert_eq!(states[0].label, None);
    assert_eq!(states[2].label.as_deref(), Some("accept"));
    assert_eq!(fa.initial(), Some(0));
    assert_eq!(fa.finals().iter().copied().collect::<Vec<_>>(), vec![2]);
    assert_eq!(
        edges(fa),
        vec![
            (0, 1, "a".to_string()),
            (1, 0, "ab".to_string()),
            (1, 2, String::new()),
            (2, 2, "b".to_string()),
        ]
    );
    let curved = fa
        .transitions()
        .iter()
        .find(|t| t.from == 1 && t.to == 0)
        .unwrap();
    assert_eq!(curved.control, Some(pt(100.0, 40.0)));
    assert!(
        fa.transitions()
            .iter()
            .filter(|t| !(t.from == 1 && t.to == 0))
            .all(|t| t.control.is_none())
    );
    assert_eq!(
        fa.notes(),
        &[Note {
            point: pt(0.0, 0.0),
            text: "hello <note> & co".to_string()
        }]
    );
}

#[test]
fn reads_fixture_written_by_jflap() {
    check_sample(&read_fa(SAMPLE).unwrap());
}

#[test]
fn reads_fixture_written_by_jflap_on_windows() {
    // CRLF line separators arrive as &#13; character references.
    assert!(SAMPLE_CRLF.contains("&#13;"));
    check_sample(&read_fa(SAMPLE_CRLF).unwrap());
}

#[test]
fn reads_older_layout_without_automaton_wrapper() {
    let fa = read_fa(OLD_LAYOUT).unwrap();
    let states: Vec<_> = fa.states().collect();
    assert_eq!(states.len(), 2);
    assert_eq!(states[0].name, "start");
    assert_eq!(states[1].name, "q1");
    assert_eq!(states[1].point, pt(130.0, 40.0));
    assert_eq!(fa.initial(), Some(0));
    assert!(fa.is_final(1) && !fa.is_final(0));
    // A transition without <read> is λ.
    assert_eq!(
        edges(&fa),
        vec![(0, 1, "x".to_string()), (1, 0, String::new())]
    );
}

#[test]
fn structure_type_names_the_structure() {
    assert_eq!(structure_type(SAMPLE).unwrap(), "fa");
    let pda = "<structure><type>pda</type><automaton/></structure>";
    assert_eq!(structure_type(pda).unwrap(), "pda");
    let lemma = "<structure><type>regular pumping lemma</type></structure>";
    assert_eq!(structure_type(lemma).unwrap(), "regular pumping lemma");
    assert_eq!(
        structure_type("<structure><automaton/></structure>"),
        Err(JffError::MissingElement {
            element: "type".to_string(),
            context: "structure".to_string()
        })
    );
}

#[test]
fn empty_read_is_lambda_and_roundtrips_as_empty_element() {
    let xml = "<structure><type>fa</type><automaton>\
        <state id=\"0\"><x>1</x><y>2</y><initial/></state>\
        <transition><from>0</from><to>0</to><read/></transition>\
        <transition><from>0</from><to>0</to><read></read></transition>\
        </automaton></structure>";
    let fa = read_fa(xml).unwrap();
    assert_eq!(fa.transitions().len(), 1);
    assert!(fa.transitions()[0].label.0.is_empty());
    assert!(write_fa(&fa).contains("<read/>"));
}

fn sample_fa() -> Fa {
    let mut fa = Fa::new();
    let a = fa.create_state(pt(10.5, 20.0));
    let b = fa.create_state(pt(110.0, 20.0));
    let c = fa.create_state(pt(60.0, 90.25));
    fa.state_mut(a).unwrap().name = "start & <go>".to_string();
    fa.state_mut(c).unwrap().label = Some("sink \"x\"".to_string());
    fa.set_initial(Some(a));
    fa.set_final(b, true);
    fa.set_final(c, true);
    fa.add_transition(Transition::new(a, b, FaLabel::from("[0-9]")));
    fa.add_transition(Transition::new(b, c, FaLabel::lambda()));
    fa.add_transition(Transition {
        from: c,
        to: a,
        label: FaLabel::from("ab"),
        control: Some(pt(30.0, 55.0)),
    });
    fa.add_transition(Transition::new(b, b, FaLabel::from("a<b&c")));
    fa.add_note(Note {
        point: pt(5.0, 6.5),
        text: "multi\nline".to_string(),
    });
    fa
}

#[test]
fn write_then_read_gives_the_same_automaton() {
    let fa = sample_fa();
    assert_eq!(read_fa(&write_fa(&fa)).unwrap(), fa);
}

#[test]
fn roundtrip_of_empty_automaton_and_sparse_ids() {
    let empty = Fa::new();
    assert_eq!(read_fa(&write_fa(&empty)).unwrap(), empty);

    let mut fa = Fa::new();
    fa.create_state_with_id(7, pt(1.0, 2.0)).unwrap();
    fa.create_state_with_id(3, pt(3.0, 4.0)).unwrap();
    fa.set_initial(Some(7));
    fa.add_transition(Transition::new(7, 3, FaLabel::from("z")));
    assert_eq!(read_fa(&write_fa(&fa)).unwrap(), fa);
}

#[test]
fn reading_the_jflap_fixture_and_writing_it_back_keeps_the_automaton() {
    let fa = read_fa(SAMPLE).unwrap();
    assert_eq!(read_fa(&write_fa(&fa)).unwrap(), fa);
}

#[test]
fn written_layout_matches_what_jflap_writes() {
    let mut fa = Fa::new();
    let a = fa.create_state(pt(50.0, 100.0));
    let b = fa.create_state(pt(150.0, 100.0));
    fa.set_initial(Some(a));
    fa.set_final(b, true);
    fa.state_mut(b).unwrap().label = Some("L".to_string());
    fa.add_transition(Transition::new(a, b, FaLabel::from("a")));
    fa.add_transition(Transition::new(b, a, FaLabel::lambda()));
    let expected = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\
<!--Created with JFLAP 6.4.--><structure>
\t<type>fa</type>
\t<automaton>
\t\t<!--The list of states.-->
\t\t<state id=\"0\" name=\"q0\">
\t\t\t<x>50.0</x>
\t\t\t<y>100.0</y>
\t\t\t<initial/>
\t\t</state>
\t\t<state id=\"1\" name=\"q1\">
\t\t\t<x>150.0</x>
\t\t\t<y>100.0</y>
\t\t\t<label>L</label>
\t\t\t<final/>
\t\t</state>
\t\t<!--The list of transitions.-->
\t\t<transition>
\t\t\t<from>0</from>
\t\t\t<to>1</to>
\t\t\t<read>a</read>
\t\t</transition>
\t\t<transition>
\t\t\t<from>1</from>
\t\t\t<to>0</to>
\t\t\t<read/>
\t\t</transition>
\t</automaton>
</structure>";
    assert_eq!(write_fa(&fa), expected);
}

#[test]
fn malformed_xml_is_rejected() {
    for bad in ["", "not xml", "<structure><type>fa</type>", "<a></b>"] {
        assert!(
            matches!(read_fa(bad), Err(JffError::BadXml(_))),
            "{bad:?} gave {:?}",
            read_fa(bad)
        );
        assert!(matches!(structure_type(bad), Err(JffError::BadXml(_))));
    }
}

#[test]
fn other_structure_type_is_rejected_by_read_fa() {
    let pda = "<structure><type>pda</type><automaton/></structure>";
    let err = read_fa(pda).unwrap_err();
    assert_eq!(
        err,
        JffError::WrongType {
            expected: "fa".to_string(),
            found: "pda".to_string()
        }
    );
    assert_eq!(
        err.to_string(),
        "expected a structure of type \"fa\" but the file has type \"pda\""
    );
}

#[test]
fn missing_from_is_rejected() {
    let xml = "<structure><type>fa</type><automaton>\
        <state id=\"0\"><x>0</x><y>0</y></state>\
        <transition><to>0</to><read>a</read></transition>\
        </automaton></structure>";
    let err = read_fa(xml).unwrap_err();
    assert_eq!(
        err,
        JffError::MissingElement {
            element: "from".to_string(),
            context: "transition".to_string()
        }
    );
    assert_eq!(err.to_string(), "missing <from> in <transition>");
}

#[test]
fn bad_numbers_and_dangling_references_are_rejected() {
    let wrap = |inner: &str| {
        format!("<structure><type>fa</type><automaton>{inner}</automaton></structure>")
    };
    assert!(matches!(
        read_fa(&wrap("<state id=\"a\"><x>0</x><y>0</y></state>")),
        Err(JffError::BadNumber { .. })
    ));
    assert!(matches!(
        read_fa(&wrap("<state><x>0</x><y>0</y></state>")),
        Err(JffError::BadNumber { .. })
    ));
    let err = read_fa(&wrap("<state id=\"0\"><x>left</x><y>0</y></state>")).unwrap_err();
    assert_eq!(
        err,
        JffError::BadNumber {
            what: "x of state 0".to_string(),
            text: "left".to_string()
        }
    );
    assert_eq!(
        read_fa(&wrap("<state id=\"0\"/><state id=\"0\"/>")),
        Err(JffError::DuplicateState(0))
    );
    assert_eq!(
        read_fa(&wrap(
            "<state id=\"0\"/><transition><from>0</from><to>5</to></transition>"
        )),
        Err(JffError::UnknownState {
            id: 5,
            context: "to of a transition".to_string()
        })
    );
}

#[test]
fn generic_codec_carries_custom_labels_and_extra_elements() {
    // What T15 does for other machines: its own label closures, the same state code.
    let mut fa = Fa::new();
    let a = fa.create_state(pt(1.0, 2.0));
    fa.set_initial(Some(a));
    fa.state_mut(a).unwrap().output = Some("out".to_string());
    fa.add_transition(Transition::new(a, a, FaLabel::from("x")));
    let mut root = write_automaton(&fa, "moore", |l, el| {
        let mut e = Element::new("transout");
        e.text = l.0.to_uppercase();
        el.push(e);
    });
    assert_eq!(root.child_text("type"), Some("moore"));
    root.push(Element::new("extra"));
    let text = write_document(&root);
    assert!(text.contains("<output>out</output>"), "{text}");
    assert!(text.contains("<transout>X</transout>"), "{text}");
    let parsed = parse_document(&text).unwrap();
    let back = read_automaton(&parsed, |t| {
        Ok(FaLabel(
            t.child_text("transout").unwrap_or("").to_lowercase(),
        ))
    })
    .unwrap();
    assert_eq!(back, fa);
}

/// Writes files for the JFLAP harness to load:
/// `RFLAP_JFF_OUT=<dir> cargo test -p rflap-core --test jff_fa -- --ignored dump`
/// then `java ... Harness.java load <dir>/written.jff`.
#[test]
#[ignore = "writes files for the manual JFLAP round trip"]
fn dump_written_files_for_jflap() {
    let dir = std::env::var("RFLAP_JFF_OUT").expect("set RFLAP_JFF_OUT to a directory");
    let dir = std::path::Path::new(&dir);
    std::fs::write(dir.join("written.jff"), write_fa(&sample_fa())).unwrap();
    let reread = read_fa(SAMPLE).unwrap();
    std::fs::write(dir.join("rewritten_sample.jff"), write_fa(&reread)).unwrap();
}
