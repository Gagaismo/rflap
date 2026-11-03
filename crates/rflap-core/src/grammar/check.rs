//! Grammar type tests (JFLAP `GrammarChecker` and `GrammarTypeTestAction`).

use serde::{Deserialize, Serialize};

use super::model::{Grammar, Production, is_variable_symbol};

/// What JFLAP's "Test for Grammar Type" reports, in the order it checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classification {
    RightLinear,
    LeftLinear,
    Cnf,
    Gnf,
    ContextFree,
    TuringConverted,
    ContextSensitive,
    Unrestricted,
}

impl Classification {
    /// The sentence JFLAP shows in its dialog.
    pub fn message(self) -> &'static str {
        match self {
            Classification::RightLinear => {
                "This is a right-linear Grammar (Regular Grammar and Context-Free Grammar)"
            }
            Classification::LeftLinear => {
                "This is a left-linear Grammar (Regular Grammar and Context-Free Grammar)"
            }
            Classification::Cnf => "This is a CNF Grammar (also Context-Free Grammar)",
            Classification::Gnf => "This is a GNF Grammar (also Context-Free Grammar)",
            Classification::ContextFree => "This is a Context-Free Grammar",
            Classification::TuringConverted => {
                "This is an Unrestricted Grammar (converted from TM)"
            }
            Classification::ContextSensitive => {
                "This is a Context-Sensitive Grammar (also Unrestricted Grammar)"
            }
            Classification::Unrestricted => "This is an Unrestricted Grammar",
        }
    }
}

/// Every property JFLAP can tell about a grammar, plus the single answer its dialog gives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeReport {
    pub right_linear: bool,
    pub left_linear: bool,
    /// Right-linear or left-linear.
    pub regular: bool,
    /// Every left side is a single variable.
    pub context_free: bool,
    pub cnf: bool,
    pub gnf: bool,
    /// No production shrinks (`|lhs| ≤ |rhs|`).
    pub context_sensitive: bool,
    /// The three starting productions JFLAP's Turing-machine conversion writes.
    pub turing_converted: bool,
    pub classification: Classification,
}

const TM_LHS: [&str; 3] = ["S", "S", "S"];
const TM_RHS: [&str; 3] = ["V(==)S", "SV(==)", "T"];

impl Grammar {
    pub fn is_right_linear(&self) -> bool {
        self.productions().iter().all(Production::is_right_linear)
    }

    pub fn is_left_linear(&self) -> bool {
        self.productions().iter().all(Production::is_left_linear)
    }

    pub fn is_regular(&self) -> bool {
        self.is_right_linear() || self.is_left_linear()
    }

    pub fn is_context_free(&self) -> bool {
        self.productions()
            .iter()
            .all(Production::is_restricted_on_lhs)
    }

    pub fn is_context_sensitive(&self) -> bool {
        self.productions()
            .iter()
            .all(|p| p.lhs.chars().count() <= p.rhs.chars().count())
    }

    /// Context-free with every production `A → BC` or `A → a`.
    pub fn is_chomsky(&self) -> bool {
        self.is_context_free() && self.productions().iter().all(is_chomsky_production)
    }

    /// JFLAP's Greibach test: every right side is empty or a terminal followed by variables.
    pub fn is_greibach(&self) -> bool {
        self.is_context_free()
            && self.productions().iter().all(|p| {
                let mut cs = p.rhs.chars();
                match cs.next() {
                    None => true,
                    Some(first) => !is_variable_symbol(first) && cs.all(is_variable_symbol),
                }
            })
    }

    /// Mirrors `TuringChecker.check`: the first three productions are JFLAP's TM template.
    pub fn is_turing_converted(&self) -> bool {
        let ps = self.productions();
        ps.len() >= 3 && (0..3).all(|i| ps[i].lhs == TM_LHS[i] && ps[i].rhs == TM_RHS[i])
    }

    pub fn type_report(&self) -> TypeReport {
        let right_linear = self.is_right_linear();
        let left_linear = self.is_left_linear();
        let context_free = self.is_context_free();
        let cnf = self.is_chomsky();
        let gnf = self.is_greibach();
        let context_sensitive = self.is_context_sensitive();
        let turing_converted = self.is_turing_converted();
        let classification = if right_linear {
            Classification::RightLinear
        } else if left_linear {
            Classification::LeftLinear
        } else if context_free {
            if cnf {
                Classification::Cnf
            } else if gnf {
                Classification::Gnf
            } else {
                Classification::ContextFree
            }
        } else if turing_converted {
            Classification::TuringConverted
        } else if context_sensitive {
            Classification::ContextSensitive
        } else {
            Classification::Unrestricted
        };
        TypeReport {
            right_linear,
            left_linear,
            regular: right_linear || left_linear,
            context_free,
            cnf,
            gnf,
            context_sensitive,
            turing_converted,
            classification,
        }
    }
}

pub(crate) fn is_chomsky_production(p: &Production) -> bool {
    let rhs: Vec<char> = p.rhs.chars().collect();
    match rhs.as_slice() {
        [t] => !is_variable_symbol(*t),
        [a, b] => is_variable_symbol(*a) && is_variable_symbol(*b),
        _ => false,
    }
}
