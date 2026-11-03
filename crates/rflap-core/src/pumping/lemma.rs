//! One predefined language plus the state of a game on it (JFLAP `pumping.PumpingLemma`,
//! `RegularPumpingLemma` and `ContextFreePumpingLemma`).

use serde::{Deserialize, Serialize};

use super::cases::{self, CaseDef};
use super::{Kind, Language, LanguageInfo};
use crate::lsystem::JavaRandom;

/// What the user entered for a case, and the `i` it was pumped with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct CaseState {
    pub i: i32,
    pub user_input: Option<[i32; 4]>,
}

impl Default for CaseState {
    fn default() -> Self {
        CaseState {
            i: -1,
            user_input: None,
        }
    }
}

/// A case as the case panel shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseView {
    pub description: String,
    /// The decomposition lengths `u, v, x, y` the user gave, or the preset for the current `m`.
    pub input: [i32; 4],
    /// The `i` it was pumped with, `-1` when none.
    pub i: i32,
}

/// A language and the state of a game on it.
///
/// `m`, `i` are `-1` and `w` is empty until chosen. `parts` hold `x, y, z` (regular) or
/// `u, v, x, y, z` (context-free) once a valid decomposition has been set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lemma {
    pub(super) language: Language,
    pub(super) first_player: String,
    pub(super) m: i32,
    pub(super) w: String,
    pub(super) i: i32,
    pub(super) decomposition: Option<Vec<i32>>,
    pub(super) parts: Vec<String>,
    pub(super) attempts: Vec<String>,
    pub(super) done_cases: Vec<usize>,
    pub(super) case_state: Vec<CaseState>,
    pub(super) random: JavaRandom,
}

/// `LemmaMath.fetchRandInt(lo, hi)`.
pub(super) fn fetch_rand_int(random: &mut JavaRandom, lo: i32, hi: i32) -> i32 {
    if hi < lo {
        // JFLAP throws IllegalArgumentException here
        return lo;
    }
    random.next_int(hi - lo + 1) + lo
}

/// `substring(from, to)` that refuses what Java would throw on.
pub(super) fn slice(s: &str, from: i32, to: i32) -> Option<&str> {
    if from < 0 || to < from {
        return None;
    }
    s.get(from as usize..to as usize)
}

pub(super) fn pump_string(s: &str, n: i32) -> String {
    s.repeat(n.max(0) as usize)
}

impl Lemma {
    /// A fresh game on `language`; `seed` feeds the generator behind the computer's choices
    /// (JFLAP uses an unseeded `java.util.Random`).
    pub fn new(language: Language, seed: i64) -> Lemma {
        let case_count = Self::case_table(language).len();
        let mut lemma = Lemma {
            language,
            first_player: String::new(),
            m: -1,
            w: String::new(),
            i: -1,
            decomposition: None,
            parts: Vec::new(),
            attempts: Vec::new(),
            done_cases: Vec::new(),
            case_state: vec![CaseState::default(); case_count],
            random: JavaRandom::new(seed),
        };
        lemma.reset();
        lemma
    }

    pub(super) fn case_table(language: Language) -> Vec<CaseDef> {
        match language {
            Language::Regular(_) => Vec::new(),
            Language::ContextFree(l) => cases::table(l),
        }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn kind(&self) -> Kind {
        self.language.kind()
    }

    pub fn info(&self) -> LanguageInfo {
        self.language.info()
    }

    pub fn title(&self) -> &'static str {
        self.language.text().title
    }

    pub fn html_title(&self) -> &'static str {
        self.language.text().html_title
    }

    /// The range of `m` JFLAP picks from and accepts.
    pub fn range(&self) -> (i32, i32) {
        self.language.text().range
    }

    /// Whether JFLAP says a valid decomposition exists for this language.
    pub fn partition_is_valid(&self) -> bool {
        self.language.text().partition_is_valid
    }

    /// JFLAP's explanation (HTML).
    pub fn explanation(&self) -> &'static str {
        self.language.text().explanation
    }

    /// `"Human"`, `"Computer"` or empty when nobody has been set.
    pub fn first_player(&self) -> &str {
        &self.first_player
    }

    pub fn set_first_player(&mut self, player: &str) {
        self.first_player = player.to_string();
    }

    /// Restarts the generator behind the computer's choices.
    pub fn set_seed(&mut self, seed: i64) {
        self.random = JavaRandom::new(seed);
    }

    pub fn m(&self) -> i32 {
        self.m
    }

    pub fn w(&self) -> &str {
        &self.w
    }

    pub fn i(&self) -> i32 {
        self.i
    }

    /// The array given to the last `set_decomposition`, valid or not.
    pub fn decomposition(&self) -> Option<&[i32]> {
        self.decomposition.as_deref()
    }

    fn part(&self, index: usize) -> &str {
        self.parts.get(index).map_or("", String::as_str)
    }

    /// `u`, `v` exist for context-free lemmas only; regular ones return the empty string.
    pub fn u(&self) -> &str {
        match self.kind() {
            Kind::Regular => "",
            Kind::ContextFree => self.part(0),
        }
    }

    pub fn v(&self) -> &str {
        match self.kind() {
            Kind::Regular => "",
            Kind::ContextFree => self.part(1),
        }
    }

    pub fn x(&self) -> &str {
        match self.kind() {
            Kind::Regular => self.part(0),
            Kind::ContextFree => self.part(2),
        }
    }

    pub fn y(&self) -> &str {
        match self.kind() {
            Kind::Regular => self.part(1),
            Kind::ContextFree => self.part(3),
        }
    }

    pub fn z(&self) -> &str {
        match self.kind() {
            Kind::Regular => self.part(2),
            Kind::ContextFree => self.part(4),
        }
    }

    pub fn attempts(&self) -> &[String] {
        &self.attempts
    }

    pub fn add_attempt(&mut self, attempt: &str) {
        self.attempts.push(attempt.to_string());
    }

    pub fn clear_attempts(&mut self) {
        self.attempts.clear();
    }

    /// `reset()`: forgets `m`, `w`, `i` and the parts; attempts and cases stay.
    pub fn reset(&mut self) {
        self.m = -1;
        self.i = -1;
        self.w = String::new();
        self.parts = vec![String::new(); if self.kind() == Kind::Regular { 3 } else { 5 }];
    }

    /// `setM(m)`: resets, then lets the computer choose `w` for this `m`.
    pub fn set_m(&mut self, m: i32) {
        self.reset();
        self.m = m;
        self.choose_w();
    }

    /// `chooseM()`: a random `m` in the range.
    pub fn choose_m(&mut self) {
        let (lo, hi) = self.range();
        self.m = fetch_rand_int(&mut self.random, lo, hi);
    }

    pub fn set_w(&mut self, w: &str) {
        self.w = w.to_string();
    }

    pub fn set_i(&mut self, i: i32) {
        self.i = i;
    }

    /// `setDecomposition(lengths)` with lengths `[|x|, |y|]` (regular) or
    /// `[|u|, |v|, |x|, |y|]` (context-free). The array is remembered even when it is not a
    /// valid decomposition (then `false` is returned and the parts stay as they were).
    pub fn set_decomposition(&mut self, d: &[i32]) -> bool {
        self.decomposition = Some(d.to_vec());
        match self.kind() {
            Kind::Regular => {
                let [n, n2] = match d {
                    [a, b, ..] => [*a, *b],
                    _ => return false,
                };
                if n + n2 > self.m || n2 < 1 || n < 0 {
                    return false;
                }
                let (Some(x), Some(y), Some(z)) = (
                    slice(&self.w, 0, n),
                    slice(&self.w, n, n + n2),
                    slice(&self.w, n + n2, self.w.len() as i32),
                ) else {
                    return false;
                };
                self.parts = vec![x.to_string(), y.to_string(), z.to_string()];
                true
            }
            Kind::ContextFree => {
                let [n, n2, n3, n4] = match d {
                    [a, b, c, e, ..] => [*a, *b, *c, *e],
                    _ => return false,
                };
                if n2 + n3 + n4 > self.m || n2 + n4 < 1 {
                    return false;
                }
                let len = self.w.len() as i32;
                let (Some(u), Some(v), Some(x), Some(y), Some(z)) = (
                    slice(&self.w, 0, n),
                    slice(&self.w, n, n + n2),
                    slice(&self.w, n + n2, n + n2 + n3),
                    slice(&self.w, n + n2 + n3, n + n2 + n3 + n4),
                    slice(&self.w, n + n2 + n3 + n4, len),
                ) else {
                    return false;
                };
                self.parts = [u, v, x, y, z].iter().map(|s| s.to_string()).collect();
                true
            }
        }
    }

    /// `setDecomposition(lengths, i)`.
    pub fn set_decomposition_with_i(&mut self, d: &[i32], i: i32) -> bool {
        self.i = i;
        self.set_decomposition(d)
    }

    /// `createPumpedString()`: `x y^i z` or `u v^i x y^i z`.
    pub fn create_pumped_string(&self) -> String {
        match self.kind() {
            Kind::Regular => format!("{}{}{}", self.x(), pump_string(self.y(), self.i), self.z()),
            Kind::ContextFree => format!(
                "{}{}{}{}{}",
                self.u(),
                pump_string(self.v(), self.i),
                self.x(),
                pump_string(self.y(), self.i),
                self.z()
            ),
        }
    }

    /// `getDecompositionAsString()`: `X = ..;   Y = ..;   Z = ..` with `λ` for empty parts.
    pub fn decomposition_as_string(&self) -> String {
        let Some(d) = &self.decomposition else {
            return String::new();
        };
        let (names, cuts): (&[&str], usize) = match self.kind() {
            Kind::Regular => (&["X", "Y", "Z"], 2),
            Kind::ContextFree => (&["U", "V", "X", "Y", "Z"], 4),
        };
        let mut pieces = Vec::new();
        let mut at = 0i32;
        for &len in d.iter().take(cuts) {
            pieces.push(slice(&self.w, at, at + len).unwrap_or("").to_string());
            at += len;
        }
        pieces.push(
            slice(&self.w, at, self.w.len() as i32)
                .unwrap_or("")
                .to_string(),
        );
        names
            .iter()
            .zip(pieces)
            .map(|(name, piece)| {
                let piece = if piece.is_empty() {
                    "λ".to_string()
                } else {
                    piece
                };
                format!("{name} = {piece}")
            })
            .collect::<Vec<_>>()
            .join(";   ")
    }

    // ---- cases ----

    /// How many cases the language has.
    pub fn case_count(&self) -> usize {
        self.case_state.len()
    }

    /// `doAll()`: shows every case.
    pub fn do_all(&mut self) {
        for i in 0..self.case_count() {
            if !self.done_cases.contains(&i) {
                self.done_cases.push(i);
            }
        }
    }

    /// `clearDoneCases()`.
    pub fn clear_done_cases(&mut self) {
        self.done_cases.clear();
        for s in &mut self.case_state {
            *s = CaseState::default();
        }
    }

    /// `clearCase(n)`: removes the n-th shown case; `false` when there is none.
    pub fn clear_case(&mut self, n: usize) -> bool {
        if n >= self.done_cases.len() {
            return false;
        }
        let index = self.done_cases.remove(n);
        self.case_state[index] = CaseState::default();
        true
    }

    /// The n-th case shown so far.
    pub fn case(&self, n: usize) -> Option<CaseView> {
        let index = *self.done_cases.get(n)?;
        Some(self.case_view(index))
    }

    fn case_view(&self, index: usize) -> CaseView {
        let table = Self::case_table(self.language);
        let state = &self.case_state[index];
        CaseView {
            description: table[index].description.clone(),
            input: state
                .user_input
                .unwrap_or_else(|| table[index].preset(self.m)),
            i: state.i,
        }
    }

    /// `getDoneCases()`: the cases shown so far.
    pub fn done_cases(&self) -> Vec<CaseView> {
        self.done_cases.iter().map(|&i| self.case_view(i)).collect()
    }

    /// `getDoneDescriptions()`.
    pub fn done_descriptions(&self) -> Vec<String> {
        self.done_cases()
            .into_iter()
            .map(|c| c.description)
            .collect()
    }

    /// The descriptions of every case of the language.
    pub fn case_descriptions(&self) -> Vec<String> {
        Self::case_table(self.language)
            .into_iter()
            .map(|c| c.description)
            .collect()
    }

    /// `addCase(lengths, i)`: sets the decomposition, then files it under the case it falls
    /// in. Returns `-1` for an illegal decomposition (or one in no case), the position of
    /// the case in the shown list when it was already there, and `case_count()` when it
    /// was added.
    pub fn add_case(&mut self, d: &[i32], i: i32) -> i32 {
        if !self.set_decomposition(d) {
            return -1;
        }
        let table = Self::case_table(self.language);
        let (v, y) = (self.v().to_string(), self.y().to_string());
        // regular lemmas test the case against (y, y); they have no cases at all
        let (cv, cy) = match self.kind() {
            Kind::Regular => (y.clone(), y.clone()),
            Kind::ContextFree => (v, y),
        };
        for (n, &index) in self.done_cases.iter().enumerate() {
            if table[index].is_case(&cv, &cy) {
                return n as i32;
            }
        }
        for (index, case) in table.iter().enumerate() {
            if case.is_case(&cv, &cy) {
                self.case_state[index] = CaseState {
                    i,
                    user_input: arr4(d),
                };
                self.done_cases.push(index);
                return table.len() as i32;
            }
        }
        -1
    }

    /// `replaceCase(lengths, i, n)`: re-files the n-th shown case with this decomposition
    /// when it falls in that case.
    pub fn replace_case(&mut self, d: &[i32], i: i32, n: usize) -> bool {
        let Some(&index) = self.done_cases.get(n) else {
            return false;
        };
        let table = Self::case_table(self.language);
        let (cv, cy) = match self.kind() {
            Kind::Regular => (self.y().to_string(), self.y().to_string()),
            Kind::ContextFree => (self.v().to_string(), self.y().to_string()),
        };
        if table[index].is_case(&cv, &cy) {
            self.case_state[index] = CaseState {
                i,
                user_input: arr4(d),
            };
            return true;
        }
        false
    }
}

fn arr4(d: &[i32]) -> Option<[i32; 4]> {
    match d {
        [a, b, c, e, ..] => Some([*a, *b, *c, *e]),
        _ => None,
    }
}
