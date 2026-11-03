//! `.jff` read/write for PDA, Turing machines, Mealy and Moore machines (JFLAP 7.1 `file.xml`).
//! The fixtures in `fixtures/jff/jflap/` were written by JFLAP 7.1 itself (see `harness/`).

use rflap_core::automaton::{Label, Point, Transition};
use rflap_core::jff::{
    JffError, read_mealy, read_moore, read_pda, read_turing, write_mealy, write_moore, write_pda,
    write_turing,
};
use rflap_core::mealy::{MealyLabel, MooreLabel, moore_output};
use rflap_core::pda::{Pda, PdaLabel};
use rflap_core::turing::{TapeAction, TmLabel, TuringMachine};

const PDA: &str = include_str!("fixtures/jff/jflap/pda.jff");
const TURING1: &str = include_str!("fixtures/jff/jflap/turing1.jff");
const TURING2: &str = include_str!("fixtures/jff/jflap/turing2.jff");
const TURING_BLOCK: &str = include_str!("fixtures/jff/jflap/turing_block.jff");
const TURING_NESTED: &str = include_str!("fixtures/jff/jflap/turing_nested.jff");
const MEALY: &str = include_str!("fixtures/jff/jflap/mealy.jff");
const MOORE: &str = include_str!("fixtures/jff/jflap/moore.jff");

fn pt(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn state_rows<'a>(
    states: impl Iterator<Item = &'a rflap_core::automaton::State>,
) -> Vec<(u32, String, Option<String>, Point)> {
    states
        .map(|s| (s.id, s.name.clone(), s.label.clone(), s.point))
        .collect()
}

fn sorted<L: Clone>(
    ts: &[Transition<L>],
    key: impl Fn(&Transition<L>) -> String,
) -> Vec<(u32, u32, L)> {
    let mut v: Vec<_> = ts.iter().collect();
    v.sort_by_key(|t| key(t));
    v.into_iter()
        .map(|t| (t.from, t.to, t.label.clone()))
        .collect()
}

fn label(read: &str, write: &str, dir: &str) -> TmLabel {
    TmLabel::single(read, write, dir).unwrap()
}

// ---------- PDA ----------

#[test]
fn reads_pda_written_by_jflap() {
    let pda = read_pda(PDA).unwrap();
    assert_eq!(
        state_rows(pda.states()),
        vec![
            (0, "q0".to_string(), None, pt(50.0, 100.0)),
            (1, "pushing".to_string(), None, pt(150.0, 100.0)),
            (
                2,
                "q2".to_string(),
                Some("done".to_string()),
                pt(250.0, 100.0)
            ),
        ]
    );
    assert_eq!(pda.initial(), Some(0));
    assert_eq!(pda.finals().iter().copied().collect::<Vec<_>>(), vec![2]);
    assert_eq!(
        sorted(pda.transitions(), |t| format!("{}{}", t.from, t.to)),
        vec![
            (0, 0, PdaLabel::new("a", "", "A")),
            (0, 1, PdaLabel::new("", "", "")),
            (1, 1, PdaLabel::new("b", "A", "")),
            (1, 2, PdaLabel::new("", "Z", "XY")),
        ]
    );
    let curved = pda.transitions().iter().find(|t| t.to == 2).unwrap();
    assert_eq!(curved.control, Some(pt(200.0, 40.0)));
    assert_eq!(pda.notes().len(), 1);
    assert_eq!(pda.notes()[0].text, "pda & <note>");
}

#[test]
fn pda_roundtrips() {
    let pda = read_pda(PDA).unwrap();
    let text = write_pda(&pda);
    assert!(text.contains("<type>pda</type>"));
    assert!(text.contains("<pop/>"));
    assert_eq!(read_pda(&text).unwrap(), pda);
}

#[test]
fn pda_missing_parts_are_empty_and_wrong_type_is_rejected() {
    let xml = "<structure><type>pda</type><automaton>\
        <state id=\"0\"><x>1</x><y>2</y><initial/></state>\
        <transition><from>0</from><to>0</to><read>a</read></transition>\
        </automaton></structure>";
    let pda: Pda = read_pda(xml).unwrap();
    assert_eq!(pda.transitions()[0].label, PdaLabel::new("a", "", ""));
    assert_eq!(
        read_pda(TURING1),
        Err(JffError::WrongType {
            expected: "pda".to_string(),
            found: "turing".to_string()
        })
    );
}

// ---------- Turing machines ----------

#[test]
fn reads_one_tape_turing_machine_written_by_jflap() {
    let tm = read_turing(TURING1).unwrap();
    assert_eq!(tm.tapes(), 1);
    let a = tm.automaton();
    assert_eq!(
        state_rows(a.states()),
        vec![
            (0, "q0".to_string(), None, pt(50.0, 100.0)),
            (1, "flip".to_string(), None, pt(150.0, 100.0)),
            // JFLAP does not write the label of a Turing machine state (the fixture set "halt").
            (2, "q2".to_string(), None, pt(250.0, 100.0)),
        ]
    );
    assert_eq!(a.initial(), Some(0));
    assert_eq!(a.finals().iter().copied().collect::<Vec<_>>(), vec![2]);
    assert_eq!(
        sorted(a.transitions(), |t| format!(
            "{}{}{}",
            t.from,
            t.to,
            t.label.description()
        )),
        vec![
            (0, 0, label("~", "~", "L")),
            (0, 1, label("0", "1", "R")),
            (1, 1, label("1", "0", "R")),
            (1, 2, label("", "", "S")),
            (2, 0, label("!x", "y", "R")),
        ]
    );
    assert!(tm.blocks().is_empty());
}

#[test]
fn reads_two_tape_turing_machine_written_by_jflap() {
    let tm = read_turing(TURING2).unwrap();
    assert_eq!(tm.tapes(), 2);
    let two = |r1: &str, w1: &str, d1: &str, r2: &str, w2: &str, d2: &str| {
        TmLabel::new(vec![
            TapeAction::new(r1, w1, d1).unwrap(),
            TapeAction::new(r2, w2, d2).unwrap(),
        ])
        .unwrap()
    };
    assert_eq!(
        sorted(tm.automaton().transitions(), |t| t.label.description()),
        vec![
            (0, 0, two("a", "a", "R", "", "a", "R")),
            (0, 0, two("b", "b", "R", "", "b", "R")),
            (0, 1, two("", "", "S", "", "", "L")),
        ]
    );
}

#[test]
fn reads_building_block_written_by_jflap() {
    let tm = read_turing(TURING_BLOCK).unwrap();
    assert_eq!(tm.automaton().states().count(), 3);
    assert_eq!(tm.automaton().state(1).unwrap().name, "writeone");
    assert_eq!(tm.blocks().keys().copied().collect::<Vec<_>>(), vec![1]);
    let inner = tm.block(1).unwrap();
    assert_eq!(inner.tapes(), 1);
    assert_eq!(
        state_rows(inner.automaton().states()),
        vec![
            (0, "q0".to_string(), None, pt(40.0, 60.0)),
            (1, "q1".to_string(), None, pt(140.0, 60.0)),
        ]
    );
    assert_eq!(inner.automaton().initial(), Some(0));
    assert!(inner.automaton().is_final(1));
    assert_eq!(
        inner.automaton().transitions()[0].label,
        label("", "1", "R")
    );
    // A block transition (block="true") has only <read>; JFLAP reads it as write ~, move S.
    assert_eq!(
        sorted(tm.automaton().transitions(), |t| format!(
            "{}{}",
            t.from, t.to
        )),
        vec![(0, 1, label("a", "a", "S")), (1, 2, label("", "~", "S")),]
    );
}

#[test]
fn reads_nested_building_blocks_written_by_jflap() {
    let tm = read_turing(TURING_NESTED).unwrap();
    let mid = tm.block(1).unwrap();
    assert_eq!(mid.automaton().states().count(), 2);
    assert_eq!(mid.automaton().transitions()[0].label, label("", "", "S"));
    let leaf = mid.block(1).unwrap();
    assert_eq!(leaf.automaton().states().count(), 2);
    assert_eq!(leaf.automaton().transitions()[0].label, label("", "x", "L"));
    assert!(leaf.blocks().is_empty());
}

#[test]
fn turing_machines_roundtrip() {
    for fixture in [TURING1, TURING2, TURING_BLOCK, TURING_NESTED] {
        let tm = read_turing(fixture).unwrap();
        let text = write_turing(&tm);
        assert_eq!(read_turing(&text).unwrap(), tm, "{text}");
    }
}

#[test]
fn written_turing_layout_follows_jflap() {
    let one = write_turing(&read_turing(TURING1).unwrap());
    assert!(one.contains("<type>turing</type>"));
    assert!(!one.contains("<tapes>"));
    assert!(!one.contains("tape=\""));
    assert!(one.contains("<block id=\"1\" name=\"flip\">"));
    assert!(one.contains("<tag>Machine1</tag>"));
    assert!(one.contains("<!--The list of automata-->"));
    assert!(one.contains("<read/>"));
    assert!(one.contains("<move>S</move>"));

    let two = write_turing(&read_turing(TURING2).unwrap());
    assert!(two.contains("<type>turing</type>\n\t<tapes>2</tapes>"));
    assert!(two.contains("<read tape=\"2\"/>"));
    assert!(two.contains("<move tape=\"2\">L</move>"));
}

#[test]
fn written_building_blocks_get_unique_tags() {
    // JFLAP cannot reload its own files when two blocks share a tag (MachineN of different
    // levels collide), so block tags are unique across the whole file.
    let text = write_turing(&read_turing(TURING_NESTED).unwrap());
    assert!(text.contains("<tag>Block1</tag>"), "{text}");
    assert!(text.contains("<tag>Block1_1</tag>"), "{text}");
    assert!(text.contains("<Block1>"));
    assert!(text.contains("<Block1_1>"));
}

#[test]
fn turing_machine_built_in_memory_roundtrips() {
    let mut inner = TuringMachine::new(2);
    let i0 = inner.create_state(pt(10.0, 10.0));
    let i1 = inner.create_state(pt(60.0, 10.0));
    inner.set_initial(Some(i0));
    inner.set_final(i1, true);
    let l = TmLabel::new(vec![
        TapeAction::new("a", "b", "R").unwrap(),
        TapeAction::new("", "", "S").unwrap(),
    ])
    .unwrap();
    inner.add_transition(i0, i1, l.clone()).unwrap();
    let mut tm = TuringMachine::new(2);
    let s0 = tm.create_state(pt(50.0, 50.0));
    let blk = tm.add_block(pt(150.0, 50.0), inner).unwrap();
    tm.state_mut(blk).unwrap().name = "blk".to_string();
    tm.state_mut(s0).unwrap().label = Some("start".to_string());
    tm.set_initial(Some(s0));
    tm.set_final(blk, true);
    tm.add_transition(s0, blk, l).unwrap();
    let text = write_turing(&tm);
    assert_eq!(read_turing(&text).unwrap(), tm, "{text}");
    if let Ok(dir) = std::env::var("RFLAP_JFF_EXPORT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join("turing_memory.jff"), &text).unwrap();
    }
}

#[test]
fn turing_rejects_wrong_number_of_tapes_in_a_transition() {
    let machine = |tapes: &str, body: &str| {
        format!(
            "<structure><type>turing</type>{tapes}<automaton>\
             <block id=\"0\" name=\"q0\"><tag>M0</tag><x>1</x><y>2</y><initial/></block>\
             <transition><from>0</from><to>0</to>{body}</transition>\
             <M0/></automaton></structure>"
        )
    };
    // two tapes declared, only tape 1 described
    let few = machine(
        "<tapes>2</tapes>",
        "<read tape=\"1\">a</read><write tape=\"1\">a</write><move tape=\"1\">R</move>",
    );
    assert_eq!(
        read_turing(&few),
        Err(JffError::TapeCount {
            expected: 2,
            found: 1
        })
    );
    // a tape beyond the declared ones
    let many = machine(
        "",
        "<read tape=\"2\">a</read><write tape=\"2\">a</write><move tape=\"2\">R</move>",
    );
    assert_eq!(
        read_turing(&many),
        Err(JffError::TapeOutOfRange { tape: 2, tapes: 1 })
    );
    // the number of tapes itself must be 1-5
    assert!(matches!(
        read_turing(&machine("<tapes>6</tapes>", "<read>a</read>")),
        Err(JffError::BadNumber { .. })
    ));
    assert!(matches!(
        read_turing(&machine("<tapes>0</tapes>", "<read>a</read>")),
        Err(JffError::BadNumber { .. })
    ));
    // a bad symbol is a label error
    let bad = machine("", "<read>ab</read><write>a</write><move>R</move>");
    assert!(matches!(read_turing(&bad), Err(JffError::BadLabel(_))));
    let bad_move = machine("", "<read>a</read><write>a</write><move>X</move>");
    assert!(matches!(read_turing(&bad_move), Err(JffError::BadLabel(_))));
}

#[test]
fn turing_rejects_unknown_block_reference() {
    let xml = "<structure><type>turing</type><automaton>\
        <block id=\"0\" name=\"q0\"><tag>Missing</tag><x>1</x><y>2</y><initial/></block>\
        </automaton></structure>";
    assert_eq!(
        read_turing(xml),
        Err(JffError::UnknownBlock {
            tag: "Missing".to_string()
        })
    );
}

#[test]
fn turing_rejects_a_block_that_contains_itself() {
    let xml = "<structure><type>turing</type><automaton>\
        <block id=\"0\" name=\"q0\"><tag>Loop</tag><x>1</x><y>2</y></block>\
        <Loop><block id=\"0\" name=\"q0\"><tag>Loop</tag><x>1</x><y>2</y></block></Loop>\
        </automaton></structure>";
    assert_eq!(
        read_turing(xml),
        Err(JffError::BlockCycle {
            tag: "Loop".to_string()
        })
    );
}

#[test]
fn turing_accepts_plain_state_elements_with_tags() {
    // Reading also accepts <state> instead of <block>.
    let xml = "<structure><type>turing</type><automaton>\
        <state id=\"0\" name=\"a\"><tag>M0</tag><x>1</x><y>2</y><initial/><final/></state>\
        <M0/></automaton></structure>";
    let tm = read_turing(xml).unwrap();
    assert_eq!(tm.automaton().states().count(), 1);
    assert!(tm.blocks().is_empty());
    assert!(tm.automaton().is_final(0));
}

// ---------- Mealy and Moore ----------

#[test]
fn reads_mealy_written_by_jflap() {
    let m = read_mealy(MEALY).unwrap();
    assert_eq!(
        state_rows(m.states()),
        vec![
            (0, "start".to_string(), None, pt(50.0, 100.0)),
            (1, "q1".to_string(), None, pt(150.0, 100.0)),
        ]
    );
    assert_eq!(m.initial(), Some(0));
    assert!(m.finals().is_empty());
    assert_eq!(
        sorted(m.transitions(), |t| format!("{}{}", t.from, t.to)),
        vec![
            (0, 1, MealyLabel::new("a", "01")),
            (1, 0, MealyLabel::new("b", "")),
            (1, 1, MealyLabel::new("", "z")),
        ]
    );
}

#[test]
fn mealy_roundtrips() {
    let m = read_mealy(MEALY).unwrap();
    let text = write_mealy(&m);
    assert!(text.contains("<type>mealy</type>"));
    assert!(text.contains("<transout>01</transout>"));
    assert!(text.contains("<transout/>"));
    assert_eq!(read_mealy(&text).unwrap(), m);
}

#[test]
fn reads_moore_written_by_jflap() {
    let m = read_moore(MOORE).unwrap();
    assert_eq!(m.states().count(), 3);
    assert_eq!(moore_output(&m, 0), "x");
    assert_eq!(moore_output(&m, 1), "yy");
    assert_eq!(moore_output(&m, 2), "");
    assert_eq!(m.state(2).unwrap().output, None);
    assert_eq!(m.initial(), Some(0));
    assert_eq!(
        sorted(m.transitions(), |t| format!("{}{}", t.from, t.to)),
        vec![
            (0, 1, MooreLabel::from("a")),
            (1, 2, MooreLabel::from("b")),
            (2, 0, MooreLabel::from("a")),
        ]
    );
}

#[test]
fn moore_roundtrips_and_writes_what_jflap_needs() {
    let m = read_moore(MOORE).unwrap();
    let text = write_moore(&m);
    assert!(text.contains("<type>moore</type>"));
    // Every state carries <output> and every transition the output of its destination as
    // <transout>: JFLAP copies <transout> into the destination state when it loads.
    assert_eq!(text.matches("<output").count(), 3, "{text}");
    assert!(text.contains("<output/>"));
    assert!(text.contains("<transout>yy</transout>"));
    assert!(text.contains("<transout>x</transout>"));
    assert_eq!(read_moore(&text).unwrap(), m);
}

#[test]
fn moore_transout_overrides_state_output_only_when_present() {
    let xml = "<structure><type>moore</type><automaton>\
        <state id=\"0\"><x>1</x><y>2</y><initial/><output>keep</output></state>\
        <state id=\"1\"><x>1</x><y>2</y><output>old</output></state>\
        <state id=\"2\"><x>1</x><y>2</y></state>\
        <transition><from>0</from><to>1</to><read>a</read><transout>new</transout></transition>\
        <transition><from>0</from><to>0</to><read>b</read></transition>\
        </automaton></structure>";
    let m = read_moore(xml).unwrap();
    assert_eq!(moore_output(&m, 0), "keep");
    assert_eq!(moore_output(&m, 1), "new");
    assert_eq!(moore_output(&m, 2), "");
}

#[test]
fn machines_reject_other_types() {
    assert!(matches!(read_mealy(MOORE), Err(JffError::WrongType { .. })));
    assert!(matches!(read_moore(MEALY), Err(JffError::WrongType { .. })));
    assert!(matches!(read_turing(PDA), Err(JffError::WrongType { .. })));
}

/// With `RFLAP_JFF_EXPORT=<dir>` the files we write are saved there so the JFLAP harness can
/// load them (`HarnessAll.java load`); without it the test only checks they are non-empty.
#[test]
fn exports_written_files_for_the_jflap_harness() {
    let files: Vec<(&str, String)> = vec![
        ("pda.jff", write_pda(&read_pda(PDA).unwrap())),
        ("turing1.jff", write_turing(&read_turing(TURING1).unwrap())),
        ("turing2.jff", write_turing(&read_turing(TURING2).unwrap())),
        (
            "turing_block.jff",
            write_turing(&read_turing(TURING_BLOCK).unwrap()),
        ),
        (
            "turing_nested.jff",
            write_turing(&read_turing(TURING_NESTED).unwrap()),
        ),
        ("mealy.jff", write_mealy(&read_mealy(MEALY).unwrap())),
        ("moore.jff", write_moore(&read_moore(MOORE).unwrap())),
    ];
    for (name, text) in &files {
        assert!(!text.is_empty(), "{name}");
        if let Ok(dir) = std::env::var("RFLAP_JFF_EXPORT") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(std::path::Path::new(&dir).join(name), text).unwrap();
        }
    }
}
