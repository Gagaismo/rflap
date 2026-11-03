//! Context-free grammar to pushdown automaton, LL and LR constructions
//! (JFLAP `CFGToPDALLConverter`, `CFGToPDALRConverter`).

use serde::{Deserialize, Serialize};

use super::{ConvertError, row_point};
use crate::automaton::{StateId, Transition};
use crate::grammar::{Grammar, Production};
use crate::pda::{INITIAL_STACK, Pda, PdaLabel};

/// A transition created by the construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CfgToPdaStep {
    /// The transition that starts the derivation (LL: pushes the start variable) or the
    /// one that pops the start variable (LR).
    Start {
        from: StateId,
        to: StateId,
        label: PdaLabel,
    },
    /// The transition that ends the run by popping `Z`.
    Accept {
        from: StateId,
        to: StateId,
        label: PdaLabel,
    },
    /// The transition for a terminal: LL matches it against the stack, LR shifts it.
    Terminal {
        symbol: char,
        from: StateId,
        to: StateId,
        label: PdaLabel,
    },
    /// The transition for a production: LL expands, LR reduces.
    Production {
        production: Production,
        from: StateId,
        to: StateId,
        label: PdaLabel,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CfgToPda {
    /// Accepts by final state.
    pub pda: Pda,
    pub steps: Vec<CfgToPdaStep>,
}

fn check(grammar: &Grammar) -> Result<(), ConvertError> {
    if grammar.productions().is_empty() {
        return Err(ConvertError::EmptyGrammar);
    }
    match grammar
        .productions()
        .iter()
        .find(|p| !p.is_restricted_on_lhs())
    {
        Some(p) => Err(ConvertError::NotContextFree {
            production: p.clone(),
        }),
        None => Ok(()),
    }
}

/// Three states in a row: initial, middle, final.
fn skeleton() -> (Pda, [StateId; 3]) {
    let mut pda = Pda::new();
    let ids = [0, 1, 2].map(|i| pda.create_state(row_point(i)));
    pda.set_initial(Some(ids[0]));
    pda.set_final(ids[2], true);
    (pda, ids)
}

fn add(pda: &mut Pda, from: StateId, to: StateId, label: &PdaLabel) {
    pda.add_transition(Transition::new(from, to, label.clone()));
}

/// LL construction (JFLAP "Convert CFG to PDA (LL)"): `q0 -λ,Z;SZ-> q1`, `q1 -λ,Z;λ-> q2`,
/// `q1 -a,a;λ-> q1` for each terminal and `q1 -λ,A;w-> q1` for each production `A -> w`. The
/// automaton simulates a leftmost derivation on its stack and accepts by final state.
///
/// Terminals are taken in character order (JFLAP uses its hash-set order).
pub fn cfg_to_pda_ll(grammar: &Grammar) -> Result<CfgToPda, ConvertError> {
    check(grammar)?;
    let (mut pda, [q0, q1, q2]) = skeleton();
    let mut steps = Vec::new();

    let start = PdaLabel::new(
        "",
        INITIAL_STACK,
        &format!("{}{INITIAL_STACK}", grammar.start()),
    );
    add(&mut pda, q0, q1, &start);
    steps.push(CfgToPdaStep::Start {
        from: q0,
        to: q1,
        label: start,
    });
    let accept = PdaLabel::new("", INITIAL_STACK, "");
    add(&mut pda, q1, q2, &accept);
    steps.push(CfgToPdaStep::Accept {
        from: q1,
        to: q2,
        label: accept,
    });
    for symbol in grammar.terminals() {
        let s = symbol.to_string();
        let label = PdaLabel::new(&s, &s, "");
        add(&mut pda, q1, q1, &label);
        steps.push(CfgToPdaStep::Terminal {
            symbol,
            from: q1,
            to: q1,
            label,
        });
    }
    for p in grammar.productions() {
        let label = PdaLabel::new("", &p.lhs, &p.rhs);
        add(&mut pda, q1, q1, &label);
        steps.push(CfgToPdaStep::Production {
            production: p.clone(),
            from: q1,
            to: q1,
            label,
        });
    }
    Ok(CfgToPda { pda, steps })
}

/// LR construction (JFLAP "Convert CFG to PDA (LR)"): `q0 -λ,S;λ-> q1`, `q1 -λ,Z;λ-> q2`,
/// `q0 -a,λ;a-> q0` (shift) for each terminal and `q0 -λ,w^R;A-> q0` (reduce) for each
/// production `A -> w`. The automaton simulates a rightmost derivation backwards and accepts
/// by final state. A production `A -> λ` reduces without popping, so the search over this
/// automaton is unbounded unless the caller bounds the stack.
///
/// Terminals are taken in character order (JFLAP uses its hash-set order).
pub fn cfg_to_pda_lr(grammar: &Grammar) -> Result<CfgToPda, ConvertError> {
    check(grammar)?;
    let (mut pda, [q0, q1, q2]) = skeleton();
    let mut steps = Vec::new();

    let start = PdaLabel::new("", grammar.start(), "");
    add(&mut pda, q0, q1, &start);
    steps.push(CfgToPdaStep::Start {
        from: q0,
        to: q1,
        label: start,
    });
    let accept = PdaLabel::new("", INITIAL_STACK, "");
    add(&mut pda, q1, q2, &accept);
    steps.push(CfgToPdaStep::Accept {
        from: q1,
        to: q2,
        label: accept,
    });
    for symbol in grammar.terminals() {
        let s = symbol.to_string();
        let label = PdaLabel::new(&s, "", &s);
        add(&mut pda, q0, q0, &label);
        steps.push(CfgToPdaStep::Terminal {
            symbol,
            from: q0,
            to: q0,
            label,
        });
    }
    for p in grammar.productions() {
        let reversed: String = p.rhs.chars().rev().collect();
        let label = PdaLabel::new("", &reversed, &p.lhs);
        add(&mut pda, q0, q0, &label);
        steps.push(CfgToPdaStep::Production {
            production: p.clone(),
            from: q0,
            to: q0,
            label,
        });
    }
    Ok(CfgToPda { pda, steps })
}
