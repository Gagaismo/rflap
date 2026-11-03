//! Automaton design from a language description (Rflap addition).
//!
//! An AI reasons about a language given in natural language and translates it to a formal
//! [`LanguageSpec`]; [`design`] then builds the automaton, minimizes it, lays it out, names
//! and explains its states, and verifies it: every example of the spec is checked and the
//! automaton is cross-checked against the semantics of the spec on all short strings. A
//! failed example is reported in the [`Verification`], never raised as an error, so the AI
//! can fix its spec.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::analysis::unreachable_states;
use crate::automaton::{Automaton, Label, Point, StateId, Transition};
use crate::fa::{Fa, FaLabel, FaStepByState};
use crate::fa_convert::fa_to_regex;
use crate::layout::{self, Algorithm, LayoutOptions};
use crate::pda::{Acceptance, Pda, PdaStepByState};
use crate::regex::Regex;
use crate::sim::{RunOutcome, fast_run};

mod compile;
mod dfa;
mod error;
mod explain;
mod spec;
mod syntax;
mod template;

pub use error::{ConstraintSyntaxError, DesignError, NotRegularReport};
pub use explain::explain;
pub use spec::{Cmp, Constraint, Examples, LanguageSource, LanguageSpec, Relation, Template};

use compile::{
    check_word, compile as compile_constraint, eval, find_non_regular, grammar_accepts,
    nfa_from_fa, parse_grammar, parse_regex, validate, validate_alphabet,
};
use dfa::Dfa;

/// Knobs of [`design`]; every field has a default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesignOptions {
    /// Longest string length of the brute-force cross-check (default 8).
    pub check_len: usize,
    /// Upper bound on strings cross-checked; the length shrinks to fit (default 200 000).
    pub max_check_strings: usize,
    /// Longest accepted string listed in `Design::accepted` (default 4).
    pub accepted_max_len: usize,
    /// Most accepted strings listed, shortest first (default 20).
    pub accepted_limit: usize,
    /// Layout of the states (default circle).
    pub layout: Algorithm,
    /// Keep the non-accepting sink of a complete DFA (default true).
    pub include_trap_state: bool,
    /// Largest automaton built before giving up (default 20 000 states).
    pub max_states: usize,
    /// Configurations explored per string when simulating a PDA (default 20 000).
    pub step_budget: usize,
    /// Skip the equivalent regular expression above this many states (default 24).
    pub max_regex_states: usize,
}

impl Default for DesignOptions {
    fn default() -> Self {
        DesignOptions {
            check_len: 8,
            max_check_strings: 200_000,
            accepted_max_len: 4,
            accepted_limit: 20,
            layout: Algorithm::Circle,
            include_trap_state: true,
            max_states: 20_000,
            step_budget: 20_000,
            max_regex_states: 24,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignKind {
    Dfa,
    Pda,
}

/// How a designed PDA accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PdaAcceptance {
    FinalState,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignAutomaton {
    Fa(Fa),
    Pda(Pda),
}

/// What a state stands for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StateInfo {
    pub name: String,
    /// Human-readable meaning (also stored in `State::label`).
    pub meaning: String,
    /// Shortest string leading to the state (DFAs only).
    pub access_word: Option<String>,
    pub initial: bool,
    pub accepting: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Stats {
    pub states: usize,
    pub transitions: usize,
    pub final_states: usize,
    pub alphabet: Vec<String>,
    /// States of the automaton before minimization (DFAs only).
    pub states_before_minimization: Option<usize>,
    pub has_trap_state: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExampleResult {
    pub word: String,
    pub expected_accept: bool,
    /// What the automaton did; `None` if a PDA search ran out of budget.
    pub actual: Option<bool>,
    pub passed: bool,
    /// Why, when the example failed: what the specification itself says.
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mismatch {
    pub word: String,
    pub expected: bool,
    pub actual: bool,
}

/// Automaton against the semantics of the spec on every string up to `max_len`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CrossCheck {
    pub requested_len: usize,
    /// Length actually covered (smaller when the alphabet is large).
    pub max_len: usize,
    pub strings_checked: usize,
    /// PDA searches that ran out of budget.
    pub inconclusive: usize,
    pub mismatch_count: usize,
    /// First mismatches, shortest first (at most 20).
    pub mismatches: Vec<Mismatch>,
    /// Whether the derived regular expression agrees too (regular languages only).
    pub regex_agrees: Option<bool>,
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Verification {
    pub examples: Vec<ExampleResult>,
    pub failed_examples: usize,
    pub cross_check: CrossCheck,
    /// Every example passed and the cross-check found no mismatch.
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Design {
    pub kind: DesignKind,
    pub automaton: DesignAutomaton,
    pub acceptance: Option<PdaAcceptance>,
    /// The language in words, normalized.
    pub description: String,
    /// Equivalent regular expression (regular languages only).
    pub regex: Option<String>,
    pub states: Vec<StateInfo>,
    pub stats: Stats,
    /// Accepted strings in shortlex order.
    pub accepted: Vec<String>,
    pub verification: Verification,
}

type Predicate<'a> = Box<dyn Fn(&[char]) -> bool + 'a>;
type Explain<'a> = Box<dyn Fn(&[char]) -> String + 'a>;

/// Builds, minimizes, lays out and verifies the automaton for `spec`.
pub fn design(spec: &LanguageSpec, options: &DesignOptions) -> Result<Design, DesignError> {
    let alphabet = validate_alphabet(&spec.alphabet)?;
    let context = |kind: &str, i: usize, w: &str| format!("examples.{kind}[{i}] = {w:?}");
    let built = match &spec.language {
        LanguageSource::Regex(pattern) => regular_from_regex(pattern, &alphabet, options)?,
        LanguageSource::RegularGrammar(text) => regular_from_grammar(text, &alphabet, options)?,
        LanguageSource::ConstraintText(text) => {
            let c = Constraint::parse(text).map_err(DesignError::ConstraintSyntax)?;
            regular_from_constraint(&c, &alphabet, options)?
        }
        LanguageSource::Constraint(c) => regular_from_constraint(c, &alphabet, options)?,
        LanguageSource::Template(t) => {
            let template = template::build(t, &alphabet)?;
            Built::Pda(template)
        }
    };
    for (kind, words) in [
        ("accept", &spec.examples.accept),
        ("reject", &spec.examples.reject),
    ] {
        for (i, w) in words.iter().enumerate() {
            check_word(w, &alphabet, &context(kind, i, w))?;
        }
    }
    match built {
        Built::Regular(r) => finish_regular(spec, &alphabet, options, r),
        Built::Pda(t) => finish_pda(spec, &alphabet, options, t),
    }
}

enum Built<'a> {
    Regular(Regular<'a>),
    Pda(template::Built),
}

struct Regular<'a> {
    raw: Dfa,
    description: String,
    accepts: Predicate<'a>,
    /// Explains why the spec accepts or rejects a word.
    detail: Explain<'a>,
}

fn regular_from_constraint<'a>(
    c: &Constraint,
    alphabet: &[char],
    options: &DesignOptions,
) -> Result<Built<'a>, DesignError> {
    let mut regexes = BTreeMap::new();
    validate(c, alphabet, &mut regexes)?;
    if let Some(report) = find_non_regular(c) {
        return Err(DesignError::NotRegular(report));
    }
    let raw = compile_constraint(c, alphabet, &regexes, options.max_states)?;
    let (semantic, explained) = (c.clone(), c.clone());
    let (regexes_a, regexes_b) = (regexes.clone(), regexes);
    Ok(Built::Regular(Regular {
        raw,
        description: c.to_string(),
        accepts: Box::new(move |w| eval(&semantic, w, &regexes_a)),
        detail: Box::new(move |w| constraint_detail(&explained, w, &regexes_b)),
    }))
}

fn constraint_detail(c: &Constraint, word: &[char], regexes: &BTreeMap<String, Regex>) -> String {
    let verdict = |x: &Constraint| {
        format!(
            "`{x}` is {}",
            if eval(x, word, regexes) {
                "true"
            } else {
                "false"
            }
        )
    };
    let whole = if eval(c, word, regexes) {
        "the constraint accepts this string"
    } else {
        "the constraint rejects this string"
    };
    match c {
        Constraint::And { constraints }
        | Constraint::Or { constraints }
        | Constraint::Xor { constraints } => {
            let parts: Vec<String> = constraints.iter().map(verdict).collect();
            format!("{whole}: {}", parts.join("; "))
        }
        _ => whole.to_string(),
    }
}

fn regular_from_regex<'a>(
    pattern: &str,
    alphabet: &[char],
    options: &DesignOptions,
) -> Result<Built<'a>, DesignError> {
    let re = parse_regex(pattern, alphabet, &format!("regex {pattern:?}"))?;
    let nfa = nfa_from_fa(&crate::fa_convert::regex_to_nfa(&re), alphabet);
    let raw = nfa.to_dfa(alphabet, options.max_states)?;
    let shown = re.to_string();
    Ok(Built::Regular(Regular {
        raw,
        description: format!("strings matching {shown}"),
        accepts: Box::new({
            let re = re.clone();
            move |w| re.matches(&w.iter().collect::<String>())
        }),
        detail: Box::new(move |w| {
            let m = re.matches(&w.iter().collect::<String>());
            format!(
                "the regular expression {shown} {} this string",
                if m { "matches" } else { "does not match" }
            )
        }),
    }))
}

fn regular_from_grammar<'a>(
    text: &str,
    alphabet: &[char],
    options: &DesignOptions,
) -> Result<Built<'a>, DesignError> {
    let lang = parse_grammar(text, alphabet)?;
    let raw = lang.nfa.to_dfa(alphabet, options.max_states)?;
    let grammar = lang.grammar;
    let shown = grammar.to_text();
    Ok(Built::Regular(Regular {
        raw,
        description: format!("language of the regular grammar:\n{shown}"),
        accepts: Box::new({
            let g = grammar.clone();
            move |w| grammar_accepts(&g, w)
        }),
        detail: Box::new(move |w| {
            format!(
                "the grammar {} this string",
                if grammar_accepts(&grammar, w) {
                    "derives"
                } else {
                    "does not derive"
                }
            )
        }),
    }))
}

// ---------------------------------------------------------------------------------------
// Words

/// Calls `f` with every string over `alphabet` of length `0..=max_len`, shortest first and
/// in alphabet order within a length; stops when `f` returns false.
fn for_each_word(alphabet: &[char], max_len: usize, mut f: impl FnMut(&[char]) -> bool) {
    for len in 0..=max_len {
        let mut digits = vec![0usize; len];
        loop {
            let word: Vec<char> = digits.iter().map(|&d| alphabet[d]).collect();
            if !f(&word) {
                return;
            }
            // Odometer step; running out of digits ends this length.
            let mut i = len;
            let mut carry = true;
            while carry && i > 0 {
                i -= 1;
                digits[i] += 1;
                if digits[i] < alphabet.len() {
                    carry = false;
                } else {
                    digits[i] = 0;
                }
            }
            if carry {
                break;
            }
        }
    }
}

/// Longest length `<= wanted` whose strings (all shorter ones included) number at most `cap`.
fn affordable_len(alphabet_size: usize, wanted: usize, cap: usize) -> usize {
    let mut total = 1usize;
    let mut level = 1usize;
    for len in 1..=wanted {
        level = level.saturating_mul(alphabet_size);
        total = total.saturating_add(level);
        if total > cap {
            return len - 1;
        }
    }
    wanted
}

enum Machine<'a> {
    Fa(&'a Fa),
    Pda(&'a Pda),
}

impl Machine<'_> {
    /// `None` when the search ran out of budget.
    fn accepts(&self, word: &str, budget: usize) -> Option<bool> {
        match self {
            Machine::Fa(fa) => decided(&fast_run(&FaStepByState::new(fa), word, budget)),
            Machine::Pda(pda) => decided(&fast_run(
                &PdaStepByState::new(pda, Acceptance::FinalState),
                word,
                budget,
            )),
        }
    }
}

fn decided<C>(outcome: &RunOutcome<C>) -> Option<bool> {
    match outcome {
        RunOutcome::Accepted { .. } => Some(true),
        RunOutcome::Rejected => Some(false),
        RunOutcome::Undecided => None,
    }
}

struct Checks<'a> {
    alphabet: &'a [char],
    options: &'a DesignOptions,
    machine: Machine<'a>,
    accepts: &'a dyn Fn(&[char]) -> bool,
    detail: &'a dyn Fn(&[char]) -> String,
    regex: Option<&'a Regex>,
}

impl Checks<'_> {
    fn verify(&self, spec: &LanguageSpec) -> Verification {
        let budget = self.options.step_budget;
        let mut examples = Vec::new();
        for (expected, words) in [
            (true, &spec.examples.accept),
            (false, &spec.examples.reject),
        ] {
            for word in words {
                let actual = self.machine.accepts(word, budget);
                let passed = actual == Some(expected);
                let chars: Vec<char> = word.chars().collect();
                examples.push(ExampleResult {
                    word: word.clone(),
                    expected_accept: expected,
                    actual,
                    passed,
                    detail: (!passed).then(|| (self.detail)(&chars)),
                });
            }
        }
        let failed_examples = examples.iter().filter(|e| !e.passed).count();
        let cross_check = self.cross_check();
        Verification {
            ok: failed_examples == 0 && cross_check.ok,
            examples,
            failed_examples,
            cross_check,
        }
    }

    fn cross_check(&self) -> CrossCheck {
        let max_len = affordable_len(
            self.alphabet.len(),
            self.options.check_len,
            self.options.max_check_strings,
        );
        let (mut checked, mut inconclusive, mut mismatch_count) = (0usize, 0usize, 0usize);
        let mut mismatches = Vec::new();
        let mut regex_agrees = self.regex.map(|_| true);
        for_each_word(self.alphabet, max_len, |word| {
            let text: String = word.iter().collect();
            let expected = (self.accepts)(word);
            checked += 1;
            match self.machine.accepts(&text, self.options.step_budget) {
                None => inconclusive += 1,
                Some(actual) if actual != expected => {
                    mismatch_count += 1;
                    if mismatches.len() < 20 {
                        mismatches.push(Mismatch {
                            word: text.clone(),
                            expected,
                            actual,
                        });
                    }
                }
                Some(_) => {}
            }
            if let Some(re) = self.regex
                && re.matches(&text) != expected
            {
                regex_agrees = Some(false);
            }
            true
        });
        CrossCheck {
            requested_len: self.options.check_len,
            max_len,
            strings_checked: checked,
            inconclusive,
            mismatch_count,
            ok: mismatch_count == 0 && inconclusive == 0 && regex_agrees != Some(false),
            mismatches,
            regex_agrees,
        }
    }

    fn accepted(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.options.accepted_limit == 0 {
            return out;
        }
        for_each_word(self.alphabet, self.options.accepted_max_len, |word| {
            let text: String = word.iter().collect();
            if self.machine.accepts(&text, self.options.step_budget) == Some(true) {
                out.push(text);
            }
            out.len() < self.options.accepted_limit
        });
        out
    }
}

// ---------------------------------------------------------------------------------------
// Regular languages

fn is_trap(d: &Dfa, s: usize) -> bool {
    s != d.start && !d.accept[s] && d.is_sink(s)
}

fn meaning(d: &Dfa, s: usize, access: &str) -> String {
    let parts = d.parts[s].join(", ");
    if is_trap(d, s) || (d.is_sink(s) && !d.accept[s]) {
        return if parts.is_empty() {
            "trap state: nothing that follows can be accepted".to_string()
        } else {
            format!("{parts}; trap state, nothing that follows can be accepted")
        };
    }
    if d.is_sink(s) && d.accept[s] {
        return if parts.is_empty() {
            "accepted whatever follows".to_string()
        } else {
            format!("{parts}; accepted whatever follows")
        };
    }
    if !parts.is_empty() {
        parts
    } else if access.is_empty() {
        "start: nothing read yet".to_string()
    } else {
        format!("reached by \"{access}\"")
    }
}

/// The DFA as a `Fa` with states `q0..` in the DFA's (breadth-first) order. Returns the
/// automaton and the DFA state behind each of its state ids.
fn dfa_to_fa(d: &Dfa, include_trap: bool, with_meaning: bool) -> (Fa, Vec<usize>) {
    let access = d.access_words();
    let mut fa = Fa::new();
    let mut ids: Vec<Option<StateId>> = vec![None; d.len()];
    let mut kept = Vec::new();
    for s in 0..d.len() {
        if !include_trap && is_trap(d, s) {
            continue;
        }
        let id = fa.create_state(Point::default());
        ids[s] = Some(id);
        kept.push(s);
        if let Some(state) = fa.state_mut(id) {
            state.name = format!("q{id}");
            if with_meaning {
                state.label = Some(meaning(d, s, &access[s]));
            }
        }
        fa.set_final(id, d.accept[s]);
    }
    fa.set_initial(ids[d.start]);
    for &s in &kept {
        for (c, &t) in d.next[s].iter().enumerate() {
            if let (Some(from), Some(to)) = (ids[s], ids[t]) {
                fa.add_transition(Transition::new(
                    from,
                    to,
                    FaLabel(d.alphabet[c].to_string()),
                ));
            }
        }
    }
    (fa, kept)
}

fn layout_options(algorithm: Algorithm) -> (Algorithm, LayoutOptions) {
    (
        algorithm,
        LayoutOptions {
            vertex_buffer: 50.0,
            ..LayoutOptions::default()
        },
    )
}

fn finish_regular(
    spec: &LanguageSpec,
    alphabet: &[char],
    options: &DesignOptions,
    r: Regular<'_>,
) -> Result<Design, DesignError> {
    let before = r.raw.len();
    let minimal = r.raw.minimize();
    let (mut fa, kept) = dfa_to_fa(&minimal, options.include_trap_state, true);
    let (algorithm, layout_opts) = layout_options(options.layout);
    layout::apply(&mut fa, algorithm, &layout_opts);

    let access = minimal.access_words();
    let states: Vec<StateInfo> = kept
        .iter()
        .enumerate()
        .map(|(id, &s)| StateInfo {
            name: format!("q{id}"),
            meaning: meaning(&minimal, s, &access[s]),
            access_word: Some(access[s].clone()),
            initial: s == minimal.start,
            accepting: minimal.accept[s],
        })
        .collect();

    // The regular expression is derived from the trap-free automaton.
    let (compact, _) = dfa_to_fa(&minimal, false, false);
    let regex_safe = !alphabet
        .iter()
        .any(|c| compile::REGEX_OPERATORS.contains(*c));
    let regex: Option<Regex> = (regex_safe && compact.states().count() <= options.max_regex_states)
        .then(|| fa_to_regex(&compact))
        .filter(|_| unreachable_states(&compact).is_empty());

    let checks = Checks {
        alphabet,
        options,
        machine: Machine::Fa(&fa),
        accepts: &*r.accepts,
        detail: &*r.detail,
        regex: regex.as_ref(),
    };
    let verification = checks.verify(spec);
    let accepted = checks.accepted();
    let transitions = fa.transitions().len();
    let final_states = fa.finals().len();
    let has_trap_state =
        (0..minimal.len()).any(|s| is_trap(&minimal, s)) && options.include_trap_state;
    Ok(Design {
        kind: DesignKind::Dfa,
        stats: Stats {
            states: states.len(),
            transitions,
            final_states,
            alphabet: alphabet.iter().map(char::to_string).collect(),
            states_before_minimization: Some(before),
            has_trap_state,
        },
        automaton: DesignAutomaton::Fa(fa),
        acceptance: None,
        description: r.description,
        regex: regex.map(|re| re.to_string()),
        states,
        accepted,
        verification,
    })
}

// ---------------------------------------------------------------------------------------
// Pushdown automata

/// Copy of `a` whose states are numbered in breadth-first order from the initial state
/// (transitions taken in insertion order); every state is kept.
fn renumber<L: Label>(a: &Automaton<L>) -> Automaton<L> {
    let mut order: Vec<StateId> = a.initial().into_iter().collect();
    let mut i = 0;
    while i < order.len() {
        for t in a.transitions_from(order[i]) {
            if !order.contains(&t.to) {
                order.push(t.to);
            }
        }
        i += 1;
    }
    for s in a.states() {
        if !order.contains(&s.id) {
            order.push(s.id);
        }
    }
    let new_id: BTreeMap<StateId, StateId> = order
        .iter()
        .enumerate()
        .map(|(i, &old)| (old, i as StateId))
        .collect();
    let mut out = Automaton::new();
    for &old in &order {
        let id = out.create_state(a.state(old).map_or_else(Point::default, |s| s.point));
        if let (Some(src), Some(dst)) = (a.state(old), out.state_mut(id)) {
            dst.label = src.label.clone();
            dst.name = format!("q{id}");
        }
        out.set_final(id, a.is_final(old));
    }
    out.set_initial(a.initial().map(|s| new_id[&s]));
    for t in a.transitions() {
        out.add_transition(Transition {
            from: new_id[&t.from],
            to: new_id[&t.to],
            label: t.label.clone(),
            control: t.control,
        });
    }
    out
}

fn finish_pda(
    spec: &LanguageSpec,
    alphabet: &[char],
    options: &DesignOptions,
    t: template::Built,
) -> Result<Design, DesignError> {
    let mut pda = renumber(&t.pda);
    layout::apply(
        &mut pda,
        options.layout,
        &LayoutOptions {
            vertex_buffer: 80.0,
            ..LayoutOptions::default()
        },
    );
    let accepts = &*t.accepts;
    let detail_fn = {
        let describe = t.description.clone();
        move |w: &[char]| {
            format!(
                "the template ({describe}) {} this string",
                if accepts(w) { "accepts" } else { "rejects" }
            )
        }
    };
    let checks = Checks {
        alphabet,
        options,
        machine: Machine::Pda(&pda),
        accepts,
        detail: &detail_fn,
        regex: None,
    };
    let verification = checks.verify(spec);
    let accepted = checks.accepted();
    let states: Vec<StateInfo> = pda
        .states()
        .map(|s| StateInfo {
            name: s.name.clone(),
            meaning: s.label.clone().unwrap_or_default(),
            access_word: None,
            initial: pda.initial() == Some(s.id),
            accepting: pda.is_final(s.id),
        })
        .collect();
    Ok(Design {
        kind: DesignKind::Pda,
        stats: Stats {
            states: states.len(),
            transitions: pda.transitions().len(),
            final_states: pda.finals().len(),
            alphabet: alphabet.iter().map(char::to_string).collect(),
            states_before_minimization: None,
            has_trap_state: false,
        },
        automaton: DesignAutomaton::Pda(pda),
        acceptance: Some(PdaAcceptance::FinalState),
        description: t.description,
        regex: None,
        states,
        accepted,
        verification,
    })
}
