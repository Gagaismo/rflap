//! Pumping lemma games for regular and context-free languages (JFLAP `pumping`).
//!
//! JFLAP ships 13 regular and 13 context-free languages ([`catalogue`]); each has a
//! membership test, the choices the computer makes (`m`, `w`, the decomposition and `i`)
//! and, for the context-free ones, a table of cases. [`Lemma`] is one language plus the
//! state of a game, [`Game`] is the state machine of the "You go first" and "Computer
//! goes first" modes with JFLAP's move order and messages, and [`jff`] reads and writes
//! the game files.
//!
//! ```
//! use rflap_core::pumping::{Game, Kind, Language, Mode, catalogue};
//!
//! // L = { a^n b^n }: the computer picks m, we answer with w = a^m b^m ...
//! let language = catalogue(Kind::Regular)[0];
//! let mut game = Game::new(language, Mode::ComputerFirst, 1);
//! let m = game.lemma().m() as usize;
//! assert!(game.choose_w(&format!("{}{}", "a".repeat(m), "b".repeat(m))).accepted);
//! // ... the computer splits w = xyz and we pump with i = 2: the string leaves the language
//! assert!(game.choose_i(2).accepted);
//! assert!(game.result().unwrap().user_wins);
//! ```

mod cases;
mod game;
mod langs;
mod lemma;
mod texts;

pub mod jff;

use serde::{Deserialize, Serialize};

pub use game::{
    Awaiting, BoxView, CasePanel, Conclusion, Explanation, Game, GameResult, Mode, Move, Reply,
    View,
};
pub use lemma::{CaseView, Lemma};

/// Which pumping lemma: for regular or for context-free languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Regular,
    ContextFree,
}

impl Kind {
    /// The `<type>` of the `.jff` file.
    pub fn jff_type(self) -> &'static str {
        match self {
            Kind::Regular => "regular pumping lemma",
            Kind::ContextFree => "context-free pumping lemma",
        }
    }
}

/// The predefined regular languages, in the order JFLAP lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegLang {
    /// a^n b^n
    AnBn,
    /// n_a(w) < n_b(w)
    NaNb,
    /// w w^R
    Palindrome,
    /// (ab)^n a^k, n > k
    ABnAk,
    /// a^n b^k c^(n+k)
    AnBkCnk,
    /// a^n b^l a^k, n > 5, l > 3, k <= l
    AnBlAk,
    /// a^n, n even
    AnEven,
    /// a^n b^k, n odd or k even
    AnBk,
    /// bba(ba)^n a^(n-1)
    BBABAnAn,
    /// b^5 w, 2 n_a(w) = 3 n_b(w)
    B5W,
    /// b^5 w, (2 n_a(w) + 5 n_b(w)) mod 3 = 0
    B5Wmod,
    /// b^k (ab)^n (ba)^n, k >= 4
    BkABnBAn,
    /// (ab)^(2n)
    AB2n,
}

/// The predefined context-free languages, in the order JFLAP lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CfLang {
    /// a^n b^n c^n
    AnBnCn,
    /// ww
    WW,
    /// a^n b^j a^n b^j
    AnBjAnBj,
    /// n_a(w) < n_b(w) < n_c(w)
    NaNbNc,
    /// n_a(w) > n_b(w) = n_c(w)
    NagNbeNc,
    /// a^i b^j c^k, i > j, i > k
    AiBjCk,
    /// a^n b^n
    AnBn,
    /// a^k b^n c^n d^j, j != k
    AkBnCnDj,
    /// w w1 w^R, |w1| >= 5
    WW1WrGrtrThanEq,
    /// w w1 w^R, |w| = |w1|
    WW1WrEquals,
    /// w1 b^n w2, n_a(w1) < n_a(w2), n_a(w1) < n
    W1BnW2,
    /// w1 c w2 c w3 c w4, w1 = w2 or w3 = w4
    W1CW2CW3CW4,
    /// w1 v v^R w2, n_a(w1) = n_a(w2), |v| >= 3
    W1VVrW2,
}

const REG_LANGS: [RegLang; 13] = [
    RegLang::AnBn,
    RegLang::NaNb,
    RegLang::Palindrome,
    RegLang::ABnAk,
    RegLang::AnBkCnk,
    RegLang::AnBlAk,
    RegLang::AnEven,
    RegLang::AnBk,
    RegLang::BBABAnAn,
    RegLang::B5W,
    RegLang::B5Wmod,
    RegLang::BkABnBAn,
    RegLang::AB2n,
];

const CF_LANGS: [CfLang; 13] = [
    CfLang::AnBnCn,
    CfLang::WW,
    CfLang::AnBjAnBj,
    CfLang::NaNbNc,
    CfLang::NagNbeNc,
    CfLang::AiBjCk,
    CfLang::AnBn,
    CfLang::AkBnCnDj,
    CfLang::WW1WrGrtrThanEq,
    CfLang::WW1WrEquals,
    CfLang::W1BnW2,
    CfLang::W1CW2CW3CW4,
    CfLang::W1VVrW2,
];

/// One of JFLAP's predefined languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Regular(RegLang),
    ContextFree(CfLang),
}

/// What JFLAP shows about a language before a game starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageInfo {
    /// The plain title, also the `<name>` of a `.jff` file (`a^n b^n : n >= 0`).
    pub title: String,
    /// The title as the HTML JFLAP renders (`<i>a<sup>n</sup>...`).
    pub html_title: String,
    /// Whether JFLAP says a valid decomposition exists (the language is regular /
    /// context-free); `false` means the lemma shows it is not.
    pub partition_is_valid: bool,
    /// JFLAP's explanation for the "Explain" button, as HTML.
    pub explanation: String,
    /// The range of `m` the computer picks from and the user may enter.
    pub range: (i32, i32),
    /// How many cases the language has (context-free only).
    pub cases: usize,
}

/// The languages of a lemma kind in the order of JFLAP's chooser.
pub fn catalogue(kind: Kind) -> Vec<Language> {
    match kind {
        Kind::Regular => REG_LANGS.iter().map(|&l| Language::Regular(l)).collect(),
        Kind::ContextFree => CF_LANGS.iter().map(|&l| Language::ContextFree(l)).collect(),
    }
}

impl Language {
    pub fn kind(self) -> Kind {
        match self {
            Language::Regular(_) => Kind::Regular,
            Language::ContextFree(_) => Kind::ContextFree,
        }
    }

    /// Position in the chooser of its kind.
    pub fn index(self) -> usize {
        match self {
            Language::Regular(l) => REG_LANGS.iter().position(|&x| x == l),
            Language::ContextFree(l) => CF_LANGS.iter().position(|&x| x == l),
        }
        .expect("every language is in its catalogue")
    }

    pub fn from_index(kind: Kind, index: usize) -> Option<Language> {
        match kind {
            Kind::Regular => REG_LANGS.get(index).map(|&l| Language::Regular(l)),
            Kind::ContextFree => CF_LANGS.get(index).map(|&l| Language::ContextFree(l)),
        }
    }

    /// `PumpingLemmaFactory.createPumpingLemma`: finds a language by its plain title.
    pub fn from_title(kind: Kind, title: &str) -> Option<Language> {
        catalogue(kind)
            .into_iter()
            .find(|l| l.text().title == title)
    }

    fn text(self) -> &'static texts::LangText {
        match self {
            Language::Regular(_) => &texts::REGULAR[self.index()],
            Language::ContextFree(_) => &texts::CONTEXT_FREE[self.index()],
        }
    }

    pub fn title(self) -> &'static str {
        self.text().title
    }

    pub fn info(self) -> LanguageInfo {
        let t = self.text();
        LanguageInfo {
            title: t.title.to_string(),
            html_title: t.html_title.to_string(),
            partition_is_valid: t.partition_is_valid,
            explanation: t.explanation.to_string(),
            range: t.range,
            cases: match self {
                Language::Regular(_) => 0,
                Language::ContextFree(l) => cases::table(l).len(),
            },
        }
    }
}
