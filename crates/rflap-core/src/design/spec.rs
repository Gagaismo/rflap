//! Language descriptions accepted by the design engine (JSON friendly).

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

/// A language to build an automaton for, as an AI translates it from natural language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageSpec {
    /// Single-character symbols. In JSON either `["a","b"]` or the string `"ab"`.
    #[serde(deserialize_with = "alphabet_from_json")]
    pub alphabet: Vec<String>,
    pub language: LanguageSource,
    #[serde(default)]
    pub examples: Examples,
}

impl LanguageSpec {
    /// A spec whose alphabet is the characters of `alphabet`, without examples.
    pub fn new(alphabet: &str, language: LanguageSource) -> Self {
        LanguageSpec {
            alphabet: alphabet.chars().map(String::from).collect(),
            language,
            examples: Examples::default(),
        }
    }

    /// The same spec with examples attached.
    pub fn with_examples(mut self, accept: &[&str], reject: &[&str]) -> Self {
        self.examples = Examples {
            accept: accept.iter().map(|s| s.to_string()).collect(),
            reject: reject.iter().map(|s| s.to_string()).collect(),
        };
        self
    }
}

fn alphabet_from_json<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Form {
        Text(String),
        List(Vec<String>),
    }
    Ok(match Form::deserialize(d)? {
        Form::Text(s) => s.chars().map(String::from).collect(),
        Form::List(v) => v,
    })
}

/// How the language is described.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageSource {
    /// A regular expression in Rflap/JFLAP syntax (`+` union, `*` star, juxtaposition).
    Regex(String),
    /// A right-linear or left-linear grammar, one `A -> aB | b | λ` production per line.
    RegularGrammar(String),
    /// A structured constraint.
    Constraint(Constraint),
    /// A constraint in the text syntax, e.g. `ends_with("ab") and count(a) % 2 == 0`.
    ConstraintText(String),
    /// A non-regular language with a ready-made pushdown automaton.
    Template(Template),
}

/// Strings the built automaton must accept and reject; checked, never fatal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Examples {
    #[serde(default)]
    pub accept: Vec<String>,
    #[serde(default)]
    pub reject: Vec<String>,
}

/// Comparison operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cmp {
    #[serde(rename = "==", alias = "eq")]
    Eq,
    #[serde(rename = "!=", alias = "ne")]
    Ne,
    #[serde(rename = "<", alias = "lt")]
    Lt,
    #[serde(rename = "<=", alias = "le")]
    Le,
    #[serde(rename = ">", alias = "gt")]
    Gt,
    #[serde(rename = ">=", alias = "ge")]
    Ge,
}

impl Cmp {
    pub fn holds(self, left: usize, right: usize) -> bool {
        match self {
            Cmp::Eq => left == right,
            Cmp::Ne => left != right,
            Cmp::Lt => left < right,
            Cmp::Le => left <= right,
            Cmp::Gt => left > right,
            Cmp::Ge => left >= right,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Cmp::Eq => "==",
            Cmp::Ne => "!=",
            Cmp::Lt => "<",
            Cmp::Le => "<=",
            Cmp::Gt => ">",
            Cmp::Ge => ">=",
        }
    }
}

/// A condition on strings; every variant but `CountCompare` describes a regular language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    StartsWith {
        word: String,
    },
    EndsWith {
        word: String,
    },
    /// `word` is a substring.
    Contains {
        word: String,
    },
    /// `length <cmp> n`.
    Length {
        cmp: Cmp,
        n: usize,
    },
    /// `length % m == r`.
    LengthMod {
        m: usize,
        r: usize,
    },
    /// Occurrences of any symbol in `symbols` (a string of symbols): `count(symbols) <cmp> n`.
    Count {
        symbols: String,
        cmp: Cmp,
        n: usize,
    },
    /// `count(symbols) % m == r`.
    CountMod {
        symbols: String,
        m: usize,
        r: usize,
    },
    /// `count(left) <cmp> count(right)`: NOT regular (use a template instead).
    CountCompare {
        left: String,
        cmp: Cmp,
        right: String,
    },
    /// The whole string matches a regular expression in Rflap syntax.
    Regex {
        pattern: String,
    },
    And {
        constraints: Vec<Constraint>,
    },
    Or {
        constraints: Vec<Constraint>,
    },
    /// True when an odd number of the constraints hold.
    Xor {
        constraints: Vec<Constraint>,
    },
    Not {
        constraint: Box<Constraint>,
    },
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

fn set_text(symbols: &str) -> String {
    symbols
        .chars()
        .map(String::from)
        .collect::<Vec<_>>()
        .join(",")
}

impl Constraint {
    fn is_compound(&self) -> bool {
        matches!(
            self,
            Constraint::And { .. } | Constraint::Or { .. } | Constraint::Xor { .. }
        )
    }

    fn operand(&self) -> String {
        if self.is_compound() {
            format!("({self})")
        } else {
            self.to_string()
        }
    }
}

/// Canonical text syntax; `Constraint::parse` reads it back.
impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let join = |cs: &[Constraint], word: &str| {
            cs.iter()
                .map(Constraint::operand)
                .collect::<Vec<_>>()
                .join(word)
        };
        match self {
            Constraint::StartsWith { word } => write!(f, "starts_with({})", quote(word)),
            Constraint::EndsWith { word } => write!(f, "ends_with({})", quote(word)),
            Constraint::Contains { word } => write!(f, "contains({})", quote(word)),
            Constraint::Length { cmp, n } => write!(f, "length {} {n}", cmp.symbol()),
            Constraint::LengthMod { m, r } => write!(f, "length % {m} == {r}"),
            Constraint::Count { symbols, cmp, n } => {
                write!(f, "count({}) {} {n}", set_text(symbols), cmp.symbol())
            }
            Constraint::CountMod { symbols, m, r } => {
                write!(f, "count({}) % {m} == {r}", set_text(symbols))
            }
            Constraint::CountCompare { left, cmp, right } => write!(
                f,
                "count({}) {} count({})",
                set_text(left),
                cmp.symbol(),
                set_text(right)
            ),
            Constraint::Regex { pattern } => write!(f, "regex({})", quote(pattern)),
            Constraint::And { constraints } => f.write_str(&join(constraints, " and ")),
            Constraint::Or { constraints } => f.write_str(&join(constraints, " or ")),
            Constraint::Xor { constraints } => f.write_str(&join(constraints, " xor ")),
            Constraint::Not { constraint } => write!(f, "not {}", constraint.operand()),
        }
    }
}

/// Relation between the exponents of `a^n b^m`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    /// n <= m
    #[serde(alias = "<=")]
    Le,
    /// n >= m
    #[serde(alias = ">=")]
    Ge,
    /// n != m
    #[serde(alias = "!=")]
    Ne,
}

fn default_pairs() -> Vec<[char; 2]> {
    vec![['(', ')']]
}

/// Non-regular languages with a ready-made PDA (accepting by final state). Symbols must be
/// in the alphabet; other alphabet symbols never occur in accepted strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum Template {
    /// `a^n b^n` with n >= `min` (0 or 1, default 0).
    AnBn {
        a: char,
        b: char,
        #[serde(default)]
        min: usize,
    },
    /// `a^n b^m` (n, m >= 0) with n <= m, n >= m or n != m.
    AnBm {
        a: char,
        b: char,
        relation: Relation,
    },
    /// Strings with as many `a` as `b` in any order.
    EqualCount { a: char, b: char },
    /// Balanced brackets (Dyck language) for one or more `[open, close]` pairs.
    BalancedBrackets {
        #[serde(default = "default_pairs")]
        pairs: Vec<[char; 2]>,
    },
    /// `w c w^R` with `w` over the alphabet without `c`.
    #[serde(rename = "wcwr")]
    WcWr { c: char },
    /// `w w^R`.
    EvenPalindrome,
    /// Every palindrome, odd length included.
    Palindrome,
}

impl Template {
    /// The snake_case name used in JSON and in not-regular reports.
    pub fn name(&self) -> &'static str {
        match self {
            Template::AnBn { .. } => "an_bn",
            Template::AnBm { .. } => "an_bm",
            Template::EqualCount { .. } => "equal_count",
            Template::BalancedBrackets { .. } => "balanced_brackets",
            Template::WcWr { .. } => "wcwr",
            Template::EvenPalindrome => "even_palindrome",
            Template::Palindrome => "palindrome",
        }
    }
}
