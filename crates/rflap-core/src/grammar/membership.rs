//! Membership by the sets Tk (CLASE 41), for grammars that are not contracting.
//!
//! `T0 = { S }` and `Tk = Tk-1 ∪ { β : |β| ≤ |w|, α ∈ Tk-1, α ⇒ β }` (one derivation step),
//! repeated until `Tk == Tk-1` (w is not in the language) or `w ∈ Tk` (it is). A contracting
//! grammar (some production has `|α| > |β|`, other than `S → λ`) can shrink a form and
//! come back, so the bound `|β| ≤ |w|` would lose derivations: it is refused.

use std::collections::HashSet;
use std::fmt;

use serde::Serialize;

use super::{Grammar, LAMBDA};

/// Forms the sets may hold together before the search gives up.
pub const DEFAULT_MAX_FORMS: usize = 100_000;

/// Why a membership check could not be done.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipError {
    /// The grammar has no start symbol (no productions).
    NoStart,
    /// A production with `|α| > |β|` that is not `S → λ`.
    Contracting { production: String },
    /// The sets grew past the budget.
    LimitExceeded { limit: usize },
}

impl fmt::Display for MembershipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MembershipError::NoStart => f.write_str("La gramática no tiene producciones."),
            MembershipError::Contracting { production } => write!(
                f,
                "La gramática es compresora (la producción {production} tiene |α| > |β|): la comprobación por los conjuntos Tk solo vale para gramáticas no compresoras."
            ),
            MembershipError::LimitExceeded { limit } => write!(
                f,
                "Los conjuntos Tk superaron las {limit} formas sentenciales; se corta la búsqueda."
            ),
        }
    }
}

impl std::error::Error for MembershipError {}

/// The sets `T0, T1, ...` computed until the answer is known.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TkReport {
    /// `|w|`.
    pub bound: usize,
    /// `T0, T1, ...`, each in insertion order; `λ` is the empty form. When the search stopped
    /// because `Tk == Tk-1`, the last set repeats the one before.
    pub sets: Vec<Vec<String>>,
    /// `w` belongs to the language.
    pub member: bool,
}

fn shown(form: &str) -> &str {
    if form.is_empty() { LAMBDA } else { form }
}

fn set_text(set: &[String]) -> String {
    format!(
        "{{ {} }}",
        set.iter()
            .map(|f| shown(f).to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

impl TkReport {
    /// The sets as the course writes them, with the conclusion.
    pub fn to_text(&self, word: &str) -> String {
        let mut out = String::new();
        for (k, set) in self.sets.iter().enumerate() {
            if k > 0 && *set == self.sets[k - 1] {
                out.push_str(&format!("T{k} = T{}\n", k - 1));
            } else {
                out.push_str(&format!("T{k} = {}\n", set_text(set)));
            }
        }
        let last = self.sets.len() - 1;
        let w = shown(word);
        if self.member {
            out.push_str(&format!(
                "\"{w}\" ∈ T{last}: la palabra pertenece al lenguaje.\n"
            ));
        } else {
            out.push_str(&format!(
                "Como T{last} = T{}, \"{w}\" no pertenece al lenguaje.\n",
                last - 1
            ));
        }
        out
    }
}

/// Membership of `word` by the sets Tk, with the default budget.
pub fn membership_tk(grammar: &Grammar, word: &str) -> Result<TkReport, MembershipError> {
    membership_tk_limited(grammar, word, DEFAULT_MAX_FORMS)
}

/// Membership of `word` by the sets Tk, giving up when the sets hold more than `max_forms`
/// forms.
pub fn membership_tk_limited(
    grammar: &Grammar,
    word: &str,
    max_forms: usize,
) -> Result<TkReport, MembershipError> {
    if grammar.productions().is_empty() {
        return Err(MembershipError::NoStart);
    }
    let start = grammar.start().to_string();
    if let Some(p) = grammar.productions().iter().find(|p| {
        p.lhs.chars().count() > p.rhs.chars().count() && !(p.lhs == start && p.rhs.is_empty())
    }) {
        return Err(MembershipError::Contracting {
            production: p.to_string(),
        });
    }
    let bound = word.chars().count();
    let mut current: Vec<String> = vec![start];
    let mut seen: HashSet<String> = current.iter().cloned().collect();
    let mut sets = vec![current.clone()];
    while !current.iter().any(|f| f == word) {
        let mut next = current.clone();
        for alpha in &current {
            let chars: Vec<char> = alpha.chars().collect();
            for p in grammar.productions() {
                let lhs: Vec<char> = p.lhs.chars().collect();
                if lhs.len() > chars.len() {
                    continue;
                }
                for at in 0..=(chars.len() - lhs.len()) {
                    if chars[at..at + lhs.len()] != lhs[..] {
                        continue;
                    }
                    let beta: String = chars[..at]
                        .iter()
                        .chain(p.rhs.chars().collect::<Vec<_>>().iter())
                        .chain(chars[at + lhs.len()..].iter())
                        .collect();
                    if beta.chars().count() <= bound && seen.insert(beta.clone()) {
                        next.push(beta);
                        if next.len() > max_forms {
                            return Err(MembershipError::LimitExceeded { limit: max_forms });
                        }
                    }
                }
            }
        }
        let unchanged = next.len() == current.len();
        sets.push(next.clone());
        current = next;
        if unchanged {
            break;
        }
    }
    let member = current.iter().any(|f| f == word);
    let sets = sets
        .into_iter()
        .map(|set| set.into_iter().map(|f| shown(&f).to_string()).collect())
        .collect();
    Ok(TkReport {
        bound,
        sets,
        member,
    })
}
