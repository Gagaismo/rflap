//! SLR(1): LR(0) item sets, ACTION/GOTO table and shift-reduce parsing (JFLAP
//! `Operations.closure/goTo`, `LRParseTableGenerator`, `LRParseController`).
//!
//! The grammar is augmented with `S' → S` as production 0, so `rN` reduces by the N-th
//! production of the augmented list (the first user production is `r1`), as in JFLAP.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use super::ops::{END_MARKER, follow_sets};
use super::{Derivation, DerivationStep, ParseError, ParseOutcome, ParseTree};
use crate::grammar::model::{Grammar, Production, is_variable_symbol};

/// An LR(0) item: production index into [`LrAutomaton::productions`] and the dot position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LrItem {
    pub production: usize,
    pub dot: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LrState {
    pub id: usize,
    /// Closed item set, sorted by production then dot.
    pub items: Vec<LrItem>,
    /// Goto on each grammar symbol (sorted).
    pub transitions: BTreeMap<String, usize>,
    /// Some item has its dot at the end (JFLAP marks these states final).
    pub is_final: bool,
}

/// The LR(0) DFA. States are numbered in JFLAP's order: breadth first, symbols in ascending
/// order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LrAutomaton {
    /// Production 0 is the augmented `S' → S`.
    pub productions: Vec<Production>,
    pub states: Vec<LrState>,
}

impl LrAutomaton {
    /// The item as JFLAP writes it, e.g. `E'→·E`.
    pub fn item_text(&self, item: &LrItem) -> String {
        let p = &self.productions[item.production];
        let mut rhs: Vec<char> = p.rhs.chars().collect();
        rhs.insert(item.dot.min(rhs.len()), '·');
        format!("{}→{}", p.lhs, rhs.into_iter().collect::<String>())
    }
}

fn augment(g: &Grammar) -> Vec<Production> {
    let mut out = Vec::with_capacity(g.productions().len() + 1);
    out.push(Production {
        lhs: format!("{}'", g.start()),
        rhs: g.start().to_string(),
    });
    out.extend(g.productions().iter().cloned());
    out
}

fn next_symbol(productions: &[Production], item: LrItem) -> Option<char> {
    productions[item.production].rhs.chars().nth(item.dot)
}

fn closure(productions: &[Production], seed: BTreeSet<LrItem>) -> BTreeSet<LrItem> {
    let mut set = seed;
    let mut pending: Vec<LrItem> = set.iter().copied().collect();
    while let Some(item) = pending.pop() {
        let Some(sym) = next_symbol(productions, item) else {
            continue;
        };
        if !is_variable_symbol(sym) {
            continue;
        }
        let lhs = sym.to_string();
        for (j, p) in productions.iter().enumerate().skip(1) {
            if p.lhs == lhs {
                let new = LrItem {
                    production: j,
                    dot: 0,
                };
                if set.insert(new) {
                    pending.push(new);
                }
            }
        }
    }
    set
}

fn goto(productions: &[Production], items: &BTreeSet<LrItem>, sym: char) -> BTreeSet<LrItem> {
    let moved: BTreeSet<LrItem> = items
        .iter()
        .filter(|&&i| next_symbol(productions, i) == Some(sym))
        .map(|i| LrItem {
            production: i.production,
            dot: i.dot + 1,
        })
        .collect();
    closure(productions, moved)
}

/// Builds the LR(0) automaton of the augmented grammar.
pub fn slr_automaton(g: &Grammar) -> LrAutomaton {
    let productions = augment(g);
    if g.productions().is_empty() {
        return LrAutomaton {
            productions,
            states: vec![LrState {
                id: 0,
                items: Vec::new(),
                transitions: BTreeMap::new(),
                is_final: false,
            }],
        };
    }
    let initial = closure(
        &productions,
        BTreeSet::from([LrItem {
            production: 0,
            dot: 0,
        }]),
    );
    let mut sets: Vec<BTreeSet<LrItem>> = vec![initial.clone()];
    let mut index: HashMap<BTreeSet<LrItem>, usize> = HashMap::from([(initial, 0)]);
    let mut transitions: Vec<BTreeMap<String, usize>> = vec![BTreeMap::new()];
    let mut i = 0;
    while i < sets.len() {
        let items = sets[i].clone();
        let symbols: BTreeSet<char> = items
            .iter()
            .filter_map(|&it| next_symbol(&productions, it))
            .collect();
        for sym in symbols {
            let target = goto(&productions, &items, sym);
            let id = match index.get(&target) {
                Some(&id) => id,
                None => {
                    let id = sets.len();
                    index.insert(target.clone(), id);
                    sets.push(target);
                    transitions.push(BTreeMap::new());
                    id
                }
            };
            transitions[i].insert(sym.to_string(), id);
        }
        i += 1;
    }
    let states = sets
        .into_iter()
        .zip(transitions)
        .enumerate()
        .map(|(id, (items, transitions))| LrState {
            id,
            is_final: items
                .iter()
                .any(|it| it.dot == productions[it.production].rhs.chars().count()),
            items: items.into_iter().collect(),
            transitions,
        })
        .collect();
    LrAutomaton {
        productions,
        states,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LrAction {
    Shift(usize),
    /// Reduce by the production with this index in the augmented list.
    Reduce(usize),
    Accept,
}

impl LrAction {
    /// JFLAP's cell text: `s3`, `r4`, `acc`.
    pub fn text(&self) -> String {
        match self {
            LrAction::Shift(n) => format!("s{n}"),
            LrAction::Reduce(n) => format!("r{n}"),
            LrAction::Accept => "acc".to_string(),
        }
    }
}

/// A cell with more than one action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LrConflict {
    pub state: usize,
    pub symbol: String,
    pub actions: Vec<LrAction>,
}

/// The SLR(1) ACTION/GOTO table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LrTable {
    /// The augmented productions (index 0 is `S' → S`).
    pub productions: Vec<Production>,
    pub automaton: LrAutomaton,
    /// ACTION columns: sorted terminals, then `$`.
    pub terminals: Vec<String>,
    /// GOTO columns: sorted variables.
    pub variables: Vec<String>,
    /// Per state: terminal → actions, sorted by their text (JFLAP's cell order).
    pub actions: Vec<BTreeMap<String, Vec<LrAction>>>,
    /// Per state: variable → state.
    pub gotos: Vec<BTreeMap<String, usize>>,
}

impl LrTable {
    /// The cell as JFLAP shows it: actions joined by a space, or the goto state; empty if none.
    pub fn cell_string(&self, state: usize, symbol: &str) -> String {
        if let Some(n) = self.gotos.get(state).and_then(|m| m.get(symbol)) {
            return n.to_string();
        }
        self.actions
            .get(state)
            .and_then(|m| m.get(symbol))
            .map(|a| a.iter().map(LrAction::text).collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
    }

    pub fn conflicts(&self) -> Vec<LrConflict> {
        let mut out = Vec::new();
        for (state, row) in self.actions.iter().enumerate() {
            for t in &self.terminals {
                if let Some(a) = row.get(t).filter(|a| a.len() > 1) {
                    out.push(LrConflict {
                        state,
                        symbol: t.clone(),
                        actions: a.clone(),
                    });
                }
            }
        }
        out
    }

    pub fn is_slr1(&self) -> bool {
        self.conflicts().is_empty()
    }
}

/// Builds the SLR(1) table: shifts and gotos from the DFA, reductions under FOLLOW of the
/// left side, `acc` on `$` for the augmented production.
pub fn slr_table(g: &Grammar) -> LrTable {
    let automaton = slr_automaton(g);
    let follow = follow_sets(g);
    let mut terminals: Vec<String> = g.terminals().iter().map(char::to_string).collect();
    terminals.push(END_MARKER.to_string());
    let variables: Vec<String> = g.variables().iter().map(char::to_string).collect();
    let mut actions: Vec<BTreeMap<String, BTreeMap<String, LrAction>>> =
        vec![BTreeMap::new(); automaton.states.len()];
    let mut gotos: Vec<BTreeMap<String, usize>> = vec![BTreeMap::new(); automaton.states.len()];
    let mut add = |state: usize, sym: &str, a: LrAction| {
        actions[state]
            .entry(sym.to_string())
            .or_default()
            .insert(a.text(), a);
    };
    for st in &automaton.states {
        for (sym, &to) in &st.transitions {
            let is_var = sym.chars().next().is_some_and(is_variable_symbol);
            if is_var {
                gotos[st.id].insert(sym.clone(), to);
            } else {
                add(st.id, sym, LrAction::Shift(to));
            }
        }
        for it in &st.items {
            let p = &automaton.productions[it.production];
            if it.dot != p.rhs.chars().count() {
                continue;
            }
            if it.production == 0 {
                add(st.id, END_MARKER, LrAction::Accept);
            } else {
                for t in follow.get(&p.lhs).into_iter().flatten() {
                    add(st.id, t, LrAction::Reduce(it.production));
                }
            }
        }
    }
    let actions = actions
        .into_iter()
        .map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, v.into_values().collect()))
                .collect()
        })
        .collect();
    LrTable {
        productions: automaton.productions.clone(),
        automaton,
        terminals,
        variables,
        actions,
        gotos,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LrStepAction {
    Shift {
        state: usize,
    },
    Reduce {
        production_index: usize,
        production: Production,
    },
    Accept,
    Reject {
        reason: String,
    },
}

/// One move, recorded with the configuration it started from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LrStep {
    /// State stack, bottom first.
    pub states: Vec<usize>,
    /// Grammar symbols on the stack, bottom first.
    pub symbols: String,
    /// Remaining input, ending with `$`.
    pub input: String,
    pub action: LrStepAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LrParseResult {
    pub outcome: ParseOutcome,
    pub steps: Vec<LrStep>,
    /// The rightmost derivation (start symbol to input) behind the reductions; only when
    /// accepted.
    pub derivation: Option<Derivation>,
    pub tree: Option<ParseTree>,
}

/// Parses `input` with the SLR(1) table; fails with [`ParseError::NotSlr1`] if the table has
/// conflicts. Symbols outside the grammar are just rejected.
pub fn slr_parse(g: &Grammar, input: &str) -> Result<LrParseResult, ParseError> {
    let table = slr_table(g);
    let conflicts = table.conflicts();
    if !conflicts.is_empty() {
        return Err(ParseError::NotSlr1 { conflicts });
    }
    let text: Vec<char> = input.chars().chain(std::iter::once('$')).collect();
    let mut p = 0usize;
    let mut states: Vec<usize> = vec![0];
    let mut symbols: Vec<char> = Vec::new();
    let mut steps = Vec::new();
    // (production, position of its left side, sentential form after the reduction)
    let mut reductions: Vec<(Production, usize, String)> = Vec::new();
    let mut guard = 0usize;
    let outcome = loop {
        guard += 1;
        let lookahead = text.get(p).copied().unwrap_or('$');
        let snapshot =
            |action: LrStepAction, states: &[usize], symbols: &[char], p: usize| LrStep {
                states: states.to_vec(),
                symbols: symbols.iter().collect(),
                input: text[p.min(text.len())..].iter().collect(),
                action,
            };
        if guard > 100_000 {
            let reason = "step limit exceeded".to_string();
            steps.push(snapshot(
                LrStepAction::Reject { reason },
                &states,
                &symbols,
                p,
            ));
            break ParseOutcome::Undecided;
        }
        let state = *states.last().expect("the state stack is never empty");
        let action = table
            .actions
            .get(state)
            .and_then(|m| m.get(&lookahead.to_string()))
            .and_then(|a| a.first())
            .copied();
        match action {
            None => {
                let reason = format!("No action for state {state} on {lookahead}.");
                steps.push(snapshot(
                    LrStepAction::Reject { reason },
                    &states,
                    &symbols,
                    p,
                ));
                break ParseOutcome::Rejected;
            }
            Some(LrAction::Accept) => {
                steps.push(snapshot(LrStepAction::Accept, &states, &symbols, p));
                break ParseOutcome::Accepted;
            }
            Some(LrAction::Shift(to)) => {
                steps.push(snapshot(
                    LrStepAction::Shift { state: to },
                    &states,
                    &symbols,
                    p,
                ));
                symbols.push(lookahead);
                states.push(to);
                p += 1;
            }
            Some(LrAction::Reduce(n)) => {
                let production = table.productions[n].clone();
                steps.push(snapshot(
                    LrStepAction::Reduce {
                        production_index: n,
                        production: production.clone(),
                    },
                    &states,
                    &symbols,
                    p,
                ));
                let k = production.rhs.chars().count();
                symbols.truncate(symbols.len() - k);
                states.truncate(states.len() - k);
                let top = *states.last().expect("the state stack is never empty");
                let Some(&to) = table.gotos[top].get(&production.lhs) else {
                    let reason = format!("No goto for {} from state {top}.", production.lhs);
                    steps.push(snapshot(
                        LrStepAction::Reject { reason },
                        &states,
                        &symbols,
                        p,
                    ));
                    break ParseOutcome::Rejected;
                };
                symbols.extend(production.lhs.chars());
                states.push(to);
                let rest: String = text[p..text.len() - 1].iter().collect();
                let form = format!("{}{}", symbols.iter().collect::<String>(), rest);
                reductions.push((production, symbols.len() - 1, form));
            }
        }
    };

    let (derivation, tree) = if outcome == ParseOutcome::Accepted {
        let mut d = Derivation::new(g.start());
        for k in (0..reductions.len()).rev() {
            let before = if k == 0 {
                input.to_string()
            } else {
                reductions[k - 1].2.clone()
            };
            d.steps.push(DerivationStep {
                production: reductions[k].0.clone(),
                position: reductions[k].1,
                result: before,
            });
        }
        let tree = d.tree();
        (Some(d), tree)
    } else {
        (None, None)
    };
    Ok(LrParseResult {
        outcome,
        steps,
        derivation,
        tree,
    })
}
