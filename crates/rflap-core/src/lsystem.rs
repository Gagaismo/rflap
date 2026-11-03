//! L-systems: rewriting and turtle rendering (JFLAP `grammar.lsystem` and `gui.lsystem`).
//!
//! An [`LSystem`] is an axiom, rewriting rules and drawing parameters, all made of
//! whitespace-separated symbols. [`Expander`] rewrites the axiom level by level;
//! [`LSystem::draw`] runs the turtle over an expansion and [`LSystem::to_svg`] prints the
//! result. The `.jff` file reader and writer are in [`jff`].
//!
//! The turtle understands the symbols JFLAP's renderer does:
//!
//! | symbol | effect |
//! |---|---|
//! | `g` / `f` | move forward by `distance`, drawing / without drawing |
//! | `+` / `-` | turn clockwise / counter-clockwise by `angle` degrees |
//! | `&` / `^` | pitch down / up by `angle` |
//! | `/` / `*` | roll right / left by `angle` |
//! | `[` / `]` | push / pop the turtle |
//! | `!` / `~` | thicken / thin the line by `lineIncrement` |
//! | `{` / `}` | start / fill a polygon |
//! | `%` | turn around (180 degrees) |
//! | `#` / `@` | shift the line colour's hue by `hueChange` degrees up / down; `##` / `@@` for the polygon colour |
//! | `name=value` | set a parameter (`angle`, `distance`, `lineWidth`, `lineIncrement`, `hueChange`, `color`, `polygonColor`, or any name) |
//! | `symbol(expr)` | same as the symbol but with an explicit amount, e.g. `g(2*3)`, `+(30)`, `color(255,0,0)` |
//!
//! Colours are `java.awt.Color` names, JFLAP's extra names (`brown`, `forestGreen`, ...)
//! or a `r,g,b` triple.

mod random;
mod render;
mod turtle;

pub mod jff;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use random::JavaRandom;
pub use render::{Bounds, DrawOptions, Drawing, Polygon, Segment, Shape};

/// An 8-bit RGB colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// `#rrggbb`.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

/// `StringTokenizer` with the default delimiters: splits on runs of whitespace.
pub fn tokenify(s: &str) -> Vec<String> {
    s.split([' ', '\t', '\n', '\r', '\u{c}'])
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// An L-system as JFLAP stores it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LSystem {
    axiom: Vec<String>,
    /// Left side (sorted, as JFLAP's `TreeMap`) to its replacements, in the order given.
    rules: BTreeMap<String, Vec<Vec<String>>>,
    /// Name/value pairs in the order they were added.
    parameters: Vec<(String, String)>,
    nondeterministic: bool,
}

impl Default for LSystem {
    fn default() -> Self {
        LSystem::new("", &[], &[])
    }
}

impl LSystem {
    /// `new LSystem(axiom, grammar, values)`. A production repeated with the same left and
    /// right text is kept once, a repeated parameter name keeps its first position and its
    /// last value.
    pub fn new(axiom: &str, productions: &[(&str, &str)], parameters: &[(&str, &str)]) -> Self {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        let mut rules: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
        let mut nondeterministic = false;
        for &(left, right) in productions {
            if seen.contains(&(left, right)) {
                continue;
            }
            seen.push((left, right));
            let replacement = tokenify(right);
            let group = rules.entry(left.to_string()).or_default();
            if group.last().is_some_and(|last| *last != replacement) {
                nondeterministic = true;
            }
            group.push(replacement);
        }
        let mut params: Vec<(String, String)> = Vec::new();
        for &(name, value) in parameters {
            match params.iter_mut().find(|(n, _)| n == name) {
                Some((_, v)) => *v = value.to_string(),
                None => params.push((name.to_string(), value.to_string())),
            }
        }
        LSystem {
            axiom: tokenify(axiom),
            rules,
            parameters: params,
            nondeterministic,
        }
    }

    pub fn axiom(&self) -> &[String] {
        &self.axiom
    }

    /// The left sides that have replacements, sorted.
    pub fn symbols_with_replacements(&self) -> impl Iterator<Item = &str> {
        self.rules.keys().map(String::as_str)
    }

    /// The replacements of a left side; empty when there are none.
    pub fn replacements(&self, symbol: &str) -> &[Vec<String>] {
        self.rules.get(symbol).map_or(&[], Vec::as_slice)
    }

    /// The drawing parameters in the order they were added.
    pub fn parameters(&self) -> &[(String, String)] {
        &self.parameters
    }

    /// The parameters in the order JFLAP's `HashMap` hands them out: the order they are
    /// written to a file and applied by the renderer.
    pub fn parameters_in_jflap_order(&self) -> Vec<(&str, &str)> {
        turtle::java_hash_order(&self.parameters)
            .into_iter()
            .map(|i| (self.parameters[i].0.as_str(), self.parameters[i].1.as_str()))
            .collect()
    }

    /// Whether some left side has replacements that differ (JFLAP's `nondeterministic()`).
    pub fn is_nondeterministic(&self) -> bool {
        self.nondeterministic
    }

    pub fn expander(&self, seed: i64) -> Expander<'_> {
        Expander::new(self, seed)
    }

    /// The symbols after `level` rewrites. Stochastic rules draw from seed 0; use
    /// [`LSystem::expander`] for another seed.
    pub fn expand(&self, level: usize) -> Vec<String> {
        self.expander(0).expansion_for_level(level).to_vec()
    }

    /// Runs the turtle over the expansion at `level`.
    pub fn draw(&self, level: usize, options: &DrawOptions) -> Drawing {
        let symbols = self
            .expander(options.seed)
            .expansion_for_level(level)
            .to_vec();
        render::render(&symbols, &self.parameters_in_jflap_order(), options)
    }

    /// The drawing at `level` as an SVG document.
    pub fn to_svg(&self, level: usize) -> String {
        self.to_svg_with(level, &DrawOptions::default())
    }

    pub fn to_svg_with(&self, level: usize, options: &DrawOptions) -> String {
        self.draw(level, options)
            .to_svg(options.background.as_deref())
    }
}

/// A rewriting rule whose left side is `n a b ...`: the symbol at index `n` of the
/// pattern is rewritten when the whole pattern matches around it.
#[derive(Clone, Debug)]
struct Context {
    tokens: Vec<String>,
    center: i32,
    results: Vec<Vec<String>>,
}

impl Context {
    fn matches(&self, list: &[String], index: usize) -> &[Vec<String>] {
        let start = index as i64 - i64::from(self.center);
        let end = start + self.tokens.len() as i64;
        if start >= 0
            && end <= list.len() as i64
            && list[start as usize..end as usize] == self.tokens[..]
        {
            &self.results
        } else {
            &[]
        }
    }
}

/// Java's `Integer.parseInt`.
fn parse_int(s: &str) -> Option<i32> {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// JFLAP's `Expander`: rewrites an L-system level by level and caches the levels.
#[derive(Clone, Debug)]
pub struct Expander<'a> {
    lsystem: &'a LSystem,
    random: JavaRandom,
    cache: Vec<Vec<String>>,
    contexts: Option<Vec<Context>>,
}

impl<'a> Expander<'a> {
    /// `seed` feeds the generator that picks among several replacements of one symbol.
    pub fn new(lsystem: &'a LSystem, seed: i64) -> Self {
        let mut contexts = Vec::new();
        let mut any_context = false;
        for (key, results) in &lsystem.rules {
            let mut tokens = tokenify(key);
            let mut center = 0;
            match tokens.len() {
                0 => continue,
                1 => {}
                _ => {
                    let Some(n) = parse_int(&tokens[0]) else {
                        continue;
                    };
                    if i64::from(n) + 1 < 0 || i64::from(n) + 1 >= tokens.len() as i64 {
                        continue;
                    }
                    center = n;
                    any_context = true;
                    tokens.remove(0);
                }
            }
            contexts.push(Context {
                tokens,
                center,
                results: results.clone(),
            });
        }
        Expander {
            lsystem,
            random: JavaRandom::new(seed),
            cache: vec![lsystem.axiom.clone()],
            contexts: any_context.then_some(contexts),
        }
    }

    /// The symbols after `level` rewrites; level 0 is the axiom.
    pub fn expansion_for_level(&mut self, level: usize) -> &[String] {
        let Expander {
            lsystem,
            random,
            cache,
            contexts,
        } = self;
        while cache.len() <= level {
            let last = cache.last().expect("level 0 is always cached");
            let next = expand_once(lsystem, contexts.as_deref(), random, last);
            cache.push(next);
        }
        &cache[level]
    }
}

/// One rewrite of `list`; several candidates for a symbol are picked among at random.
fn expand_once(
    lsystem: &LSystem,
    contexts: Option<&[Context]>,
    random: &mut JavaRandom,
    list: &[String],
) -> Vec<String> {
    let mut out = Vec::new();
    for (i, symbol) in list.iter().enumerate() {
        let candidates: Vec<&Vec<String>> = match contexts {
            None => lsystem.replacements(symbol).iter().collect(),
            Some(contexts) => contexts
                .iter()
                .flat_map(|c| c.matches(list, i).iter())
                .collect(),
        };
        let chosen = match candidates.len() {
            0 => None,
            1 => Some(candidates[0]),
            n => Some(candidates[random.next_int(n as i32) as usize]),
        };
        match chosen {
            None => out.push(symbol.clone()),
            Some(replacement) => out.extend(replacement.iter().cloned()),
        }
    }
    out
}
