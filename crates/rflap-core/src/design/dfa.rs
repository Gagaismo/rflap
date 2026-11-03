//! Complete DFAs over a fixed alphabet, with a meaning for every state.
//!
//! Every state carries `parts`: short phrases, one per leaf condition of the constraint it
//! was built from. Minimization keeps the phrases all merged states agree on, so the label
//! of a minimal state says exactly what that state remembers.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::error::DesignError;
use super::spec::Cmp;

#[derive(Clone, Debug)]
pub(crate) struct Dfa {
    pub alphabet: Vec<char>,
    /// `next[state][symbol index]`.
    pub next: Vec<Vec<usize>>,
    pub accept: Vec<bool>,
    pub start: usize,
    pub parts: Vec<Vec<String>>,
}

pub(crate) fn too_large(limit: usize) -> DesignError {
    DesignError::TooLarge {
        message: format!(
            "the automaton needs more than {limit} states; simplify the constraint \
             or raise `max_states`"
        ),
    }
}

impl Dfa {
    pub fn len(&self) -> usize {
        self.next.len()
    }

    /// Single accepting or rejecting state looping on everything.
    pub fn constant(alphabet: &[char], accepting: bool, part: &str) -> Dfa {
        Dfa {
            alphabet: alphabet.to_vec(),
            next: vec![vec![0; alphabet.len()]],
            accept: vec![accepting],
            start: 0,
            parts: vec![vec![part.to_string()]],
        }
    }

    pub fn negate(mut self) -> Dfa {
        for a in &mut self.accept {
            *a = !*a;
        }
        self
    }

    /// Product automaton over the reachable pairs; `op` combines the acceptance bits.
    pub fn product(
        &self,
        other: &Dfa,
        op: impl Fn(bool, bool) -> bool,
        max_states: usize,
    ) -> Result<Dfa, DesignError> {
        let k = self.alphabet.len();
        let mut index: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut pairs = vec![(self.start, other.start)];
        index.insert(pairs[0], 0);
        let mut next: Vec<Vec<usize>> = Vec::new();
        let mut i = 0;
        while i < pairs.len() {
            let (p, q) = pairs[i];
            let mut row = Vec::with_capacity(k);
            for c in 0..k {
                let target = (self.next[p][c], other.next[q][c]);
                let id = match index.get(&target) {
                    Some(id) => *id,
                    None => {
                        if pairs.len() >= max_states {
                            return Err(too_large(max_states));
                        }
                        pairs.push(target);
                        index.insert(target, pairs.len() - 1);
                        pairs.len() - 1
                    }
                };
                row.push(id);
            }
            next.push(row);
            i += 1;
        }
        Ok(Dfa {
            alphabet: self.alphabet.clone(),
            accept: pairs
                .iter()
                .map(|&(p, q)| op(self.accept[p], other.accept[q]))
                .collect(),
            parts: pairs
                .iter()
                .map(|&(p, q)| [self.parts[p].clone(), other.parts[q].clone()].concat())
                .collect(),
            start: 0,
            next,
        })
    }

    /// Minimal equivalent DFA: reachable states only, merged by Moore refinement, numbered in
    /// breadth-first order from the start (symbols in alphabet order).
    pub fn minimize(&self) -> Dfa {
        let k = self.alphabet.len();
        // Reachable states in BFS order.
        let mut order = vec![self.start];
        let mut seen: BTreeSet<usize> = BTreeSet::from([self.start]);
        let mut i = 0;
        while i < order.len() {
            for c in 0..k {
                let t = self.next[order[i]][c];
                if seen.insert(t) {
                    order.push(t);
                }
            }
            i += 1;
        }
        let mut class: BTreeMap<usize, usize> = order
            .iter()
            .map(|&s| (s, usize::from(self.accept[s])))
            .collect();
        let mut count = class.values().collect::<BTreeSet<_>>().len();
        loop {
            let mut signatures: BTreeMap<Vec<usize>, usize> = BTreeMap::new();
            let mut fresh: BTreeMap<usize, usize> = BTreeMap::new();
            for &s in &order {
                let mut sig = vec![class[&s]];
                sig.extend((0..k).map(|c| class[&self.next[s][c]]));
                let n = signatures.len();
                let id = *signatures.entry(sig).or_insert(n);
                fresh.insert(s, id);
            }
            let new_count = signatures.len();
            class = fresh;
            if new_count == count {
                break;
            }
            count = new_count;
        }
        // Quotient, renumbered in BFS order over classes.
        let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for &s in &order {
            members.entry(class[&s]).or_default().push(s);
        }
        let start_class = class[&self.start];
        let mut names: BTreeMap<usize, usize> = BTreeMap::from([(start_class, 0)]);
        let mut queue = VecDeque::from([start_class]);
        let mut result = Dfa {
            alphabet: self.alphabet.clone(),
            next: Vec::new(),
            accept: Vec::new(),
            start: 0,
            parts: Vec::new(),
        };
        while let Some(c) = queue.pop_front() {
            let group = &members[&c];
            let rep = group[0];
            let mut row = Vec::with_capacity(k);
            for symbol in 0..k {
                let target = class[&self.next[rep][symbol]];
                let n = names.len();
                let id = *names.entry(target).or_insert_with(|| {
                    queue.push_back(target);
                    n
                });
                row.push(id);
            }
            result.next.push(row);
            result.accept.push(self.accept[rep]);
            result
                .parts
                .push(common_parts(group.iter().map(|&s| &self.parts[s])));
        }
        result
    }

    /// Whether `state` is a non-accepting (or accepting) state that never leaves itself.
    pub fn is_sink(&self, state: usize) -> bool {
        self.next[state].iter().all(|&t| t == state)
    }

    /// Shortest word reaching each state (smallest in alphabet order); all states reachable.
    pub fn access_words(&self) -> Vec<String> {
        let mut words: Vec<Option<String>> = vec![None; self.len()];
        words[self.start] = Some(String::new());
        let mut queue = VecDeque::from([self.start]);
        while let Some(s) = queue.pop_front() {
            let base = words[s].clone().unwrap_or_default();
            for (c, &t) in self.next[s].iter().enumerate() {
                if words[t].is_none() {
                    words[t] = Some(format!("{base}{}", self.alphabet[c]));
                    queue.push_back(t);
                }
            }
        }
        words.into_iter().map(Option::unwrap_or_default).collect()
    }
}

/// Phrases present, at the same position, in every member.
fn common_parts<'a>(mut members: impl Iterator<Item = &'a Vec<String>>) -> Vec<String> {
    let Some(first) = members.next() else {
        return Vec::new();
    };
    let mut keep: Vec<Option<&String>> = first.iter().map(Some).collect();
    for m in members {
        for (slot, part) in keep.iter_mut().zip(m) {
            if *slot != Some(part) {
                *slot = None;
            }
        }
    }
    keep.into_iter().flatten().cloned().collect()
}

fn lengths_text(symbols: &[char]) -> String {
    symbols
        .iter()
        .map(char::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn counter_name(subject: &Subject) -> String {
    match subject {
        Subject::Length => "length".to_string(),
        Subject::Count(set) => format!("count({})", lengths_text(set)),
    }
}

/// What is being counted.
pub(crate) enum Subject {
    Length,
    Count(Vec<char>),
}

impl Subject {
    fn counts(&self, c: char) -> bool {
        match self {
            Subject::Length => true,
            Subject::Count(set) => set.contains(&c),
        }
    }
}

fn counting_dfa(
    alphabet: &[char],
    subject: &Subject,
    states: usize,
    bump: impl Fn(usize) -> usize,
    accept: impl Fn(usize) -> bool,
    part: impl Fn(usize) -> String,
) -> Dfa {
    Dfa {
        alphabet: alphabet.to_vec(),
        next: (0..states)
            .map(|s| {
                alphabet
                    .iter()
                    .map(|&c| if subject.counts(c) { bump(s) } else { s })
                    .collect()
            })
            .collect(),
        accept: (0..states).map(accept).collect(),
        start: 0,
        parts: (0..states).map(|s| vec![part(s)]).collect(),
    }
}

/// `subject <cmp> n`: a counter that saturates at `n + 1`.
pub(crate) fn compare(alphabet: &[char], subject: &Subject, cmp: Cmp, n: usize) -> Dfa {
    let name = counter_name(subject);
    counting_dfa(
        alphabet,
        subject,
        n + 2,
        |s| (s + 1).min(n + 1),
        |s| cmp.holds(s, n),
        |s| {
            if s > n {
                format!("{name} > {n}")
            } else {
                format!("{name} = {s}")
            }
        },
    )
}

/// `subject % m == r`.
pub(crate) fn modulo(alphabet: &[char], subject: &Subject, m: usize, r: usize) -> Dfa {
    let name = counter_name(subject);
    counting_dfa(
        alphabet,
        subject,
        m,
        |s| (s + 1) % m,
        |s| s == r,
        |s| match (m, s) {
            (2, 0) => format!("{name} is even"),
            (2, _) => format!("{name} is odd"),
            _ => format!("{name} mod {m} is {s}"),
        },
    )
}

/// Longest `j` such that the first `j` symbols of `word` are a suffix of `word[..k] + c`.
fn kmp_step(word: &[char], k: usize, c: char) -> usize {
    let mut text: Vec<char> = word[..k].to_vec();
    text.push(c);
    (0..=text.len().min(word.len()))
        .rev()
        .find(|&j| text[text.len() - j..] == word[..j])
        .unwrap_or(0)
}

fn show(chars: &[char]) -> String {
    format!("\"{}\"", chars.iter().collect::<String>())
}

/// `starts_with(word)`.
pub(crate) fn starts_with(alphabet: &[char], word: &[char]) -> Dfa {
    let n = word.len();
    if n == 0 {
        return Dfa::constant(alphabet, true, "starts_with(\"\") always holds");
    }
    // States 0..n: matched that many symbols; n: done; n + 1: mismatch.
    let (done, dead) = (n, n + 1);
    let mut next = Vec::new();
    let mut parts = Vec::new();
    for k in 0..n {
        next.push(
            alphabet
                .iter()
                .map(|&c| if c == word[k] { k + 1 } else { dead })
                .collect(),
        );
        parts.push(vec![if k == 0 {
            format!("starts_with({}) not started", show(word))
        } else {
            format!("read {}, the start of {}", show(&word[..k]), show(word))
        }]);
    }
    next.push(vec![done; alphabet.len()]);
    parts.push(vec![format!("starts with {}", show(word))]);
    next.push(vec![dead; alphabet.len()]);
    parts.push(vec![format!("does not start with {}", show(word))]);
    let mut accept = vec![false; n + 2];
    accept[done] = true;
    Dfa {
        alphabet: alphabet.to_vec(),
        next,
        accept,
        start: 0,
        parts,
    }
}

/// `ends_with(word)` or, with `absorbing`, `contains(word)`.
pub(crate) fn suffix_match(alphabet: &[char], word: &[char], absorbing: bool) -> Dfa {
    let n = word.len();
    let name = if absorbing { "contains" } else { "ends_with" };
    if n == 0 {
        return Dfa::constant(alphabet, true, &format!("{name}(\"\") always holds"));
    }
    let mut next = Vec::new();
    let mut parts = Vec::new();
    for k in 0..=n {
        if absorbing && k == n {
            next.push(vec![n; alphabet.len()]);
        } else {
            next.push(alphabet.iter().map(|&c| kmp_step(word, k, c)).collect());
        }
        parts.push(vec![match (absorbing, k) {
            (true, k) if k == n => format!("contains {}", show(word)),
            (true, 0) => format!("no {} yet, no partial match", show(word)),
            (true, k) => format!(
                "no {} yet, last symbols are {}",
                show(word),
                show(&word[..k])
            ),
            (false, k) if k == n => format!("ends with {}", show(word)),
            (false, 0) if n == 1 => format!("does not end with {}", show(word)),
            (false, 0) => format!("ends with no start of {}", show(word)),
            (false, k) => format!(
                "ends with {}, the start of {}",
                show(&word[..k]),
                show(word)
            ),
        }]);
    }
    let mut accept = vec![false; n + 1];
    accept[n] = true;
    Dfa {
        alphabet: alphabet.to_vec(),
        next,
        accept,
        start: 0,
        parts,
    }
}

/// Automaton for a finite-state description with λ moves.
#[derive(Clone, Debug)]
pub(crate) struct Nfa {
    pub count: usize,
    pub starts: BTreeSet<usize>,
    pub accepts: BTreeSet<usize>,
    pub edges: Vec<(usize, Option<char>, usize)>,
}

impl Nfa {
    /// The NFA of the reversed language.
    pub fn reversed(&self) -> Nfa {
        Nfa {
            count: self.count,
            starts: self.accepts.clone(),
            accepts: self.starts.clone(),
            edges: self.edges.iter().map(|&(a, c, b)| (b, c, a)).collect(),
        }
    }

    fn closure(&self, set: &BTreeSet<usize>) -> BTreeSet<usize> {
        let mut out = set.clone();
        let mut stack: Vec<usize> = set.iter().copied().collect();
        while let Some(s) = stack.pop() {
            for &(a, c, b) in &self.edges {
                if a == s && c.is_none() && out.insert(b) {
                    stack.push(b);
                }
            }
        }
        out
    }

    /// Subset construction over `alphabet` (the empty set becomes an ordinary dead state).
    pub fn to_dfa(&self, alphabet: &[char], max_states: usize) -> Result<Dfa, DesignError> {
        let start = self.closure(&self.starts);
        let mut index: BTreeMap<BTreeSet<usize>, usize> = BTreeMap::from([(start.clone(), 0)]);
        let mut sets = vec![start];
        let mut next: Vec<Vec<usize>> = Vec::new();
        let mut i = 0;
        while i < sets.len() {
            let mut row = Vec::new();
            for &c in alphabet {
                let moved: BTreeSet<usize> = self
                    .edges
                    .iter()
                    .filter(|&&(a, l, _)| l == Some(c) && sets[i].contains(&a))
                    .map(|&(_, _, b)| b)
                    .collect();
                let target = self.closure(&moved);
                let id = match index.get(&target) {
                    Some(id) => *id,
                    None => {
                        if sets.len() >= max_states {
                            return Err(too_large(max_states));
                        }
                        sets.push(target.clone());
                        index.insert(target, sets.len() - 1);
                        sets.len() - 1
                    }
                };
                row.push(id);
            }
            next.push(row);
            i += 1;
        }
        Ok(Dfa {
            alphabet: alphabet.to_vec(),
            accept: sets
                .iter()
                .map(|s| s.iter().any(|q| self.accepts.contains(q)))
                .collect(),
            parts: vec![Vec::new(); sets.len()],
            start: 0,
            next,
        })
    }
}
