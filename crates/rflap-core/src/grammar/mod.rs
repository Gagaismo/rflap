//! Grammars: model, type checks, transformations and parsers (JFLAP `grammar`).
//!
//! Symbols follow JFLAP: a variable is an uppercase character, everything else is a terminal,
//! an empty right side is λ, and the start variable is the left side of the first production.
//! Conversions between grammars and automata live elsewhere; the model is a plain ordered list
//! of [`Production`]s so they are easy to build.

mod check;
mod membership;
mod model;
pub mod parse;
mod transform;

pub use check::{Classification, TypeReport};
pub use membership::{
    DEFAULT_MAX_FORMS, MembershipError, TkReport, membership_tk, membership_tk_limited,
};
pub use model::{Grammar, GrammarError, LAMBDA, Production, is_variable_symbol};
pub use parse::*;
pub use transform::{
    ChomskyPipeline, CnfConversion, CnfKind, CnfStep, LambdaRemoval, LambdaStep, TransformError,
    UnitRemoval, UnitStep, UselessRemoval, UselessStep, chomsky_pipeline,
    remove_lambda_productions, remove_unit_productions, remove_useless_productions,
    to_chomsky_normal_form,
};
