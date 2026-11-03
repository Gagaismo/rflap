//! Turing machines, single and multi-tape, with building blocks (JFLAP `automata.turing`).
//! Mealy and Moore machines (JFLAP `automata.mealy`) live in `mealy`. The notation of the
//! course "Sintaxis y Semántica de los Lenguajes" lives in `catedra`.

pub mod catedra;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::automaton::{Automaton, Label, Point, State, StateId, Transition};
use crate::sim::{Machine, RunOutcome};

/// The blank tape symbol (JFLAP `Tape.BLANK`).
pub const BLANK: char = '□';

/// Invalid label, block or input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TmError {
    /// A transition with no tapes.
    NoTapes,
    BadRead(String),
    BadWrite(String),
    BadDirection(String),
    /// A label, block or input list does not match the tape count of the machine.
    TapeCount {
        expected: usize,
        found: usize,
    },
    /// A label text that is neither the cátedra `(e,s,m)` nor the JFLAP `e;s,m` form.
    BadLabel(String),
}

impl fmt::Display for TmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TmError::NoTapes => f.write_str("a transition needs at least one tape"),
            TmError::BadRead(s) => write!(f, "invalid read symbol {s:?}"),
            TmError::BadWrite(s) => {
                write!(f, "write string must have exactly one character: {s:?}")
            }
            TmError::BadDirection(s) => write!(f, "direction must be L, R, or S: {s:?}"),
            TmError::TapeCount { expected, found } => {
                write!(f, "expected {expected} tapes but found {found}")
            }
            TmError::BadLabel(s) => write!(f, "invalid transition label {s:?}"),
        }
    }
}

impl std::error::Error for TmError {}

/// Head movement of one tape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[serde(alias = "Left")]
    Left,
    #[serde(alias = "Right")]
    Right,
    #[serde(alias = "Stay")]
    Stay,
}

impl Direction {
    /// `L`, `R` or `S`.
    pub fn symbol(self) -> char {
        match self {
            Direction::Left => 'L',
            Direction::Right => 'R',
            Direction::Stay => 'S',
        }
    }

    pub fn from_symbol(s: &str) -> Option<Direction> {
        match s {
            "L" => Some(Direction::Left),
            "R" => Some(Direction::Right),
            "S" => Some(Direction::Stay),
            _ => None,
        }
    }
}

/// One tape: JFLAP keeps a string with the head index, growing it with blanks on demand.
#[derive(Clone, Debug)]
pub struct Tape {
    cells: Vec<char>,
    head: usize,
}

impl Tape {
    /// A tape holding `input`, head on the first cell; empty input is a single blank.
    pub fn new(input: &str) -> Self {
        let mut cells: Vec<char> = input.chars().collect();
        if cells.is_empty() {
            cells.push(BLANK);
        }
        Tape { cells, head: 0 }
    }

    /// Symbol under the head.
    pub fn read(&self) -> char {
        self.cells[self.head]
    }

    pub fn write(&mut self, c: char) {
        self.cells[self.head] = c;
    }

    /// Moves the head, extending the tape with blanks when it leaves it.
    pub fn move_head(&mut self, d: Direction) {
        match d {
            Direction::Stay => {}
            Direction::Right => {
                self.head += 1;
                if self.head >= self.cells.len() {
                    self.cells.resize(self.head + 1, BLANK);
                }
            }
            Direction::Left => {
                if self.head == 0 {
                    self.cells.insert(0, BLANK);
                } else {
                    self.head -= 1;
                }
            }
        }
    }

    /// The whole tape, blank padding included.
    pub fn contents(&self) -> String {
        self.cells.iter().collect()
    }

    /// The tape without leading and trailing blanks.
    pub fn trimmed(&self) -> String {
        let (start, end) = self.data_range();
        self.cells[start..end].iter().collect()
    }

    pub fn head(&self) -> usize {
        self.head
    }

    /// From the head up to the next blank: what JFLAP shows as the output of a tape
    /// (`Tape.getOutput`).
    pub fn output(&self) -> String {
        self.cells[self.head..]
            .iter()
            .take_while(|&&c| c != BLANK)
            .collect()
    }

    /// Range of cells between the first and the last non-blank (JFLAP `nonTrivial`).
    fn data_range(&self) -> (usize, usize) {
        let mut end = self.cells.len() - 1;
        while end > 0 && self.cells[end] == BLANK {
            end -= 1;
        }
        if self.cells[end] != BLANK {
            end += 1;
        }
        let mut start = 0;
        while start < end && self.cells[start] == BLANK {
            start += 1;
        }
        (start, end)
    }

    /// Identity of a tape: its data and the head position relative to it.
    fn key(&self) -> (isize, &[char]) {
        let (start, end) = self.data_range();
        (self.head as isize - start as isize, &self.cells[start..end])
    }
}

impl PartialEq for Tape {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for Tape {}

impl Hash for Tape {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

/// What one tape does in a transition: read pattern, symbol written and head movement.
///
/// A read pattern is one symbol, `~` (any symbol), `!x` (any symbol but `x`, a lone `!`
/// is `!□`) or `a,b}v` (one of `a`, `b`, remembered in variable `v`). A written `~` keeps
/// the symbol; a written variable name writes the remembered symbol.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TapeAction {
    read: String,
    write: char,
    direction: Direction,
}

impl TapeAction {
    /// Builds an action from JFLAP's text forms; an empty read or write is the blank.
    pub fn new(read: &str, write: &str, direction: &str) -> Result<Self, TmError> {
        let read = match read {
            "" => BLANK.to_string(),
            "!" => format!("!{BLANK}"),
            r => r.to_string(),
        };
        if ReadSymbol::parse(&read).is_none() {
            return Err(TmError::BadRead(read));
        }
        let mut chars = write.chars();
        let write = match (chars.next(), chars.next()) {
            (None, _) => BLANK,
            (Some(c), None) => c,
            _ => return Err(TmError::BadWrite(write.to_string())),
        };
        let direction = Direction::from_symbol(direction)
            .ok_or(TmError::BadDirection(direction.to_string()))?;
        Ok(TapeAction {
            read,
            write,
            direction,
        })
    }

    pub fn read(&self) -> &str {
        &self.read
    }

    pub fn write(&self) -> char {
        self.write
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }
}

/// Parsed read pattern.
enum ReadSymbol {
    Any,
    Char(char),
    Not(char),
    OneOf { chars: Vec<char>, var: char },
}

impl ReadSymbol {
    /// `None` for strings JFLAP rejects (`TMTransition.setRead`).
    fn parse(s: &str) -> Option<ReadSymbol> {
        if let Some((set, var)) = s.split_once('}') {
            if s.contains('!') {
                return None;
            }
            let mut var_chars = var.chars();
            let var = var_chars.next()?;
            let chars: Option<Vec<char>> = set
                .split(',')
                .map(|e| {
                    let mut it = e.chars();
                    match (it.next(), it.next()) {
                        (Some(c), None) => Some(c),
                        _ => None,
                    }
                })
                .collect();
            return Some(ReadSymbol::OneOf { chars: chars?, var });
        }
        if s == "~" {
            return Some(ReadSymbol::Any);
        }
        if let Some(rest) = s.strip_prefix('!') {
            return rest.chars().next().map(ReadSymbol::Not);
        }
        let mut it = s.chars();
        match (it.next(), it.next()) {
            (Some(c), None) => Some(ReadSymbol::Char(c)),
            _ => None,
        }
    }

    fn matches(&self, c: char) -> bool {
        match self {
            ReadSymbol::Any => true,
            ReadSymbol::Char(x) => *x == c,
            ReadSymbol::Not(x) => *x != c,
            ReadSymbol::OneOf { chars, .. } => chars.contains(&c),
        }
    }
}

/// Transition label: one `TapeAction` per tape (JFLAP `TMTransition`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TmLabel {
    actions: Vec<TapeAction>,
}

impl TmLabel {
    pub fn new(actions: Vec<TapeAction>) -> Result<Self, TmError> {
        if actions.is_empty() {
            return Err(TmError::NoTapes);
        }
        Ok(TmLabel { actions })
    }

    /// Label of a one-tape machine, from JFLAP's text forms.
    pub fn single(read: &str, write: &str, direction: &str) -> Result<Self, TmError> {
        TmLabel::new(vec![TapeAction::new(read, write, direction)?])
    }

    pub fn tapes(&self) -> usize {
        self.actions.len()
    }

    pub fn actions(&self) -> &[TapeAction] {
        &self.actions
    }
}

impl Label for TmLabel {
    /// JFLAP's `TMLambdaTransitionChecker`: a Turing machine has no λ transitions.
    fn is_lambda(&self) -> bool {
        false
    }

    /// `read ; write , direction` per tape, joined by ` | `.
    fn description(&self) -> String {
        self.actions
            .iter()
            .map(|a| format!("{} ; {} , {}", a.read, a.write, a.direction.symbol()))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// JFLAP `TMNondeterminismDetector`, which compares tape by tape and answers true
    /// as soon as the read patterns of any one tape overlap.
    fn conflicts(&self, other: &Self) -> bool {
        self.actions
            .iter()
            .zip(&other.actions)
            .any(|(a, b)| reads_conflict(&a.read, &b.read))
    }
}

fn reads_conflict(a: &str, b: &str) -> bool {
    if a == b || a == "~" || b == "~" {
        return true;
    }
    let (negated, other) = if a.starts_with('!') { (a, b) } else { (b, a) };
    if !negated.starts_with('!') {
        return false;
    }
    if other.starts_with('!') {
        return true;
    }
    other.chars().any(|c| c != ',' && !negated.contains(c))
}

/// A Turing machine of `tapes` tapes. A state may hold an inner machine (a building
/// block) that runs on the shared tapes (JFLAP `TMState.getInnerTM`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TuringMachine {
    automaton: Automaton<TmLabel>,
    tapes: usize,
    blocks: BTreeMap<StateId, TuringMachine>,
    /// The declared input alphabet Σ_E of the cátedra definition; `None` when it is inferred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    input_alphabet: Option<BTreeSet<char>>,
}

impl TuringMachine {
    /// An empty machine; zero tapes count as one.
    pub fn new(tapes: usize) -> Self {
        TuringMachine {
            automaton: Automaton::new(),
            tapes: tapes.max(1),
            blocks: BTreeMap::new(),
            input_alphabet: None,
        }
    }

    pub fn tapes(&self) -> usize {
        self.tapes
    }

    /// The declared input alphabet Σ_E, `None` when the machine does not declare one.
    pub fn input_alphabet(&self) -> Option<&BTreeSet<char>> {
        self.input_alphabet.as_ref()
    }

    pub fn set_input_alphabet(&mut self, alphabet: Option<BTreeSet<char>>) {
        self.input_alphabet = alphabet;
    }

    /// The underlying states, transitions, initial and final states.
    pub fn automaton(&self) -> &Automaton<TmLabel> {
        &self.automaton
    }

    /// Mutable access to the underlying automaton, for operations written for any
    /// automaton (combine, removing useless states). It does not touch the blocks: after
    /// removing states, drop their blocks with [`TuringMachine::remove_state`], which is a
    /// no-op on the automaton for a state that is already gone.
    pub fn automaton_mut(&mut self) -> &mut Automaton<TmLabel> {
        &mut self.automaton
    }

    pub fn create_state(&mut self, point: Point) -> StateId {
        self.automaton.create_state(point)
    }

    pub fn create_state_with_id(&mut self, id: StateId, point: Point) -> Option<StateId> {
        self.automaton.create_state_with_id(id, point)
    }

    /// Removes a state with its transitions and its block.
    pub fn remove_state(&mut self, id: StateId) {
        self.automaton.remove_state(id);
        self.blocks.remove(&id);
    }

    pub fn state_mut(&mut self, id: StateId) -> Option<&mut State> {
        self.automaton.state_mut(id)
    }

    pub fn set_initial(&mut self, id: Option<StateId>) {
        self.automaton.set_initial(id);
    }

    pub fn set_final(&mut self, id: StateId, is_final: bool) {
        self.automaton.set_final(id, is_final);
    }

    /// Adds a transition; `Ok(false)` for duplicates and dangling states, an error when
    /// the label does not have one action per tape.
    pub fn add_transition(
        &mut self,
        from: StateId,
        to: StateId,
        label: TmLabel,
    ) -> Result<bool, TmError> {
        if label.tapes() != self.tapes {
            return Err(TmError::TapeCount {
                expected: self.tapes,
                found: label.tapes(),
            });
        }
        Ok(self
            .automaton
            .add_transition(Transition::new(from, to, label)))
    }

    pub fn remove_transition(&mut self, t: &Transition<TmLabel>) -> bool {
        self.automaton.remove_transition(t)
    }

    /// Makes `state` a building block running `inner`; a block without states is an
    /// ordinary state. The inner machine must have the same number of tapes.
    pub fn set_block(&mut self, state: StateId, inner: TuringMachine) -> Result<(), TmError> {
        if inner.tapes != self.tapes {
            return Err(TmError::TapeCount {
                expected: self.tapes,
                found: inner.tapes,
            });
        }
        if self.automaton.state(state).is_some() {
            self.blocks.insert(state, inner);
        }
        Ok(())
    }

    /// Creates a state that is a building block.
    pub fn add_block(&mut self, point: Point, inner: TuringMachine) -> Result<StateId, TmError> {
        if inner.tapes != self.tapes {
            return Err(TmError::TapeCount {
                expected: self.tapes,
                found: inner.tapes,
            });
        }
        let id = self.create_state(point);
        self.blocks.insert(id, inner);
        Ok(id)
    }

    /// The inner machine of a block state.
    pub fn block(&self, state: StateId) -> Option<&TuringMachine> {
        self.blocks.get(&state)
    }

    pub fn blocks(&self) -> &BTreeMap<StateId, TuringMachine> {
        &self.blocks
    }

    /// The inner machine of `state` if it has states (so it runs when entered).
    fn active_block(&self, state: StateId) -> Option<&TuringMachine> {
        self.blocks
            .get(&state)
            .filter(|b| b.automaton.states().next().is_some())
    }
}

/// When a Turing machine accepts (JFLAP `AcceptanceFilter`); a configuration accepts if
/// any enabled filter accepts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AcceptanceFilters {
    /// A top-level final state was reached (`AcceptByFinalStateFilter`).
    pub final_state: bool,
    /// No transition applies any more (`AcceptByHaltingFilter`).
    pub halting: bool,
}

impl AcceptanceFilters {
    pub fn by_final_state() -> Self {
        AcceptanceFilters {
            final_state: true,
            halting: false,
        }
    }

    pub fn by_halting() -> Self {
        AcceptanceFilters {
            final_state: false,
            halting: true,
        }
    }
}

impl Default for AcceptanceFilters {
    /// JFLAP's defaults: accept by final state only.
    fn default() -> Self {
        Self::by_final_state()
    }
}

/// A simulation configuration (JFLAP `TMConfiguration`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TmConfig {
    /// Current state as the chain of block states entered, outermost first.
    pub path: Vec<StateId>,
    pub tapes: Vec<Tape>,
    /// Set on the extra configuration produced when no transition applies.
    pub halted: bool,
    /// Variables assigned by `a,b}v` reads.
    pub variables: BTreeMap<char, char>,
}

impl TmConfig {
    /// Current state, in the innermost machine of `path`.
    pub fn state(&self) -> StateId {
        *self.path.last().expect("configuration without a state")
    }

    /// Output of a tape: from its head to the next blank.
    pub fn output(&self, tape: usize) -> Option<String> {
        self.tapes.get(tape).map(Tape::output)
    }
}

/// The output of every tape of the accepting configuration of a run (JFLAP shows it for
/// transducer-style runs), `None` when the run did not accept.
pub fn tape_outputs(outcome: &RunOutcome<TmConfig>) -> Option<Vec<String>> {
    match outcome {
        RunOutcome::Accepted { trace } => trace
            .last()
            .map(|c| c.tapes.iter().map(Tape::output).collect()),
        _ => None,
    }
}

/// The final contents of every tape of the accepting configuration, without the blanks around
/// the data (Rflap addition: `tape_outputs` shows only what is under and after the head),
/// `None` when the run did not accept.
pub fn tape_contents(outcome: &RunOutcome<TmConfig>) -> Option<Vec<String>> {
    match outcome {
        RunOutcome::Accepted { trace } => trace
            .last()
            .map(|c| c.tapes.iter().map(Tape::trimmed).collect()),
        _ => None,
    }
}

/// Simulator for Turing machines (JFLAP `TMSimulator`). Like JFLAP it is deterministic:
/// a step follows the first applicable transition (those with a `!` read first tape last).
/// When none applies inside a block, the transitions leaving the block are tried; when
/// none applies at the top level the machine halts.
pub struct TmSimulator<'a> {
    machine: &'a TuringMachine,
    filters: AcceptanceFilters,
    inputs: Option<Vec<String>>,
}

impl<'a> TmSimulator<'a> {
    /// The input string given to `initial_configs` goes on every tape.
    pub fn new(machine: &'a TuringMachine, filters: AcceptanceFilters) -> Self {
        TmSimulator {
            machine,
            filters,
            inputs: None,
        }
    }

    /// One input string per tape; the input given to `initial_configs` is ignored.
    pub fn with_inputs(
        machine: &'a TuringMachine,
        filters: AcceptanceFilters,
        inputs: Vec<String>,
    ) -> Result<Self, TmError> {
        if inputs.len() != machine.tapes {
            return Err(TmError::TapeCount {
                expected: machine.tapes,
                found: inputs.len(),
            });
        }
        Ok(TmSimulator {
            machine,
            filters,
            inputs: Some(inputs),
        })
    }
}

/// Machines of the chain `path[..len - 1]`, outermost first, ending with the innermost.
fn machine_chain<'m>(root: &'m TuringMachine, path: &[StateId]) -> Option<Vec<&'m TuringMachine>> {
    let mut chain = vec![root];
    for id in &path[..path.len().saturating_sub(1)] {
        chain.push(chain.last()?.blocks.get(id)?);
    }
    Some(chain)
}

fn label_matches(label: &TmLabel, reads: &[char]) -> bool {
    label.actions.len() == reads.len()
        && label
            .actions
            .iter()
            .zip(reads)
            .all(|(a, &c)| ReadSymbol::parse(&a.read).is_some_and(|r| r.matches(c)))
}

/// Applies a matched label to a copy of the configuration's tapes and variables.
fn apply(label: &TmLabel, tapes: &mut [Tape], variables: &mut BTreeMap<char, char>) {
    let reads: Vec<char> = tapes.iter().map(Tape::read).collect();
    for (a, &c) in label.actions.iter().zip(&reads) {
        if let Some(ReadSymbol::OneOf { var, .. }) = ReadSymbol::parse(&a.read) {
            variables.insert(var, c);
        }
    }
    for ((a, tape), &c) in label.actions.iter().zip(tapes.iter_mut()).zip(&reads) {
        let written = if a.write == '~' {
            c
        } else {
            variables.get(&a.write).copied().unwrap_or(a.write)
        };
        tape.write(written);
        tape.move_head(a.direction);
    }
}

impl Machine for TmSimulator<'_> {
    type Config = TmConfig;

    fn initial_configs(&self, input: &str) -> Vec<TmConfig> {
        let Some(initial) = self.machine.automaton.initial() else {
            return Vec::new();
        };
        let tapes = match &self.inputs {
            Some(inputs) => inputs.iter().map(|s| Tape::new(s)).collect(),
            None => vec![Tape::new(input); self.machine.tapes],
        };
        vec![TmConfig {
            path: vec![initial],
            tapes,
            halted: false,
            variables: BTreeMap::new(),
        }]
    }

    fn step(&self, c: &TmConfig) -> Vec<TmConfig> {
        if c.halted {
            return Vec::new();
        }
        let Some(mut chain) = machine_chain(self.machine, &c.path) else {
            return Vec::new();
        };
        let mut path = c.path.clone();
        // Enter blocks down to their initial states.
        while let Some(inner) = chain
            .last()
            .zip(path.last())
            .and_then(|(m, &s)| m.active_block(s))
        {
            let Some(initial) = inner.automaton.initial() else {
                return Vec::new();
            };
            path.push(initial);
            chain.push(inner);
        }
        let reads: Vec<char> = c.tapes.iter().map(Tape::read).collect();
        // Innermost machine first; a machine without an applicable transition hands over
        // to the transitions leaving its block.
        for depth in (1..=path.len()).rev() {
            let (machine, state) = (chain[depth - 1], path[depth - 1]);
            let mut candidates: Vec<&Transition<TmLabel>> =
                machine.automaton.transitions_from(state).collect();
            candidates.sort_by_key(|t| {
                t.label
                    .actions
                    .first()
                    .is_some_and(|a| a.read.starts_with('!'))
            });
            if let Some(t) = candidates
                .into_iter()
                .find(|t| label_matches(&t.label, &reads))
            {
                let mut next = c.clone();
                next.path = path[..depth - 1].to_vec();
                next.path.push(t.to);
                apply(&t.label, &mut next.tapes, &mut next.variables);
                return vec![next];
            }
        }
        let mut halted = c.clone();
        halted.halted = true;
        vec![halted]
    }

    fn is_accepting(&self, c: &TmConfig) -> bool {
        (self.filters.halting && c.halted)
            || (self.filters.final_state
                && c.path.len() == 1
                && self.machine.automaton.is_final(c.path[0]))
    }
}
