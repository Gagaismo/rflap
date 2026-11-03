//! Regular expressions (JFLAP `regular`).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Regular expression AST. `Concat` and `Union` hold at least two operands when built by the parser.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Regex {
    /// The empty language (JFLAP `ø`).
    Empty,
    /// The empty string.
    Lambda,
    Symbol(char),
    Union(Vec<Regex>),
    Concat(Vec<Regex>),
    Star(Box<Regex>),
}

/// A syntax error: `position` is the index (in characters) of the offending character,
/// or the input length when the expression ends too soon.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseError {
    pub position: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at position {})", self.message, self.position)
    }
}

impl std::error::Error for ParseError {}

impl Regex {
    /// Parses JFLAP syntax: `+` is union, `*` is star, juxtaposition is concatenation
    /// (precedence star > concatenation > union), `λ`, `ε` or `!` is the empty string and
    /// `ø` the empty language. Whitespace is ignored.
    pub fn parse(input: &str) -> Result<Regex, ParseError> {
        let tokens: Vec<(usize, char)> = input
            .chars()
            .enumerate()
            .filter(|(_, c)| !c.is_whitespace())
            .collect();
        let mut parser = Parser {
            tokens,
            at: 0,
            end: input.chars().count(),
        };
        let re = parser.union()?;
        match parser.peek() {
            None => Ok(re),
            Some((position, _)) => Err(ParseError {
                position,
                message: "The parentheses are unbalanced!".to_string(),
            }),
        }
    }

    /// Printed like JFLAP with its default empty-string symbol `!`.
    pub fn to_jflap_string(&self) -> String {
        self.render("!")
    }

    /// Whether the whole input is in the language (Brzozowski derivatives).
    pub fn matches(&self, input: &str) -> bool {
        let mut re = self.clone();
        for c in input.chars() {
            re = re.derivative(c);
            if re == Regex::Empty {
                return false;
            }
        }
        re.is_nullable()
    }

    /// Whether the empty string is in the language.
    pub fn is_nullable(&self) -> bool {
        match self {
            Regex::Empty | Regex::Symbol(_) => false,
            Regex::Lambda | Regex::Star(_) => true,
            Regex::Union(v) => v.iter().any(Regex::is_nullable),
            Regex::Concat(v) => v.iter().all(Regex::is_nullable),
        }
    }

    fn derivative(&self, c: char) -> Regex {
        match self {
            Regex::Empty | Regex::Lambda => Regex::Empty,
            Regex::Symbol(s) => {
                if *s == c {
                    Regex::Lambda
                } else {
                    Regex::Empty
                }
            }
            Regex::Union(v) => mk_union(v.iter().map(|r| r.derivative(c)).collect()),
            Regex::Star(r) => mk_concat(vec![r.derivative(c), mk_star(r.as_ref().clone())]),
            Regex::Concat(v) => match v.split_first() {
                None => Regex::Empty,
                Some((first, rest)) => {
                    let rest = mk_concat(rest.to_vec());
                    let head = mk_concat(vec![first.derivative(c), rest.clone()]);
                    if first.is_nullable() {
                        mk_union(vec![head, rest.derivative(c)])
                    } else {
                        head
                    }
                }
            },
        }
    }

    fn render(&self, lambda: &str) -> String {
        let mut out = String::new();
        self.write(&mut out, 0, lambda);
        out
    }

    /// Writes `self`, in parentheses when its binding is weaker than `need`
    /// (union 0, concatenation 1, star 2, atoms 3).
    fn write(&self, out: &mut String, need: u8, lambda: &str) {
        match self {
            Regex::Empty => out.push('ø'),
            Regex::Lambda => out.push_str(lambda),
            Regex::Symbol(c) => out.push(*c),
            Regex::Union(v) if v.is_empty() => out.push('ø'),
            Regex::Concat(v) if v.is_empty() => out.push_str(lambda),
            Regex::Union(v) | Regex::Concat(v) if v.len() == 1 => v[0].write(out, need, lambda),
            Regex::Union(v) => {
                let paren = need > 0;
                if paren {
                    out.push('(');
                }
                for (i, r) in v.iter().enumerate() {
                    if i > 0 {
                        out.push('+');
                    }
                    r.write(out, 1, lambda);
                }
                if paren {
                    out.push(')');
                }
            }
            Regex::Concat(v) => {
                let paren = need > 1;
                if paren {
                    out.push('(');
                }
                for r in v {
                    if matches!(r, Regex::Lambda) {
                        out.push('(');
                        r.write(out, 0, lambda);
                        out.push(')');
                    } else {
                        r.write(out, 2, lambda);
                    }
                }
                if paren {
                    out.push(')');
                }
            }
            Regex::Star(r) => {
                r.write(out, 2, lambda);
                out.push('*');
            }
        }
    }
}

impl fmt::Display for Regex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render("λ"))
    }
}

impl std::str::FromStr for Regex {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        Regex::parse(s)
    }
}

/// Union with nested unions flattened, `ø` dropped, operands sorted and deduplicated.
fn mk_union(items: Vec<Regex>) -> Regex {
    let mut flat = Vec::new();
    for r in items {
        match r {
            Regex::Empty => {}
            Regex::Union(v) => flat.extend(v),
            r => flat.push(r),
        }
    }
    flat.sort();
    flat.dedup();
    match flat.len() {
        0 => Regex::Empty,
        1 => flat.remove(0),
        _ => Regex::Union(flat),
    }
}

/// Concatenation with nested ones flattened, `λ` dropped and `ø` absorbing.
fn mk_concat(items: Vec<Regex>) -> Regex {
    let mut flat = Vec::new();
    for r in items {
        match r {
            Regex::Empty => return Regex::Empty,
            Regex::Lambda => {}
            Regex::Concat(v) => flat.extend(v),
            r => flat.push(r),
        }
    }
    match flat.len() {
        0 => Regex::Lambda,
        1 => flat.remove(0),
        _ => Regex::Concat(flat),
    }
}

fn mk_star(r: Regex) -> Regex {
    match r {
        Regex::Empty | Regex::Lambda => Regex::Lambda,
        Regex::Star(_) => r,
        r => Regex::Star(Box::new(r)),
    }
}

struct Parser {
    /// Non-blank characters with their index in the input.
    tokens: Vec<(usize, char)>,
    at: usize,
    end: usize,
}

impl Parser {
    fn peek(&self) -> Option<(usize, char)> {
        self.tokens.get(self.at).copied()
    }

    fn position(&self) -> usize {
        self.peek().map_or(self.end, |(p, _)| p)
    }

    fn error<T>(&self, message: &str) -> Result<T, ParseError> {
        Err(ParseError {
            position: self.position(),
            message: message.to_string(),
        })
    }

    fn union(&mut self) -> Result<Regex, ParseError> {
        let mut operands = vec![self.concat()?];
        while self.peek().is_some_and(|(_, c)| c == '+') {
            self.at += 1;
            operands.push(self.concat()?);
        }
        Ok(if operands.len() == 1 {
            operands.remove(0)
        } else {
            Regex::Union(operands)
        })
    }

    fn concat(&mut self) -> Result<Regex, ParseError> {
        let mut factors = Vec::new();
        // Position of an unparenthesized, unstarred λ among the factors.
        let mut bare_lambda = None;
        while self
            .peek()
            .is_some_and(|(_, c)| !matches!(c, '+' | '*' | ')'))
        {
            let position = self.position();
            let (atom, bare) = self.atom()?;
            let mut factor = atom;
            let mut starred = false;
            while self.peek().is_some_and(|(_, c)| c == '*') {
                self.at += 1;
                factor = Regex::Star(Box::new(factor));
                starred = true;
            }
            if bare && !starred {
                bare_lambda = bare_lambda.or(Some(position));
            }
            factors.push(factor);
        }
        if factors.is_empty() {
            return self.error("Operators are poorly formatted.");
        }
        if let (Some(position), true) = (bare_lambda, factors.len() > 1) {
            return Err(ParseError {
                position,
                message: "Lambda character must not cat with anything else.".to_string(),
            });
        }
        Ok(if factors.len() == 1 {
            factors.remove(0)
        } else {
            Regex::Concat(factors)
        })
    }

    /// An atom and whether it is a bare λ (not parenthesized).
    fn atom(&mut self) -> Result<(Regex, bool), ParseError> {
        let Some((_, c)) = self.peek() else {
            return self.error("The expression ends too soon.");
        };
        self.at += 1;
        Ok(match c {
            '(' => {
                let inner = self.union()?;
                match self.peek() {
                    Some((_, ')')) => self.at += 1,
                    _ => return self.error("The parentheses are unbalanced!"),
                }
                (inner, false)
            }
            'λ' | 'ε' | '!' => (Regex::Lambda, true),
            'ø' | '∅' => (Regex::Empty, false),
            c => (Regex::Symbol(c), false),
        })
    }
}
