//! Pushdown automaton to context-free grammar (JFLAP `PDAToCFGConverter` driven by
//! `PDAConvertController`): the triple-variable construction, then JFLAP's purge of the
//! productions that cannot derive a string or cannot be reached from the start variable.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{ConvertError, TransitionRef};
use crate::automaton::{StateId, Transition};
use crate::grammar::{Grammar, Production};
use crate::pda::{INITIAL_STACK, Pda, PdaLabel};

/// A production written over triple variables such as `(q0Zq2)`, before they get letters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TripleProduction {
    pub lhs: String,
    pub rhs: String,
}

/// What a triple variable `(p X q)` stands for and the letter it ends up with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TripleVariable {
    pub variable: char,
    pub triple: String,
    pub from: StateId,
    pub stack: char,
    pub to: StateId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PdaToCfgStep {
    /// The productions JFLAP reveals for a transition, over triple variables.
    Transition {
        from: StateId,
        to: StateId,
        label: PdaLabel,
        productions: Vec<TripleProduction>,
    },
    /// JFLAP's trim before exporting: the productions that cannot derive a string or cannot
    /// be reached from `S` are dropped.
    Purge { removed: Vec<TripleProduction> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdaToCfg {
    pub grammar: Grammar,
    /// The variables left after the purge, in order of the grammar's productions.
    pub variables: Vec<TripleVariable>,
    pub steps: Vec<PdaToCfgStep>,
}

fn refs<'a>(ts: impl Iterator<Item = &'a Transition<PdaLabel>>) -> Vec<TransitionRef> {
    ts.map(|t| TransitionRef {
        from: t.from,
        to: t.to,
        label: t.label.clone(),
    })
    .collect()
}

/// The checks JFLAP makes before converting, in its order and with its messages: an initial
/// state, exactly one final state, transitions to the final state that pop only `Z`, and
/// transitions that pop one symbol and push none or two.
pub fn check_pda_form(pda: &Pda) -> Result<(), ConvertError> {
    if pda.initial().is_none() {
        return Err(ConvertError::NoInitialState);
    }
    if pda.finals().len() != 1 {
        return Err(ConvertError::NotExactlyOneFinalState {
            count: pda.finals().len(),
        });
    }
    let final_state = *pda.finals().iter().next().expect("one final state");
    let bad_final = refs(
        pda.transitions_to(final_state)
            .filter(|t| t.label.pop != INITIAL_STACK),
    );
    if !bad_final.is_empty() {
        return Err(ConvertError::FinalTransitionsMustPopZ {
            transitions: bad_final,
        });
    }
    let bad_form = refs(pda.transitions().iter().filter(|t| {
        let push = t.label.push.chars().count();
        t.label.pop.chars().count() != 1 || (push != 0 && push != 2)
    }));
    if !bad_form.is_empty() {
        return Err(ConvertError::TransitionsMustPopOnePushZeroOrTwo {
            transitions: bad_form,
        });
    }
    Ok(())
}

/// A production before simplification: `lhs -> input t1 t2` over triples.
struct Raw {
    lhs: String,
    input: String,
    tokens: Vec<String>,
}

impl Raw {
    fn text(&self) -> TripleProduction {
        TripleProduction {
            lhs: self.lhs.clone(),
            rhs: format!("{}{}", self.input, self.tokens.concat()),
        }
    }
}

/// Hands out the letters the way `PDAToCFGConverter.getUniqueVariable` does: `A`, `B`, ...
/// skipping `S`, and past `Z` the following code points (they are renamed after the purge).
struct Namer {
    names: HashMap<String, char>,
    triples: Vec<(String, StateId, char, StateId)>,
    next: u32,
    start: String,
}

impl Namer {
    fn variable(&mut self, from: (StateId, &str), stack: char, to: (StateId, &str)) -> String {
        let triple = format!("({}{}{})", from.1, stack, to.1);
        if !self.names.contains_key(&triple) {
            let name = if triple == self.start {
                'S'
            } else {
                let c = char::from_u32(65 + self.next).expect("code point");
                self.next += 1;
                if 65 + self.next == 83 {
                    self.next += 1;
                }
                c
            };
            self.names.insert(triple.clone(), name);
            self.triples.push((triple.clone(), from.0, stack, to.0));
        }
        triple
    }
}

fn first_char(s: &str) -> char {
    s.chars().next().expect("checked: one popped symbol")
}

/// Triple-variable construction (JFLAP "Convert to Grammar" for pushdown automata), after
/// [`check_pda_form`]. A transition `p -a,X;λ-> q` gives `(pXq) -> a` and `p -a,X;YW-> q`
/// gives `(pXr) -> a (qYs)(sWr)` for every pair of states `r`, `s`. Only the variables that
/// derive a string and are reachable from the start variable `(i Z f)` survive; they take
/// the letters JFLAP gives them (`S` for the start variable).
///
/// Differences from JFLAP: productions are listed in transition order with the productions
/// of `S` first (JFLAP's sort comparator is not a valid ordering), and the letters assigned
/// after the trim follow JFLAP's hash-map order of the triple strings.
pub fn pda_to_cfg(pda: &Pda) -> Result<PdaToCfg, ConvertError> {
    check_pda_form(pda)?;
    let initial = pda.initial().expect("checked");
    let final_state = *pda.finals().iter().next().expect("checked");
    let name_of = |id: StateId| pda.state(id).map(|s| s.name.clone()).unwrap_or_default();
    let states: Vec<(StateId, String)> = pda.states().map(|s| (s.id, s.name.clone())).collect();
    let (initial_name, final_name) = (name_of(initial), name_of(final_state));

    let mut namer = Namer {
        names: HashMap::new(),
        triples: Vec::new(),
        next: 0,
        start: format!("({initial_name}{INITIAL_STACK}{final_name})"),
    };
    let mut raw: Vec<Raw> = Vec::new();
    let mut steps = Vec::new();
    for t in pda.transitions() {
        let (from, to) = ((t.from, name_of(t.from)), (t.to, name_of(t.to)));
        let pop = first_char(&t.label.pop);
        let mut made = Vec::new();
        if t.label.push.is_empty() {
            let lhs = namer.variable((from.0, &from.1), pop, (to.0, &to.1));
            made.push(Raw {
                lhs,
                input: t.label.input.clone(),
                tokens: Vec::new(),
            });
        } else {
            let mut push = t.label.push.chars();
            let (y, w) = (
                push.next().expect("two symbols"),
                push.next().expect("two symbols"),
            );
            for (i, i_name) in &states {
                for (j, j_name) in &states {
                    let lhs = namer.variable((from.0, &from.1), pop, (*i, i_name));
                    let first = namer.variable((to.0, &to.1), y, (*j, j_name));
                    let second = namer.variable((*j, j_name), w, (*i, i_name));
                    made.push(Raw {
                        lhs,
                        input: t.label.input.clone(),
                        tokens: vec![first, second],
                    });
                }
            }
        }
        steps.push(PdaToCfgStep::Transition {
            from: t.from,
            to: t.to,
            label: t.label.clone(),
            productions: made.iter().map(Raw::text).collect(),
        });
        raw.extend(made);
    }

    // Variables that derive a string.
    let mut generating: HashSet<&str> = HashSet::new();
    loop {
        let mut changed = false;
        for p in &raw {
            if !generating.contains(p.lhs.as_str())
                && p.tokens.iter().all(|t| generating.contains(t.as_str()))
            {
                generating.insert(&p.lhs);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // Productions reachable from the start variable through generating ones.
    let mut kept = vec![false; raw.len()];
    let mut pending = vec![namer.start.clone()];
    let mut visited: HashSet<String> = HashSet::new();
    while let Some(v) = pending.pop() {
        if !visited.insert(v.clone()) {
            continue;
        }
        for (i, p) in raw.iter().enumerate() {
            if p.lhs == v && p.tokens.iter().all(|t| generating.contains(t.as_str())) {
                kept[i] = true;
                pending.extend(p.tokens.iter().cloned());
            }
        }
    }
    let removed: Vec<TripleProduction> = raw
        .iter()
        .zip(&kept)
        .filter(|(_, k)| !**k)
        .map(|(p, _)| p.text())
        .collect();
    if !removed.is_empty() {
        steps.push(PdaToCfgStep::Purge { removed });
    }

    // Letters: those up to Z stay, the others are renamed with the unused ones in the order
    // JFLAP's HashMap over the surviving variables iterates them.
    let mut order: Vec<&str> = Vec::new();
    for (p, _) in raw.iter().zip(&kept).rev().filter(|(_, k)| **k) {
        if !order.contains(&p.lhs.as_str()) {
            order.push(&p.lhs);
        }
    }
    let mut name: HashMap<String, char> = HashMap::new();
    let used: HashSet<char> = order
        .iter()
        .map(|v| namer.names[*v])
        .filter(|c| *c <= 'Z')
        .collect();
    let mut free = ('A'..='Z').filter(|c| !used.contains(c));
    for v in java_hash_order(&order) {
        let old = namer.names[v];
        if old <= 'Z' {
            name.insert(v.to_string(), old);
        } else {
            let Some(new) = free.next() else {
                return Err(ConvertError::TooManyVariables { count: order.len() });
            };
            name.insert(v.to_string(), new);
        }
    }

    let mut productions: Vec<Production> = raw
        .iter()
        .zip(&kept)
        .filter(|(_, k)| **k)
        .map(|(p, _)| {
            let rhs: String = p.tokens.iter().map(|t| name[t]).collect();
            Production::new(&name[&p.lhs].to_string(), &format!("{}{rhs}", p.input))
        })
        .collect();
    let (mut ordered, rest): (Vec<_>, Vec<_>) = productions.drain(..).partition(|p| p.lhs == "S");
    ordered.extend(rest);
    let grammar = Grammar::from_parts("S", ordered).expect("left sides are never empty");

    let mut variables = Vec::new();
    for p in grammar.productions() {
        let letter = first_char(&p.lhs);
        if variables
            .iter()
            .any(|v: &TripleVariable| v.variable == letter)
        {
            continue;
        }
        if let Some((triple, from, stack, to)) = namer
            .triples
            .iter()
            .find(|(t, ..)| name.get(t) == Some(&letter))
        {
            variables.push(TripleVariable {
                variable: letter,
                triple: triple.clone(),
                from: *from,
                stack: *stack,
                to: *to,
            });
        }
    }
    Ok(PdaToCfg {
        grammar,
        variables,
        steps,
    })
}

/// The keys in the order a Java `HashMap<String, _>` iterates them after inserting them in
/// the given order: by bucket of the spread `String.hashCode`, ties by insertion.
fn java_hash_order<'a>(keys: &[&'a str]) -> Vec<&'a str> {
    let mut capacity = 16usize;
    while keys.len() > capacity * 3 / 4 {
        capacity *= 2;
    }
    let mut keyed: Vec<(usize, usize, &str)> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let h = k
                .encode_utf16()
                .fold(0i32, |h, u| h.wrapping_mul(31).wrapping_add(i32::from(u)))
                as u32;
            (((h ^ (h >> 16)) as usize) & (capacity - 1), i, *k)
        })
        .collect();
    keyed.sort();
    keyed.into_iter().map(|(_, _, k)| k).collect()
}
