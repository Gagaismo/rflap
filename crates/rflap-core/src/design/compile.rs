//! Validation of specs and compilation of constraints, regular expressions and regular
//! grammars to DFAs, plus the direct (automaton-free) semantics used to cross-check them.

use std::collections::{BTreeMap, BTreeSet};

use crate::automaton::StateId;
use crate::fa::{Fa, split_multi_char_labels};
use crate::fa_convert::regex_to_nfa;
use crate::grammar::{Grammar, is_variable_symbol};
use crate::regex::Regex;

use super::dfa::{Dfa, Nfa, Subject, compare, modulo, starts_with, suffix_match};
use super::error::{DesignError, NotRegularReport};
use super::spec::{Cmp, Constraint, Template};

/// Largest number accepted in `length`/`count` comparisons and moduli.
pub(crate) const MAX_NUMBER: usize = 5_000;
const MAX_WORD: usize = 256;
/// Never a symbol: they stand for the empty string or the empty language.
const RESERVED: &str = "λεø∅";
/// Operators of the regular expression syntax; only a problem when a regex is involved.
pub(crate) const REGEX_OPERATORS: &str = "()+*!";

pub(crate) fn alphabet_text(alphabet: &[char]) -> String {
    let list: Vec<String> = alphabet.iter().map(char::to_string).collect();
    format!("{{{}}}", list.join(", "))
}

pub(crate) fn validate_alphabet(raw: &[String]) -> Result<Vec<char>, DesignError> {
    let bad = |message: String| DesignError::InvalidAlphabet { message };
    if raw.is_empty() {
        return Err(bad(
            "the alphabet is empty; list at least one symbol".to_string()
        ));
    }
    let mut out: Vec<char> = Vec::new();
    for symbol in raw {
        let mut chars = symbol.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return Err(bad(format!(
                "alphabet symbol {symbol:?} must be exactly one character; \
                 split multi-character tokens into single-character symbols"
            )));
        };
        if c.is_whitespace() {
            return Err(bad(format!(
                "alphabet symbol {symbol:?} is whitespace; list symbols as [\"a\",\"b\"]"
            )));
        }
        if RESERVED.contains(c) {
            return Err(bad(format!(
                "alphabet symbol '{c}' is reserved (one of {RESERVED} stand for the \
                 empty string or the empty language); use another character"
            )));
        }
        if out.contains(&c) {
            return Err(bad(format!("alphabet symbol '{c}' is listed twice")));
        }
        out.push(c);
    }
    Ok(out)
}

/// `Err(AlphabetViolation)` for the first symbol of `word` outside the alphabet.
pub(crate) fn check_word(word: &str, alphabet: &[char], context: &str) -> Result<(), DesignError> {
    for (position, symbol) in word.chars().enumerate() {
        if !alphabet.contains(&symbol) {
            let hint = if matches!(symbol, 'λ' | 'ε') {
                "; write the empty string as \"\""
            } else {
                ""
            };
            return Err(DesignError::AlphabetViolation {
                context: context.to_string(),
                symbol,
                position,
                message: format!(
                    "symbol '{symbol}' (position {position}) in {context} is not in the \
                     alphabet {}{hint}",
                    alphabet_text(alphabet)
                ),
            });
        }
    }
    Ok(())
}

fn invalid(c: &Constraint, message: impl Into<String>) -> DesignError {
    DesignError::InvalidParameter {
        context: c.to_string(),
        message: message.into(),
    }
}

fn regex_symbols(re: &Regex, out: &mut Vec<char>) {
    match re {
        Regex::Empty | Regex::Lambda => {}
        Regex::Symbol(c) => out.push(*c),
        Regex::Union(v) | Regex::Concat(v) => v.iter().for_each(|r| regex_symbols(r, out)),
        Regex::Star(r) => regex_symbols(r, out),
    }
}

/// Parses a regular expression and checks that its symbols belong to the alphabet.
pub(crate) fn parse_regex(
    pattern: &str,
    alphabet: &[char],
    context: &str,
) -> Result<Regex, DesignError> {
    if let Some(op) = alphabet.iter().find(|c| REGEX_OPERATORS.contains(**c)) {
        return Err(DesignError::InvalidAlphabet {
            message: format!(
                "alphabet symbol '{op}' is an operator of the regular expression syntax \
                 ({REGEX_OPERATORS}), so {context} cannot use it; use another symbol, or \
                 describe the language with a constraint or a grammar"
            ),
        });
    }
    let re = Regex::parse(pattern).map_err(|e| DesignError::RegexSyntax {
        position: e.position,
        message: e.message,
    })?;
    let mut symbols = Vec::new();
    regex_symbols(&re, &mut symbols);
    for symbol in symbols {
        if !alphabet.contains(&symbol) {
            return Err(DesignError::AlphabetViolation {
                context: context.to_string(),
                symbol,
                position: pattern.chars().position(|c| c == symbol).unwrap_or(0),
                message: format!(
                    "symbol '{symbol}' in {context} is not in the alphabet {}",
                    alphabet_text(alphabet)
                ),
            });
        }
    }
    Ok(re)
}

fn check_symbols(
    c: &Constraint,
    symbols: &str,
    alphabet: &[char],
) -> Result<Vec<char>, DesignError> {
    if symbols.is_empty() {
        return Err(invalid(c, "the symbol set is empty"));
    }
    check_word(symbols, alphabet, &c.to_string())?;
    Ok(symbols.chars().collect())
}

fn check_number(c: &Constraint, n: usize) -> Result<(), DesignError> {
    if n > MAX_NUMBER {
        return Err(DesignError::TooLarge {
            message: format!("{c}: {n} is above the supported maximum {MAX_NUMBER}"),
        });
    }
    Ok(())
}

fn check_mod(c: &Constraint, m: usize, r: usize) -> Result<(), DesignError> {
    if m == 0 {
        return Err(invalid(c, "the modulus must be at least 1"));
    }
    if r >= m {
        return Err(invalid(
            c,
            format!(
                "the remainder {r} must be less than the modulus {m} (use {})",
                r % m
            ),
        ));
    }
    check_number(c, m)
}

/// Validates alphabet usage, parameters and regular expressions of the whole tree and
/// returns the parsed regular expressions by pattern.
pub(crate) fn validate(
    c: &Constraint,
    alphabet: &[char],
    regexes: &mut BTreeMap<String, Regex>,
) -> Result<(), DesignError> {
    let name = c.to_string();
    match c {
        Constraint::StartsWith { word }
        | Constraint::EndsWith { word }
        | Constraint::Contains { word } => {
            check_word(word, alphabet, &name)?;
            if word.chars().count() > MAX_WORD {
                return Err(DesignError::TooLarge {
                    message: format!("{name}: the word is longer than {MAX_WORD} symbols"),
                });
            }
        }
        Constraint::Length { n, .. } => check_number(c, *n)?,
        Constraint::LengthMod { m, r } => check_mod(c, *m, *r)?,
        Constraint::Count { symbols, n, .. } => {
            check_symbols(c, symbols, alphabet)?;
            check_number(c, *n)?;
        }
        Constraint::CountMod { symbols, m, r } => {
            check_symbols(c, symbols, alphabet)?;
            check_mod(c, *m, *r)?;
        }
        Constraint::CountCompare { left, right, .. } => {
            check_symbols(c, left, alphabet)?;
            check_symbols(c, right, alphabet)?;
        }
        Constraint::Regex { pattern } => {
            let re = parse_regex(pattern, alphabet, &name)?;
            regexes.insert(pattern.clone(), re);
        }
        Constraint::And { constraints }
        | Constraint::Or { constraints }
        | Constraint::Xor { constraints } => {
            if constraints.is_empty() {
                return Err(invalid(c, "needs at least one constraint"));
            }
            for inner in constraints {
                validate(inner, alphabet, regexes)?;
            }
        }
        Constraint::Not { constraint } => validate(constraint, alphabet, regexes)?,
    }
    Ok(())
}

fn symbol_set(symbols: &str) -> String {
    symbols
        .chars()
        .map(String::from)
        .collect::<Vec<_>>()
        .join(",")
}

/// The first condition no finite automaton can check, as a report with the template to use.
pub(crate) fn find_non_regular(c: &Constraint) -> Option<NotRegularReport> {
    match c {
        Constraint::CountCompare { left, cmp, right } => {
            let (a, b) = (symbol_set(left), symbol_set(right));
            let single = left.chars().count() == 1 && right.chars().count() == 1 && left != right;
            let pick = |template: &Template| {
                #[derive(serde::Serialize)]
                struct Language<'a> {
                    template: &'a Template,
                }
                serde_json::to_string(&Language { template }).unwrap_or_default()
            };
            let (template, suggestion, reason) = if single && *cmp == Cmp::Eq {
                let (l, r) = (left.chars().next()?, right.chars().next()?);
                let t = Template::EqualCount { a: l, b: r };
                (
                    Some(t.name().to_string()),
                    format!("Use the `equal_count` template: {}", pick(&t)),
                    format!(
                        "`count({a}) == count({b})` compares two unbounded counters, which a finite \
                         automaton cannot do (pumping lemma); it is a pushdown language."
                    ),
                )
            } else {
                (
                    None,
                    "If the symbols must appear as a^n b^m, use the `an_bm` template \
                     (relation le, ge or ne) or `an_bn`; otherwise compare each count with a \
                     constant (count(a) >= 2) or compare residues (count(a) % 2 == count(b) % 2 \
                     is regular but must be written with `and`/`or` of mod conditions)."
                        .to_string(),
                    format!(
                        "`count({a}) {} count({b})` compares two unbounded counters, which a \
                         finite automaton cannot do; it is not regular.",
                        cmp.symbol()
                    ),
                )
            };
            Some(NotRegularReport {
                constraint: c.to_string(),
                reason,
                template,
                suggestion,
            })
        }
        Constraint::And { constraints }
        | Constraint::Or { constraints }
        | Constraint::Xor { constraints } => constraints.iter().find_map(find_non_regular),
        Constraint::Not { constraint } => find_non_regular(constraint),
        _ => None,
    }
}

/// Direct semantics of a validated, regular-or-not constraint.
pub(crate) fn eval(c: &Constraint, word: &[char], regexes: &BTreeMap<String, Regex>) -> bool {
    let count = |symbols: &str| word.iter().filter(|w| symbols.contains(**w)).count();
    let chars = |w: &str| w.chars().collect::<Vec<_>>();
    match c {
        Constraint::StartsWith { word: w } => word.starts_with(&chars(w)),
        Constraint::EndsWith { word: w } => word.ends_with(&chars(w)),
        Constraint::Contains { word: w } => {
            let w = chars(w);
            w.is_empty() || word.windows(w.len()).any(|win| win == w.as_slice())
        }
        Constraint::Length { cmp, n } => cmp.holds(word.len(), *n),
        Constraint::LengthMod { m, r } => word.len() % m == *r,
        Constraint::Count { symbols, cmp, n } => cmp.holds(count(symbols), *n),
        Constraint::CountMod { symbols, m, r } => count(symbols) % m == *r,
        Constraint::CountCompare { left, cmp, right } => cmp.holds(count(left), count(right)),
        Constraint::Regex { pattern } => regexes
            .get(pattern)
            .is_some_and(|re| re.matches(&word.iter().collect::<String>())),
        Constraint::And { constraints } => constraints.iter().all(|x| eval(x, word, regexes)),
        Constraint::Or { constraints } => constraints.iter().any(|x| eval(x, word, regexes)),
        Constraint::Xor { constraints } => {
            constraints
                .iter()
                .filter(|x| eval(x, word, regexes))
                .count()
                % 2
                == 1
        }
        Constraint::Not { constraint } => !eval(constraint, word, regexes),
    }
}

/// NFA of a finite automaton, keeping only single-symbol and λ moves.
pub(crate) fn nfa_from_fa(fa: &Fa, alphabet: &[char]) -> Nfa {
    let fa = split_multi_char_labels(fa);
    let index: BTreeMap<StateId, usize> = fa.states().enumerate().map(|(i, s)| (s.id, i)).collect();
    let mut edges = Vec::new();
    for t in fa.transitions() {
        let (from, to) = (index[&t.from], index[&t.to]);
        if let Some((lo, hi)) = t.label.range() {
            edges.extend(
                alphabet
                    .iter()
                    .filter(|c| (lo..=hi).contains(*c))
                    .map(|c| (from, Some(*c), to)),
            );
        } else {
            match t.label.0.chars().next() {
                None => edges.push((from, None, to)),
                Some(c) => edges.push((from, Some(c), to)),
            }
        }
    }
    Nfa {
        count: index.len(),
        starts: fa.initial().map(|i| index[&i]).into_iter().collect(),
        accepts: fa.finals().iter().map(|f| index[f]).collect(),
        edges,
    }
}

/// DFA of a validated constraint (every condition must be regular).
pub(crate) fn compile(
    c: &Constraint,
    alphabet: &[char],
    regexes: &BTreeMap<String, Regex>,
    max_states: usize,
) -> Result<Dfa, DesignError> {
    let word = |w: &str| w.chars().collect::<Vec<_>>();
    let set = |s: &str| Subject::Count(s.chars().collect());
    Ok(match c {
        Constraint::StartsWith { word: w } => starts_with(alphabet, &word(w)),
        Constraint::EndsWith { word: w } => suffix_match(alphabet, &word(w), false),
        Constraint::Contains { word: w } => suffix_match(alphabet, &word(w), true),
        Constraint::Length { cmp, n } => compare(alphabet, &Subject::Length, *cmp, *n),
        Constraint::LengthMod { m, r } => modulo(alphabet, &Subject::Length, *m, *r),
        Constraint::Count { symbols, cmp, n } => compare(alphabet, &set(symbols), *cmp, *n),
        Constraint::CountMod { symbols, m, r } => modulo(alphabet, &set(symbols), *m, *r),
        Constraint::CountCompare { .. } => {
            return Err(find_non_regular(c)
                .map_or_else(|| invalid(c, "not regular"), DesignError::NotRegular));
        }
        Constraint::Regex { pattern } => {
            let re = regexes
                .get(pattern)
                .ok_or_else(|| invalid(c, "regular expression was not validated"))?;
            nfa_from_fa(&regex_to_nfa(re), alphabet).to_dfa(alphabet, max_states)?
        }
        Constraint::And { constraints } => {
            fold(constraints, alphabet, regexes, max_states, |a, b| a && b)?
        }
        Constraint::Or { constraints } => {
            fold(constraints, alphabet, regexes, max_states, |a, b| a || b)?
        }
        Constraint::Xor { constraints } => {
            fold(constraints, alphabet, regexes, max_states, |a, b| a != b)?
        }
        Constraint::Not { constraint } => {
            compile(constraint, alphabet, regexes, max_states)?.negate()
        }
    })
}

fn fold(
    constraints: &[Constraint],
    alphabet: &[char],
    regexes: &BTreeMap<String, Regex>,
    max_states: usize,
    op: impl Fn(bool, bool) -> bool + Copy,
) -> Result<Dfa, DesignError> {
    let mut iter = constraints.iter();
    let first = iter.next().ok_or_else(|| DesignError::InvalidParameter {
        context: "constraint".to_string(),
        message: "needs at least one constraint".to_string(),
    })?;
    let mut acc = compile(first, alphabet, regexes, max_states)?;
    for next in iter {
        let dfa = compile(next, alphabet, regexes, max_states)?;
        acc = acc.product(&dfa, op, max_states)?;
    }
    Ok(acc)
}

/// What a grammar contributes: the NFA and a direct membership test.
pub(crate) struct GrammarLanguage {
    pub nfa: Nfa,
    pub grammar: Grammar,
}

fn grammar_error(message: String) -> DesignError {
    DesignError::NotRegularGrammar { message }
}

pub(crate) fn parse_grammar(text: &str, alphabet: &[char]) -> Result<GrammarLanguage, DesignError> {
    let grammar = Grammar::parse(text).map_err(|e| DesignError::GrammarSyntax {
        message: e.to_string(),
    })?;
    if grammar.productions().is_empty() {
        return Err(DesignError::GrammarSyntax {
            message: "the grammar has no productions".to_string(),
        });
    }
    for p in grammar.productions() {
        let terminals: String = p.rhs.chars().filter(|c| !is_variable_symbol(*c)).collect();
        check_word(&terminals, alphabet, &format!("production `{p}`"))?;
    }
    if !grammar.is_regular() {
        let bad = grammar
            .productions()
            .iter()
            .find(|p| !p.is_right_linear() && !p.is_left_linear())
            .or_else(|| grammar.productions().iter().find(|p| !p.is_right_linear()));
        return Err(grammar_error(format!(
            "the grammar is not regular: {}. Use only right-linear productions (A -> aB, A -> a, \
             A -> λ) or only left-linear ones (A -> Ba, A -> a, A -> λ); for context-free \
             languages use a template instead",
            bad.map_or_else(
                || "it mixes right-linear and left-linear productions".to_string(),
                |p| format!("production `{p}` is not linear")
            )
        )));
    }
    let right = grammar.is_right_linear();
    let nfa = grammar_nfa(&grammar, right);
    Ok(GrammarLanguage {
        nfa: if right { nfa } else { nfa.reversed() },
        grammar,
    })
}

/// NFA of a right-linear grammar; for a left-linear one, of the grammar with every right side
/// reversed (the reversed language).
fn grammar_nfa(g: &Grammar, right: bool) -> Nfa {
    let mut vars: Vec<char> = vec![g.start().chars().next().unwrap_or('S')];
    for p in g.productions() {
        for v in p
            .lhs
            .chars()
            .chain(p.rhs.chars().filter(|c| is_variable_symbol(*c)))
        {
            if !vars.contains(&v) {
                vars.push(v);
            }
        }
    }
    let var_state = |v: char| vars.iter().position(|x| *x == v).unwrap_or(0);
    let accept_state = vars.len();
    let mut nfa = Nfa {
        count: vars.len() + 1,
        starts: BTreeSet::from([0]),
        accepts: BTreeSet::from([accept_state]),
        edges: Vec::new(),
    };
    for p in g.productions() {
        let from = var_state(p.lhs.chars().next().unwrap_or('S'));
        let mut body: Vec<char> = p.rhs.chars().collect();
        if !right {
            // A -> B w   becomes   A -> w^R B.
            if body.first().is_some_and(|c| is_variable_symbol(*c)) {
                let v = body.remove(0);
                body.reverse();
                body.push(v);
            } else {
                body.reverse();
            }
        }
        let (target, word): (Option<usize>, &[char]) = match body.last() {
            Some(c) if is_variable_symbol(*c) => (Some(var_state(*c)), &body[..body.len() - 1]),
            _ => (None, &body[..]),
        };
        if word.is_empty() && target.is_none() {
            nfa.accepts.insert(from);
            continue;
        }
        let end = target.unwrap_or(accept_state);
        let mut at = from;
        for (i, c) in word.iter().enumerate() {
            let to = if i + 1 == word.len() {
                end
            } else {
                nfa.count += 1;
                nfa.count - 1
            };
            nfa.edges.push((at, Some(*c), to));
            at = to;
        }
        if word.is_empty() {
            nfa.edges.push((from, None, end));
        }
    }
    nfa
}

/// Direct membership of `word` in a regular grammar: derivable spans, to a fixpoint.
pub(crate) fn grammar_accepts(g: &Grammar, word: &[char]) -> bool {
    let n = word.len();
    let mut derives: BTreeSet<(char, usize, usize)> = BTreeSet::new();
    let right = g.is_right_linear();
    let start = g.start().chars().next().unwrap_or('S');
    loop {
        let before = derives.len();
        for p in g.productions() {
            let Some(lhs) = p.lhs.chars().next() else {
                continue;
            };
            let rhs: Vec<char> = p.rhs.chars().collect();
            for i in 0..=n {
                for j in i..=n {
                    if derives.contains(&(lhs, i, j)) {
                        continue;
                    }
                    if production_derives(&rhs, right, word, i, j, &derives) {
                        derives.insert((lhs, i, j));
                    }
                }
            }
        }
        if derives.len() == before {
            break;
        }
    }
    derives.contains(&(start, 0, n))
}

fn production_derives(
    rhs: &[char],
    right: bool,
    word: &[char],
    i: usize,
    j: usize,
    derives: &BTreeSet<(char, usize, usize)>,
) -> bool {
    let span = &word[i..j];
    let (var, terminals): (Option<char>, &[char]) = match (right, rhs.first(), rhs.last()) {
        (true, _, Some(&v)) if is_variable_symbol(v) => (Some(v), &rhs[..rhs.len() - 1]),
        (false, Some(&v), _) if is_variable_symbol(v) => (Some(v), &rhs[1..]),
        _ => (None, rhs),
    };
    match var {
        None => span == terminals,
        Some(v) if right => {
            span.starts_with(terminals) && derives.contains(&(v, i + terminals.len(), j))
        }
        Some(v) => span.ends_with(terminals) && derives.contains(&(v, i, j - terminals.len())),
    }
}
