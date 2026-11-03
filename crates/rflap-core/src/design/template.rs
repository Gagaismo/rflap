//! PDAs (accepting by final state) for the non-regular templates, with the language each one
//! stands for written as an independent predicate so the PDA can be checked against it.

use crate::automaton::{Point, StateId, Transition};
use crate::pda::{INITIAL_STACK, Pda, PdaLabel};

use super::error::DesignError;
use super::spec::{Relation, Template};

pub(crate) type Predicate = Box<dyn Fn(&[char]) -> bool>;

/// A template turned into an automaton.
pub(crate) struct Built {
    pub pda: Pda,
    pub accepts: Predicate,
    pub description: String,
}

struct Builder {
    pda: Pda,
}

impl Builder {
    fn new() -> Self {
        Builder { pda: Pda::new() }
    }

    fn state(&mut self, meaning: &str, is_final: bool) -> StateId {
        let id = self.pda.create_state(Point::default());
        if let Some(s) = self.pda.state_mut(id) {
            s.label = Some(meaning.to_string());
        }
        self.pda.set_final(id, is_final);
        if id == 0 {
            self.pda.set_initial(Some(0));
        }
        id
    }

    fn edge(&mut self, from: StateId, to: StateId, input: &str, pop: &str, push: &str) {
        self.pda
            .add_transition(Transition::new(from, to, PdaLabel::new(input, pop, push)));
    }
}

fn invalid(template: &Template, message: String) -> DesignError {
    DesignError::InvalidParameter {
        context: format!("template {}", template.name()),
        message,
    }
}

fn require_in_alphabet(
    template: &Template,
    alphabet: &[char],
    symbols: &[char],
) -> Result<(), DesignError> {
    for (position, &symbol) in symbols.iter().enumerate() {
        if !alphabet.contains(&symbol) {
            let list: Vec<String> = alphabet.iter().map(char::to_string).collect();
            return Err(DesignError::AlphabetViolation {
                context: format!("template {}", template.name()),
                symbol,
                position,
                message: format!(
                    "symbol '{symbol}' of template {} is not in the alphabet {{{}}}",
                    template.name(),
                    list.join(", ")
                ),
            });
        }
    }
    Ok(())
}

fn require_distinct(template: &Template, symbols: &[char]) -> Result<(), DesignError> {
    for (i, s) in symbols.iter().enumerate() {
        if symbols[..i].contains(s) {
            return Err(invalid(
                template,
                format!("the symbol '{s}' is used for two different roles; they must be distinct"),
            ));
        }
    }
    Ok(())
}

fn reserved_stack_symbol(template: &Template, alphabet: &[char]) -> Result<(), DesignError> {
    let bottom = INITIAL_STACK.chars().next().unwrap_or('Z');
    if alphabet.contains(&bottom) {
        return Err(invalid(
            template,
            format!(
                "the alphabet contains '{bottom}', which this template's PDA uses as the \
                 bottom-of-stack marker; rename that symbol"
            ),
        ));
    }
    Ok(())
}

pub(crate) fn build(template: &Template, alphabet: &[char]) -> Result<Built, DesignError> {
    let (pda, accepts, description): (Pda, Predicate, String) = match template {
        Template::AnBn { a, b, min } => {
            require_in_alphabet(template, alphabet, &[*a, *b])?;
            require_distinct(template, &[*a, *b])?;
            if *min > 1 {
                return Err(invalid(template, "`min` must be 0 or 1".to_string()));
            }
            let (a, b, min) = (*a, *b, *min);
            (
                an_bn(a, b, min),
                Box::new(move |w: &[char]| {
                    let n = w.iter().take_while(|c| **c == a).count();
                    n >= min && w.len() == 2 * n && w[n..].iter().all(|c| *c == b)
                }) as Predicate,
                format!("{a}^n {b}^n with n >= {min}"),
            )
        }
        Template::AnBm { a, b, relation } => {
            require_in_alphabet(template, alphabet, &[*a, *b])?;
            require_distinct(template, &[*a, *b])?;
            let (a, b, relation) = (*a, *b, *relation);
            let sign = match relation {
                Relation::Le => "<=",
                Relation::Ge => ">=",
                Relation::Ne => "!=",
            };
            (
                an_bm(a, b, relation),
                Box::new(move |w: &[char]| {
                    let n = w.iter().take_while(|c| **c == a).count();
                    let m = w.len() - n;
                    if !w[n..].iter().all(|c| *c == b) {
                        return false;
                    }
                    match relation {
                        Relation::Le => n <= m,
                        Relation::Ge => n >= m,
                        Relation::Ne => n != m,
                    }
                }),
                format!("{a}^n {b}^m with n {sign} m"),
            )
        }
        Template::EqualCount { a, b } => {
            require_in_alphabet(template, alphabet, &[*a, *b])?;
            require_distinct(template, &[*a, *b])?;
            let (a, b) = (*a, *b);
            (
                equal_count(a, b),
                Box::new(move |w: &[char]| {
                    w.iter().all(|c| *c == a || *c == b)
                        && w.iter().filter(|c| **c == a).count()
                            == w.iter().filter(|c| **c == b).count()
                }),
                format!("as many {a} as {b}"),
            )
        }
        Template::BalancedBrackets { pairs } => {
            if pairs.is_empty() {
                return Err(invalid(
                    template,
                    "`pairs` needs at least one pair".to_string(),
                ));
            }
            let flat: Vec<char> = pairs.iter().flatten().copied().collect();
            require_in_alphabet(template, alphabet, &flat)?;
            require_distinct(template, &flat)?;
            reserved_stack_symbol(template, &flat)?;
            let pairs = pairs.clone();
            let text: Vec<String> = pairs.iter().map(|[o, c]| format!("{o}{c}")).collect();
            (
                balanced(&pairs),
                Box::new({
                    let pairs = pairs.clone();
                    move |w: &[char]| {
                        let mut stack: Vec<char> = Vec::new();
                        for c in w {
                            if let Some([o, _]) = pairs.iter().find(|p| p[0] == *c) {
                                stack.push(*o);
                            } else if let Some([o, _]) = pairs.iter().find(|p| p[1] == *c) {
                                if stack.pop() != Some(*o) {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                        stack.is_empty()
                    }
                }),
                format!("balanced brackets {}", text.join(" ")),
            )
        }
        Template::WcWr { c } => {
            require_in_alphabet(template, alphabet, &[*c])?;
            reserved_stack_symbol(template, alphabet)?;
            let c = *c;
            let others: Vec<char> = alphabet.iter().copied().filter(|x| *x != c).collect();
            (
                wcwr(c, &others),
                Box::new(move |w: &[char]| {
                    let mid = w.len() / 2;
                    w.len() % 2 == 1
                        && w[mid] == c
                        && !w[..mid].contains(&c)
                        && w[..mid].iter().eq(w[mid + 1..].iter().rev())
                }),
                format!("w {c} w^R with w over the alphabet without {c}"),
            )
        }
        Template::EvenPalindrome => {
            reserved_stack_symbol(template, alphabet)?;
            (
                palindrome(alphabet, false),
                Box::new(|w: &[char]| w.len().is_multiple_of(2) && w.iter().eq(w.iter().rev())),
                "even-length palindromes w w^R".to_string(),
            )
        }
        Template::Palindrome => {
            reserved_stack_symbol(template, alphabet)?;
            (
                palindrome(alphabet, true),
                Box::new(|w: &[char]| w.iter().eq(w.iter().rev())),
                "palindromes".to_string(),
            )
        }
    };
    Ok(Built {
        pda,
        accepts,
        description,
    })
}

fn s(c: char) -> String {
    c.to_string()
}

fn an_bn(a: char, b: char, min: usize) -> Pda {
    let mut t = Builder::new();
    let reading_a = t.state(&format!("reading {a}s, one A pushed per {a}"), false);
    let reading_b = t.state(&format!("reading {b}s, one A popped per {b}"), false);
    let accept = t.state(&format!("accept: as many {b}s as {a}s"), true);
    t.edge(reading_a, reading_a, &s(a), "", "A");
    t.edge(reading_a, reading_b, &s(b), "A", "");
    t.edge(reading_b, reading_b, &s(b), "A", "");
    t.edge(reading_b, accept, "", INITIAL_STACK, INITIAL_STACK);
    if min == 0 {
        t.edge(reading_a, accept, "", INITIAL_STACK, INITIAL_STACK);
    }
    t.pda
}

fn an_bm(a: char, b: char, relation: Relation) -> Pda {
    let mut t = Builder::new();
    let reading_a = t.state(&format!("reading {a}s, one A pushed per {a}"), false);
    let reading_b = t.state(
        &format!("reading {b}s, one A popped per {b}"),
        relation == Relation::Ge,
    );
    t.edge(reading_a, reading_a, &s(a), "", "A");
    t.edge(reading_a, reading_b, "", "", "");
    t.edge(reading_b, reading_b, &s(b), "A", "");
    match relation {
        Relation::Ge => {}
        Relation::Le => {
            let extra = t.state(&format!("all {a}s matched, further {b}s are free"), true);
            t.edge(reading_b, extra, "", INITIAL_STACK, INITIAL_STACK);
            t.edge(extra, extra, &s(b), INITIAL_STACK, INITIAL_STACK);
        }
        Relation::Ne => {
            let more_a = t.state(&format!("accept: unmatched {a}s remain on the stack"), true);
            let more_b = t.state(&format!("accept: more {b}s than {a}s"), true);
            t.edge(reading_b, more_a, "", "A", "A");
            t.edge(reading_b, more_b, &s(b), INITIAL_STACK, INITIAL_STACK);
            t.edge(more_b, more_b, &s(b), INITIAL_STACK, INITIAL_STACK);
        }
    }
    t.pda
}

fn equal_count(a: char, b: char) -> Pda {
    let mut t = Builder::new();
    let counting = t.state(
        &format!("counting: the stack holds the excess of {a} (X) or of {b} (Y)"),
        false,
    );
    let accept = t.state(&format!("accept: as many {a} as {b}"), true);
    for (this, other, mine, theirs) in [(a, b, "X", "Y"), (b, a, "Y", "X")] {
        let _ = other;
        t.edge(
            counting,
            counting,
            &s(this),
            INITIAL_STACK,
            &format!("{mine}{INITIAL_STACK}"),
        );
        t.edge(counting, counting, &s(this), mine, &format!("{mine}{mine}"));
        t.edge(counting, counting, &s(this), theirs, "");
    }
    t.edge(counting, accept, "", INITIAL_STACK, INITIAL_STACK);
    t.pda
}

fn balanced(pairs: &[[char; 2]]) -> Pda {
    let mut t = Builder::new();
    let reading = t.state("reading: the stack holds the brackets still open", false);
    let accept = t.state("accept: every bracket is closed", true);
    let mut tops: Vec<String> = vec![INITIAL_STACK.to_string()];
    tops.extend(pairs.iter().map(|p| s(p[0])));
    for [open, close] in pairs {
        for top in &tops {
            t.edge(reading, reading, &s(*open), top, &format!("{open}{top}"));
        }
        t.edge(reading, reading, &s(*close), &s(*open), "");
    }
    t.edge(reading, accept, "", INITIAL_STACK, INITIAL_STACK);
    t.pda
}

/// Pushes the symbols of `w` (any of `symbols`) onto the stack.
fn push_loop(t: &mut Builder, state: StateId, symbols: &[char]) {
    for x in symbols {
        t.edge(
            state,
            state,
            &s(*x),
            INITIAL_STACK,
            &format!("{x}{INITIAL_STACK}"),
        );
        for y in symbols {
            t.edge(state, state, &s(*x), &s(*y), &format!("{x}{y}"));
        }
    }
}

fn wcwr(c: char, others: &[char]) -> Pda {
    let mut t = Builder::new();
    let pushing = t.state("reading w: its symbols are pushed", false);
    let matching = t.state(
        &format!("after {c}: each symbol must match the top of the stack"),
        false,
    );
    let accept = t.state("accept: w^R matched w", true);
    push_loop(&mut t, pushing, others);
    t.edge(pushing, matching, &s(c), "", "");
    for x in others {
        t.edge(matching, matching, &s(*x), &s(*x), "");
    }
    t.edge(matching, accept, "", INITIAL_STACK, INITIAL_STACK);
    t.pda
}

fn palindrome(alphabet: &[char], odd_too: bool) -> Pda {
    let mut t = Builder::new();
    let pushing = t.state("first half: its symbols are pushed", false);
    let matching = t.state(
        "second half: each symbol must match the top of the stack",
        false,
    );
    let accept = t.state("accept: the second half mirrors the first", true);
    push_loop(&mut t, pushing, alphabet);
    t.edge(pushing, matching, "", "", "");
    if odd_too {
        for x in alphabet {
            t.edge(pushing, matching, &s(*x), "", "");
        }
    }
    for x in alphabet {
        t.edge(matching, matching, &s(*x), &s(*x), "");
    }
    t.edge(matching, accept, "", INITIAL_STACK, INITIAL_STACK);
    t.pda
}
