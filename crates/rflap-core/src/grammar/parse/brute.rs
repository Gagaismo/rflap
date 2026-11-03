//! Brute-force parsing (JFLAP `BruteParser`, `RestrictedBruteParser`, `UnrestrictedBruteParser`
//! and `Unrestricted`).
//!
//! The search is JFLAP's breadth-first one: each node is a sentential form, and its children
//! apply, in one go, any number of productions at disjoint places (each at the first
//! occurrence of its left side after the previous change). Forms already seen are dropped, and
//! forms that cannot reach the target are pruned: JFLAP's minimum-length rule for every grammar
//! and, for context-free ones, a check of the terminal runs against the target.

use std::collections::{HashSet, VecDeque};
use std::ops::ControlFlow;

use serde::{Deserialize, Serialize};

use super::{
    Derivation, DerivationStep, ParseError, ParseOutcome, ParseTree, check_input, find_from,
    replace_at,
};
use crate::grammar::model::{Grammar, Production, is_variable_symbol};

/// Which JFLAP parser applies: `Unrestricted` as soon as a left side is not a single symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BruteKind {
    Restricted,
    Unrestricted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BruteResult {
    pub outcome: ParseOutcome,
    pub kind: BruteKind,
    /// Sequential derivation; the productions JFLAP applies together in one search step are
    /// listed left to right.
    pub derivation: Option<Derivation>,
    /// Only for restricted grammars.
    pub tree: Option<ParseTree>,
    /// Search nodes created (JFLAP's "considered nodes").
    pub nodes_considered: usize,
    /// Candidate forms looked at, the quantity `max_nodes` bounds.
    pub candidates_examined: usize,
}

/// JFLAP `Unrestricted.optimize`: drops productions that can never finish (a variable on the
/// right that derives nothing) and moves the first usable start production to the front.
fn optimize(g: &Grammar) -> Option<Vec<Production>> {
    let prods = g.productions();
    let mut usable = vec![false; prods.len()];
    let mut known: HashSet<char> = HashSet::new();
    let symbols = |p: &Production| -> Vec<char> { p.lhs.chars().chain(p.rhs.chars()).collect() };
    for (i, p) in prods.iter().enumerate() {
        if p.variables_on_rhs().is_empty() {
            known.extend(symbols(p));
            usable[i] = true;
        }
    }
    loop {
        let mut changed = false;
        for (i, p) in prods.iter().enumerate() {
            if !usable[i] && p.variables_on_rhs().iter().all(|v| known.contains(v)) {
                known.extend(symbols(p));
                usable[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let first = (0..prods.len()).find(|&i| usable[i] && prods[i].lhs == g.start())?;
    let mut out = vec![prods[first].clone()];
    out.extend(
        (0..prods.len())
            .filter(|&i| i != first && usable[i])
            .map(|i| prods[i].clone()),
    );
    Some(out)
}

fn min_length(s: &[char], smaller: &HashSet<char>) -> usize {
    s.iter().filter(|c| !smaller.contains(c)).count()
}

/// JFLAP `Unrestricted.smallerSymbols`: symbols that can only shrink, so they do not count
/// towards the length of a form.
fn smaller_symbols(prods: &[Production]) -> HashSet<char> {
    let mut set: HashSet<char> = HashSet::new();
    loop {
        let mut changed = false;
        for p in prods {
            let lhs: Vec<char> = p.lhs.chars().collect();
            let rhs: Vec<char> = p.rhs.chars().collect();
            if min_length(&lhs, &set) <= min_length(&rhs, &set) {
                continue;
            }
            for &c in &lhs {
                let count = |s: &[char]| s.iter().filter(|x| **x == c).count();
                if set.contains(&c) || count(&lhs) <= count(&rhs) {
                    continue;
                }
                set.insert(c);
                changed = true;
            }
        }
        if !changed {
            return set;
        }
    }
}

/// The context-free pruning of `RestrictedBruteParser.isPossibleDerivation`.
fn restricted_possible(form: &[char], target: &[char], smaller: &HashSet<char>) -> bool {
    if min_length(form, smaller) > target.len() {
        return false;
    }
    let starts_terminal = form.first().is_some_and(|c| !is_variable_symbol(*c));
    let ends_terminal = form.last().is_some_and(|c| !is_variable_symbol(*c));
    let mut runs: Vec<Vec<char>> = Vec::new();
    let mut run_start: Option<usize> = None;
    for n in 0..=form.len() {
        let is_terminal = n < form.len() && !is_variable_symbol(form[n]);
        if is_terminal {
            run_start.get_or_insert(n);
        } else if let Some(s) = run_start.take() {
            runs.push(form[s..n].to_vec());
        }
    }
    let mut at = 0usize;
    for (i, run) in runs.iter().enumerate() {
        if starts_terminal && i == 0 {
            if !target.starts_with(run) {
                return false;
            }
            at = run.len();
            continue;
        }
        if ends_terminal && i == runs.len() - 1 {
            if target.ends_with(run) {
                continue;
            }
            return false;
        }
        match find_from(target, run, at) {
            Some(idx) => at = idx + run.len(),
            None => return false,
        }
    }
    true
}

pub(crate) fn possible_with(
    kind: BruteKind,
    form: &[char],
    target: &[char],
    smaller: &HashSet<char>,
) -> bool {
    match kind {
        BruteKind::Restricted => restricted_possible(form, target, smaller),
        BruteKind::Unrestricted => min_length(form, smaller) <= target.len(),
    }
}

pub(crate) fn smaller_symbols_of(prods: &[Production]) -> HashSet<char> {
    smaller_symbols(prods)
}

pub(crate) fn kind_of(g: &Grammar) -> BruteKind {
    if g.productions().iter().any(|p| p.lhs.chars().count() != 1) {
        BruteKind::Unrestricted
    } else {
        BruteKind::Restricted
    }
}

struct Node {
    form: String,
    parent: Option<usize>,
    /// (production index, position in the parent's form)
    apps: Vec<(usize, usize)>,
}

type Apps = Vec<(usize, usize)>;

/// JFLAP `getPossibilities`, lazily and in the same order: either keep the next character, or
/// apply a production at the first occurrence of its left side, then continue after it.
fn expand(
    prods: &[(Vec<char>, Vec<char>)],
    s: &[char],
    offset: usize,
    prefix: &mut Vec<char>,
    apps: &mut Apps,
    f: &mut dyn FnMut(&[char], &Apps) -> ControlFlow<()>,
) -> ControlFlow<()> {
    if offset == s.len() {
        return f(prefix, apps);
    }
    let keep = prefix.len();
    prefix.push(s[offset]);
    expand(prods, s, offset + 1, prefix, apps, f)?;
    prefix.truncate(keep);
    for (i, (lhs, rhs)) in prods.iter().enumerate() {
        let Some(n) = find_from(s, lhs, offset) else {
            continue;
        };
        prefix.extend_from_slice(&s[offset..n]);
        prefix.extend_from_slice(rhs);
        apps.push((i, n));
        expand(prods, s, n + lhs.len(), prefix, apps, f)?;
        apps.pop();
        prefix.truncate(keep);
    }
    ControlFlow::Continue(())
}

/// Brute-force parse. `max_nodes` bounds the candidate forms examined; when it runs out the
/// outcome is [`ParseOutcome::Undecided`]. The input must only use terminals of the grammar.
pub fn brute_parse(g: &Grammar, input: &str, max_nodes: usize) -> Result<BruteResult, ParseError> {
    check_input(g, input)?;
    let kind = kind_of(g);
    let blank = |outcome| BruteResult {
        outcome,
        kind,
        derivation: None,
        tree: None,
        nodes_considered: 0,
        candidates_examined: 0,
    };
    let Some(prods) = optimize(g) else {
        return Ok(blank(ParseOutcome::Rejected));
    };
    let smaller = smaller_symbols(&prods);
    let target: Vec<char> = input.chars().collect();
    let chars: Vec<(Vec<char>, Vec<char>)> = prods
        .iter()
        .map(|p| (p.lhs.chars().collect(), p.rhs.chars().collect()))
        .collect();

    let mut nodes = vec![Node {
        form: g.start().to_string(),
        parent: None,
        apps: Vec::new(),
    }];
    let mut queue: VecDeque<usize> = VecDeque::from([0]);
    let mut seen: HashSet<String> = HashSet::new();
    let mut examined = 0usize;
    let mut answer: Option<usize> = None;
    let mut out_of_budget = false;

    'search: while let Some(id) = queue.pop_front() {
        let form: Vec<char> = nodes[id].form.chars().collect();
        let mut found = Vec::new();
        let flow = expand(
            &chars,
            &form,
            0,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut |cand, apps| {
                if examined >= max_nodes {
                    out_of_budget = true;
                    return ControlFlow::Break(());
                }
                examined += 1;
                let text: String = cand.iter().collect();
                if !seen.insert(text.clone()) || !possible_with(kind, cand, &target, &smaller) {
                    return ControlFlow::Continue(());
                }
                let is_answer = cand == target.as_slice();
                found.push((text, apps.clone(), is_answer));
                if is_answer {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            },
        );
        for (text, apps, is_answer) in found {
            nodes.push(Node {
                form: text,
                parent: Some(id),
                apps,
            });
            queue.push_back(nodes.len() - 1);
            if is_answer {
                answer = Some(nodes.len() - 1);
            }
        }
        if flow.is_break() {
            break 'search;
        }
    }

    let nodes_considered = nodes.len() - 1;
    let Some(answer) = answer else {
        let outcome = if out_of_budget {
            ParseOutcome::Undecided
        } else {
            ParseOutcome::Rejected
        };
        return Ok(BruteResult {
            nodes_considered,
            candidates_examined: examined,
            ..blank(outcome)
        });
    };

    let mut path = vec![answer];
    while let Some(parent) = nodes[*path.last().expect("path is not empty")].parent {
        path.push(parent);
    }
    path.reverse();
    let mut derivation = Derivation::new(g.start());
    for pair in path.windows(2) {
        let (parent, node) = (&nodes[pair[0]], &nodes[pair[1]]);
        let mut current = parent.form.clone();
        let mut delta: isize = 0;
        for &(pi, pos) in &node.apps {
            let production = &prods[pi];
            let at = (pos as isize + delta) as usize;
            current = replace_at(
                &current,
                at,
                production.lhs.chars().count(),
                &production.rhs,
            );
            delta +=
                production.rhs.chars().count() as isize - production.lhs.chars().count() as isize;
            derivation.steps.push(DerivationStep {
                production: production.clone(),
                position: at,
                result: current.clone(),
            });
        }
    }
    let tree = if kind == BruteKind::Restricted {
        derivation.tree()
    } else {
        None
    };
    Ok(BruteResult {
        outcome: ParseOutcome::Accepted,
        kind,
        derivation: Some(derivation),
        tree,
        nodes_considered,
        candidates_examined: examined,
    })
}
