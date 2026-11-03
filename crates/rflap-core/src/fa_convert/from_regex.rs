//! Regular expression to NFA by de-expressionifying a transition graph
//! (JFLAP `REToFSAController`).

use std::collections::VecDeque;

use super::to_regex::{EMPTY, cat_parts, delambda, or_parts};
use super::{GtgEdge, NfaConstruction, ReAction, ReStep};
use crate::automaton::{Point, StateId, Transition};
use crate::fa::{Fa, FaLabel};
use crate::regex::Regex;

pub(super) fn regex_to_nfa(regex: &Regex) -> NfaConstruction {
    let mut nfa = Fa::new();
    let start = nfa.create_state(Point { x: 60.0, y: 40.0 });
    let end = nfa.create_state(Point { x: 450.0, y: 250.0 });
    nfa.set_initial(Some(start));
    nfa.set_final(end, true);
    let mut steps = Vec::new();

    let label = delambda(&regex.to_string());
    nfa.add_transition(Transition::new(start, end, FaLabel(label.clone())));
    let mut todo: VecDeque<GtgEdge> = VecDeque::new();
    if required_action(&label).is_some() {
        todo.push_back(edge(start, end, &label));
    }

    while let Some(current) = todo.pop_front() {
        let Some(action) = required_action(&current.label) else {
            continue;
        };
        let (from, to) = (current.from, current.to);
        nfa.remove_transition(&Transition::new(from, to, FaLabel(current.label.clone())));
        let mut step = ReStep {
            action,
            transition: shown(&current),
            new_states: Vec::new(),
            added: Vec::new(),
        };
        let label = current.label.as_str();
        if action == ReAction::Deparens {
            let inner = delambda(&label[1..label.len() - 1]);
            add(&mut nfa, &mut step, &mut todo, from, to, &inner, true);
        } else {
            let pieces: Vec<String> = match action {
                ReAction::Destar => vec![delambda(&label[..label.len() - 1])],
                ReAction::Deor => or_parts(label),
                _ => cat_parts(label),
            };
            let n = pieces.len();
            let (p, q) = (point_of(&nfa, from), point_of(&nfa, to));
            let (dx, dy) = (q.x - p.x, q.y - p.y);
            let (dist, angle) = (dx.hypot(dy), dy.atan2(dx));
            let place = |along: f64, across: f64| {
                let (sin, cos) = angle.sin_cos();
                let at = |v: f64| (v.max(20.0) + 0.5).floor();
                Point {
                    x: at(p.x + dist * (along * cos - across * sin)),
                    y: at(p.y + dist * (along * sin + across * cos)),
                }
            };
            let mut pairs: Vec<(StateId, StateId)> = Vec::new();
            for (i, piece) in pieces.iter().enumerate() {
                let across = if n > 1 {
                    (i as f64 / (n as f64 - 1.0) - 0.5) * 0.5
                } else {
                    0.0
                };
                let s = nfa.create_state(place(0.2, across));
                let t = nfa.create_state(place(0.8, across));
                step.new_states.extend([s, t]);
                pairs.push((s, t));
                add(&mut nfa, &mut step, &mut todo, s, t, piece, true);
            }
            let lambda = |nfa: &mut Fa, step: &mut ReStep, a: StateId, b: StateId| {
                add(nfa, step, &mut VecDeque::new(), a, b, "", false);
            };
            match action {
                ReAction::Deor => {
                    for &(s, t) in &pairs {
                        lambda(&mut nfa, &mut step, from, s);
                        lambda(&mut nfa, &mut step, t, to);
                    }
                }
                ReAction::Decat => {
                    lambda(&mut nfa, &mut step, from, pairs[0].0);
                    for w in pairs.windows(2) {
                        lambda(&mut nfa, &mut step, w[0].1, w[1].0);
                    }
                    lambda(&mut nfa, &mut step, pairs[n - 1].1, to);
                }
                _ => {
                    lambda(&mut nfa, &mut step, from, pairs[0].0);
                    lambda(&mut nfa, &mut step, pairs[0].1, to);
                    lambda(&mut nfa, &mut step, from, to);
                    lambda(&mut nfa, &mut step, to, from);
                }
            }
        }
        steps.push(step);
    }

    // The empty language has no transition.
    for t in nfa.transitions().to_vec() {
        if t.label.0 == EMPTY {
            nfa.remove_transition(&t);
            steps.push(ReStep {
                action: ReAction::DropEmpty,
                transition: edge(t.from, t.to, &t.label.0),
                new_states: Vec::new(),
                added: Vec::new(),
            });
        }
    }
    NfaConstruction { nfa, steps }
}

fn edge(from: StateId, to: StateId, label: &str) -> GtgEdge {
    GtgEdge {
        from,
        to,
        label: label.to_string(),
    }
}

/// Edge as shown to the user: λ for the empty label.
fn shown(e: &GtgEdge) -> GtgEdge {
    edge(
        e.from,
        e.to,
        if e.label.is_empty() { "λ" } else { &e.label },
    )
}

fn point_of(nfa: &Fa, id: StateId) -> Point {
    nfa.state(id).map(|s| s.point).unwrap_or_default()
}

/// Adds the transition, records it in the step and queues it when it needs more work.
fn add(
    nfa: &mut Fa,
    step: &mut ReStep,
    todo: &mut VecDeque<GtgEdge>,
    from: StateId,
    to: StateId,
    label: &str,
    queue: bool,
) {
    nfa.add_transition(Transition::new(from, to, FaLabel(label.to_string())));
    step.added.push(shown(&edge(from, to, label)));
    if queue && required_action(label).is_some() {
        todo.push_back(edge(from, to, label));
    }
}

/// What must still be done to a label before it is a symbol or λ.
fn required_action(label: &str) -> Option<ReAction> {
    if label.chars().count() <= 1 {
        return None;
    }
    if or_parts(label).len() > 1 {
        Some(ReAction::Deor)
    } else if cat_parts(label).len() > 1 {
        Some(ReAction::Decat)
    } else if label.ends_with('*') {
        Some(ReAction::Destar)
    } else if label.starts_with('(') && label.ends_with(')') {
        Some(ReAction::Deparens)
    } else {
        None
    }
}
