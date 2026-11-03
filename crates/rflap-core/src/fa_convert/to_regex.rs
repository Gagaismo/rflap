//! FA to regular expression by state elimination (JFLAP `FSAToRegularExpressionConverter`).
//!
//! Labels are JFLAP strings: `""` is λ and `"ø"` the empty language.

use std::collections::BTreeMap;

use super::common::grid_point;
use super::{FaToRegexStep, GtgEdge, RegexConversion};
use crate::automaton::{StateId, Transition};
use crate::fa::{Fa, FaLabel};
use crate::regex::Regex;

pub(super) const EMPTY: &str = "ø";
pub(super) const LAMBDA_DISPLAY: &str = "λ";

pub(super) fn fa_to_regex(fa: &Fa) -> RegexConversion {
    let mut steps = Vec::new();
    let Some(initial) = fa.initial() else {
        return RegexConversion {
            regex: Regex::Empty,
            steps,
        };
    };
    let mut work = fa.clone();

    // A single final state different from the initial one.
    let old_finals: Vec<StateId> = work.finals().iter().copied().collect();
    if old_finals.len() != 1 || old_finals[0] == initial {
        let state = work.create_state(grid_point(work.states().count()));
        for &f in &old_finals {
            work.add_transition(Transition::new(f, state, FaLabel::lambda()));
            work.set_final(f, false);
        }
        work.set_final(state, true);
        steps.push(FaToRegexStep::AddFinalState { state, old_finals });
    }
    let last = work.finals().iter().next().copied().unwrap_or(initial);

    let ids: Vec<StateId> = work.states().map(|s| s.id).collect();
    let mut parallel: BTreeMap<(StateId, StateId), Vec<String>> = BTreeMap::new();
    for t in work.transitions() {
        parallel
            .entry((t.from, t.to))
            .or_default()
            .push(label_text(&t.label));
    }

    // One transition per pair: collapse the parallel ones, add ø where there is none.
    let mut gtg: BTreeMap<(StateId, StateId), String> = BTreeMap::new();
    for &i in &ids {
        for &j in &ids {
            match parallel.get(&(i, j)) {
                Some(labels) if labels.len() > 1 => {
                    let descriptions: Vec<String> = labels
                        .iter()
                        .map(|l| {
                            if l.is_empty() {
                                LAMBDA_DISPLAY.to_string()
                            } else {
                                l.clone()
                            }
                        })
                        .collect();
                    let label = descriptions[1..]
                        .iter()
                        .fold(descriptions[0].clone(), |acc, d| or(&acc, d));
                    steps.push(FaToRegexStep::CollapseParallel {
                        from: i,
                        to: j,
                        labels: descriptions,
                        label: label.clone(),
                    });
                    gtg.insert((i, j), label);
                }
                Some(labels) => {
                    gtg.insert((i, j), labels[0].clone());
                }
                None => {}
            }
        }
    }
    let mut empty_pairs = Vec::new();
    for &i in &ids {
        for &j in &ids {
            gtg.entry((i, j)).or_insert_with(|| {
                empty_pairs.push((i, j));
                EMPTY.to_string()
            });
        }
    }
    if !empty_pairs.is_empty() {
        steps.push(FaToRegexStep::AddEmptyTransitions { pairs: empty_pairs });
    }

    // Eliminate every other state in id order.
    let mut remaining = ids.clone();
    for &r in &ids {
        if r == initial || r == last {
            continue;
        }
        remaining.retain(|&s| s != r);
        let mut next = BTreeMap::new();
        let mut edges = Vec::new();
        for &i in &remaining {
            for &j in &remaining {
                let through = concatenate(
                    &concatenate(&gtg[&(i, r)], &star(&gtg[&(r, r)])),
                    &gtg[&(r, j)],
                );
                let label = or(&gtg[&(i, j)], &through);
                edges.push(GtgEdge {
                    from: i,
                    to: j,
                    label: shown(&label),
                });
                next.insert((i, j), label);
            }
        }
        gtg = next;
        steps.push(FaToRegexStep::EliminateState { state: r, edges });
    }

    let (ii, ij, jj, ji) = (
        &gtg[&(initial, initial)],
        &gtg[&(initial, last)],
        &gtg[&(last, last)],
        &gtg[&(last, initial)],
    );
    let text = final_expression(ii, ij, jj, ji);
    let regex = match text.as_str() {
        "" => Regex::Lambda,
        EMPTY => Regex::Empty,
        _ => Regex::parse(&text).unwrap_or(Regex::Empty),
    };
    steps.push(FaToRegexStep::Result {
        expression: regex.to_string(),
    });
    RegexConversion { regex, steps }
}

fn shown(label: &str) -> String {
    if label.is_empty() {
        LAMBDA_DISPLAY.to_string()
    } else {
        label.to_string()
    }
}

/// Label as a JFLAP expression: ranges become unions of their characters.
fn label_text(label: &FaLabel) -> String {
    match label.range() {
        Some((lo, hi)) => (lo as u32..=hi as u32)
            .filter_map(char::from_u32)
            .map(String::from)
            .collect::<Vec<_>>()
            .join("+"),
        None => label.0.clone(),
    }
}

fn final_expression(ii: &str, ij: &str, jj: &str, ji: &str) -> String {
    let s5 = concatenate(&star(ii), &concatenate(ij, &concatenate(&star(jj), ji)));
    let s6 = concatenate(&star(ii), &concatenate(ij, &star(jj)));
    concatenate(&star(&s5), &s6)
}

pub(super) fn concatenate(a: &str, b: &str) -> String {
    if a == EMPTY || b == EMPTY {
        return EMPTY.to_string();
    }
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }
    let paren = |s: &str| {
        if or_parts(s).len() > 1 {
            format!("({s})")
        } else {
            s.to_string()
        }
    };
    paren(a) + &paren(b)
}

pub(super) fn star(s: &str) -> String {
    if s == EMPTY || s.is_empty() {
        return String::new();
    }
    if or_parts(s).len() > 1 || cat_parts(s).len() > 1 {
        return format!("({s})*");
    }
    if s.ends_with('*') {
        return s.to_string();
    }
    format!("{s}*")
}

pub(super) fn or(a: &str, b: &str) -> String {
    if a == EMPTY {
        return b.to_string();
    }
    if b == EMPTY {
        return a.to_string();
    }
    if a.is_empty() && b.is_empty() {
        return String::new();
    }
    let show = |s: &str| {
        if s.is_empty() {
            LAMBDA_DISPLAY.to_string()
        } else {
            s.to_string()
        }
    };
    format!("{}+{}", show(a), show(b))
}

/// `λ` standing alone is the empty string.
pub(super) fn delambda(s: &str) -> String {
    if s == LAMBDA_DISPLAY {
        String::new()
    } else {
        s.to_string()
    }
}

/// Top-level alternatives (JFLAP `Discretizer.or`).
pub(super) fn or_parts(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let (mut parts, mut start, mut depth) = (Vec::new(), 0, 0i32);
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c == '+' && depth == 0 {
            parts.push(delambda(&chars[start..i].iter().collect::<String>()));
            start = i + 1;
        }
    }
    parts.push(delambda(&chars[start..].iter().collect::<String>()));
    parts
}

/// Top-level factors, each with its stars (JFLAP `Discretizer.cat`).
pub(super) fn cat_parts(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let (mut parts, mut start, mut depth) = (Vec::new(), 0, 0i32);
    for (i, &c) in chars.iter().enumerate() {
        if c == ')' {
            depth -= 1;
            continue;
        }
        if c == '(' {
            depth += 1;
        }
        if (c != '(' || depth != 1) && depth != 0 {
            continue;
        }
        if c == '*' || i == 0 {
            continue;
        }
        parts.push(delambda(&chars[start..i].iter().collect::<String>()));
        start = i;
    }
    parts.push(delambda(&chars[start..].iter().collect::<String>()));
    parts
}
