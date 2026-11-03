//! Grammar transformations with replayable steps: λ-, unit- and useless-production removal and
//! Chomsky normal form (JFLAP `LambdaProductionRemover`, `UnitProductionRemover`,
//! `UselessProductionRemover`, `CNFConverter` and the `gui.grammar.transform` controllers).
//!
//! Each transformation returns the new grammar and the steps JFLAP walks the student through,
//! in JFLAP's order, so a UI can replay them one by one.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use serde::{Deserialize, Serialize};

use super::model::{Grammar, Production};

/// Why a transformation refused a grammar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformError {
    /// A production whose left side is not a single variable.
    NotContextFree(Production),
    /// CNF conversion needs a grammar without λ- and unit productions.
    NotApplicable(Production),
    /// JFLAP reserves `(` and `)` for the working variables of the CNF conversion.
    ReservedSymbol(char),
    /// More working variables than unused capital letters (`needed` counts all variables).
    TooManyVariables { needed: usize },
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransformError::NotContextFree(p) => {
                write!(
                    f,
                    "the production {p} is unrestricted on the left hand side"
                )
            }
            TransformError::NotApplicable(p) => {
                write!(
                    f,
                    "the production {p} must be removed before converting to CNF"
                )
            }
            TransformError::ReservedSymbol(c) => {
                write!(f, "Grammar has the {c} character, which is reserved.")
            }
            TransformError::TooManyVariables { needed } => {
                write!(f, "26 variables available, but {needed} needed!")
            }
        }
    }
}

impl std::error::Error for TransformError {}

fn require_context_free(g: &Grammar) -> Result<(), TransformError> {
    match g.productions().iter().find(|p| !p.is_restricted_on_lhs()) {
        Some(p) => Err(TransformError::NotContextFree(p.clone())),
        None => Ok(()),
    }
}

fn single(lhs: &str) -> char {
    lhs.chars().next().unwrap_or('\0')
}

// ---------------------------------------------------------------------------------------------
// λ-production removal
// ---------------------------------------------------------------------------------------------

/// One step of the λ-removal walk-through.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LambdaStep {
    /// `variable` joins the set of variables that derive λ, because of `production`.
    DerivesLambda {
        variable: char,
        production: Production,
    },
    /// A λ-production is deleted.
    RemoveLambda { production: Production },
    /// `production` is kept and `added` are all its variants with nullable variables dropped
    /// (sorted, never empty; it includes `production` itself).
    Expand {
        production: Production,
        added: Vec<Production>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LambdaRemoval {
    /// The variables that derive λ.
    pub lambda_set: BTreeSet<char>,
    /// JFLAP warns that the new grammar no longer generates λ when this holds.
    pub start_derives_lambda: bool,
    pub steps: Vec<LambdaStep>,
    pub grammar: Grammar,
}

/// The variables that derive λ, in the order JFLAP finds them, with the production that proves
/// each one.
fn lambda_derivations(g: &Grammar) -> Vec<(char, Production)> {
    let order = g.variables_jflap_order();
    let mut set: BTreeSet<char> = BTreeSet::new();
    let mut found = Vec::new();
    loop {
        let next = order.iter().filter(|v| !set.contains(v)).find_map(|&v| {
            let lhs = v.to_string();
            let own: Vec<&Production> = g.productions_for(&lhs).collect();
            own.iter()
                .find(|p| p.is_lambda())
                .or_else(|| {
                    own.iter().find(|p| {
                        p.terminals_on_rhs().is_empty()
                            && p.variables_on_rhs().iter().all(|x| set.contains(x))
                    })
                })
                .map(|p| (v, (*p).clone()))
        });
        match next {
            Some((v, p)) => {
                set.insert(v);
                found.push((v, p));
            }
            None => return found,
        }
    }
}

/// The variants of `p` with any subset of the nullable variables removed, sorted, without the
/// empty string (JFLAP `getProductionsToAddForProduction`).
fn lambda_expansions(p: &Production, nullable: &BTreeSet<char>) -> Vec<Production> {
    let mut combos: Vec<String> = vec![String::new()];
    for c in p.rhs.chars() {
        if nullable.contains(&c) {
            let mut next: Vec<String> = combos
                .iter()
                .map(|s| {
                    let mut t = s.clone();
                    t.push(c);
                    t
                })
                .collect();
            next.extend(combos.iter().cloned());
            combos = next;
        } else {
            for s in &mut combos {
                s.push(c);
            }
        }
    }
    combos.sort();
    combos.dedup();
    combos
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|rhs| Production {
            lhs: p.lhs.clone(),
            rhs,
        })
        .collect()
}

/// Removes λ-productions the way JFLAP does: first the set of variables that derive λ, then
/// every production is replaced by its variants without nullable variables. Note that, as in
/// JFLAP, the result cannot generate λ when the start variable derives it.
pub fn remove_lambda_productions(g: &Grammar) -> Result<LambdaRemoval, TransformError> {
    require_context_free(g)?;
    let derivations = lambda_derivations(g);
    let lambda_set: BTreeSet<char> = derivations.iter().map(|(v, _)| *v).collect();
    let mut steps: Vec<LambdaStep> = derivations
        .into_iter()
        .map(|(variable, production)| LambdaStep::DerivesLambda {
            variable,
            production,
        })
        .collect();
    for p in g.productions().iter().filter(|p| p.rhs.is_empty()) {
        steps.push(LambdaStep::RemoveLambda {
            production: p.clone(),
        });
    }
    let mut out = Vec::new();
    for p in g.productions().iter().filter(|p| !p.rhs.is_empty()) {
        let added = lambda_expansions(p, &lambda_set);
        out.extend(added.iter().cloned());
        steps.push(LambdaStep::Expand {
            production: p.clone(),
            added,
        });
    }
    let start_derives_lambda = single(g.start()) != '\0' && lambda_set.contains(&single(g.start()));
    let grammar = Grammar::from_parts(g.start(), out).expect("left sides are never empty");
    Ok(LambdaRemoval {
        lambda_set,
        start_derives_lambda,
        steps,
        grammar,
    })
}

// ---------------------------------------------------------------------------------------------
// unit-production removal
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitStep {
    /// An edge of the variable dependency graph, one per unit production.
    Edge {
        from: char,
        to: char,
        production: Production,
    },
    /// The variables `variable` depends on (reachable through unit productions, not itself).
    Reach {
        variable: char,
        reachable: Vec<char>,
    },
    /// A unit production is deleted.
    RemoveUnit { production: Production },
    /// `production` is new: it copies a non-unit production of `via`, which `production.lhs`
    /// depends on.
    Add { production: Production, via: char },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitRemoval {
    /// Edges of the variable dependency graph (`A → B` for each unit production `A → B`).
    pub edges: Vec<(char, char)>,
    /// `(A, B)` when `A` derives `B` through unit productions only, `A ≠ B`.
    pub unit_pairs: Vec<(char, char)>,
    pub steps: Vec<UnitStep>,
    pub grammar: Grammar,
}

/// Removes unit productions: every non-unit production of a variable `B` is copied to each
/// variable `A` that derives `B` through unit productions (JFLAP `UnitProductionRemover`).
pub fn remove_unit_productions(g: &Grammar) -> Result<UnitRemoval, TransformError> {
    require_context_free(g)?;
    let units: Vec<&Production> = g.productions().iter().filter(|p| p.is_unit()).collect();
    let edges: Vec<(char, char)> = units
        .iter()
        .map(|p| (single(&p.lhs), single(&p.rhs)))
        .collect();
    let mut steps: Vec<UnitStep> = units
        .iter()
        .zip(&edges)
        .map(|(p, &(from, to))| UnitStep::Edge {
            from,
            to,
            production: (*p).clone(),
        })
        .collect();

    let order = g.variables_jflap_order();
    let mut deps: Vec<(char, Vec<char>)> = Vec::new();
    for &v in &order {
        let reach = reachable_from(v, &edges);
        let list: Vec<char> = order
            .iter()
            .copied()
            .filter(|w| *w != v && reach.contains(w))
            .collect();
        if !list.is_empty() {
            steps.push(UnitStep::Reach {
                variable: v,
                reachable: list.clone(),
            });
        }
        deps.push((v, list));
    }
    let unit_pairs: Vec<(char, char)> = deps
        .iter()
        .flat_map(|(v, list)| list.iter().map(move |w| (*v, *w)))
        .collect();

    for p in &units {
        steps.push(UnitStep::RemoveUnit {
            production: (*p).clone(),
        });
    }
    let mut out: Vec<Production> = g
        .productions()
        .iter()
        .filter(|p| !p.is_unit())
        .cloned()
        .collect();
    for (v, list) in &deps {
        for &w in list {
            let wl = w.to_string();
            for q in g.productions_for(&wl).filter(|q| !q.is_unit()) {
                let np = Production {
                    lhs: v.to_string(),
                    rhs: q.rhs.clone(),
                };
                if !out.contains(&np) {
                    out.push(np.clone());
                    steps.push(UnitStep::Add {
                        production: np,
                        via: w,
                    });
                }
            }
        }
    }
    let grammar = Grammar::from_parts(g.start(), out).expect("left sides are never empty");
    Ok(UnitRemoval {
        edges,
        unit_pairs,
        steps,
        grammar,
    })
}

fn reachable_from(start: char, edges: &[(char, char)]) -> BTreeSet<char> {
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(v) = queue.pop_front() {
        for &(a, b) in edges {
            if a == v && seen.insert(b) {
                queue.push_back(b);
            }
        }
    }
    seen
}

// ---------------------------------------------------------------------------------------------
// useless-production removal
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UselessStep {
    /// `variable` derives a terminal string, because of `production`.
    DerivesTerminals {
        variable: char,
        production: Production,
    },
    /// A production that mentions a variable deriving no terminal string is deleted.
    RemoveNonDeriving { production: Production },
    /// `variable` is reachable from the start variable (`from` is where it was found).
    Reachable { variable: char, from: Option<char> },
    /// A production that mentions a variable unreachable from the start is deleted.
    RemoveUnreachable { production: Production },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UselessRemoval {
    pub derives_terminals: BTreeSet<char>,
    /// Reachable variables, computed on the grammar without non-deriving productions.
    pub reachable: BTreeSet<char>,
    /// The start variable derives no terminal string: no production survives.
    pub start_useless: bool,
    pub steps: Vec<UselessStep>,
    pub grammar: Grammar,
}

/// Removes useless productions in JFLAP's two phases: first the variables that derive a
/// terminal string, then the ones reachable from the start variable.
pub fn remove_useless_productions(g: &Grammar) -> Result<UselessRemoval, TransformError> {
    require_context_free(g)?;
    let order = g.variables_jflap_order();
    let mut derives: BTreeSet<char> = BTreeSet::new();
    let mut steps = Vec::new();
    loop {
        let next = order
            .iter()
            .filter(|v| !derives.contains(v))
            .find_map(|&v| {
                let lhs = v.to_string();
                g.productions_for(&lhs)
                    .find(|p| p.variables_on_rhs().iter().all(|x| derives.contains(x)))
                    .map(|p| (v, p.clone()))
            });
        match next {
            Some((variable, production)) => {
                derives.insert(variable);
                steps.push(UselessStep::DerivesTerminals {
                    variable,
                    production,
                });
            }
            None => break,
        }
    }
    let (reduced, dropped): (Vec<&Production>, Vec<&Production>) = g
        .productions()
        .iter()
        .partition(|p| p.variables().iter().all(|v| derives.contains(v)));
    for p in dropped {
        steps.push(UselessStep::RemoveNonDeriving {
            production: p.clone(),
        });
    }

    let start = single(g.start());
    if !derives.contains(&start) {
        for p in reduced {
            steps.push(UselessStep::RemoveUnreachable {
                production: p.clone(),
            });
        }
        return Ok(UselessRemoval {
            derives_terminals: derives,
            reachable: BTreeSet::new(),
            start_useless: true,
            steps,
            grammar: Grammar::from_parts(g.start(), Vec::new()).expect("no productions"),
        });
    }

    let mut reachable = BTreeSet::from([start]);
    steps.push(UselessStep::Reachable {
        variable: start,
        from: None,
    });
    let mut queue = VecDeque::from([start]);
    while let Some(v) = queue.pop_front() {
        for p in reduced.iter().filter(|p| single(&p.lhs) == v) {
            for w in p.variables_on_rhs() {
                if w != v && reachable.insert(w) {
                    steps.push(UselessStep::Reachable {
                        variable: w,
                        from: Some(v),
                    });
                    queue.push_back(w);
                }
            }
        }
    }
    let mut kept = Vec::new();
    for p in reduced {
        if p.variables().iter().all(|v| reachable.contains(v)) {
            kept.push(p.clone());
        } else {
            steps.push(UselessStep::RemoveUnreachable {
                production: p.clone(),
            });
        }
    }
    Ok(UselessRemoval {
        derives_terminals: derives,
        reachable,
        start_useless: false,
        steps,
        grammar: Grammar::from_parts(g.start(), kept).expect("left sides are never empty"),
    })
}

// ---------------------------------------------------------------------------------------------
// Chomsky normal form
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CnfKind {
    /// Terminals in a longer right side are replaced by variables (`B(a) → a`).
    Terminals,
    /// A right side with three or more variables is split (`D(1) → rest`).
    Split,
}

/// One replacement JFLAP's Chomsky pane performs. Working variables are written the way JFLAP
/// shows them before export: `B(a)` for a terminal `a`, `D(n)` for the tail of a split.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CnfStep {
    /// 1-based pass over the table (JFLAP's "Do All" repeats until nothing is left).
    pub round: usize,
    pub production: Production,
    pub replacement: Vec<Production>,
    pub kind: CnfKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CnfConversion {
    pub input: Grammar,
    /// The table with JFLAP's working variable names.
    pub intermediate: Vec<Production>,
    /// Working variable → capital letter chosen on export (sorted by working name).
    pub renames: Vec<(String, char)>,
    pub steps: Vec<CnfStep>,
    pub grammar: Grammar,
}

/// JFLAP's `CNFConverter.separateString`: one symbol per character, except that `X(...)` working
/// variables stay in one piece.
pub(crate) fn separate_symbols(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out: VecDeque<String> = VecDeque::new();
    let mut i = chars.len() as isize - 1;
    while i >= 0 {
        let end = i as usize;
        if chars[end] != ')' {
            out.push_front(chars[end].to_string());
        } else {
            while i > 0 && chars[i as usize] != '(' {
                i -= 1;
            }
            if i > 0 {
                i -= 1;
            }
            out.push_front(chars[i as usize..=end].iter().collect());
        }
        i -= 1;
    }
    out.into()
}

struct CnfState<'a> {
    grammar: &'a Grammar,
    rhs_to_lhs: BTreeMap<String, String>,
    counter: usize,
    left_added: bool,
}

impl CnfState<'_> {
    fn new(grammar: &Grammar) -> CnfState<'_> {
        let mut by_lhs: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for p in grammar.productions() {
            let rhs = by_lhs.entry(&p.lhs).or_default();
            if !rhs.contains(&p.rhs.as_str()) {
                rhs.push(&p.rhs);
            }
        }
        let mut rhs_to_lhs = BTreeMap::new();
        for (lhs, rhs) in by_lhs {
            if let [only] = rhs.as_slice() {
                rhs_to_lhs.insert(only.to_string(), lhs.to_string());
            }
        }
        CnfState {
            grammar,
            rhs_to_lhs,
            counter: 0,
            left_added: false,
        }
    }

    fn is_terminal(&self, symbol: &str) -> bool {
        let mut cs = symbol.chars();
        match (cs.next(), cs.next()) {
            (Some(c), None) => self.grammar.is_terminal(c),
            _ => false,
        }
    }

    fn is_chomsky(&self, p: &Production) -> bool {
        let symbols = separate_symbols(&p.rhs);
        match symbols.as_slice() {
            [a] => self.is_terminal(a),
            [a, b] => !self.is_terminal(a) && !self.is_terminal(b),
            _ => false,
        }
    }

    /// The variable that stands for `rhs`, creating a working variable if there is none yet.
    fn left_for(&mut self, rhs: &str) -> String {
        if let Some(v) = self.rhs_to_lhs.get(rhs) {
            self.left_added = false;
            return v.clone();
        }
        self.left_added = true;
        let name = if self.is_terminal(rhs) {
            format!("B({rhs})")
        } else {
            self.counter += 1;
            format!("D({})", self.counter)
        };
        self.rhs_to_lhs.insert(rhs.to_string(), name.clone());
        name
    }

    fn replacements(&mut self, p: &Production) -> Option<(Vec<Production>, CnfKind)> {
        let symbols = separate_symbols(&p.rhs);
        if symbols.len() <= 1 {
            return None;
        }
        if symbols.iter().any(|s| self.is_terminal(s)) {
            let mut main = String::new();
            let mut extra = Vec::new();
            for s in &symbols {
                if self.is_terminal(s) {
                    let v = self.left_for(s);
                    if self.left_added {
                        extra.push(Production {
                            lhs: v.clone(),
                            rhs: s.clone(),
                        });
                    }
                    main.push_str(&v);
                } else {
                    main.push_str(s);
                }
            }
            let mut out = vec![Production {
                lhs: p.lhs.clone(),
                rhs: main,
            }];
            out.extend(extra);
            return Some((out, CnfKind::Terminals));
        }
        if symbols.len() <= 2 {
            return None;
        }
        let tail: String = p.rhs.chars().skip(symbols[0].chars().count()).collect();
        let v = self.left_for(&tail);
        let mut out = vec![Production {
            lhs: p.lhs.clone(),
            rhs: format!("{}{}", symbols[0], v),
        }];
        if self.left_added {
            out.push(Production { lhs: v, rhs: tail });
        }
        Some((out, CnfKind::Split))
    }
}

/// Converts a grammar without λ- or unit productions to Chomsky normal form with JFLAP's
/// algorithm: repeated passes replace each non-CNF production (last one first) until none is
/// left, then the working variables get unused capital letters in alphabetical order.
pub fn to_chomsky_normal_form(g: &Grammar) -> Result<CnfConversion, TransformError> {
    require_context_free(g)?;
    for p in g.productions() {
        if let Some(c) = p.rhs.chars().find(|c| *c == '(' || *c == ')') {
            return Err(TransformError::ReservedSymbol(c));
        }
    }
    if let Some(p) = g
        .productions()
        .iter()
        .find(|p| p.rhs.is_empty() || p.is_unit())
    {
        return Err(TransformError::NotApplicable(p.clone()));
    }

    let mut state = CnfState::new(g);
    let mut table: Vec<Production> = g.productions().to_vec();
    let mut steps = Vec::new();
    let mut round = 0;
    loop {
        let need: Vec<usize> = (0..table.len())
            .filter(|&i| !state.is_chomsky(&table[i]))
            .collect();
        if need.is_empty() {
            break;
        }
        round += 1;
        let mut progress = false;
        for &idx in need.iter().rev() {
            let production = table[idx].clone();
            if let Some((replacement, kind)) = state.replacements(&production) {
                table.splice(idx..=idx, replacement.iter().cloned());
                steps.push(CnfStep {
                    round,
                    production,
                    replacement,
                    kind,
                });
                progress = true;
            }
        }
        if !progress {
            let stuck = table[need[0]].clone();
            return Err(TransformError::NotApplicable(stuck));
        }
    }

    let (final_productions, renames) = rename_working_variables(&table)?;
    let grammar =
        Grammar::from_parts(g.start(), final_productions).expect("left sides are never empty");
    Ok(CnfConversion {
        input: g.clone(),
        intermediate: table,
        renames,
        steps,
        grammar,
    })
}

/// The renamed productions and the `(working variable, letter)` pairs used.
type Renamed = (Vec<Production>, Vec<(String, char)>);

/// JFLAP's `CNFConverter.convert`: every working variable gets the next unused capital letter.
fn rename_working_variables(table: &[Production]) -> Result<Renamed, TransformError> {
    let mut available: BTreeSet<char> = ('A'..='Z').collect();
    let mut working: BTreeSet<String> = BTreeSet::new();
    for p in table {
        for s in separate_symbols(&p.rhs) {
            let mut cs = s.chars();
            match (cs.next(), cs.next()) {
                (Some(c), None) => {
                    available.remove(&c);
                }
                _ => {
                    working.insert(s);
                }
            }
        }
        if p.lhs.chars().count() == 1 {
            available.remove(&single(&p.lhs));
        }
    }
    if working.len() > available.len() {
        return Err(TransformError::TooManyVariables {
            needed: working.len() + 26 - available.len(),
        });
    }
    let map: BTreeMap<String, char> = working.into_iter().zip(available).collect();
    let rename =
        |s: &str| -> String { map.get(s).map_or_else(|| s.to_string(), |c| c.to_string()) };
    let productions = table
        .iter()
        .map(|p| Production {
            lhs: if p.lhs.chars().count() == 1 {
                p.lhs.clone()
            } else {
                rename(&p.lhs)
            },
            rhs: separate_symbols(&p.rhs)
                .iter()
                .map(|s| {
                    if s.chars().count() == 1 {
                        s.clone()
                    } else {
                        rename(s)
                    }
                })
                .collect(),
        })
        .collect();
    Ok((productions, map.into_iter().collect()))
}

// ---------------------------------------------------------------------------------------------
// λ → unit → useless → CNF (JFLAP "Transform Grammar")
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChomskyPipeline {
    /// Present when the grammar had variables deriving λ.
    pub lambda: Option<LambdaRemoval>,
    /// Present when unit productions remained.
    pub unit: Option<UnitRemoval>,
    /// Present when some production was useless.
    pub useless: Option<UselessRemoval>,
    /// Present when the grammar was not yet in Chomsky normal form.
    pub cnf: Option<CnfConversion>,
    pub grammar: Grammar,
}

/// JFLAP's "Transform Grammar": each stage runs only if it has something to do.
pub fn chomsky_pipeline(g: &Grammar) -> Result<ChomskyPipeline, TransformError> {
    require_context_free(g)?;
    let mut current = g.clone();

    let lambda = if lambda_derivations(&current).is_empty() {
        None
    } else {
        let r = remove_lambda_productions(&current)?;
        current = r.grammar.clone();
        Some(r)
    };
    let unit = if current.productions().iter().any(Production::is_unit) {
        let r = remove_unit_productions(&current)?;
        current = r.grammar.clone();
        Some(r)
    } else {
        None
    };
    let useless = {
        let r = remove_useless_productions(&current)?;
        if r.grammar.productions().len() < current.productions().len() {
            current = r.grammar.clone();
            Some(r)
        } else {
            None
        }
    };
    let already_cnf = {
        let state = CnfState::new(&current);
        current.productions().iter().all(|p| state.is_chomsky(p))
    };
    let cnf = if already_cnf {
        None
    } else {
        let r = to_chomsky_normal_form(&current)?;
        current = r.grammar.clone();
        Some(r)
    };
    Ok(ChomskyPipeline {
        lambda,
        unit,
        useless,
        cnf,
        grammar: current,
    })
}
