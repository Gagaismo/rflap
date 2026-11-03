//! The case tables of the context-free pumping lemma games (JFLAP `pumping.Case`).
//!
//! A case describes where the pumped substrings `v` and `y` fall in `w` ("`v` is a string
//! of `a`s and `y` is a string of `a`s followed by `b`s"). Each one knows how to decide
//! whether a decomposition belongs to it and a preset decomposition for the current `m`.

use super::CfLang;

/// Which letters a substring holds and, if it matters, in which order they first appear.
struct Cond {
    letters: Vec<char>,
    /// `Some((a, b))`: the first `a` comes before the first `b`.
    order: Option<(char, char)>,
}

impl Cond {
    /// `"a"`, `"ab"` (both letters, any order) or `"a<b"` (both, `a` first).
    fn parse(spec: &str) -> Cond {
        match spec.split_once('<') {
            Some((a, b)) => {
                let (a, b) = (
                    a.chars().next().unwrap_or('a'),
                    b.chars().next().unwrap_or('b'),
                );
                Cond {
                    letters: vec![a, b],
                    order: Some((a, b)),
                }
            }
            None => Cond {
                letters: spec.chars().collect(),
                order: None,
            },
        }
    }

    fn matches(&self, s: &str, alphabet: &str) -> bool {
        if alphabet
            .chars()
            .any(|c| self.letters.contains(&c) != s.contains(c))
        {
            return false;
        }
        match self.order {
            None => true,
            Some((a, b)) => matches!((s.find(a), s.find(b)), (Some(i), Some(j)) if i < j),
        }
    }
}

enum Rule {
    Letters(Cond, Cond),
    VEmptyYNonEmpty,
    VNonEmptyYEmpty,
}

/// One entry of a language's case table.
pub(super) struct CaseDef {
    pub description: String,
    alphabet: &'static str,
    rule: Rule,
    preset: Box<dyn Fn(i32) -> [i32; 4] + Send + Sync>,
}

impl CaseDef {
    /// JFLAP's `isCase(v, y)`.
    pub fn is_case(&self, v: &str, y: &str) -> bool {
        match &self.rule {
            Rule::Letters(cv, cy) => cv.matches(v, self.alphabet) && cy.matches(y, self.alphabet),
            Rule::VEmptyYNonEmpty => v.is_empty() && !y.is_empty(),
            Rule::VNonEmptyYEmpty => !v.is_empty() && y.is_empty(),
        }
    }

    /// JFLAP's `getPreset()` for the given `m`.
    pub fn preset(&self, m: i32) -> [i32; 4] {
        (self.preset)(m)
    }
}

/// `"a"` is "a string of "a"s", `"ab"` is "a string of "a"s followed by "b"s".
fn phrase(token: &str) -> String {
    let runs: Vec<String> = token.chars().map(|c| format!("\"{c}\"s")).collect();
    format!("a string of {}", runs.join(" followed by "))
}

type Row = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    fn(i32) -> [i32; 4],
);

fn build(alphabet: &'static str, rows: &[Row], empty_at: fn(i32) -> i32) -> Vec<CaseDef> {
    let mut cases: Vec<CaseDef> = rows
        .iter()
        .map(|&(v, y, vd, yd, preset)| CaseDef {
            description: format!("v is {} and y is {}", phrase(vd), phrase(yd)),
            alphabet,
            rule: Rule::Letters(Cond::parse(v), Cond::parse(y)),
            preset: Box::new(preset),
        })
        .collect();
    cases.push(CaseDef {
        description: "v is an empty string and y is a non-empty string".to_string(),
        alphabet,
        rule: Rule::VEmptyYNonEmpty,
        preset: Box::new(move |m| [empty_at(m), 0, 1, 1]),
    });
    cases.push(CaseDef {
        description: "v is a non-empty string and y is an empty string".to_string(),
        alphabet,
        rule: Rule::VNonEmptyYEmpty,
        preset: Box::new(move |m| [empty_at(m), 1, 0, 0]),
    });
    cases
}

/// The case table of a language; empty when JFLAP defines none.
pub(super) fn table(language: CfLang) -> Vec<CaseDef> {
    match language {
        CfLang::AnBn => build(
            "ab",
            &[
                ("a", "a", "a", "a", |_| [0, 1, 0, 1]),
                ("a", "ab", "a", "ab", |m| [m - 2, 1, 0, 2]),
                ("a", "b", "a", "b", |m| [1, m - 1, 0, 1]),
                ("ab", "b", "ab", "b", |m| [m - 1, 2, 0, 1]),
                ("b", "b", "b", "b", |m| [m, 1, 0, 1]),
            ],
            |m| m,
        ),
        CfLang::AnBnCn => build(
            "abc",
            &[
                ("a", "a", "a", "a", |_| [0, 1, 0, 1]),
                ("a", "ab", "a", "ab", |m| [1, 1, 0, m - 1]),
                ("a", "b", "a", "b", |m| [m - 1, 1, 0, 1]),
                ("ab", "b", "ab", "b", |m| [m - 1, 2, 0, 1]),
                ("b", "b", "b", "b", |m| [m, 1, 0, 1]),
                ("b", "bc", "b", "bc", |m| [2 * m - 2, 1, 0, 2]),
                ("b", "c", "b", "c", |m| [2 * m - 1, 1, 0, 1]),
                ("bc", "c", "bc", "c", |m| [2 * m - 1, 2, 0, 1]),
                ("c", "c", "c", "c", |m| [2 * m, 1, 0, 1]),
            ],
            |m| 2 * m,
        ),
        CfLang::NaNbNc => build(
            "abc",
            &[
                ("a", "a", "a", "a", |_| [0, 1, 0, 1]),
                ("a", "ab", "a", "ab", |m| [1, 1, 0, m - 1]),
                ("a", "b", "a", "b", |m| [m - 1, 1, 0, 1]),
                ("ab", "b", "ab", "b", |m| [m - 1, 2, 0, 1]),
                ("b", "b", "b", "b", |m| [m, 1, 0, 1]),
                ("b", "bc", "b", "bc", |m| [2 * m - 1, 1, 0, 2]),
                ("b", "c", "b", "c", |m| [2 * m, 1, 0, 1]),
                ("bc", "c", "bc", "c", |m| [2 * m, 2, 0, 1]),
                ("c", "c", "c", "c", |m| [2 * m + 1, 1, 0, 1]),
            ],
            |m| 2 * m,
        ),
        // a^i b^j c^k with i > j, i > k and w with n_a > n_b = n_c share one table
        CfLang::NagNbeNc | CfLang::AiBjCk => build(
            "abc",
            &[
                ("a", "a", "a", "a", |_| [0, 1, 0, 1]),
                ("a", "ab", "a", "ab", |m| [m - 1, 1, 0, 2]),
                ("a", "b", "a", "b", |m| [m, 1, 0, 1]),
                ("ab", "b", "ab", "b", |m| [m, 2, 0, 1]),
                ("b", "b", "b", "b", |m| [m + 1, 1, 0, 1]),
                ("b", "bc", "b", "bc", |m| [2 * m - 1, 1, 0, 2]),
                ("b", "c", "b", "c", |m| [2 * m, 1, 0, 1]),
                ("bc", "c", "bc", "c", |m| [2 * m, 2, 0, 1]),
                ("c", "c", "c", "c", |m| [2 * m + 1, 1, 0, 1]),
            ],
            |m| 2 * m,
        ),
        CfLang::AkBnCnDj => build(
            "abcd",
            &[
                ("a", "a", "a", "a", |_| [0, 1, 0, 1]),
                ("a", "ab", "a", "ab", |m| [m - 2, 1, 0, 2]),
                ("a", "b", "a", "b", |m| [m - 1, 1, 0, 1]),
                ("ab", "b", "ab", "b", |m| [m - 1, 2, 0, 1]),
                ("b", "b", "b", "b", |m| [m, 1, 0, 1]),
                ("b", "bc", "b", "bc", |m| [2 * m - 2, 1, 0, 2]),
                ("b", "c", "b", "c", |m| [2 * m - 1, 1, 0, 1]),
                ("bc", "c", "bc", "c", |m| [2 * m - 1, 2, 0, 1]),
                ("c", "c", "c", "c", |m| [2 * m, 1, 0, 1]),
                ("c", "cd", "c", "cd", |m| [3 * m - 2, 1, 0, 2]),
                ("c", "d", "c", "d", |m| [3 * m - 1, 1, 0, 1]),
                ("cd", "d", "cd", "d", |m| [3 * m - 1, 2, 0, 1]),
                ("d", "d", "d", "d", |m| [3 * m, 1, 0, 1]),
            ],
            |m| 2 * m,
        ),
        // a^n b^j a^n b^j and ww
        CfLang::AnBjAnBj | CfLang::WW => build(
            "ab",
            &[
                ("a", "a", "a", "a", |_| [0, 1, 0, 1]),
                ("a", "a<b", "a", "ab", |m| [1, 1, 0, m - 1]),
                ("a", "b", "a", "b", |m| [m - 1, 1, 0, 1]),
                ("a<b", "b", "ab", "b", |m| [m - 1, 2, 0, 1]),
                ("b", "b", "b", "b", |m| [m, 1, 0, 1]),
                ("b", "b<a", "b", "ba", |m| [2 * m - 2, 1, 0, 2]),
                ("b", "a", "b", "a", |m| [2 * m - 1, 1, 0, 1]),
                ("b<a", "ab", "ba", "a", |m| [2 * m - 1, 2, 0, 1]),
            ],
            |m| 2 * m,
        ),
        CfLang::WW1WrGrtrThanEq
        | CfLang::WW1WrEquals
        | CfLang::W1BnW2
        | CfLang::W1CW2CW3CW4
        | CfLang::W1VVrW2 => Vec::new(),
    }
}
