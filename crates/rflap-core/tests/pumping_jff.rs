//! `.jff` read/write for the pumping lemma games (JFLAP `RegPumpingLemmaTransducer`,
//! `CFPumpingLemmaTransducer`). The `pumping_*.jff` fixtures were written by JFLAP 7.1
//! itself (`fixtures/t8/PHarness.java`).

use rflap_core::jff::{JffError, structure_type};
use rflap_core::pumping::{CfLang, Game, Kind, Language, Lemma, Mode, RegLang, jff};

const REG_ANBN: &str = include_str!("fixtures/t8/pumping_reg_anbn.jff");
const REG_FRESH: &str = include_str!("fixtures/t8/pumping_reg_fresh.jff");
const REG_ANEVEN: &str = include_str!("fixtures/t8/pumping_reg_aneven.jff");
const CF_ANBNCN: &str = include_str!("fixtures/t8/pumping_cf_anbncn.jff");
const CF_FRESH: &str = include_str!("fixtures/t8/pumping_cf_fresh.jff");
const CF_WW: &str = include_str!("fixtures/t8/pumping_cf_ww.jff");

#[test]
fn reads_a_regular_game_jflap_wrote() {
    assert_eq!(structure_type(REG_ANBN).unwrap(), "regular pumping lemma");
    let l = jff::read(REG_ANBN).unwrap();
    assert_eq!(l.language(), Language::Regular(RegLang::AnBn));
    assert_eq!(l.kind(), Kind::Regular);
    assert_eq!(l.first_player(), "Computer");
    assert_eq!((l.m(), l.w(), l.i()), (5, "aaaaabbbbb", 0));
    assert_eq!((l.x(), l.y(), l.z()), ("a", "aa", "aabbbbb"));
    assert_eq!(l.decomposition(), Some(&[1, 2][..]));
    assert_eq!(
        l.attempts(),
        [
            "X = a;   Y = aa;   Z = aaabbbbb; I = 0; <i>Won</i>",
            "X = λ;   Y = a;   Z = aaaabbbbb; I = 2; <i>Won</i>"
        ]
    );
    assert_eq!(l.create_pumped_string(), "aaabbbbb");
    assert!(!l.is_in_lang(&l.create_pumped_string()));
}

#[test]
fn reads_a_fresh_game_and_one_in_progress() {
    let fresh = jff::read(REG_FRESH).unwrap();
    assert_eq!(fresh.language(), Language::Regular(RegLang::AnEven));
    assert_eq!((fresh.m(), fresh.w(), fresh.i()), (-1, "", -1));
    assert_eq!(fresh.first_player(), "Human");
    assert_eq!(fresh.decomposition(), Some(&[0, 0][..]));
    assert_eq!((fresh.x(), fresh.y(), fresh.z()), ("", "", ""));

    let l = jff::read(REG_ANEVEN).unwrap();
    assert_eq!((l.m(), l.w(), l.i()), (3, "aaaa", 3));
    assert_eq!((l.x(), l.y(), l.z()), ("", "aa", "aa"));
    assert_eq!(l.create_pumped_string(), "aaaaaaaa");
}

#[test]
fn reads_a_context_free_game_with_its_cases() {
    assert_eq!(
        structure_type(CF_ANBNCN).unwrap(),
        "context-free pumping lemma"
    );
    let l = jff::read(CF_ANBNCN).unwrap();
    assert_eq!(l.language(), Language::ContextFree(CfLang::AnBnCn));
    assert_eq!((l.m(), l.w(), l.i()), (4, "aaaabbbbcccc", 2));
    assert_eq!(l.first_player(), "Human");
    // the last addCase of the file set the decomposition (7, 1, 0, 1)
    assert_eq!(
        (l.u(), l.v(), l.x(), l.y(), l.z()),
        ("aaaabbb", "b", "", "c", "ccc")
    );
    assert_eq!(l.attempts().len(), 1);
    let cases = l.done_cases();
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[0].input, [0, 1, 0, 1]);
    assert_eq!(cases[0].i, 2);
    assert_eq!(
        cases[0].description,
        "v is a string of \"a\"s and y is a string of \"a\"s"
    );
    assert_eq!(cases[1].input, [7, 1, 0, 1]);
    assert_eq!(cases[1].i, 0);
    assert_eq!(
        cases[1].description,
        "v is a string of \"b\"s and y is a string of \"c\"s"
    );

    let fresh = jff::read(CF_FRESH).unwrap();
    assert_eq!(fresh.language(), Language::ContextFree(CfLang::W1BnW2));
    assert_eq!(fresh.first_player(), "Computer");
    assert_eq!((fresh.m(), fresh.w(), fresh.i()), (3, "aaabbbbaaaa", -1));
    let ww = jff::read(CF_WW).unwrap();
    assert_eq!(ww.first_player(), "");
    assert_eq!(ww.language(), Language::ContextFree(CfLang::WW));
}

#[test]
fn rewrites_every_file_jflap_wrote_byte_for_byte() {
    for (name, text) in [
        ("reg_anbn", REG_ANBN),
        ("reg_fresh", REG_FRESH),
        ("reg_aneven", REG_ANEVEN),
        ("cf_anbncn", CF_ANBNCN),
        ("cf_fresh", CF_FRESH),
        ("cf_ww", CF_WW),
    ] {
        let l = jff::read(text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(jff::write(&l), text, "{name}");
    }
}

fn same_state(a: &Lemma, b: &Lemma) {
    assert_eq!(a.language(), b.language());
    assert_eq!(a.first_player(), b.first_player());
    assert_eq!((a.m(), a.w(), a.i()), (b.m(), b.w(), b.i()));
    assert_eq!(
        (a.u(), a.v(), a.x(), a.y(), a.z()),
        (b.u(), b.v(), b.x(), b.y(), b.z())
    );
    assert_eq!(a.attempts(), b.attempts());
    assert_eq!(a.done_cases(), b.done_cases());
}

#[test]
fn a_regular_game_survives_a_round_trip() {
    let mut game = Game::new(Language::Regular(RegLang::AnBn), Mode::ComputerFirst, 21);
    let m = game.lemma().m() as usize;
    game.choose_w(&format!("{}{}", "a".repeat(m), "b".repeat(m)));
    game.choose_i(2);
    game.choose_i(0);
    let text = jff::write(game.lemma());
    assert!(text.contains("<type>regular pumping lemma</type>"));
    assert!(text.contains("<name>a^n b^n : n &gt;= 0</name>"));
    assert!(text.contains("<first_player>Computer</first_player>"));
    let back = jff::read(&text).unwrap();
    same_state(game.lemma(), &back);
    assert_eq!(jff::write(&back), text);
    // the resumed game shows the verdict again
    let resumed = Game::from_lemma(back);
    assert_eq!(resumed.mode(), Mode::ComputerFirst);
    assert_eq!(resumed.view().notice, "File loaded.");
    assert!(resumed.result().unwrap().user_wins);
    assert_eq!(resumed.view().awaiting, rflap_core::pumping::Awaiting::I);
}

#[test]
fn a_context_free_game_with_cases_survives_a_round_trip() {
    let mut game = Game::new(Language::ContextFree(CfLang::AnBnCn), Mode::HumanFirst, 4);
    game.choose_m(5);
    game.choose_decomposition(&[0, 1, 0, 1]);
    game.add_case();
    game.choose_decomposition(&[5, 1, 0, 1]);
    game.add_case();
    game.choose_decomposition(&[0, 0, 0, 1]);
    let text = jff::write(game.lemma());
    assert_eq!(text.matches("<case>").count(), 2);
    let back = jff::read(&text).unwrap();
    same_state(game.lemma(), &back);
    assert_eq!(jff::write(&back), text);
    let resumed = Game::from_lemma(back);
    assert_eq!(resumed.mode(), Mode::HumanFirst);
    assert_eq!(resumed.view().cases.unwrap().rows.len(), 2);
    assert!(resumed.result().is_some());
}

#[test]
fn rejects_files_that_are_not_pumping_games() {
    let fa = "<structure><type>fa</type><automaton/></structure>";
    assert!(matches!(jff::read(fa), Err(JffError::WrongType { .. })));
    let unknown = REG_FRESH.replace("a^n : n is even", "no such language");
    assert!(
        matches!(jff::read(&unknown), Err(JffError::BadXml(m)) if m.contains("no such language"))
    );
    // a context-free title in a regular file is just as unknown
    let swapped = REG_FRESH.replace("a^n : n is even", "ww : w element_of {ab}*");
    assert!(matches!(jff::read(&swapped), Err(JffError::BadXml(_))));
    let no_m = REG_FRESH.replace("<m>-1</m>", "");
    assert!(
        matches!(jff::read(&no_m), Err(JffError::MissingElement { element, .. }) if element == "m")
    );
    let bad_i = REG_FRESH.replace("<i>-1</i>", "<i> 3</i>");
    assert!(matches!(jff::read(&bad_i), Err(JffError::BadNumber { .. })));
    let bad_x = CF_WW.replace("<xLength>0</xLength>", "<xLength>x</xLength>");
    assert!(matches!(jff::read(&bad_x), Err(JffError::BadNumber { .. })));
    let no_y = CF_ANBNCN.replace("<yLength>1</yLength>", "");
    assert!(
        matches!(jff::read(&no_y), Err(JffError::MissingElement { element, .. }) if element == "yLength")
    );
}

/// Writes samples for `PHarness load` (JFLAP's own reader):
/// `RFLAP_T8_OUT=<dir> cargo test -p rflap-core --test pumping_jff -- --ignored`.
#[test]
#[ignore = "writes files for the JFLAP jar to read"]
fn write_samples_for_the_jflap_jar() {
    let dir = std::env::var("RFLAP_T8_OUT").expect("set RFLAP_T8_OUT to a directory");
    let mut reg = Game::new(Language::Regular(RegLang::AnBn), Mode::ComputerFirst, 21);
    let m = reg.lemma().m() as usize;
    reg.choose_w(&format!("{}{}", "a".repeat(m), "b".repeat(m)));
    reg.choose_i(2);
    reg.choose_i(0);
    std::fs::write(format!("{dir}/rust_reg.jff"), jff::write(reg.lemma())).unwrap();
    let mut cf = Game::new(Language::ContextFree(CfLang::AnBnCn), Mode::HumanFirst, 4);
    cf.choose_m(5);
    cf.choose_decomposition(&[0, 1, 0, 1]);
    cf.add_case();
    cf.choose_decomposition(&[5, 1, 0, 1]);
    cf.add_case();
    std::fs::write(format!("{dir}/rust_cf.jff"), jff::write(cf.lemma())).unwrap();
}
