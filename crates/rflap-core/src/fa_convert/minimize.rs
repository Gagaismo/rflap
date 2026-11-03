//! DFA minimization by distinguishability tree (JFLAP `Minimizer`, `MinimizeTreeNode`).

use super::common::{grid_point, join_ids, normalize};
use super::subset::determinize;
use super::{Minimization, MinimizeNode, MinimizeStep};
use crate::analysis::{nondeterministic_states, unreachable_states};
use crate::automaton::{StateId, Transition};
use crate::fa::{Fa, add_trap_state, alphabet};

struct Node {
    states: Vec<StateId>,
    symbol: Option<String>,
    children: Vec<usize>,
}

pub(super) fn minimize(fa: &Fa) -> Minimization {
    let mut steps = Vec::new();
    let normalized = normalize(fa);
    let mut work = if nondeterministic_states(&normalized).is_empty() {
        normalized
    } else {
        determinize(&normalized).dfa
    };
    let Some(initial) = work.initial() else {
        return Minimization {
            dfa: Fa::new(),
            tree: MinimizeNode {
                states: Vec::new(),
                symbol: None,
                children: Vec::new(),
            },
            steps,
        };
    };

    let unreachable: Vec<StateId> = unreachable_states(&work).into_iter().collect();
    for &id in &unreachable {
        work.remove_state(id);
    }
    if !unreachable.is_empty() {
        steps.push(MinimizeStep::RemoveUnreachable {
            states: unreachable,
        });
    }

    let symbols = alphabet(&work);
    let incomplete = work.states().any(|s| {
        symbols
            .iter()
            .any(|sym| !work.transitions_from(s.id).any(|t| &t.label.0 == sym))
    });
    let trap = incomplete.then(|| add_trap_state(&mut work));
    if let Some(state) = trap {
        steps.push(MinimizeStep::AddTrapState { state });
    }

    let all: Vec<StateId> = work.states().map(|s| s.id).collect();
    let (finals, others): (Vec<StateId>, Vec<StateId>) =
        all.iter().partition(|&&id| work.is_final(id));
    let mut nodes = vec![Node {
        states: all,
        symbol: None,
        children: Vec::new(),
    }];
    for group in [others, finals] {
        if !group.is_empty() {
            nodes.push(Node {
                states: group,
                symbol: None,
                children: Vec::new(),
            });
            let child = nodes.len() - 1;
            nodes[0].children.push(child);
        }
    }
    steps.push(MinimizeStep::InitialPartition {
        groups: nodes[0]
            .children
            .iter()
            .map(|&c| nodes[c].states.clone())
            .collect(),
    });

    while let Some((node, symbol, parts)) = next_split(&work, &symbols, &nodes) {
        steps.push(MinimizeStep::Split {
            group: nodes[node].states.clone(),
            symbol: symbol.clone(),
            into: parts.clone(),
        });
        nodes[node].symbol = Some(symbol);
        for part in parts {
            nodes.push(Node {
                states: part,
                symbol: None,
                children: Vec::new(),
            });
            let child = nodes.len() - 1;
            nodes[node].children.push(child);
        }
    }

    // Build the minimum DFA, one state per leaf; the trap group is dropped unless it
    // holds the initial state (empty language).
    let leaves = leaves_of(&nodes, 0);
    let mut dfa = Fa::new();
    let mut group_state: Vec<Option<StateId>> = vec![None; leaves.len()];
    for (k, &leaf) in leaves.iter().enumerate() {
        let group = &nodes[leaf].states;
        let has_initial = group.contains(&initial);
        if trap.is_some_and(|t| group.contains(&t)) && !has_initial {
            continue;
        }
        let id = dfa.create_state(grid_point(dfa.states().count()));
        if let Some(s) = dfa.state_mut(id) {
            s.label = Some(join_ids(group));
        }
        if has_initial {
            dfa.set_initial(Some(id));
        }
        if group.iter().any(|&s| work.is_final(s)) {
            dfa.set_final(id, true);
        }
        steps.push(MinimizeStep::MergeGroup {
            group: group.clone(),
            state: id,
        });
        group_state[k] = Some(id);
    }
    let leaf_index = |state: StateId| {
        leaves
            .iter()
            .position(|&l| nodes[l].states.contains(&state))
    };
    for (k, &leaf) in leaves.iter().enumerate() {
        let Some(from) = group_state[k] else { continue };
        let representative = nodes[leaf].states[0];
        for t in work.transitions_from(representative) {
            let target = leaf_index(t.to).and_then(|g| group_state[g]);
            if let Some(to) = target {
                dfa.add_transition(Transition::new(from, to, t.label.clone()));
            }
        }
    }

    Minimization {
        dfa,
        tree: export(&nodes, 0),
        steps,
    }
}

/// Leaves in depth-first order.
fn leaves_of(nodes: &[Node], at: usize) -> Vec<usize> {
    if nodes[at].children.is_empty() {
        return vec![at];
    }
    nodes[at]
        .children
        .iter()
        .flat_map(|&c| leaves_of(nodes, c))
        .collect()
}

fn export(nodes: &[Node], at: usize) -> MinimizeNode {
    MinimizeNode {
        states: nodes[at].states.clone(),
        symbol: nodes[at].symbol.clone(),
        children: nodes[at]
            .children
            .iter()
            .map(|&c| export(nodes, c))
            .collect(),
    }
}

/// The first leaf (depth-first) with some symbol (first-use order) that sends its states
/// to different leaves, with the split it causes: states grouped by destination leaf.
fn next_split(
    dfa: &Fa,
    symbols: &[String],
    nodes: &[Node],
) -> Option<(usize, String, Vec<Vec<StateId>>)> {
    let leaves = leaves_of(nodes, 0);
    let leaf_of = |state: StateId| {
        leaves
            .iter()
            .position(|&l| nodes[l].states.contains(&state))
    };
    for &leaf in &leaves {
        let group = &nodes[leaf].states;
        if group.len() < 2 {
            continue;
        }
        for symbol in symbols {
            let targets: Vec<Option<usize>> = group
                .iter()
                .map(|&s| {
                    dfa.transitions_from(s)
                        .find(|t| &t.label.0 == symbol)
                        .and_then(|t| leaf_of(t.to))
                })
                .collect();
            let mut parts: Vec<Vec<StateId>> = Vec::new();
            for l in 0..leaves.len() {
                let part: Vec<StateId> = group
                    .iter()
                    .zip(&targets)
                    .filter(|(_, t)| **t == Some(l))
                    .map(|(&s, _)| s)
                    .collect();
                if !part.is_empty() {
                    parts.push(part);
                }
            }
            if parts.len() > 1 {
                return Some((leaf, symbol.clone(), parts));
            }
        }
    }
    None
}
