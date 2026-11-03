//! Language equality of two FAs with the shortest distinguishing string.
//!
//! JFLAP (`FSAEqualityChecker`) converts both to DFAs, minimizes them and checks that they are
//! isomorphic. Rflap compares the languages directly with a breadth-first search over the
//! product of the two DFAs, which gives the same verdict and a shortest counterexample.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use super::Equivalence;
use super::common::normalize;
use super::subset::determinize;
use crate::automaton::StateId;
use crate::fa::Fa;

struct Dfa {
    initial: Option<StateId>,
    finals: BTreeSet<StateId>,
    next: HashMap<(StateId, char), StateId>,
    symbols: BTreeSet<char>,
}

fn dfa_of(fa: &Fa) -> Dfa {
    let conv = determinize(&normalize(fa));
    let mut next = HashMap::new();
    let mut symbols = BTreeSet::new();
    for t in conv.dfa.transitions() {
        if let Some(c) = t.label.0.chars().next() {
            next.insert((t.from, c), t.to);
            symbols.insert(c);
        }
    }
    Dfa {
        initial: conv.dfa.initial(),
        finals: conv.dfa.finals().clone(),
        next,
        symbols,
    }
}

pub(super) fn equivalent(a: &Fa, b: &Fa) -> Equivalence {
    let (da, db) = (dfa_of(a), dfa_of(b));
    let symbols: BTreeSet<char> = da.symbols.union(&db.symbols).copied().collect();
    let accepting = |d: &Dfa, s: Option<StateId>| s.is_some_and(|s| d.finals.contains(&s));
    let step = |d: &Dfa, s: Option<StateId>, c: char| s.and_then(|s| d.next.get(&(s, c)).copied());

    let start = (da.initial, db.initial);
    let mut seen: HashSet<(Option<StateId>, Option<StateId>)> = HashSet::from([start]);
    let mut queue = VecDeque::from([(start, String::new())]);
    while let Some(((x, y), word)) = queue.pop_front() {
        if accepting(&da, x) != accepting(&db, y) {
            return Equivalence {
                equivalent: false,
                counterexample: Some(word),
            };
        }
        for &c in &symbols {
            let pair = (step(&da, x, c), step(&db, y, c));
            if seen.insert(pair) {
                queue.push_back((pair, format!("{word}{c}")));
            }
        }
    }
    Equivalence {
        equivalent: true,
        counterexample: None,
    }
}
