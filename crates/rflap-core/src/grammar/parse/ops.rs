//! FIRST and FOLLOW sets (JFLAP `grammar.parse.Operations`).

use std::collections::{BTreeMap, BTreeSet};

use crate::grammar::model::Grammar;

/// JFLAP's end-of-input marker, a member of FOLLOW sets and the last column of parse tables.
pub const END_MARKER: &str = "$";

type Sets = BTreeMap<String, BTreeSet<String>>;

/// FIRST of every terminal (itself) and every variable. The empty string `""` stands for λ.
pub fn first_sets(g: &Grammar) -> Sets {
    let mut first: Sets = BTreeMap::new();
    for t in g.terminals() {
        first.insert(t.to_string(), BTreeSet::from([t.to_string()]));
    }
    for v in g.variables() {
        first.insert(v.to_string(), BTreeSet::new());
    }
    loop {
        let mut changed = false;
        for p in g.productions() {
            let add = first_of_string(&first, &p.rhs);
            let entry = first.entry(p.lhs.clone()).or_default();
            for s in add {
                changed |= entry.insert(s);
            }
        }
        if !changed {
            return first;
        }
    }
}

/// FIRST of a string of symbols given FIRST of each symbol (`""` marks λ).
pub fn first_of_string(first: &Sets, s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut nullable = true;
    for c in s.chars() {
        let empty = BTreeSet::new();
        let set = first.get(&c.to_string()).unwrap_or(&empty);
        out.extend(set.iter().filter(|x| !x.is_empty()).cloned());
        if !set.contains("") {
            nullable = false;
            break;
        }
    }
    if nullable {
        out.insert(String::new());
    }
    out
}

/// FOLLOW of every variable; the start variable's set contains [`END_MARKER`].
pub fn follow_sets(g: &Grammar) -> Sets {
    let first = first_sets(g);
    let mut follow: Sets = BTreeMap::new();
    for v in g.variables() {
        follow.insert(v.to_string(), BTreeSet::new());
    }
    if !g.start().is_empty() {
        follow
            .entry(g.start().to_string())
            .or_default()
            .insert(END_MARKER.to_string());
    }
    loop {
        let mut changed = false;
        for p in g.productions() {
            let rhs: Vec<char> = p.rhs.chars().collect();
            for (j, &c) in rhs.iter().enumerate() {
                if !g.is_variable(c) {
                    continue;
                }
                let rest: String = rhs[j + 1..].iter().collect();
                let mut f = first_of_string(&first, &rest);
                let target = c.to_string();
                if f.remove("") {
                    let lhs_follow = follow.get(&p.lhs).cloned().unwrap_or_default();
                    let entry = follow.entry(target.clone()).or_default();
                    for s in lhs_follow {
                        changed |= entry.insert(s);
                    }
                }
                let entry = follow.entry(target).or_default();
                for s in f {
                    changed |= entry.insert(s);
                }
            }
        }
        if !changed {
            return follow;
        }
    }
}
