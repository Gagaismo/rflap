//! `convert` and `equivalent`.

use rflap_core::analysis::clean_useless;
use rflap_core::convert::{
    cfg_to_pda_ll, cfg_to_pda_lr, fa_to_grammar, grammar_to_fa, pda_to_cfg, tm_to_grammar,
};
use rflap_core::fa::{Fa, add_trap_state, split_multi_char_labels};
use rflap_core::fa_convert;
use rflap_core::grammar::{
    Grammar, chomsky_pipeline, remove_lambda_productions, remove_unit_productions,
    remove_useless_productions,
};
use serde::Serialize;
use serde_json::{Value, json};

use super::{Object, core_error, no_initial_state, object, state_name, to_value, wrong_type};
use crate::document::{Document, RegexDoc};
use crate::error::{ApiError, code};
use crate::request::ConvertTo;

fn failed<E: Serialize + std::fmt::Display>(e: E) -> ApiError {
    core_error(code::CONVERSION_FAILED, &e)
}

/// A core result as `{ "document": <model>, ...the rest }`.
fn result<T: Serialize>(to: ConvertTo, core: &T, model_key: &str, document: Document) -> Value {
    let mut out: Object = object(core);
    out.remove(model_key);
    out.insert("to".into(), json!(to.name()));
    out.insert("document".into(), document.to_value());
    Value::Object(out)
}

fn grammar_of(doc: &Document, to: ConvertTo) -> Result<&Grammar, ApiError> {
    match doc {
        Document::Grammar(g) => Ok(g),
        other => Err(wrong_type(
            "convert",
            &format!("a grammar document to convert to `{}`", to.name()),
            other,
        )),
    }
}

fn fa_of(doc: &Document, to: ConvertTo) -> Result<&Fa, ApiError> {
    match doc {
        Document::Fa(a) => Ok(a),
        other => Err(wrong_type(
            "convert",
            &format!("a fa document to convert to `{}`", to.name()),
            other,
        )),
    }
}

/// The finite automaton a document stands for: a fa, a regex (through its NFA) or a
/// right-linear grammar.
pub fn as_fa(doc: &Document, op: &str) -> Result<Fa, ApiError> {
    match doc {
        Document::Fa(a) => Ok(a.clone()),
        Document::Regex(r) => Ok(fa_convert::regex_to_nfa(&r.parse()?)),
        Document::Grammar(g) => Ok(grammar_to_fa(g).map_err(failed)?.fa),
        other => Err(wrong_type(
            op,
            "a fa, a regex or a right-linear grammar document",
            other,
        )),
    }
}

pub fn convert(doc: &Document, to: ConvertTo) -> Result<Value, ApiError> {
    Ok(match to {
        ConvertTo::Dfa => {
            let fa = as_fa(doc, "convert to `dfa`")?;
            let conversion = fa_convert::nfa_to_dfa(&fa);
            let dfa = conversion.dfa.clone();
            result(to, &conversion, "dfa", Document::Fa(dfa))
        }
        ConvertTo::MinimalDfa => {
            let fa = as_fa(doc, "convert to `minimal_dfa`")?;
            if fa.initial().is_none() {
                return Err(no_initial_state());
            }
            let minimization = fa_convert::minimize(&fa);
            let dfa = minimization.dfa.clone();
            result(to, &minimization, "dfa", Document::Fa(dfa))
        }
        ConvertTo::Regex => {
            let fa = fa_of(doc, to)?;
            if fa.initial().is_none() {
                return Err(no_initial_state());
            }
            let conversion = fa_convert::fa_to_regex_steps(fa);
            let regex = RegexDoc::of(&conversion.regex);
            let mut out = result(to, &conversion, "regex", Document::Regex(regex.clone()));
            out["expression"] = json!(regex.expression);
            out
        }
        ConvertTo::Nfa => match doc {
            Document::Regex(r) => {
                let construction = fa_convert::regex_to_nfa_steps(&r.parse()?);
                let nfa = construction.nfa.clone();
                result(to, &construction, "nfa", Document::Fa(nfa))
            }
            other => {
                return Err(wrong_type(
                    "convert",
                    "a regex document to convert to `nfa`",
                    other,
                ));
            }
        },
        ConvertTo::RegularGrammar => {
            let conversion = fa_to_grammar(fa_of(doc, to)?).map_err(failed)?;
            let grammar = conversion.grammar.clone();
            result(to, &conversion, "grammar", Document::Grammar(grammar))
        }
        ConvertTo::Fa => {
            let conversion = grammar_to_fa(grammar_of(doc, to)?).map_err(failed)?;
            let fa = conversion.fa.clone();
            result(to, &conversion, "fa", Document::Fa(fa))
        }
        ConvertTo::PdaLl | ConvertTo::PdaLr => {
            let g = grammar_of(doc, to)?;
            let conversion = if to == ConvertTo::PdaLl {
                cfg_to_pda_ll(g)
            } else {
                cfg_to_pda_lr(g)
            }
            .map_err(failed)?;
            let pda = conversion.pda.clone();
            result(to, &conversion, "pda", Document::Pda(pda))
        }
        ConvertTo::Cfg => match doc {
            Document::Pda(pda) => {
                let conversion = pda_to_cfg(pda).map_err(failed)?;
                let grammar = conversion.grammar.clone();
                result(to, &conversion, "grammar", Document::Grammar(grammar))
            }
            other => {
                return Err(wrong_type(
                    "convert",
                    "a pda document to convert to `cfg`",
                    other,
                ));
            }
        },
        ConvertTo::UnrestrictedGrammar => match doc {
            Document::Turing(tm) => {
                let conversion = tm_to_grammar(tm).map_err(failed)?;
                let grammar = conversion.grammar.clone();
                result(to, &conversion, "grammar", Document::Grammar(grammar))
            }
            other => {
                return Err(wrong_type(
                    "convert",
                    "a turing document to convert to `unrestricted_grammar`",
                    other,
                ));
            }
        },
        ConvertTo::RemoveLambda => {
            let removal = remove_lambda_productions(grammar_of(doc, to)?).map_err(failed)?;
            let grammar = removal.grammar.clone();
            result(to, &removal, "grammar", Document::Grammar(grammar))
        }
        ConvertTo::RemoveUnit => {
            let removal = remove_unit_productions(grammar_of(doc, to)?).map_err(failed)?;
            let grammar = removal.grammar.clone();
            result(to, &removal, "grammar", Document::Grammar(grammar))
        }
        ConvertTo::RemoveUseless => {
            let removal = remove_useless_productions(grammar_of(doc, to)?).map_err(failed)?;
            let grammar = removal.grammar.clone();
            result(to, &removal, "grammar", Document::Grammar(grammar))
        }
        ConvertTo::Cnf => {
            let pipeline = chomsky_pipeline(grammar_of(doc, to)?).map_err(failed)?;
            let grammar = pipeline.grammar.clone();
            let mut stages = object(&pipeline);
            stages.remove("grammar");
            json!({
                "to": to.name(),
                "document": Document::Grammar(grammar),
                "steps": stages,
            })
        }
        ConvertTo::Complete => {
            let mut fa = fa_of(doc, to)?.clone();
            let before = fa.transitions().len();
            let trap = add_trap_state(&mut fa);
            let name = state_name(&fa, trap);
            let steps = json!([{
                "kind": "add_trap_state",
                "state": name,
                "transitions_added": fa.transitions().len() - before,
            }]);
            json!({"to": to.name(), "document": Document::Fa(fa), "trap_state": name, "steps": steps})
        }
        ConvertTo::SplitLabels => {
            let original = fa_of(doc, to)?;
            let fa = split_multi_char_labels(original);
            // the transitions that no longer exist are the ones that were split
            let steps: Vec<Value> = original
                .transitions()
                .iter()
                .filter(|t| !fa.transitions().contains(t))
                .map(|t| {
                    json!({
                        "kind": "split_label",
                        "from": state_name(original, t.from),
                        "to": state_name(original, t.to),
                        "label": t.label.0,
                    })
                })
                .collect();
            json!({"to": to.name(), "document": Document::Fa(fa), "steps": steps})
        }
        ConvertTo::RemoveUselessStates => {
            let document = match doc {
                Document::Fa(a) => Document::Fa(clean_useless(a).map_err(|_| no_initial_state())?),
                Document::Pda(a) => {
                    Document::Pda(clean_useless(a).map_err(|_| no_initial_state())?)
                }
                Document::Mealy(a) => {
                    Document::Mealy(clean_useless(a).map_err(|_| no_initial_state())?)
                }
                Document::Moore(a) => {
                    Document::Moore(clean_useless(a).map_err(|_| no_initial_state())?)
                }
                other => {
                    return Err(wrong_type(
                        "convert",
                        "a fa, pda, mealy or moore document to convert to `remove_useless_states`",
                        other,
                    ));
                }
            };
            let removed = removed_states(doc, &document);
            json!({
                "to": to.name(),
                "document": document,
                "steps": [{"kind": "remove_states", "states": removed}],
            })
        }
    })
}

pub fn equivalent(a: &Document, b: &Document) -> Result<Value, ApiError> {
    let x = as_fa(a, "equivalent")?;
    let y = as_fa(b, "equivalent")?;
    Ok(to_value(&fa_convert::equivalent(&x, &y)))
}

/// Names of the states of `before` that `after` no longer has.
fn removed_states(before: &Document, after: &Document) -> Vec<String> {
    fn names(doc: &Document) -> Vec<String> {
        super::with_automaton!(doc, a => a.states().map(|s| s.name.clone()).collect(), _ => Vec::new())
    }
    let kept = names(after);
    names(before)
        .into_iter()
        .filter(|n| !kept.contains(n))
        .collect()
}
