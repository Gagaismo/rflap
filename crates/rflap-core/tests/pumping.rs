//! The pumping lemma games (JFLAP `pumping` and `gui.pumping`).

use rflap_core::pumping::{
    Awaiting, CfLang, Conclusion, Game, Kind, Language, Lemma, Mode, Move, RegLang, catalogue,
};

fn titles(kind: Kind) -> Vec<&'static str> {
    catalogue(kind).iter().map(|l| l.title()).collect()
}

fn reg(l: RegLang) -> Language {
    Language::Regular(l)
}

fn cf(l: CfLang) -> Language {
    Language::ContextFree(l)
}

#[test]
fn the_regular_catalogue_is_jflaps_in_order() {
    assert_eq!(
        titles(Kind::Regular),
        [
            "a^n b^n : n >= 0",
            "w element_of {ab}* : na(w) < nb(w)",
            "w w^R : w element_of {ab}*",
            "(ab)^n a^k : n > k, k >= 0",
            "a^n b^k c^(n+k) : n >= 0, k >= 0",
            "a^n b^l a^k : n > 5, l > 3, k <= l",
            "a^n : n is even",
            "a^n b^k : n is odd or k is even",
            "bba(ba)^n a^(n-1)",
            "b^5w: w element_of {ab}* : 2na(w) = 3nb(w)",
            "b^5w: w element_of {ab}* : (2na(w) + 5nb(w)) % 3 = 0",
            "b^k (ab)^n (ba)^n: k>=4, n = 1,2,...",
            "(ab)^2n : n = 1,2,...",
        ]
    );
}

#[test]
fn the_context_free_catalogue_is_jflaps_in_order() {
    assert_eq!(
        titles(Kind::ContextFree),
        [
            "a^n b^n c^n",
            "ww : w element_of {ab}*",
            "a^n b^j a^n b^j : n >= 0, j >= 0",
            "w element_of {a, b, c}* : na(w) < nb(w) < nc(w)",
            "w element_of {abc}* : na(w) > nb(w) = nc(w)",
            "a^i b^j c^k : i > j, i > k",
            "a^n b^n : n >= 0",
            "a^k b^n c^n d^j : j != k",
            "w w1 w^R : |w1| >= 5, w & w1 element_of {ab}*",
            "w w1 w^R : |w| = |w1|, w & w1 element_of {ab}*",
            "w1 + b^n + w2 : na(w1) < na(w2) & na(w1) < n, w1 & w2 element_of {ab}*",
            "w1cw2cw3cw4 : w1 = w2 or w3 = w4, wi element_of {ab}*, |wi| >= 5",
            "w1 v v^R w2 : na(w1) = na(w2), |v|>=3, w1 & w2 element_of {ab}*",
        ]
    );
}

#[test]
fn a_language_carries_its_description_and_range() {
    let info = reg(RegLang::AnBn).info();
    assert_eq!(
        info.html_title,
        "<i>a<sup>n</sup>b<sup>n</sup></i> : <i>n</i> &#8805; 0"
    );
    assert!(!info.partition_is_valid);
    assert_eq!(info.range, (4, 18));
    assert_eq!(info.cases, 0);
    assert!(
        info.explanation
            .ends_with("Thus, the language is not regular.")
    );
    let info = cf(CfLang::AnBnCn).info();
    assert_eq!((info.range, info.cases), ((3, 11), 11));
    assert_eq!(cf(CfLang::AkBnCnDj).info().cases, 15);
    assert!(reg(RegLang::AnEven).info().partition_is_valid);
    assert_eq!(
        Language::from_title(Kind::Regular, "a^n : n is even"),
        Some(reg(RegLang::AnEven))
    );
    assert_eq!(
        Language::from_title(Kind::ContextFree, "a^n : n is even"),
        None
    );
    assert_eq!(cf(CfLang::W1VVrW2).index(), 12);
}

#[test]
fn membership_follows_jflaps_tests() {
    let l = |lang| Lemma::new(lang, 0);
    let anbn = l(reg(RegLang::AnBn));
    assert!(anbn.is_in_lang("") && anbn.is_in_lang("aabb"));
    assert!(!anbn.is_in_lang("aab") && !anbn.is_in_lang("ba"));
    let pal = l(reg(RegLang::Palindrome));
    assert!(pal.is_in_lang("abba") && pal.is_in_lang("") && !pal.is_in_lang("abb"));
    let ww = l(cf(CfLang::WW));
    assert!(ww.is_in_lang("abab") && !ww.is_in_lang("abba") && !ww.is_in_lang("aba"));
    let abc = l(cf(CfLang::AnBnCn));
    assert!(abc.is_in_lang("aabbcc") && !abc.is_in_lang("abcabc"));
    let w1cw2 = l(cf(CfLang::W1CW2CW3CW4));
    assert!(w1cw2.is_in_lang("abcabcabcb") && !w1cw2.is_in_lang("abcbacabcb"));
    let vvr = l(cf(CfLang::W1VVrW2));
    assert!(vvr.is_in_lang("abbbba") && !vvr.is_in_lang("abab"));
}

/// The regular game, computer first, played on a^n b^n.
#[test]
fn computer_first_anbn_reaches_not_regular() {
    let mut game = Game::new(reg(RegLang::AnBn), Mode::ComputerFirst, 11);
    let view = game.view();
    assert_eq!(view.awaiting, Awaiting::W);
    assert_eq!(
        view.objective,
        "Prevent the computer from finding a valid partition."
    );
    assert_eq!(view.title, "L = {a^n b^n : n >= 0} Regular Pumping Lemma");
    assert_eq!(
        view.prompt,
        "Please enter a possible value for w and press \"Enter\"."
    );
    assert_eq!(
        view.boxes[0].caption,
        "I have selected a value for m, displayed below."
    );
    let m = game.lemma().m();
    assert!((4..=18).contains(&m));
    assert_eq!(view.boxes[0].content, m.to_string());

    // too short, then out of the language
    let r = game.choose_w("ab");
    assert!(!r.accepted);
    assert_eq!(r.message, "Remember |w| must be >= m");
    let r = game.choose_w(&"a".repeat(m as usize + 1));
    assert_eq!(
        r.message,
        "That string was not in the language.  Please enter another."
    );
    assert!(!game.view().boxes[2].visible);
    assert_eq!(game.choose_i(2).message, "Enter w first (box 2).");

    let w = format!("{}{}", "a".repeat(m as usize), "b".repeat(m as usize));
    assert!(game.choose_w(&w).accepted);
    assert_eq!(game.view().awaiting, Awaiting::I);
    assert!(game.view().boxes[3].visible);
    let text = game.view().boxes[2].content.clone();
    assert!(text.starts_with("X = ") && text.contains(";   Y = ") && text.contains(";   Z = "));
    let x = game.lemma().x().len();
    let y = game.lemma().y().len();
    assert!(y >= 1 && x + y <= m as usize);

    for bad in [1, -1, 13] {
        let r = game.choose_i(bad);
        assert!(!r.accepted);
        assert_eq!(
            r.message,
            "Please enter a positive integer in range [0, 2...12] for best results."
        );
    }
    assert!(game.result().is_none());

    let r = game.choose_i(2);
    assert!(r.accepted);
    let result = game.result().unwrap().clone();
    assert!(result.user_wins && !result.in_language);
    assert!(
        result
            .message
            .ends_with("is NOT in the language.  YOU WIN!")
    );
    assert!(
        result
            .message
            .starts_with("<i>xy</i><sup>2</sup><i>z</i> = ")
    );
    assert_eq!(game.conclusion(), Some(Conclusion::NotRegular));
    let explain = game.explain();
    assert!(!explain.valid);
    assert_eq!(
        explain.headline,
        "Unfortunately no valid partition of w exists."
    );
    assert!(
        explain
            .explanation
            .contains("Thus, the language is not regular.")
    );
    assert_eq!(explain.attempts.len(), 1);
    assert!(explain.attempts[0].1.ends_with("I = 2; <i>Won</i>"));

    // a second attempt is listed first
    game.choose_i(0);
    let attempts = game.explain().attempts;
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].0, 2);
    assert!(attempts[0].1.ends_with("I = 0; <i>Won</i>"));
    // the verdict text for i = 0 pumps y away
    assert_eq!(game.view().pumped, game.result().unwrap().pumped);
    assert!(!game.lemma().is_in_lang(&game.view().pumped));
}

#[test]
fn computer_first_keeps_the_computers_valid_partition_in_a_regular_language() {
    // a^n : n is even - the computer pumps "aa" and every i stays in the language
    let mut game = Game::new(reg(RegLang::AnEven), Mode::ComputerFirst, 5);
    let m = game.lemma().m() as usize;
    let w = "a".repeat(m + m % 2);
    assert!(game.choose_w(&w).accepted);
    for i in [0, 2, 3, 7, 12] {
        let r = game.choose_i(i);
        assert!(r.accepted, "{i}");
        assert!(!game.result().unwrap().user_wins, "{i}");
        assert!(
            r.message
                .ends_with("is in the language.  Please try again."),
            "{i}: {}",
            r.message
        );
        assert!(!game.result().unwrap().message.contains("YOU WIN"));
    }
    assert_eq!(game.conclusion(), None);
    assert!(game.explain().valid);
    assert_eq!(game.explain().headline, "A valid partition of w exists!");
    assert!(
        game.explain()
            .attempts
            .iter()
            .all(|(_, a)| a.ends_with("<i>Failed</i>"))
    );
}

#[test]
fn the_computer_finds_a_valid_decomposition_for_every_regular_language_jflap_marks_regular() {
    let regular = [
        RegLang::AnEven,
        RegLang::AnBk,
        RegLang::B5Wmod,
        RegLang::AB2n,
    ];
    for lang in regular {
        let language = reg(lang);
        assert!(language.info().partition_is_valid, "{lang:?}");
        let (lo, hi) = language.info().range;
        for seed in 0..6 {
            for m in lo..=hi {
                let mut l = Lemma::new(language, seed);
                l.set_m(m);
                let w = l.w().to_string();
                assert!(
                    l.is_in_lang(&w) && w.len() as i32 >= m,
                    "{lang:?} m={m} w={w}"
                );
                l.choose_decomposition();
                let (x, y, z) = (l.x().to_string(), l.y().to_string(), l.z().to_string());
                assert_eq!(format!("{x}{y}{z}"), w, "{lang:?} m={m}");
                assert!(
                    !y.is_empty() && (x.len() + y.len()) as i32 <= m,
                    "{lang:?} m={m}"
                );
                for i in [0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
                    l.set_i(i);
                    let pumped = l.create_pumped_string();
                    assert!(l.is_in_lang(&pumped), "{lang:?} m={m} i={i} {pumped}");
                }
            }
        }
    }
}

#[test]
fn the_computer_cannot_pump_the_languages_jflap_marks_not_regular() {
    // JFLAP's own w and the computer's split: its chooseI breaks the string
    let not_regular = [
        RegLang::AnBn,
        RegLang::NaNb,
        RegLang::Palindrome,
        RegLang::ABnAk,
        RegLang::AnBkCnk,
        RegLang::AnBlAk,
        RegLang::BBABAnAn,
        RegLang::B5W,
        RegLang::BkABnBAn,
    ];
    for lang in not_regular {
        let language = reg(lang);
        assert!(!language.info().partition_is_valid, "{lang:?}");
        let (lo, hi) = language.info().range;
        for m in lo..=hi {
            let mut l = Lemma::new(language, 3);
            l.set_m(m);
            let w = l.w().to_string();
            l.choose_decomposition();
            l.choose_i();
            // the computer's i is one of its own: 0, 1.. - at least one pumping leaves the language
            let broken = (0..=3).any(|i| {
                l.set_i(i);
                !l.is_in_lang(&l.create_pumped_string())
            });
            assert!(broken, "{lang:?} m={m} w={w}");
        }
    }
}

#[test]
fn human_first_regular_game_rejects_bad_moves_with_jflaps_messages() {
    let mut game = Game::new(reg(RegLang::AnEven), Mode::HumanFirst, 2);
    assert_eq!(game.view().awaiting, Awaiting::M);
    assert!(!game.view().boxes[1].visible);
    assert!(
        game.view()
            .prompt
            .starts_with("Please select a value for m in Box 1")
    );
    assert_eq!(
        game.choose_i(2).message,
        "The computer picks i in this mode."
    );
    assert_eq!(
        game.choose_decomposition(&[0, 2]).message,
        "Choose m first (box 1)."
    );
    let r = game.choose_m(1);
    assert!(!r.accepted);
    assert_eq!(
        r.message,
        "Please enter a positive integer in range [2, 18] for best results."
    );
    assert_eq!(game.view().boxes[0].message, r.message);
    assert!(!game.choose_m(19).accepted);

    assert!(game.choose_m(5).accepted);
    assert_eq!(game.lemma().w(), "aaaaaa");
    assert!(game.view().boxes[1].visible && game.view().boxes[2].visible);
    assert_eq!(game.view().boxes[2].message, "Condition violated: |y| > 0");
    assert_eq!(
        game.choose_decomposition(&[1, 0]).message,
        "Condition violated: |y| > 0"
    );
    assert_eq!(
        game.choose_decomposition(&[4, 2]).message,
        "Condition violated: |xy| <= m"
    );
    assert!(!game.view().boxes[3].visible);

    // x = a, y = aa, i chosen by the computer: a^(1 + 2i + 3) is even only for odd... it pumps fine
    let r = game.choose_decomposition(&[0, 2]);
    assert!(r.accepted);
    let result = game.result().unwrap();
    assert!(result.in_language && result.user_wins);
    assert!(result.message.ends_with("is in the language.  YOU WIN!"));
    assert_eq!(
        result.takeaway.as_deref(),
        Some("YOU WIN. Does this mean the language is regular?")
    );
    assert!(game.view().boxes[3].visible && game.view().boxes[4].visible);
    assert_eq!(game.lemma().attempts().len(), 1);
    assert!(game.lemma().attempts()[0].starts_with("X = λ;   Y = aa;   Z = aaaa; I = "));
    assert!(game.lemma().attempts()[0].ends_with("<i>Won</i>"));
    // choosing m again starts over but keeps the attempts
    assert!(game.choose_m(4).accepted);
    assert!(game.result().is_none());
    assert_eq!(game.lemma().attempts().len(), 1);
}

#[test]
fn human_first_context_free_game_plays_a_round_and_files_cases() {
    let mut game = Game::new(cf(CfLang::AnBnCn), Mode::HumanFirst, 3);
    assert!(game.view().cases.is_some());
    assert_eq!(game.view().cases.as_ref().unwrap().total, 11);
    assert_eq!(game.list_cases().message, "Choose m first (box 1).");
    assert_eq!(
        game.choose_m(2).message,
        "Please enter a positive integer in range [3, 11] for best results."
    );
    assert!(game.choose_m(4).accepted);
    assert_eq!(game.lemma().w(), "aaaabbbbcccc");
    assert_eq!(
        game.view().boxes[2].message,
        "Condition violated: |vy| >= 1"
    );
    assert_eq!(
        game.choose_decomposition(&[0, 3, 3, 3]).message,
        "Condition violated: |vxy| <= m"
    );
    assert_eq!(
        game.choose_decomposition(&[0, 0, 0, 0]).message,
        "Condition violated: |vy| >= 1"
    );
    assert_eq!(
        game.choose_decomposition(&[0, 1, 0]).message,
        "Give 4 lengths."
    );

    // v = a, y = a: only i = 1 keeps the counts equal, the computer picks 0 or 2
    let r = game.choose_decomposition(&[0, 1, 0, 1]);
    assert!(r.accepted);
    let result = game.result().unwrap().clone();
    assert!(!result.user_wins && !result.in_language);
    assert!(
        result
            .message
            .ends_with("is NOT in the language.  Please try again.")
    );
    assert_eq!(
        result.takeaway.as_deref(),
        Some("I WIN. Do you want to play again or concede that the language is not context-free?")
    );
    assert!(
        game.lemma().attempts()[0]
            .starts_with("U = λ;   V = a;   X = λ;   Y = a;   Z = aabbbbcccc; I = ")
    );
    assert!(game.lemma().attempts()[0].ends_with("<i>Failed</i>"));

    // file the round under its case, twice
    let r = game.add_case();
    assert_eq!(r.message, "Case added.");
    assert_eq!(
        game.view().cases.as_ref().unwrap().rows,
        ["v is a string of \"a\"s and y is a string of \"a\"s"]
    );
    assert_eq!(game.add_case().message, "This case is similar to #1.");
    assert_eq!(game.cases_done().message, "10 cases left.");
    assert_eq!(game.list_cases().message, "All cases for m = 4 shown.");
    assert_eq!(
        game.list_cases().message,
        "All cases for m = 4 are already shown."
    );
    assert_eq!(game.view().cases.as_ref().unwrap().rows.len(), 11);
    assert_eq!(game.cases_done().message, "All cases done.");
    assert_eq!(game.delete_case(3).message, "Case #4 deleted.");
    assert_eq!(game.cases_done().message, "1 case left.");
    assert_eq!(
        game.delete_case(40).message,
        "Please select a case to clear."
    );
    // a case that was never played shows its preset without a verdict
    let shown = game.show_case(1);
    assert_eq!(shown.message, "Showing case #2.");
    assert!(!game.view().boxes[3].visible);
    // the case played shows its i and the pumped string again
    assert_eq!(game.show_case(0).message, "Showing case #1.");
    assert!(game.view().boxes[3].visible);
    assert_eq!(game.replace_case(0).message, "Case #1 replaced.");
    assert_eq!(game.replace_case(2).message, "Wrong case selected.");
    assert_eq!(game.clear_cases().message, "All cases cleared.");
    assert!(game.view().cases.unwrap().rows.is_empty());
    assert_eq!(game.cases_done().message, "There are no cases yet.");
}

#[test]
fn context_free_presets_are_legal_decompositions_that_fall_in_their_own_case() {
    for lang in [
        CfLang::AnBn,
        CfLang::AnBnCn,
        CfLang::NaNbNc,
        CfLang::NagNbeNc,
        CfLang::AiBjCk,
        CfLang::AkBnCnDj,
        CfLang::AnBjAnBj,
        CfLang::WW,
    ] {
        let language = cf(lang);
        let (lo, hi) = language.info().range;
        for m in lo..=hi {
            let mut l = Lemma::new(language, 1);
            l.set_m(m);
            l.do_all();
            let cases = l.done_cases();
            assert_eq!(cases.len(), l.case_count());
            for (n, c) in cases.iter().enumerate() {
                // JFLAP quirk, kept: the preset of the last letter case of a^n b^j a^n b^j and ww
                // puts a lone `a` in y, but its predicate wants both letters in y.
                if matches!(lang, CfLang::AnBjAnBj | CfLang::WW) && n == 7 {
                    continue;
                }
                let mut probe = l.clone();
                let found = probe.add_case(&c.input, 2);
                assert!(found != -1, "{lang:?} m={m} case {n} preset {:?}", c.input);
                // already listed: the add reports the case's own row
                assert_eq!(
                    found, n as i32,
                    "{lang:?} m={m} case {n}: {}",
                    c.description
                );
            }
        }
    }
}

#[test]
fn computer_first_context_free_game_is_won_by_pumping_a_letter_out_of_balance() {
    let mut game = Game::new(cf(CfLang::AnBnCn), Mode::ComputerFirst, 8);
    assert!(game.view().cases.is_none());
    let m = game.lemma().m() as usize;
    let w = format!("{}{}{}", "a".repeat(m), "b".repeat(m), "c".repeat(m));
    assert!(game.choose_w(&w).accepted);
    let d = game.view().boxes[2].content.clone();
    assert!(d.starts_with("U = ") && d.contains(";   V = ") && d.contains(";   Z = "));
    assert!(game.choose_i(2).accepted);
    assert!(game.result().unwrap().user_wins);
    assert!(
        game.result()
            .unwrap()
            .message
            .contains("<i>uv</i><sup>2</sup><i>xy</i><sup>2</sup><i>z</i> = ")
    );
    assert_eq!(game.conclusion(), Some(Conclusion::NotContextFree));
    assert!(
        game.explain()
            .explanation
            .contains("this is not a context-free language")
    );
}

#[test]
fn clear_all_starts_over_but_keeps_the_attempts() {
    let mut game = Game::new(reg(RegLang::AnBn), Mode::ComputerFirst, 4);
    let m = game.lemma().m() as usize;
    game.choose_w(&format!("{}{}", "a".repeat(m), "b".repeat(m)));
    game.choose_i(2);
    assert_eq!(game.lemma().attempts().len(), 1);
    game.clear_all();
    assert_eq!(game.view().awaiting, Awaiting::W);
    assert!(game.result().is_none());
    assert_eq!(game.lemma().attempts().len(), 1);
    assert!(game.lemma().m() >= 4);
    assert_eq!(game.lemma().w(), "");
}

#[test]
fn a_game_is_plain_data_and_moves_are_json() {
    let mut game = Game::new(cf(CfLang::AnBnCn), Mode::HumanFirst, 9);
    game.play(&Move::ChooseM { m: 4 });
    game.play(&Move::ChooseDecomposition {
        lengths: vec![0, 1, 0, 1],
    });
    let json = serde_json::to_string(&game).unwrap();
    let back: Game = serde_json::from_str(&json).unwrap();
    assert_eq!(back, game);
    // the resumed game goes on identically
    let (mut a, mut b) = (game, back);
    assert_eq!(a.play(&Move::AddCase), b.play(&Move::AddCase));
    assert_eq!(a, b);

    let mv: Move =
        serde_json::from_str(r#"{"move":"choose_decomposition","lengths":[1,2]}"#).unwrap();
    assert_eq!(
        mv,
        Move::ChooseDecomposition {
            lengths: vec![1, 2]
        }
    );
    assert_eq!(
        serde_json::to_string(&Move::ChooseM { m: 5 }).unwrap(),
        r#"{"move":"choose_m","m":5}"#
    );
    assert_eq!(
        serde_json::to_string(&Move::ClearAll).unwrap(),
        r#"{"move":"clear_all"}"#
    );
    let view = serde_json::to_value(a.view()).unwrap();
    assert_eq!(view["awaiting"], "decomposition");
    assert_eq!(view["mode"], "human_first");
}

#[test]
fn seeded_games_repeat() {
    let play = |seed| {
        let mut g = Game::new(reg(RegLang::AB2n), Mode::ComputerFirst, seed);
        let m = g.lemma().m() as usize;
        g.choose_w(&"ab".repeat(m + m % 2));
        g.choose_i(0);
        g.view()
    };
    assert_eq!(play(3), play(3));
}
