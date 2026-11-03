//! `rflap catedra ...` and `rflap membership`: the notation of the course "Sintaxis y
//! Semántica de los Lenguajes".

mod common;
use common::*;

/// The palindrome MT of CLASE 39, in the course's label notation.
const PALINDROME: &str = r#"{"type":"turing","states":["1","2","3","4","5","6","7","8"],"initial":"1","finals":["8"],"transitions":[
["1","(a,Δ,D)","2"],["1","(b,Δ,D)","5"],["1","(Δ,Δ,D)","8"],
["2","(a,a,D) / (b,b,D)","2"],["2","(Δ,Δ,I)","3"],
["3","(a,Δ,I)","4"],["3","(Δ,Δ,D)","8"],
["4","(a,a,I) / (b,b,I)","4"],["4","(Δ,Δ,D)","1"],
["5","(a,a,D) / (b,b,D)","5"],["5","(Δ,Δ,I)","6"],
["6","(b,Δ,I)","7"],["6","(Δ,Δ,D)","8"],
["7","(a,a,I) / (b,b,I)","7"],["7","(Δ,Δ,D)","1"]]}"#;

/// The ALL "IGUAL" of CLASE 39.
const IGUAL: &str = r#"{"type":"turing","states":["I","A","B","C","D","F"],"initial":"I","finals":["F"],"transitions":[
["I","a,X,I","A"],["I","(b,b,D) / (X,X,D)","I"],["I","(⟩,⟩,I)","D"],
["A","(b,b,I) / (X,X,I)","A"],["A","(⟨,⟨,D)","B"],
["B","(a,a,D)","B"],["B","(b,X,I)","C"],["B","(X,X,D)","B"],
["C","(a,a,I) / (X,X,I)","C"],["C","(⟨,⟨,D)","I"],
["D","(X,X,I)","D"],["D","(⟨,⟨,D)","F"]]}"#;

/// A tiny machine: a, then any a or b, then stop.
const TINY: &str = r#"{"type":"turing","states":["1","2","3"],"initial":"1","finals":["3"],"transitions":[["1","(a,a,D)","2"],["2","(a,a,D) / (b,b,D)","2"],["2","(Δ,Δ,N)","3"]]}"#;

const COURSE_GRAMMAR: &str = "S -> T\nDB -> BD\nD -> c\nT -> aTBD\nT -> abD\nbB -> bb";

#[test]
fn table_prints_aligned_text() {
    let t = Temp::new();
    let f = t.write("tiny.json", TINY);
    let out = rflap(&["catedra", "table", &f]);
    assert_eq!(
        out.stdout,
        "\
Estado Actual  Símbolo Entrada  Estado Nuevo  Símbolo Salida  Movimiento Cabezal
→1             a                2             a               D
2              a                2             a               D
2              b                2             b               D
2              Δ                3             Δ               N
*3             -                -             -               -
"
    );
    assert_eq!(out.stderr, "");
    assert_eq!(out.code, 0);
}

#[test]
fn table_markdown() {
    let t = Temp::new();
    let f = t.write("tiny.json", TINY);
    let out = rflap(&["catedra", "table", &f, "--markdown"]);
    assert_eq!(
        out.stdout,
        "\
| Estado Actual | Símbolo Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal |
|---|---|---|---|---|
| →1 | a | 2 | a | D |
| 2 | a | 2 | a | D |
| 2 | b | 2 | b | D |
| 2 | Δ | 3 | Δ | N |
| *3 | - | - | - | - |
"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn definition_prints_the_sets() {
    let t = Temp::new();
    let f = t.write("pal.json", PALINDROME);
    let out = rflap(&["catedra", "definition", &f]);
    assert_eq!(
        out.stdout,
        "MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩\nQ = {1, 2, 3, 4, 5, 6, 7, 8}\nΣ_E = {a, b} (inferido)\nΣ_A = {Δ}\nq0 = 1\nF = {8}\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn definition_lists_the_warnings() {
    let t = Temp::new();
    let f = t.write(
        "bad.json",
        r#"{"type":"turing","initial":"p","finals":["q"],"transitions":[["p","a,a,D","q"],["q","a,a,D","p"]]}"#,
    );
    let out = rflap(&["catedra", "definition", &f, "--model", "mt"]);
    assert_eq!(
        out.stdout,
        "MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩\nQ = {p, q}\nΣ_E = {a} (inferido)\nΣ_A = {Δ}\nq0 = p\nF = {q}\nAvisos:\n- El dominio no incluye a F: el estado final q tiene la transición q --(a,a,D)--> p.\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn run_prints_the_configuration_sequences() {
    let t = Temp::new();
    let f = t.write("pal.json", PALINDROME);
    let out = rflap(&["catedra", "run", &f, "aba", "bb"]);
    assert_eq!(
        out.stdout,
        "\"aba\": Δ1abaΔ ├─ ΔΔ2baΔ ├─ ΔΔb2aΔ ├─ ΔΔba2Δ ├─ ΔΔb3aΔ ├─ ΔΔ4bΔΔ ├─ Δ4ΔbΔΔ ├─ ΔΔ1bΔΔ ├─ ΔΔΔ5ΔΔ ├─ ΔΔ6ΔΔΔ ├─ ΔΔΔ8ΔΔ final/acepta\n\"bb\": Δ1bbΔ ├─ ΔΔ5bΔ ├─ ΔΔb5Δ ├─ ΔΔ6bΔ ├─ Δ7ΔΔΔ ├─ ΔΔ1ΔΔ ├─ ΔΔΔ8Δ final/acepta\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn run_exit_1_when_an_input_is_rejected() {
    let t = Temp::new();
    let f = t.write("pal.json", PALINDROME);
    let out = rflap(&["catedra", "run", &f, "ab", "λ"]);
    assert_eq!(
        out.stdout,
        "\"ab\": Δ1abΔ ├─ ΔΔ2bΔ ├─ ΔΔb2Δ ├─ ΔΔ3bΔ bloqueo/rechazo\n\"\": Δ1Δ ├─ ΔΔ8Δ final/acepta\n"
    );
    assert_eq!(out.code, 1);
}

#[test]
fn run_all_uses_the_linear_bounded_automaton_model() {
    let t = Temp::new();
    let f = t.write("igual.json", IGUAL);
    let out = rflap(&["catedra", "run", &f, "ab", "--all"]);
    assert_eq!(
        out.stdout,
        "\"ab\": ⟨Iab⟩ ├─ A⟨Xb⟩ ├─ ⟨BXb⟩ ├─ ⟨XBb⟩ ├─ ⟨CXX⟩ ├─ C⟨XX⟩ ├─ ⟨IXX⟩ ├─ ⟨XIX⟩ ├─ ⟨XXI⟩ ├─ ⟨XDX⟩ ├─ ⟨DXX⟩ ├─ D⟨XX⟩ ├─ ⟨FXX⟩ final/acepta\n"
    );
    assert_eq!(out.code, 0);
    let out = rflap(&["catedra", "run", &f, "b", "--all"]);
    assert_eq!(out.stdout, "\"b\": ⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza\n");
    assert_eq!(out.code, 1);
}

#[test]
fn run_exit_3_when_the_machine_does_not_stop() {
    let t = Temp::new();
    let f = t.write(
        "loop.json",
        r#"{"type":"turing","initial":"p","finals":["q"],"transitions":[["p","(Δ,Δ,D)","p"]]}"#,
    );
    let out = rflap(&["catedra", "run", &f, "λ", "--max-steps", "2"]);
    assert_eq!(
        out.stdout,
        "\"\": ΔpΔ ├─ ΔΔpΔ ├─ ΔΔΔpΔ no se detiene (límite de 2 pasos)\n"
    );
    assert_eq!(out.code, 3);
}

#[test]
fn run_json_prints_the_raw_api_answer() {
    let t = Temp::new();
    let f = t.write("tiny.json", TINY);
    let out = rflap(&["catedra", "run", &f, "a", "--json"]);
    assert_eq!(
        out.stdout,
        "{\"ok\":true,\"op\":\"catedra_run\",\"result\":{\"max_steps\":1000,\"model\":\"mt\",\"results\":[{\"accepted\":true,\"configurations\":[\"Δ1aΔ\",\"Δa2Δ\",\"Δa3Δ\"],\"input\":\"a\",\"model\":\"mt\",\"notes\":[],\"sequence\":\"Δ1aΔ ├─ Δa2Δ ├─ Δa3Δ final/acepta\",\"steps\":2,\"verdict\":\"accepted\"}]}}\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn run_needs_an_input() {
    let t = Temp::new();
    let f = t.write("tiny.json", TINY);
    let out = rflap(&["catedra", "run", &f]);
    assert_eq!(out.code, 2);
    assert_eq!(
        out.stderr,
        "error: invalid_argument: give at least one input (arguments or --inputs-file)\n"
    );
}

#[test]
fn run_refuses_a_multi_tape_machine() {
    let t = Temp::new();
    let f = t.write(
        "two.json",
        r#"{"type":"turing","initial":"p","transitions":[["p","a;a,R|b;b,R","p"]]}"#,
    );
    let out = rflap(&["catedra", "run", &f, "a"]);
    assert_eq!(out.code, 2);
    assert_eq!(
        out.stderr,
        "error: unsupported_option: La notación de la cátedra es para máquinas de una cinta; esta tiene 2 cintas.\n"
    );
}

#[test]
fn all_check_accepts_igual() {
    let t = Temp::new();
    let f = t.write("igual.json", IGUAL);
    let out = rflap(&["catedra", "all-check", &f, "b"]);
    assert_eq!(
        out.stdout,
        "\"b\": ⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza\nLa máquina cumple las reglas de un ALL.\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn all_check_lists_the_violations() {
    let t = Temp::new();
    let f = t.write(
        "bad.json",
        r#"{"type":"turing","states":["I","D","F"],"initial":"I","finals":["F"],"transitions":[["I","(b,b,D)","I"],["I","(⟩,⟩,D)","D"],["D","(⟨,⟨,D)","F"]]}"#,
    );
    let out = rflap(&["catedra", "all-check", &f, "b"]);
    assert_eq!(
        out.stdout,
        "\"b\": ⟨Ib⟩ ├─ ⟨bI⟩ violación del ALL: Paso 2 con la entrada \"b\": la transición I --(⟩,⟩,D)--> D mueve el cabezal a la derecha de ⟩.\nLa máquina NO cumple las reglas de un ALL:\n- La transición I --(⟩,⟩,D)--> D lee ⟩: debe escribir ⟩ y moverse a la izquierda (I).\n- Paso 2 con la entrada \"b\": la transición I --(⟩,⟩,D)--> D mueve el cabezal a la derecha de ⟩.\n"
    );
    assert_eq!(out.code, 1);
}

#[test]
fn compose_writes_the_composite_machine() {
    let t = Temp::new();
    let a = t.write(
        "a.json",
        r#"{"type":"turing","initial":"s0","finals":["s1"],"transitions":[["s0","(a,a,D)","s1"]]}"#,
    );
    let b = t.write(
        "b.json",
        r#"{"type":"turing","initial":"s0","finals":["s1"],"transitions":[["s0","(b,b,D)","s1"]]}"#,
    );
    let o = t.path("ab.json");
    let out = rflap(&["catedra", "compose", &a, &b, "-o", &o]);
    assert_eq!(out.stdout, format!("wrote {o}\n"));
    assert_eq!(out.code, 0);
    let run = rflap(&["catedra", "run", &o, "ab"]);
    assert_eq!(
        run.stdout,
        "\"ab\": Δs0abΔ ├─ Δas0'bΔ ├─ Δabs1'Δ final/acepta\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn membership_prints_the_sets_and_exits_1_when_the_word_is_not_in() {
    let out = rflap(&["membership", "--grammar", COURSE_GRAMMAR, "abbc"]);
    assert_eq!(
        out.stdout,
        "T0 = { S }\nT1 = { S, T }\nT2 = { S, T, aTBD, abD }\nT3 = { S, T, aTBD, abD, aTBc, abc }\nT4 = T3\nComo T4 = T3, \"abbc\" no pertenece al lenguaje.\n"
    );
    assert_eq!(out.code, 1);
}

#[test]
fn membership_reads_a_grammar_file_and_exits_0_for_a_member() {
    let t = Temp::new();
    let f = t.write("g.txt", COURSE_GRAMMAR);
    let out = rflap(&["membership", &f, "abc"]);
    assert_eq!(
        out.stdout,
        "T0 = { S }\nT1 = { S, T }\nT2 = { S, T, abD }\nT3 = { S, T, abD, abc }\n\"abc\" ∈ T3: la palabra pertenece al lenguaje.\n"
    );
    assert_eq!(out.code, 0);
}

#[test]
fn membership_refuses_contracting_grammars() {
    let out = rflap(&[
        "membership",
        "--grammar",
        "S -> aAb\\naA -> a\\nA -> c",
        "ab",
    ]);
    assert_eq!(out.code, 2);
    assert_eq!(
        out.stderr,
        "error: contracting_grammar: La gramática es compresora (la producción aA → a tiene |α| > |β|): la comprobación por los conjuntos Tk solo vale para gramáticas no compresoras.\n"
    );
}

// ALL shrink (S1) and delimiters in the input (S4)

const SHRINK: &str =
    include_str!("../../rflap-api/tests/fixtures/catedra/ALL_2.1e_m_igual_2p_mas_1.jff");

#[test]
fn all_check_prints_the_shrink_notes_and_passes() {
    let t = Temp::new();
    let f = t.write("e.jff", SHRINK);
    let out = rflap(&["catedra", "all-check", &f, "aab", "aabb"]);
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(
        out.stdout
            .contains("ℹ️ q0 escribe ⟨ sobre a: la cinta se achica\n"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.starts_with(
            "\"aab\": ⟨q0aab⟩ ├─ ⟨⟨q1ab⟩ ├─ ⟨⟨⟨q2b⟩ ├─ ⟨⟨⟨⟨q3⟩ ├─ ⟨⟨⟨qf⟨⟩ final/acepta\n"
        ),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .ends_with("La máquina cumple las reglas de un ALL.\n"),
        "{}",
        out.stdout
    );
}

#[test]
fn all_check_no_shrink_restores_the_strict_rule() {
    let t = Temp::new();
    let f = t.write("e.jff", SHRINK);
    let out = rflap(&["catedra", "all-check", &f, "aab", "--no-shrink"]);
    assert_eq!(out.code, 1);
    assert!(!out.stdout.contains("ℹ️"), "{}", out.stdout);
    assert!(
        out.stdout.contains(
            "- La transición q0 --(a,⟨,D)--> q1 escribe ⟨ sobre a: ⟨ y ⟩ solo se escriben sobre sí mismos.\n"
        ),
        "{}",
        out.stdout
    );
}

#[test]
fn run_prints_the_notes_after_the_sequence_and_honors_no_shrink() {
    let t = Temp::new();
    let f = t.write("e.jff", SHRINK);
    let out = rflap(&["catedra", "run", &f, "<aab>"]);
    assert_eq!(out.code, 0, "{}", out.stdout);
    let lines: Vec<&str> = out.stdout.lines().collect();
    assert!(
        lines[0].starts_with("\"<aab>\": ⟨q0aab⟩ ├─"),
        "{}",
        out.stdout
    );
    assert_eq!(
        lines[1],
        "ℹ️ La entrada «<aab>» ya traía los delimitadores; se quitaron y se corre ⟨aab⟩."
    );
    assert!(lines.contains(&"ℹ️ q0 escribe ⟨ sobre a: la cinta se achica"));
    let strict = rflap(&["catedra", "run", &f, "aab", "--no-shrink"]);
    assert_eq!(strict.code, 1);
    assert!(
        strict.stdout.contains("violación del ALL"),
        "{}",
        strict.stdout
    );
}
