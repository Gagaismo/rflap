//! Rflap addition: rewrites any PDA that accepts by final state into the form JFLAP's PDA to
//! CFG conversion requires (see [`check_pda_form`](super::check_pda_form)): a single final
//! state reached with an empty stack, every transition popping exactly one symbol and
//! pushing zero or two.
//!
//! The construction is the textbook one, kept visible as steps:
//!
//! 1. the stack symbol `Z` of the original machine is renamed to a fresh symbol, and a new
//!    initial state puts a new bottom marker `Z` below it, so the stack never runs empty
//!    while the original machine runs;
//! 2. every transition is rewritten to pop one symbol and push zero or two, through new
//!    intermediate states;
//! 3. whenever the original machine reaches a final state it may empty the stack and enter
//!    the only final state by popping the marker `Z`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{ConvertError, TransitionRef, check_pda_form};
use crate::automaton::{Point, StateId, Transition};
use crate::pda::{INITIAL_STACK, Pda, PdaLabel};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PdaToJflapFormStep {
    /// The machine already satisfies the preconditions; nothing was changed.
    AlreadyInForm,
    /// The original bottom symbol `Z` is renamed and a new initial state puts a new `Z`
    /// below it.
    BottomMarker {
        /// The symbol the original `Z` is called now.
        renamed_bottom: char,
        initial: StateId,
        transition: TransitionRef,
    },
    /// A transition that did not pop exactly one symbol and push zero or two was replaced.
    Rewrite {
        from: StateId,
        to: StateId,
        label: PdaLabel,
        /// Why: `pops_nothing`, `pops_several`, `pushes_one` or `pushes_several`.
        reason: String,
        transitions: Vec<TransitionRef>,
        new_states: Vec<StateId>,
    },
    /// The only final state, and the state that empties the stack on the way to it.
    EmptyStackAtFinal {
        final_state: StateId,
        drain: Option<StateId>,
        transitions: Vec<TransitionRef>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PdaToJflapForm {
    pub pda: Pda,
    pub steps: Vec<PdaToJflapFormStep>,
}

/// Builds the new machine and remembers what each step added.
struct Builder {
    pda: Pda,
    /// Every symbol a transition may find on top of the stack, the marker `Z` included.
    stack_symbols: Vec<char>,
    names: BTreeSet<String>,
    emitted: Vec<TransitionRef>,
    created: Vec<StateId>,
}

impl Builder {
    fn state(&mut self, base: &str, at: Point) -> StateId {
        let mut name = base.to_string();
        let mut n = 1;
        while self.names.contains(&name) {
            n += 1;
            name = format!("{base}{n}");
        }
        self.names.insert(name.clone());
        let id = self.pda.create_state(at);
        if let Some(s) = self.pda.state_mut(id) {
            s.name = name;
        }
        self.created.push(id);
        id
    }

    fn edge(&mut self, from: StateId, to: StateId, input: &str, pop: char, push: &[char]) {
        let label = PdaLabel::new(input, &pop.to_string(), &push.iter().collect::<String>());
        if self
            .pda
            .add_transition(Transition::new(from, to, label.clone()))
        {
            self.emitted.push(TransitionRef { from, to, label });
        }
    }

    /// A spot between two states, `k` rows below their line.
    fn between(&self, a: StateId, b: StateId, k: usize) -> Point {
        let at = |id| self.pda.state(id).map_or(Point::default(), |s| s.point);
        let (a, b) = (at(a), at(b));
        Point {
            x: (a.x + b.x) / 2.0,
            y: (a.y + b.y) / 2.0 + 60.0 + 30.0 * k as f64,
        }
    }

    /// `from --input, pop; push--> to` for a transition that already pops one symbol.
    fn push_normalized(
        &mut self,
        from: StateId,
        to: StateId,
        input: &str,
        pop: char,
        push: &[char],
    ) {
        match push.len() {
            0 | 2 => self.edge(from, to, input, pop, push),
            1 => {
                // Pop, then put the new symbol on top of whatever is below.
                let at = self.between(from, to, 0);
                let middle = self.state("t", at);
                self.edge(from, middle, input, pop, &[]);
                for s in self.stack_symbols.clone() {
                    self.edge(middle, to, "", s, &[push[0], s]);
                }
            }
            m => {
                // Push the two deepest symbols, then one more on top of the last, and so on.
                let at = self.between(from, to, 0);
                let mut next = self.state("t", at);
                self.edge(from, next, input, pop, &push[m - 2..]);
                for j in (0..m - 2).rev() {
                    let source = next;
                    next = if j == 0 {
                        to
                    } else {
                        let at = self.between(from, to, m - 2 - j);
                        self.state("t", at)
                    };
                    self.edge(source, next, "", push[j + 1], &push[j..j + 2]);
                }
            }
        }
    }
}

/// A symbol that no transition uses, to stand for the original bottom `Z`.
fn fresh_symbol(used: &BTreeSet<char>) -> Option<char> {
    ('A'..='Y')
        .rev()
        .chain('a'..='z')
        .chain('0'..='9')
        .chain('α'..='ω')
        .find(|c| !used.contains(c))
}

/// Rewrites `pda` (accepting by final state) into the form `pda_to_cfg` accepts, keeping its
/// language: the result accepts a string exactly when the original reaches a final state
/// after reading it, and does so in its only final state with an empty stack. A machine that
/// is already in form is returned unchanged.
pub fn pda_to_jflap_form(pda: &Pda) -> Result<PdaToJflapForm, ConvertError> {
    let Some(initial) = pda.initial() else {
        return Err(ConvertError::NoInitialState);
    };
    if check_pda_form(pda).is_ok() {
        return Ok(PdaToJflapForm {
            pda: pda.clone(),
            steps: vec![PdaToJflapFormStep::AlreadyInForm],
        });
    }
    let marker = INITIAL_STACK.chars().next().expect("Z");
    let used: BTreeSet<char> = pda
        .transitions()
        .iter()
        .flat_map(|t| t.label.pop.chars().chain(t.label.push.chars()))
        .collect();
    let bottom = fresh_symbol(&used).ok_or(ConvertError::NoFreeStackSymbol)?;
    let rename = |s: &str| -> Vec<char> {
        s.chars()
            .map(|c| if c == marker { bottom } else { c })
            .collect()
    };

    // The symbols the original machine can find on its stack, then the marker.
    let mut original_symbols: BTreeSet<char> = used
        .iter()
        .map(|&c| if c == marker { bottom } else { c })
        .collect();
    original_symbols.insert(bottom);
    let mut stack_symbols: Vec<char> = original_symbols.iter().copied().collect();
    stack_symbols.push(marker);

    let (mut min_x, mut max_x, mut max_y) = (f64::MAX, f64::MIN, f64::MIN);
    let mut out = Pda::new();
    let mut names = BTreeSet::new();
    for s in pda.states() {
        out.create_state_with_id(s.id, s.point);
        if let Some(copy) = out.state_mut(s.id) {
            copy.name = s.name.clone();
            copy.label = s.label.clone();
            copy.output = s.output.clone();
        }
        names.insert(s.name.clone());
        min_x = min_x.min(s.point.x);
        max_x = max_x.max(s.point.x);
        max_y = max_y.max(s.point.y);
    }
    if min_x > max_x {
        (min_x, max_x, max_y) = (0.0, 0.0, 0.0);
    }
    let initial_y = pda.state(initial).map_or(0.0, |s| s.point.y);
    let mut b = Builder {
        pda: out,
        stack_symbols,
        names,
        emitted: Vec::new(),
        created: Vec::new(),
    };
    let mut steps = Vec::new();

    // 1. A new bottom marker below the original stack.
    let start = b.state(
        "start",
        Point {
            x: min_x - 120.0,
            y: initial_y,
        },
    );
    b.pda.set_initial(Some(start));
    b.edge(start, initial, "", marker, &[bottom, marker]);
    steps.push(PdaToJflapFormStep::BottomMarker {
        renamed_bottom: bottom,
        initial: start,
        transition: b.emitted.pop().expect("the start transition"),
    });

    // 2. Every transition pops one symbol and pushes zero or two.
    for t in pda.transitions() {
        let pop = rename(&t.label.pop);
        let push = rename(&t.label.push);
        b.emitted.clear();
        b.created.clear();
        let reason = if pop.is_empty() {
            for s in b.stack_symbols.clone() {
                let mut pushed = push.clone();
                pushed.push(s);
                b.push_normalized(t.from, t.to, &t.label.input, s, &pushed);
            }
            "pops_nothing"
        } else if pop.len() == 1 {
            if push.is_empty() || push.len() == 2 {
                b.edge(t.from, t.to, &t.label.input, pop[0], &push);
                continue;
            }
            b.push_normalized(t.from, t.to, &t.label.input, pop[0], &push);
            if push.len() == 1 {
                "pushes_one"
            } else {
                "pushes_several"
            }
        } else {
            let mut at = t.from;
            for (i, &symbol) in pop.iter().enumerate().take(pop.len() - 1) {
                let spot = b.between(t.from, t.to, i);
                let next = b.state("t", spot);
                let input = if i == 0 { t.label.input.as_str() } else { "" };
                b.edge(at, next, input, symbol, &[]);
                at = next;
            }
            let last = pop[pop.len() - 1];
            b.push_normalized(at, t.to, "", last, &push);
            "pops_several"
        };
        steps.push(PdaToJflapFormStep::Rewrite {
            from: t.from,
            to: t.to,
            label: t.label.clone(),
            reason: reason.to_string(),
            transitions: std::mem::take(&mut b.emitted),
            new_states: std::mem::take(&mut b.created),
        });
    }

    // 3. Reaching a final state, the machine may empty its stack into the only final state.
    b.emitted.clear();
    b.created.clear();
    let accept = b.state(
        "accept",
        Point {
            x: max_x + 240.0,
            y: initial_y,
        },
    );
    b.pda.set_final(accept, true);
    let finals: Vec<StateId> = pda.finals().iter().copied().collect();
    let drain = if finals.is_empty() {
        None
    } else {
        Some(b.state(
            "drain",
            Point {
                x: max_x + 120.0,
                y: max_y + 120.0,
            },
        ))
    };
    if let Some(drain) = drain {
        let symbols = b.stack_symbols.clone();
        for &f in &finals {
            for &s in &symbols {
                let to = if s == marker { accept } else { drain };
                b.edge(f, to, "", s, &[]);
            }
        }
        for &s in &symbols {
            let to = if s == marker { accept } else { drain };
            b.edge(drain, to, "", s, &[]);
        }
    }
    steps.push(PdaToJflapFormStep::EmptyStackAtFinal {
        final_state: accept,
        drain,
        transitions: std::mem::take(&mut b.emitted),
    });

    Ok(PdaToJflapForm { pda: b.pda, steps })
}
