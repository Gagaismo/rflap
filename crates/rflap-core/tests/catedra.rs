//! Turing machines and linear bounded automata in the notation of the course
//! "Sintaxis y Semántica de los Lenguajes" (Ing. Jorge Buabud, UTN FRT).

use rflap_core::turing::TuringMachine;
use rflap_core::turing::catedra::{
    self, CatedraError, Model, Notation, Verdict, compose, format_edge, format_label, parse_label,
};

/// Builds a one-tape machine from rows `state read next write move` (cátedra symbols).
fn machine(order: &[&str], initial: &str, finals: &[&str], rows: &[&str]) -> TuringMachine {
    let mut m = TuringMachine::new(1);
    let mut ids = std::collections::BTreeMap::new();
    for name in order {
        let id = m.create_state(Default::default());
        m.state_mut(id).unwrap().name = name.to_string();
        ids.insert(name.to_string(), id);
    }
    m.set_initial(Some(ids[initial]));
    for f in finals {
        m.set_final(ids[*f], true);
    }
    for row in rows {
        let p: Vec<&str> = row.split_whitespace().collect();
        let label = parse_label(&format!("({},{},{})", p[1], p[3], p[4])).unwrap();
        m.add_transition(ids[p[0]], ids[p[2]], label).unwrap();
    }
    m
}

fn palindrome() -> TuringMachine {
    machine(
        &["1", "2", "3", "4", "5", "6", "7", "8"],
        "1",
        &["8"],
        &[
            "1 a 2 Δ D",
            "1 b 5 Δ D",
            "1 Δ 8 Δ D",
            "2 a 2 a D",
            "2 b 2 b D",
            "2 Δ 3 Δ I",
            "3 a 4 Δ I",
            "3 Δ 8 Δ D",
            "4 a 4 a I",
            "4 b 4 b I",
            "4 Δ 1 Δ D",
            "5 a 5 a D",
            "5 b 5 b D",
            "5 Δ 6 Δ I",
            "6 b 7 Δ I",
            "6 Δ 8 Δ D",
            "7 a 7 a I",
            "7 b 7 b I",
            "7 Δ 1 Δ D",
        ],
    )
}

fn igual() -> TuringMachine {
    machine(
        &["I", "A", "B", "C", "D", "F"],
        "I",
        &["F"],
        &[
            "I a A X I",
            "I b I b D",
            "I X I X D",
            "I ⟩ D ⟩ I",
            "A b A b I",
            "A X A X I",
            "A ⟨ B ⟨ D",
            "B a B a D",
            "B b C X I",
            "B X B X D",
            "C a C a I",
            "C X C X I",
            "C ⟨ I ⟨ D",
            "D X D X I",
            "D ⟨ F ⟨ D",
        ],
    )
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn labels_parse_in_every_syntax_and_format_back() {
    let cat = parse_label("(a,X,D)").unwrap();
    assert_eq!(parse_label("a,X,D").unwrap(), cat);
    assert_eq!(parse_label(" ( a , X , D ) ").unwrap(), cat);
    assert_eq!(parse_label("a;X,R").unwrap(), cat);
    assert_eq!(format_label(&cat, Notation::Catedra), "(a,X,D)");
    assert_eq!(format_label(&cat, Notation::Jflap), "a;X,R");

    let blank = parse_label("(Δ,Δ,I)").unwrap();
    assert_eq!(blank, parse_label("□;□,L").unwrap());
    assert_eq!(blank.actions()[0].read(), "□");
    assert_eq!(format_label(&blank, Notation::Catedra), "(Δ,Δ,I)");
    assert_eq!(format_label(&blank, Notation::Jflap), "□;□,L");

    let stay = parse_label("(b,b,N)").unwrap();
    assert_eq!(format_label(&stay, Notation::Jflap), "b;b,S");
    assert_eq!(parse_label("b;b,S").unwrap(), stay);
}

#[test]
fn multi_tape_labels_keep_the_jflap_notation_with_delta() {
    let two = parse_label("a;Δ,R|b;b,S").unwrap();
    assert_eq!(two.tapes(), 2);
    assert_eq!(format_label(&two, Notation::Catedra), "a;Δ,R|b;b,S");
    assert_eq!(format_label(&two, Notation::Jflap), "a;□,R|b;b,S");
}

#[test]
fn bad_labels_are_errors() {
    for bad in ["(a,b)", "(a,b,X)", "(a,bb,D)", "", "a;b", "(a,b,D,I)"] {
        assert!(parse_label(bad).is_err(), "{bad:?} should not parse");
    }
}

#[test]
fn edges_with_several_labels_are_joined_with_slashes() {
    let a = parse_label("(a,a,D)").unwrap();
    let b = parse_label("(b,b,D)").unwrap();
    assert_eq!(
        format_edge(&[&a, &b], Notation::Catedra),
        "(a,a,D) / (b,b,D)"
    );
    assert_eq!(format_edge(&[&a, &b], Notation::Jflap), "a;a,R / b;b,R");
    assert_eq!(
        catedra::split_labels("(a,a,D) / (b,b,D)"),
        vec!["(a,a,D)", "(b,b,D)"]
    );
}

#[test]
fn palindrome_definition() {
    let d = catedra::definition(&palindrome(), None).unwrap();
    assert_eq!(d.model, Model::Mt);
    assert_eq!(d.states, strings(&["1", "2", "3", "4", "5", "6", "7", "8"]));
    assert_eq!(d.input_alphabet, strings(&["a", "b"]));
    assert!(d.input_alphabet_inferred);
    assert_eq!(d.auxiliary_alphabet, strings(&["Δ"]));
    assert_eq!(d.initial.as_deref(), Some("1"));
    assert_eq!(d.finals, strings(&["8"]));
    assert!(d.deterministic);
    assert!(d.issues.is_empty());
    assert_eq!(
        d.text,
        "MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩\nQ = {1, 2, 3, 4, 5, 6, 7, 8}\nΣ_E = {a, b} (inferido)\nΣ_A = {Δ}\nq0 = 1\nF = {8}\n"
    );
}

#[test]
fn igual_definition_is_an_all() {
    let d = catedra::definition(&igual(), None).unwrap();
    assert_eq!(d.model, Model::All);
    assert_eq!(d.states, strings(&["I", "A", "B", "C", "D", "F"]));
    assert_eq!(d.input_alphabet, strings(&["a", "b"]));
    assert_eq!(d.auxiliary_alphabet, strings(&["⟨", "⟩", "X"]));
    assert_eq!(
        d.text,
        "ALL = ⟨Q, Σ_E, Σ_A, q0, F, f⟩\nQ = {I, A, B, C, D, F}\nΣ_E = {a, b} (inferido)\nΣ_A = {⟨, ⟩, X}\nq0 = I\nF = {F}\n"
    );
}

#[test]
fn a_declared_input_alphabet_is_not_inferred() {
    let mut m = palindrome();
    m.set_input_alphabet(Some("ab".chars().collect()));
    let d = catedra::definition(&m, None).unwrap();
    assert!(!d.input_alphabet_inferred);
    assert!(d.text.contains("Σ_E = {a, b}\n"));

    m.set_input_alphabet(Some("a".chars().collect()));
    let d = catedra::definition(&m, None).unwrap();
    // `b` is read by the machine: it is not in Σ_E, so it is auxiliary.
    assert_eq!(d.input_alphabet, strings(&["a"]));
    assert_eq!(d.auxiliary_alphabet, strings(&["b", "Δ"]));
}

#[test]
fn transitions_leaving_a_final_state_and_nondeterminism_are_reported() {
    let m = machine(
        &["p", "q"],
        "p",
        &["q"],
        &["p a q a D", "p a p b D", "q a p a D"],
    );
    let d = catedra::definition(&m, None).unwrap();
    assert!(!d.deterministic);
    let kinds: Vec<&str> = d.issues.iter().map(|i| i.kind.as_str()).collect();
    assert_eq!(kinds, vec!["no_determinista", "dominio_incluye_final"]);
    assert_eq!(
        d.issues[0].message,
        "Hay dos transiciones para el estado p y el símbolo a: p --(a,a,D)--> q y p --(a,b,D)--> p."
    );
    assert_eq!(d.issues[0].transitions, vec![0, 1]);
    assert_eq!(
        d.issues[1].message,
        "El dominio no incluye a F: el estado final q tiene la transición q --(a,a,D)--> p."
    );
    assert_eq!(d.issues[1].transitions, vec![2]);
}

#[test]
fn palindrome_table_in_markdown() {
    let t = catedra::transition_table(&palindrome()).unwrap();
    let expected = "\
| Estado Actual | Símbolo Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal |
|---|---|---|---|---|
| →1 | a | 2 | Δ | D |
| →1 | b | 5 | Δ | D |
| →1 | Δ | 8 | Δ | D |
| 2 | a | 2 | a | D |
| 2 | b | 2 | b | D |
| 2 | Δ | 3 | Δ | I |
| 3 | a | 4 | Δ | I |
| 3 | Δ | 8 | Δ | D |
| 4 | a | 4 | a | I |
| 4 | b | 4 | b | I |
| 4 | Δ | 1 | Δ | D |
| 5 | a | 5 | a | D |
| 5 | b | 5 | b | D |
| 5 | Δ | 6 | Δ | I |
| 6 | b | 7 | Δ | I |
| 6 | Δ | 8 | Δ | D |
| 7 | a | 7 | a | I |
| 7 | b | 7 | b | I |
| 7 | Δ | 1 | Δ | D |
| *8 | - | - | - | - |
";
    assert_eq!(t.to_markdown(), expected);
}

#[test]
fn igual_table_orders_symbols_input_auxiliary_delimiters() {
    let t = catedra::transition_table(&igual()).unwrap();
    let expected = "\
Estado Actual  Símbolo Entrada  Estado Nuevo  Símbolo Salida  Movimiento Cabezal
→I             a                A             X               I
→I             b                I             b               D
→I             X                I             X               D
→I             ⟩                D             ⟩               I
A              b                A             b               I
A              X                A             X               I
A              ⟨                B             ⟨               D
B              a                B             a               D
B              b                C             X               I
B              X                B             X               D
C              a                C             a               I
C              X                C             X               I
C              ⟨                I             ⟨               D
D              X                D             X               I
D              ⟨                F             ⟨               D
*F             -                -             -               -
";
    assert_eq!(t.to_text(), expected);
}

#[test]
fn palindrome_runs_reproduce_the_course_examples() {
    let m = palindrome();
    let run = |w: &str| catedra::run(&m, w, Model::Mt, 1000).unwrap();

    let aba = run("aba");
    assert_eq!(aba.verdict, Verdict::Accepted);
    assert_eq!(
        aba.sequence,
        "Δ1abaΔ ├─ ΔΔ2baΔ ├─ ΔΔb2aΔ ├─ ΔΔba2Δ ├─ ΔΔb3aΔ ├─ ΔΔ4bΔΔ ├─ Δ4ΔbΔΔ ├─ ΔΔ1bΔΔ ├─ ΔΔΔ5ΔΔ ├─ ΔΔ6ΔΔΔ ├─ ΔΔΔ8ΔΔ final/acepta"
    );
    assert_eq!(aba.configurations.len(), 11);
    assert_eq!(aba.steps, 10);

    let ab = run("ab");
    assert_eq!(ab.verdict, Verdict::Rejected);
    assert_eq!(
        ab.sequence,
        "Δ1abΔ ├─ ΔΔ2bΔ ├─ ΔΔb2Δ ├─ ΔΔ3bΔ bloqueo/rechazo"
    );

    assert_eq!(
        run("bb").sequence,
        "Δ1bbΔ ├─ ΔΔ5bΔ ├─ ΔΔb5Δ ├─ ΔΔ6bΔ ├─ Δ7ΔΔΔ ├─ ΔΔ1ΔΔ ├─ ΔΔΔ8Δ final/acepta"
    );
    // The course writes "Δ1Δ ├─ ΔΔ8" for λ; the window rule gives one more blank.
    assert_eq!(run("").sequence, "Δ1Δ ├─ ΔΔ8Δ final/acepta");
}

#[test]
fn igual_runs_reproduce_the_course_examples() {
    let m = igual();
    let ab = catedra::run(&m, "ab", Model::All, 1000).unwrap();
    assert_eq!(ab.verdict, Verdict::Accepted);
    assert_eq!(
        ab.sequence,
        "⟨Iab⟩ ├─ A⟨Xb⟩ ├─ ⟨BXb⟩ ├─ ⟨XBb⟩ ├─ ⟨CXX⟩ ├─ C⟨XX⟩ ├─ ⟨IXX⟩ ├─ ⟨XIX⟩ ├─ ⟨XXI⟩ ├─ ⟨XDX⟩ ├─ ⟨DXX⟩ ├─ D⟨XX⟩ ├─ ⟨FXX⟩ final/acepta"
    );
    let b = catedra::run(&m, "b", Model::All, 1000).unwrap();
    assert_eq!(b.verdict, Verdict::Rejected);
    assert_eq!(b.sequence, "⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza");
    // λ: the head starts on ⟩
    let empty = catedra::run(&m, "", Model::All, 1000).unwrap();
    assert_eq!(empty.configurations[0], "⟨I⟩");
}

#[test]
fn a_machine_that_never_stops_hits_the_step_budget() {
    let m = machine(&["p", "q"], "p", &["q"], &["p Δ p Δ D"]);
    let r = catedra::run(&m, "", Model::Mt, 5).unwrap();
    assert_eq!(r.verdict, Verdict::NoHalt);
    assert_eq!(r.steps, 5);
    assert_eq!(
        r.sequence,
        "ΔpΔ ├─ ΔΔpΔ ├─ ΔΔΔpΔ ├─ ΔΔΔΔpΔ ├─ ΔΔΔΔΔpΔ ├─ ΔΔΔΔΔΔpΔ no se detiene (límite de 5 pasos)"
    );
    assert_eq!(r.configurations.len(), 6);
}

#[test]
fn runs_refuse_what_the_course_notation_does_not_cover() {
    let mut two = TuringMachine::new(2);
    let s = two.create_state(Default::default());
    two.set_initial(Some(s));
    assert_eq!(
        catedra::run(&two, "a", Model::Mt, 10).unwrap_err(),
        CatedraError::MultiTape { tapes: 2 }
    );
    let none = TuringMachine::new(1);
    assert_eq!(
        catedra::run(&none, "a", Model::Mt, 10).unwrap_err(),
        CatedraError::NoInitialState
    );
    assert_eq!(
        catedra::run(&igual(), "a⟩", Model::All, 10).unwrap_err(),
        CatedraError::InvalidInput {
            input: "a⟩".into(),
            reason: "la palabra de entrada de un ALL no puede contener ⟨, ⟩ ni Δ".into()
        }
    );
}

#[test]
fn igual_is_a_valid_all() {
    let r = catedra::all_check(&igual(), &strings(&["ab", "b"]), 1000).unwrap();
    assert!(r.ok);
    assert!(r.violations.is_empty());
    assert_eq!(r.runs.len(), 2);
    assert_eq!(r.runs[0].verdict, Verdict::Accepted);
    assert_eq!(r.runs[1].verdict, Verdict::Rejected);
}

#[test]
fn writing_x_over_the_right_delimiter_is_reported_statically_and_dynamically() {
    let mut rows = vec!["I a A X I", "I b I b D", "I X I X D", "I ⟩ D X I"];
    rows.extend(["A b A b I", "A ⟨ B ⟨ D", "D ⟨ F ⟨ D"]);
    let m = machine(&["I", "A", "B", "D", "F"], "I", &["F"], &rows);
    let r = catedra::all_check(&m, &strings(&["b"]), 100).unwrap();
    assert!(!r.ok);
    assert_eq!(r.violations.len(), 2);
    let s = &r.violations[0];
    assert_eq!(s.kind, "delimitador_derecho");
    assert_eq!(
        (s.step, s.input.as_deref(), s.transition),
        (None, None, Some(3))
    );
    assert_eq!(s.transition_text.as_deref(), Some("I --(⟩,X,I)--> D"));
    assert_eq!(
        s.message,
        "La transición I --(⟩,X,I)--> D lee ⟩: debe escribir ⟩ y moverse a la izquierda (I)."
    );
    let d = &r.violations[1];
    assert_eq!(d.kind, "sobrescribe_delimitador");
    assert_eq!(
        (d.step, d.input.as_deref(), d.transition),
        (Some(2), Some("b"), Some(3))
    );
    assert_eq!(
        d.message,
        "Paso 2 con la entrada \"b\": la transición I --(⟩,X,I)--> D escribe X sobre el delimitador ⟩."
    );
    assert_eq!(r.runs[0].verdict, Verdict::Violation);
    assert_eq!(
        r.runs[0].sequence,
        "⟨Ib⟩ ├─ ⟨bI⟩ violación del ALL: Paso 2 con la entrada \"b\": la transición I --(⟩,X,I)--> D escribe X sobre el delimitador ⟩."
    );
}

#[test]
fn moving_right_past_the_right_delimiter_is_a_violation() {
    let m = machine(
        &["I", "D", "F"],
        "I",
        &["F"],
        &["I b I b D", "I ⟩ D ⟩ D", "D ⟨ F ⟨ D"],
    );
    let r = catedra::all_check(&m, &strings(&["b"]), 100).unwrap();
    assert!(!r.ok);
    assert_eq!(r.violations.len(), 2);
    assert_eq!(r.violations[0].kind, "delimitador_derecho");
    let d = &r.violations[1];
    assert_eq!(d.kind, "sale_por_derecha");
    assert_eq!((d.step, d.transition), (Some(2), Some(1)));
    assert_eq!(
        d.message,
        "Paso 2 con la entrada \"b\": la transición I --(⟩,⟩,D)--> D mueve el cabezal a la derecha de ⟩."
    );
}

#[test]
fn moving_left_of_the_left_delimiter_and_using_blanks_are_violations() {
    let m = machine(
        &["I", "F"],
        "I",
        &["F"],
        &["I a I a I", "I ⟨ I ⟨ I", "I Δ F Δ D"],
    );
    let r = catedra::all_check(&m, &strings(&["a"]), 100).unwrap();
    let kinds: Vec<(&str, Option<usize>)> = r
        .violations
        .iter()
        .map(|v| (v.kind.as_str(), v.step))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("delimitador_izquierdo", None),
            ("usa_blanco", None),
            ("sale_por_izquierda", Some(2)),
        ]
    );
    assert_eq!(
        r.violations[0].message,
        "La transición I --(⟨,⟨,I)--> I lee ⟨: debe escribir ⟨ y moverse a la derecha (D)."
    );
    assert_eq!(
        r.violations[1].message,
        "La transición I --(Δ,Δ,D)--> F lee o escribe Δ: un ALL no usa Δ."
    );
}

#[test]
fn writing_a_delimiter_over_another_symbol_is_a_violation() {
    let m = machine(&["I", "F"], "I", &["F"], &["I a F ⟨ D"]);
    let r = catedra::all_check(&m, &[], 100).unwrap();
    assert_eq!(r.violations.len(), 1);
    assert_eq!(r.violations[0].kind, "escribe_delimitador");
    assert_eq!(
        r.violations[0].message,
        "La transición I --(a,⟨,D)--> F escribe ⟨ sobre a: ⟨ y ⟩ solo se escriben sobre sí mismos."
    );
}

#[test]
fn composition_redirects_the_final_states_of_the_first_machine() {
    let m1 = machine(&["s0", "s1"], "s0", &["s1"], &["s0 a s1 a D"]);
    let m2 = machine(&["s0", "s1"], "s0", &["s1"], &["s0 b s1 b D"]);
    let c = compose(&m1, &m2).unwrap();
    let a = c.automaton();
    let names: Vec<&str> = a.states().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["s0", "s1", "s0'", "s1'"]);
    let name = |id| a.state(id).unwrap().name.clone();
    assert_eq!(name(a.initial().unwrap()), "s0");
    let finals: Vec<String> = a.finals().iter().map(|f| name(*f)).collect();
    assert_eq!(finals, vec!["s1'"]);
    let edges: Vec<String> = a
        .transitions()
        .iter()
        .map(|t| {
            format!(
                "{} {} {}",
                name(t.from),
                format_label(&t.label, Notation::Catedra),
                name(t.to)
            )
        })
        .collect();
    assert_eq!(edges, vec!["s0 (a,a,D) s0'", "s0' (b,b,D) s1'"]);
    // MT2 sits to the right of MT1
    let max_x1 = a
        .states()
        .take(2)
        .map(|s| s.point.x)
        .fold(f64::MIN, f64::max);
    let min_x2 = a
        .states()
        .skip(2)
        .map(|s| s.point.x)
        .fold(f64::MAX, f64::min);
    assert!(min_x2 > max_x1);
    // and the composite does what MT1 followed by MT2 does
    let r = catedra::run(&c, "ab", Model::Mt, 100).unwrap();
    assert_eq!(r.verdict, Verdict::Accepted);
    assert_eq!(r.sequence, "Δs0abΔ ├─ Δas0'bΔ ├─ Δabs1'Δ final/acepta");
}

#[test]
fn composition_needs_two_machines_with_an_initial_state_and_equal_tapes() {
    let m1 = machine(&["s0", "s1"], "s0", &["s1"], &["s0 a s1 a D"]);
    let empty = TuringMachine::new(1);
    assert_eq!(
        compose(&m1, &empty).unwrap_err(),
        CatedraError::NoInitialState
    );
    let two = TuringMachine::new(2);
    assert_eq!(
        compose(&m1, &two).unwrap_err(),
        CatedraError::TapeMismatch { a: 1, b: 2 }
    );
}

#[test]
fn multipista_means_every_head_moves_the_same_way() {
    let mut yes = TuringMachine::new(2);
    let (p, q) = (
        yes.create_state(Default::default()),
        yes.create_state(Default::default()),
    );
    yes.add_transition(p, q, parse_label("a;a,R|b;b,R").unwrap())
        .unwrap();
    yes.add_transition(q, p, parse_label("a;a,L|b;c,L").unwrap())
        .unwrap();
    let r = catedra::multipista(&yes);
    assert!(r.applicable);
    assert!(r.is_multipista);
    assert!(r.offending.is_empty());

    let mut no = yes.clone();
    no.add_transition(p, p, parse_label("a;a,R|b;b,S").unwrap())
        .unwrap();
    let r = catedra::multipista(&no);
    assert!(r.applicable);
    assert!(!r.is_multipista);
    assert_eq!(r.offending, vec![2]);

    let r = catedra::multipista(&palindrome());
    assert!(!r.applicable);
}

#[test]
fn states_are_listed_initial_first_then_in_natural_order() {
    // Created in a scrambled order, as a compact document lists them by first use.
    let m = machine(
        &["8", "10", "2", "1"],
        "1",
        &["10"],
        &["1 a 2 Δ D", "2 a 8 a D", "8 a 10 a D"],
    );
    let def = catedra::definition(&m, Some(Model::Mt)).unwrap();
    assert_eq!(def.states, strings(&["1", "2", "8", "10"]));
    let table = catedra::transition_table(&m).unwrap();
    let first_column: Vec<&str> = table.rows.iter().map(|r| r[0].as_str()).collect();
    assert_eq!(first_column, ["→1", "2", "8", "*10"]);

    let igual_scrambled = machine(
        &["F", "D", "C", "B", "A", "I"],
        "I",
        &["F"],
        &[
            "I a A X I",
            "A ⟨ B ⟨ D",
            "B b C X I",
            "C ⟨ I ⟨ D",
            "I ⟩ D ⟩ I",
            "D ⟨ F ⟨ D",
        ],
    );
    let def = catedra::definition(&igual_scrambled, Some(Model::All)).unwrap();
    assert_eq!(def.states, strings(&["I", "A", "B", "C", "D", "F"]));
}
