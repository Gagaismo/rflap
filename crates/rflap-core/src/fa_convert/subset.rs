//! NFA to DFA by subset construction (JFLAP `NFAToDFA`).

use std::collections::{BTreeSet, HashMap, VecDeque};

use super::common::{grid_point, join_ids, normalize};
use super::{Conversion, DfaState, NfaToDfaStep};
use crate::analysis::nondeterministic_states;
use crate::automaton::{StateId, Transition};
use crate::fa::{Fa, FaLabel, alphabet, lambda_closure};

pub(super) fn nfa_to_dfa(nfa: &Fa) -> Conversion {
    if nfa.initial().is_none() {
        return Conversion {
            dfa: Fa::new(),
            dfa_states: Vec::new(),
            steps: Vec::new(),
        };
    }
    if nondeterministic_states(nfa).is_empty() {
        return Conversion {
            dfa: nfa.clone(),
            dfa_states: nfa
                .states()
                .map(|s| DfaState {
                    state: s.id,
                    nfa_states: vec![s.id],
                })
                .collect(),
            steps: Vec::new(),
        };
    }
    determinize(&normalize(nfa))
}

/// Subset construction over an automaton whose labels read one character or nothing.
pub(super) fn determinize(nfa: &Fa) -> Conversion {
    let mut conv = Conversion {
        dfa: Fa::new(),
        dfa_states: Vec::new(),
        steps: Vec::new(),
    };
    let Some(initial) = nfa.initial() else {
        return conv;
    };
    let symbols = alphabet(nfa);
    let closure_of = |ids: &[StateId]| -> Vec<StateId> {
        let all: BTreeSet<StateId> = ids.iter().flat_map(|&id| lambda_closure(nfa, id)).collect();
        all.into_iter().collect()
    };
    let mut known: HashMap<Vec<StateId>, StateId> = HashMap::new();
    let mut queue: VecDeque<StateId> = VecDeque::new();

    let start = create(
        nfa,
        &mut conv,
        &mut known,
        &mut queue,
        closure_of(&[initial]),
    );
    conv.dfa.set_initial(Some(start));
    while let Some(state) = queue.pop_front() {
        let members = conv
            .dfa_states
            .iter()
            .find(|d| d.state == state)
            .map(|d| d.nfa_states.clone())
            .unwrap_or_default();
        for symbol in &symbols {
            let reached: Vec<StateId> = members
                .iter()
                .flat_map(|&m| nfa.transitions_from(m))
                .filter(|t| &t.label.0 == symbol)
                .map(|t| t.to)
                .collect();
            if reached.is_empty() {
                continue;
            }
            let target = closure_of(&reached);
            let to = match known.get(&target) {
                Some(&id) => id,
                None => create(nfa, &mut conv, &mut known, &mut queue, target),
            };
            conv.dfa
                .add_transition(Transition::new(state, to, FaLabel(symbol.clone())));
            conv.steps.push(NfaToDfaStep::AddTransition {
                from: state,
                to,
                symbol: symbol.clone(),
            });
        }
    }
    conv
}

/// Adds the DFA state for an NFA state set and queues it for expansion.
fn create(
    nfa: &Fa,
    conv: &mut Conversion,
    known: &mut HashMap<Vec<StateId>, StateId>,
    queue: &mut VecDeque<StateId>,
    set: Vec<StateId>,
) -> StateId {
    let id = conv.dfa.create_state(grid_point(conv.dfa_states.len()));
    if let Some(s) = conv.dfa.state_mut(id) {
        s.label = Some(join_ids(&set));
    }
    if set.iter().any(|&s| nfa.is_final(s)) {
        conv.dfa.set_final(id, true);
    }
    conv.steps.push(NfaToDfaStep::NewState {
        state: id,
        nfa_states: set.clone(),
    });
    conv.dfa_states.push(DfaState {
        state: id,
        nfa_states: set.clone(),
    });
    known.insert(set, id);
    queue.push_back(id);
    id
}
