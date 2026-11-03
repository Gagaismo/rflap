//! Text syntax for constraints:
//!
//! ```text
//! ends_with("ab") and count(a) % 2 == 0 and not contains("bb")
//! ```
//!
//! Conditions: `starts_with("w")`, `ends_with("w")`, `contains("w")`, `regex("r")`,
//! `length <cmp> n`, `length % m == r`, `count(a) <cmp> n`, `count(a,b) % m == r` and
//! `count(a) <cmp> count(b)`. Combinators, tightest first: `not` (`!`), `and` (`&&`),
//! `xor` (`^`), `or` (`||`), with parentheses. `<cmp>` is `==`, `!=`, `<`, `<=`, `>` or `>=`.

use std::str::FromStr;

use super::error::ConstraintSyntaxError;
use super::spec::{Cmp, Constraint};

impl Constraint {
    /// Parses the text syntax; errors carry the character position and what was expected.
    pub fn parse(input: &str) -> Result<Constraint, ConstraintSyntaxError> {
        let mut p = Parser {
            chars: input.chars().collect(),
            pos: 0,
        };
        let c = p.or()?;
        p.skip_ws();
        if p.pos < p.chars.len() {
            return Err(p.error(p.pos, "`and`, `or`, `xor` or the end of the expression"));
        }
        Ok(c)
    }
}

impl FromStr for Constraint {
    type Err = ConstraintSyntaxError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Constraint::parse(s)
    }
}

const CONDITIONS: &str = "a condition: `starts_with(\"..\")`, `ends_with(\"..\")`, `contains(\"..\")`, \
`regex(\"..\")`, `length`, `count(..)`, `not` or `(`";

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

type Res<T> = Result<T, ConstraintSyntaxError>;

impl Parser {
    fn error(&self, at: usize, expected: &str) -> ConstraintSyntaxError {
        let found = match self.chars.get(at) {
            None => "end of input".to_string(),
            Some(c) if c.is_alphanumeric() || *c == '_' => {
                let word: String = self.chars[at..]
                    .iter()
                    .take_while(|c| c.is_alphanumeric() || **c == '_')
                    .collect();
                format!("`{word}`")
            }
            Some(c) => format!("`{c}`"),
        };
        ConstraintSyntaxError {
            position: at,
            expected: expected.to_string(),
            message: format!("at position {at}: expected {expected}, found {found}"),
            found,
        }
    }

    fn skip_ws(&mut self) {
        while self.chars.get(self.pos).is_some_and(|c| c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_ws();
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self, text: &str) -> bool {
        self.skip_ws();
        let n = text.chars().count();
        if self.chars.len() >= self.pos + n
            && self.chars[self.pos..self.pos + n]
                .iter()
                .eq(text.chars().collect::<Vec<_>>().iter())
        {
            self.pos += n;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, text: &str) -> Res<()> {
        if self.eat(text) {
            Ok(())
        } else {
            self.skip_ws();
            Err(self.error(self.pos, &format!("`{text}`")))
        }
    }

    /// The identifier at the cursor, without consuming it.
    fn ident(&mut self) -> String {
        self.skip_ws();
        self.chars[self.pos.min(self.chars.len())..]
            .iter()
            .enumerate()
            .take_while(|(i, c)| c.is_alphabetic() || **c == '_' || (*i > 0 && c.is_ascii_digit()))
            .map(|(_, c)| *c)
            .collect()
    }

    fn eat_keyword(&mut self, word: &str, symbol: &str) -> bool {
        if self.ident() == word {
            self.pos += word.chars().count();
            true
        } else {
            self.eat(symbol)
        }
    }

    fn or(&mut self) -> Res<Constraint> {
        let mut items = vec![self.xor()?];
        while self.eat_keyword("or", "||") {
            items.push(self.xor()?);
        }
        Ok(one_or(items, |constraints| Constraint::Or { constraints }))
    }

    fn xor(&mut self) -> Res<Constraint> {
        let mut items = vec![self.and()?];
        while self.eat_keyword("xor", "^") {
            items.push(self.and()?);
        }
        Ok(one_or(items, |constraints| Constraint::Xor { constraints }))
    }

    fn and(&mut self) -> Res<Constraint> {
        let mut items = vec![self.not()?];
        while self.eat_keyword("and", "&&") {
            items.push(self.not()?);
        }
        Ok(one_or(items, |constraints| Constraint::And { constraints }))
    }

    fn not(&mut self) -> Res<Constraint> {
        if self.ident() == "not" {
            self.pos += 3;
            return Ok(Constraint::Not {
                constraint: Box::new(self.not()?),
            });
        }
        if self.peek() == Some('!') {
            self.pos += 1;
            return Ok(Constraint::Not {
                constraint: Box::new(self.not()?),
            });
        }
        self.atom()
    }

    fn atom(&mut self) -> Res<Constraint> {
        if self.peek() == Some('(') {
            self.pos += 1;
            let inner = self.or()?;
            self.expect(")")?;
            return Ok(inner);
        }
        let start = self.pos;
        let name = self.ident();
        match name.as_str() {
            "starts_with" | "ends_with" | "contains" | "regex" => {
                self.pos += name.chars().count();
                self.expect("(")?;
                let text = self.string()?;
                self.expect(")")?;
                Ok(match name.as_str() {
                    "starts_with" => Constraint::StartsWith { word: text },
                    "ends_with" => Constraint::EndsWith { word: text },
                    "contains" => Constraint::Contains { word: text },
                    _ => Constraint::Regex { pattern: text },
                })
            }
            "length" | "len" => {
                self.pos += name.chars().count();
                if self.eat("(") {
                    self.expect(")")?;
                }
                self.numeric(None)
            }
            "count" => {
                self.pos += 5;
                self.expect("(")?;
                let symbols = self.symbols()?;
                self.expect(")")?;
                self.numeric(Some(symbols))
            }
            "" => Err(self.error(start, CONDITIONS)),
            _ => Err(self.error(start, CONDITIONS)),
        }
    }

    /// The rest of `length ...` or `count(..) ...`.
    fn numeric(&mut self, symbols: Option<String>) -> Res<Constraint> {
        if self.eat("%") {
            let m = self.number()?;
            let negate = if self.eat("!=") {
                true
            } else if self.eat("==") || self.eat("=") {
                false
            } else {
                self.skip_ws();
                return Err(self.error(self.pos, "`==` or `!=` after the modulus"));
            };
            let r = self.number()?;
            let base = match symbols {
                Some(symbols) => Constraint::CountMod { symbols, m, r },
                None => Constraint::LengthMod { m, r },
            };
            return Ok(if negate {
                Constraint::Not {
                    constraint: Box::new(base),
                }
            } else {
                base
            });
        }
        let cmp = self.cmp()?;
        if let Some(left) = symbols.as_ref().filter(|_| self.ident() == "count") {
            self.pos += 5;
            self.expect("(")?;
            let right = self.symbols()?;
            self.expect(")")?;
            return Ok(Constraint::CountCompare {
                left: left.clone(),
                cmp,
                right,
            });
        }
        let n = self.number()?;
        Ok(match symbols {
            Some(symbols) => Constraint::Count { symbols, cmp, n },
            None => Constraint::Length { cmp, n },
        })
    }

    fn cmp(&mut self) -> Res<Cmp> {
        for (text, cmp) in [
            ("==", Cmp::Eq),
            ("!=", Cmp::Ne),
            ("<=", Cmp::Le),
            (">=", Cmp::Ge),
            ("<", Cmp::Lt),
            (">", Cmp::Gt),
            ("=", Cmp::Eq),
        ] {
            if self.eat(text) {
                return Ok(cmp);
            }
        }
        self.skip_ws();
        Err(self.error(
            self.pos,
            "a comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`) or `%`",
        ))
    }

    fn number(&mut self) -> Res<usize> {
        self.skip_ws();
        let start = self.pos;
        while self.chars.get(self.pos).is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        let digits: String = self.chars[start..self.pos].iter().collect();
        if digits.is_empty() {
            return Err(self.error(start, "a non-negative integer"));
        }
        digits
            .parse()
            .map_err(|_| self.error(start, "an integer that fits in 64 bits"))
    }

    /// A quoted string with `\"` and `\\` escapes; single or double quotes.
    fn string(&mut self) -> Res<String> {
        self.skip_ws();
        let start = self.pos;
        let quote = match self.chars.get(start) {
            Some(q @ ('"' | '\'')) => *q,
            _ => return Err(self.error(start, "a quoted string such as \"ab\"")),
        };
        self.pos += 1;
        let mut out = String::new();
        while let Some(&c) = self.chars.get(self.pos) {
            self.pos += 1;
            if c == quote {
                return Ok(out);
            }
            if c == '\\' {
                match self.chars.get(self.pos) {
                    Some(&e) => {
                        out.push(e);
                        self.pos += 1;
                    }
                    None => break,
                }
            } else {
                out.push(c);
            }
        }
        Err(self.error(start, "a closing quote for this string"))
    }

    /// Symbols inside `count( )`: bare characters (commas and blanks ignored) or a quoted string.
    fn symbols(&mut self) -> Res<String> {
        self.skip_ws();
        let start = self.pos;
        let raw: String = if matches!(self.chars.get(start), Some('"' | '\'')) {
            self.string()?
        } else {
            let mut raw = String::new();
            while let Some(&c) = self.chars.get(self.pos).filter(|c| **c != ')') {
                raw.push(c);
                self.pos += 1;
            }
            raw
        };
        let mut symbols = String::new();
        for c in raw.chars().filter(|c| *c != ',' && !c.is_whitespace()) {
            if !symbols.contains(c) {
                symbols.push(c);
            }
        }
        if symbols.is_empty() {
            return Err(self.error(start, "at least one symbol, e.g. `count(a)`"));
        }
        Ok(symbols)
    }
}

fn one_or(
    mut items: Vec<Constraint>,
    make: impl FnOnce(Vec<Constraint>) -> Constraint,
) -> Constraint {
    if items.len() == 1 {
        items.remove(0)
    } else {
        make(items)
    }
}
