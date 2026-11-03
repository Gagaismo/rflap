//! Finite automaton conversions: NFA to DFA, minimization, FA to/from regular expression, equivalence.

use serde::{Deserialize, Serialize};

mod common;
mod equivalence;
mod from_regex;
mod minimize;
mod subset;
mod to_regex;

use crate::automaton::StateId;
use crate::fa::Fa;
use crate::regex::Regex;

/// Result of the subset construction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conversion {
    pub dfa: Fa,
    /// NFA states (sorted) behind each DFA state, in creation order.
    pub dfa_states: Vec<DfaState>,
    pub steps: Vec<NfaToDfaStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DfaState {
    pub state: StateId,
    pub nfa_states: Vec<StateId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NfaToDfaStep {
    NewState {
        state: StateId,
        nfa_states: Vec<StateId>,
    },
    AddTransition {
        from: StateId,
        to: StateId,
        symbol: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinimizeNode {
    pub states: Vec<StateId>,
    pub symbol: Option<String>,
    pub children: Vec<MinimizeNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MinimizeStep {
    RemoveUnreachable {
        states: Vec<StateId>,
    },
    AddTrapState {
        state: StateId,
    },
    InitialPartition {
        groups: Vec<Vec<StateId>>,
    },
    Split {
        group: Vec<StateId>,
        symbol: String,
        into: Vec<Vec<StateId>>,
    },
    MergeGroup {
        group: Vec<StateId>,
        state: StateId,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Minimization {
    pub dfa: Fa,
    pub tree: MinimizeNode,
    pub steps: Vec<MinimizeStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GtgEdge {
    pub from: StateId,
    pub to: StateId,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FaToRegexStep {
    AddFinalState {
        state: StateId,
        old_finals: Vec<StateId>,
    },
    CollapseParallel {
        from: StateId,
        to: StateId,
        labels: Vec<String>,
        label: String,
    },
    AddEmptyTransitions {
        pairs: Vec<(StateId, StateId)>,
    },
    EliminateState {
        state: StateId,
        edges: Vec<GtgEdge>,
    },
    Result {
        expression: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegexConversion {
    pub regex: Regex,
    pub steps: Vec<FaToRegexStep>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReAction {
    Deparens,
    Destar,
    Deor,
    Decat,
    DropEmpty,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReStep {
    pub action: ReAction,
    pub transition: GtgEdge,
    pub new_states: Vec<StateId>,
    pub added: Vec<GtgEdge>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NfaConstruction {
    pub nfa: Fa,
    pub steps: Vec<ReStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equivalence {
    pub equivalent: bool,
    pub counterexample: Option<String>,
}

/// Subset construction (JFLAP `NFAToDFA`). A DFA state is labelled with the NFA states it
/// stands for ("0,1,3"); transitions that would go to the empty set are not created (no trap
/// state). An automaton that is already deterministic is returned as is, without steps.
/// Without an initial state the DFA is empty.
pub fn nfa_to_dfa(nfa: &Fa) -> Conversion {
    subset::nfa_to_dfa(nfa)
}

/// Minimization by distinguishability tree (JFLAP `Minimizer`): converts an NFA first,
/// removes unreachable states, completes with a trap state if needed, splits groups until
/// none is distinguishable and drops the group holding the trap state.
pub fn minimize(fa: &Fa) -> Minimization {
    minimize::minimize(fa)
}

/// State elimination over a generalized transition graph (JFLAP `FSAToRegularExpressionConverter`).
/// Labels must not contain `(`, `)`, `+` or `*`; otherwise the result is not meaningful.
pub fn fa_to_regex_steps(fa: &Fa) -> RegexConversion {
    to_regex::fa_to_regex(fa)
}

/// `fa_to_regex_steps` without the steps.
pub fn fa_to_regex(fa: &Fa) -> Regex {
    fa_to_regex_steps(fa).regex
}

/// De-expressionify construction (JFLAP `REToFSAController`): a two-state graph whose only
/// transition carries the expression is resolved until every label is a symbol or λ.
pub fn regex_to_nfa_steps(regex: &Regex) -> NfaConstruction {
    from_regex::regex_to_nfa(regex)
}

/// `regex_to_nfa_steps` without the steps.
pub fn regex_to_nfa(regex: &Regex) -> Fa {
    regex_to_nfa_steps(regex).nfa
}

/// Language equality (JFLAP `FSAEqualityChecker`) plus the shortest string, smallest in
/// character order, accepted by exactly one of the automata.
pub fn equivalent(a: &Fa, b: &Fa) -> Equivalence {
    equivalence::equivalent(a, b)
}
