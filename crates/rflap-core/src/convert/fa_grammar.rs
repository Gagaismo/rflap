//! Finite automaton to right-linear grammar and back (JFLAP `FSAToRegularGrammarConverter`,
//! `RightLinearGrammarToFSAConverter`).

use serde::{Deserialize, Serialize};

use super::ConvertError;
use crate::automaton::{StateId, Transition};
use crate::fa::{Fa, FaLabel};
use crate::grammar::{Grammar, Production, is_variable_symbol};
use crate::layout::{self, Algorithm, LayoutOptions};

/// The variable that stands for a state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateVariable {
    pub state: StateId,
    pub variable: char,
}

/// One object of the automaton and the productions it gives (JFLAP reveals them one by one).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FaToGrammarStep {
    /// `A -> xB` for a transition `A -x-> B` (one production per character of a range).
    Transition {
        from: StateId,
        to: StateId,
        label: String,
        productions: Vec<Production>,
    },
    /// `A -> λ` for a final state.
    FinalState {
        state: StateId,
        production: Production,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaToGrammar {
    pub grammar: Grammar,
    /// Variable of every state, by state id.
    pub variables: Vec<StateVariable>,
    pub steps: Vec<FaToGrammarStep>,
}

/// Right-linear grammar of an automaton (JFLAP's "Convert to Grammar" for finite automata).
///
/// The initial state is `S` and the others take `A`, `B`, ... in state id order. Each
/// transition `p -x-> q` gives `P -> xQ` and each final state `P -> λ`. The grammar lists
/// the productions of `S` first and keeps the generation order otherwise. Like JFLAP it needs
/// an initial state, at most 26 states and no capital letters in labels; a range label
/// `[a-c]` is expanded into one production per character.
pub fn fa_to_grammar(fa: &Fa) -> Result<FaToGrammar, ConvertError> {
    let initial = fa.initial().ok_or(ConvertError::NoInitialState)?;
    let count = fa.states().count();
    if count > 26 {
        return Err(ConvertError::TooManyStates { count });
    }
    if let Some(t) = fa
        .transitions()
        .iter()
        .find(|t| t.label.0.chars().any(is_variable_symbol))
    {
        return Err(ConvertError::CapitalInTransition {
            from: t.from,
            to: t.to,
            label: t.label.0.clone(),
        });
    }

    let mut letters = ('A'..='Z').filter(|&c| c != 'S');
    let mut variables = vec![StateVariable {
        state: initial,
        variable: 'S',
    }];
    for s in fa.states().filter(|s| s.id != initial) {
        let variable = letters.next().expect("at most 25 other states");
        variables.push(StateVariable {
            state: s.id,
            variable,
        });
    }
    variables.sort_by_key(|v| v.state);
    let var_of = |state: StateId| {
        variables
            .iter()
            .find(|v| v.state == state)
            .map_or('S', |v| v.variable)
    };

    let mut steps = Vec::new();
    let mut productions = Vec::new();
    for t in fa.transitions() {
        let (lhs, to) = (var_of(t.from).to_string(), var_of(t.to));
        let prefixes: Vec<String> = match t.label.range() {
            Some((lo, hi)) => (lo as u32..=hi as u32)
                .filter_map(char::from_u32)
                .map(String::from)
                .collect(),
            None => vec![t.label.0.clone()],
        };
        let made: Vec<Production> = prefixes
            .iter()
            .map(|p| Production::new(&lhs, &format!("{p}{to}")))
            .collect();
        productions.extend(made.iter().cloned());
        steps.push(FaToGrammarStep::Transition {
            from: t.from,
            to: t.to,
            label: t.label.0.clone(),
            productions: made,
        });
    }
    for &state in fa.finals() {
        let production = Production::new(&var_of(state).to_string(), "");
        productions.push(production.clone());
        steps.push(FaToGrammarStep::FinalState { state, production });
    }

    // The productions of S come first (JFLAP sorts them so), the rest keep their order.
    let (mut ordered, rest): (Vec<_>, Vec<_>) = productions.into_iter().partition(|p| p.lhs == "S");
    ordered.extend(rest);
    let grammar = Grammar::from_parts("S", ordered).expect("left sides are never empty");
    Ok(FaToGrammar {
        grammar,
        variables,
        steps,
    })
}

/// The state that stands for a variable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableState {
    pub variable: String,
    pub state: StateId,
}

/// The transition a production gives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarToFaStep {
    pub production: Production,
    pub from: StateId,
    pub to: StateId,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrammarToFa {
    pub fa: Fa,
    /// State of every variable, in the order the states were created.
    pub variables: Vec<VariableState>,
    /// The extra final state that receives the productions without a variable.
    pub final_state: StateId,
    pub steps: Vec<GrammarToFaStep>,
}

/// Automaton of a right-linear grammar (JFLAP `RightLinearGrammarToFSAConverter`).
///
/// One state per variable (labelled with it, initial for the start variable) plus a final
/// state; `A -> wB` gives `A -w-> B` and `A -> w` gives `A -w-> final`. JFLAP converts only
/// right-linear grammars: the first production that is not right linear is the error.
pub fn grammar_to_fa(grammar: &Grammar) -> Result<GrammarToFa, ConvertError> {
    if grammar.productions().is_empty() {
        return Err(ConvertError::EmptyGrammar);
    }
    if let Some(p) = grammar.productions().iter().find(|p| !p.is_right_linear()) {
        return Err(ConvertError::NotRightLinear {
            production: p.clone(),
        });
    }

    let mut fa = Fa::new();
    let mut variables = Vec::new();
    for v in grammar.variables_jflap_order() {
        let id = fa.create_state(Default::default());
        let name = v.to_string();
        if name == grammar.start() {
            fa.set_initial(Some(id));
        }
        if let Some(s) = fa.state_mut(id) {
            s.label = Some(name.clone());
        }
        variables.push(VariableState {
            variable: name,
            state: id,
        });
    }
    let final_state = fa.create_state(Default::default());
    fa.set_final(final_state, true);
    let state_of = |name: &str| {
        variables
            .iter()
            .find(|v| v.variable == name)
            .map(|v| v.state)
    };

    let mut steps = Vec::new();
    for p in grammar.productions() {
        let from = state_of(&p.lhs).expect("the left side is a variable of the grammar");
        let with_variable = p.variables_on_rhs().len() == 1
            && p.rhs.chars().next_back().is_some_and(is_variable_symbol);
        let (to, label) = if with_variable {
            let var = p.variables_on_rhs()[0].to_string();
            let label = p.rhs[..p.rhs.len() - var.len()].to_string();
            (state_of(&var).expect("variable of the grammar"), label)
        } else {
            (final_state, p.rhs.clone())
        };
        fa.add_transition(Transition::new(from, to, FaLabel(label.clone())));
        steps.push(GrammarToFaStep {
            production: p.clone(),
            from,
            to,
            label,
        });
    }
    layout::apply(&mut fa, Algorithm::Circle, &LayoutOptions::default());
    Ok(GrammarToFa {
        fa,
        variables,
        final_state,
        steps,
    })
}
