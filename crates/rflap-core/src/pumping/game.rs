//! The pumping lemma game (JFLAP `gui.pumping`): the "You go first" and "Computer goes
//! first" panes as a state machine.
//!
//! JFLAP lays a game out in five boxes that appear as it advances:
//!
//! | box | "You go first" | "Computer goes first" |
//! |---|---|---|
//! | 1 | you enter `m` | the computer picks `m` |
//! | 2 | the computer picks `w` | you enter `w` |
//! | 3 | you split `w` (x y z or u v x y z) | the computer splits `w` |
//! | 4 | the computer picks `i` | you enter `i` |
//! | 5 | the animation and the verdict | |
//!
//! Every move returns a [`Reply`] carrying the message JFLAP shows for it. The state is
//! serializable, so a game can be stored and resumed between calls.

use serde::{Deserialize, Serialize};

use super::lemma::Lemma;
use super::{Kind, Language};

const OBJECTIVE_HUMAN: &str = "Find a valid partition that can be pumped.";
const OBJECTIVE_COMPUTER: &str = "Prevent the computer from finding a valid partition.";
const PROMPT_M: &str = "Please select a value for m in Box 1 and press \"Enter\".";
const DESCRIBE_M: &str = "I have selected a value for m, displayed below.";
const DESCRIBE_W: &str = "I have selected w such that |w| >= m. It is displayed in Box 2.";
const PROMPT_W: &str = "Please enter a possible value for w and press \"Enter\".";
const DESCRIBE_DECOMPOSITION: &str = "I have decomposed w into the following...";
const DESCRIBE_I: &str = "I have selected i to give a contradition. It is displayed in Box 4.";
const PROMPT_I: &str = "Please enter a possible value for i and press \"Enter\".";
const I_WIN_REGULAR: &str =
    "I WIN. Do you want to play again or concede that the language is not regular?";
const YOU_WIN_REGULAR: &str = "YOU WIN. Does this mean the language is regular?";
const I_WIN_CONTEXT_FREE: &str =
    "I WIN. Do you want to play again or concede that the language is not context-free?";
const YOU_WIN_CONTEXT_FREE: &str = "YOU WIN. Does this mean the language is context-free?";

/// Who makes the first move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// "You go first": you pick `m`, the computer `w`, you split `w`, the computer picks `i`.
    HumanFirst,
    /// "Computer goes first": the computer picks `m`, you `w`, the computer splits `w`, you pick `i`.
    ComputerFirst,
}

/// The step the game waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Awaiting {
    /// You enter `m` (box 1).
    M,
    /// You enter `w` (box 2).
    W,
    /// You split `w` (box 3).
    Decomposition,
    /// You enter `i` (box 4).
    I,
}

/// A move of the game, for callers that pass moves around as data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "move", rename_all = "snake_case")]
pub enum Move {
    ChooseM {
        m: i32,
    },
    ChooseW {
        w: String,
    },
    /// `[|x|, |y|]` for the regular lemma, `[|u|, |v|, |x|, |y|]` for the context-free one.
    ChooseDecomposition {
        lengths: Vec<i32>,
    },
    ChooseI {
        i: i32,
    },
    /// The "Clear All" button.
    ClearAll,
    /// The case panel's buttons (context-free, "You go first"). Row numbers start at 0.
    AddCase,
    ReplaceCase {
        row: usize,
    },
    ShowCase {
        row: usize,
    },
    DeleteCase {
        row: usize,
    },
    ListCases,
    ClearCases,
    CasesDone,
}

/// What a move did: whether it was accepted and the message JFLAP shows for it (or an
/// explanation of why the move is not available, worded here, not by JFLAP).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reply {
    pub accepted: bool,
    pub message: String,
}

impl Reply {
    fn ok(message: impl Into<String>) -> Reply {
        Reply {
            accepted: true,
            message: message.into(),
        }
    }

    fn no(message: impl Into<String>) -> Reply {
        Reply {
            accepted: false,
            message: message.into(),
        }
    }
}

/// The verdict of a round: the pumped string and who won.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameResult {
    /// `x y^i z` or `u v^i x y^i z`.
    pub pumped: String,
    pub in_language: bool,
    /// You win when the pumped string is in the language ("You go first") or is not
    /// ("Computer goes first").
    pub user_wins: bool,
    /// The sentence JFLAP prints under the animation.
    pub message: String,
    /// JFLAP's follow-up question after a round in "You go first".
    pub takeaway: Option<String>,
}

/// What a won round shows about the language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Conclusion {
    /// The computer's decomposition was pumped out of the language and JFLAP knows no valid
    /// decomposition exists: the language is not regular.
    NotRegular,
    /// Same for the context-free lemma.
    NotContextFree,
}

/// The text behind the "Explain" button.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Explanation {
    /// Whether a valid decomposition of `w` exists.
    pub valid: bool,
    /// "A valid partition of w exists!" or "Unfortunately no valid partition of w exists."
    pub headline: String,
    /// JFLAP's explanation as HTML.
    pub explanation: String,
    /// "My Attempts", newest first, numbered from 1.
    pub attempts: Vec<(usize, String)>,
}

/// One of the five boxes of the game window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoxView {
    pub number: usize,
    pub caption: String,
    pub visible: bool,
    pub content: String,
    pub message: String,
}

/// The case panel (context-free, "You go first").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasePanel {
    pub message: String,
    /// Descriptions of the cases listed so far, in row order.
    pub rows: Vec<String>,
    pub total: usize,
    pub list_enabled: bool,
    pub add_replace_enabled: bool,
    pub(super) temp_decomposition: Option<Vec<i32>>,
    pub(super) temp_i: i32,
    pub selected: Option<usize>,
}

/// Everything a front end needs to draw the game.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    /// `L = {a^n b^n : n >= 0} Regular Pumping Lemma`.
    pub title: String,
    /// The same as the HTML JFLAP shows.
    pub html_title: String,
    pub objective: String,
    /// The line above the attempts ("File loaded." after opening a file).
    pub notice: String,
    pub mode: Mode,
    pub awaiting: Awaiting,
    /// The caption of the box the game waits on.
    pub prompt: String,
    pub boxes: Vec<BoxView>,
    pub pumped: String,
    pub attempts: Vec<String>,
    pub result: Option<GameResult>,
    pub cases: Option<CasePanel>,
}

/// A pumping lemma game.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    lemma: Lemma,
    mode: Mode,
    visible: [bool; 6],
    messages: [String; 6],
    pumped: String,
    result: Option<GameResult>,
    cases: Option<CasePanel>,
}

/// `PumpingLemmaInputPane.toHTMLString`: runs of one letter as `<i>a</i><sup>n</sup>`.
fn to_html_string(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 2 {
        return s.to_string();
    }
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let mut n = 1;
        while i + n < chars.len() && chars[i + n] == chars[i] {
            n += 1;
        }
        out.push_str(&format!("<i>{}</i>", chars[i]));
        if n > 1 {
            out.push_str(&format!("<sup>{n}</sup>"));
        }
        i += n;
    }
    out
}

fn java_len(s: &str) -> i32 {
    s.encode_utf16().count() as i32
}

impl Game {
    /// A new game on `language` (what the "Select" button of JFLAP's chooser does). `seed`
    /// feeds the computer's random choices.
    pub fn new(language: Language, mode: Mode, seed: i64) -> Game {
        let mut lemma = Lemma::new(language, seed);
        lemma.set_first_player(match mode {
            Mode::HumanFirst => "Human",
            Mode::ComputerFirst => "Computer",
        });
        let panel = mode == Mode::HumanFirst && lemma.case_count() > 1;
        let mut game = Game {
            lemma,
            mode,
            visible: [true, true, mode == Mode::ComputerFirst, false, false, false],
            messages: Default::default(),
            pumped: String::new(),
            result: None,
            cases: panel.then(|| CasePanel {
                message: String::new(),
                rows: Vec::new(),
                total: 0,
                list_enabled: false,
                add_replace_enabled: false,
                temp_decomposition: None,
                temp_i: -1,
                selected: None,
            }),
        };
        if mode == Mode::ComputerFirst {
            game.lemma.choose_m();
        }
        game.sync_cases();
        game
    }

    /// Resumes a stored game, as JFLAP does when it opens a file: `first_player` selects
    /// the mode ("Computer", anything else is "You go first").
    pub fn from_lemma(lemma: Lemma) -> Game {
        let mode = if lemma.first_player() == "Computer" {
            Mode::ComputerFirst
        } else {
            Mode::HumanFirst
        };
        let mut game = Game::new(lemma.language(), mode, 0);
        let (m, i) = (lemma.m(), lemma.i());
        let decomposition = lemma.decomposition().map(<[i32]>::to_vec);
        game.lemma = lemma;
        game.lemma.set_first_player(match mode {
            Mode::HumanFirst => "Human",
            Mode::ComputerFirst => "Computer",
        });
        game.messages[0] = "File loaded.".to_string();
        let cut = if game.lemma.kind() == Kind::Regular {
            2
        } else {
            4
        };
        let lengths: Vec<i32> = decomposition
            .as_deref()
            .map(|d| d.iter().take(cut).copied().collect())
            .unwrap_or_default();
        match mode {
            Mode::HumanFirst => {
                if m == -1 {
                    return game;
                }
                game.visible[2] = true;
                game.visible[3] = true;
                let parts_len: usize = [
                    game.lemma.u(),
                    game.lemma.v(),
                    game.lemma.x(),
                    game.lemma.y(),
                ]
                .iter()
                .map(|s| s.len())
                .sum();
                if parts_len == 0 {
                    game.messages[3] = game.violation_text(&[0; 4][..cut]).unwrap_or_default();
                } else {
                    game.visible[4] = true;
                    game.visible[5] = true;
                    game.messages[5] = "Click \"Restart\" to restart the animation.".to_string();
                    game.show_end();
                }
                if let Some(panel) = &mut game.cases {
                    panel.temp_decomposition = Some(lengths);
                    panel.temp_i = i;
                    panel.list_enabled = true;
                    panel.add_replace_enabled = game.visible[4];
                }
                game.sync_cases();
            }
            Mode::ComputerFirst => {
                let zero = lengths.iter().all(|&n| n == 0);
                if zero {
                    return game;
                }
                game.visible[3] = true;
                game.visible[4] = true;
                if i == -1 {
                    return game;
                }
                game.visible[5] = true;
                game.messages[5] = "Click \"Restart\" to restart the animation.".to_string();
                game.show_end();
            }
        }
        game
    }

    pub fn lemma(&self) -> &Lemma {
        &self.lemma
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The verdict of the last round, if one was played since `m` or the decomposition
    /// last changed.
    pub fn result(&self) -> Option<&GameResult> {
        self.result.as_ref()
    }

    /// Set when the last round was won against a language JFLAP knows is not regular /
    /// context-free.
    pub fn conclusion(&self) -> Option<Conclusion> {
        let result = self.result.as_ref()?;
        if self.mode == Mode::ComputerFirst && result.user_wins && !self.lemma.partition_is_valid()
        {
            return Some(match self.lemma.kind() {
                Kind::Regular => Conclusion::NotRegular,
                Kind::ContextFree => Conclusion::NotContextFree,
            });
        }
        None
    }

    /// What the game waits for.
    pub fn awaiting(&self) -> Awaiting {
        match self.mode {
            Mode::HumanFirst if !self.visible[3] => Awaiting::M,
            Mode::HumanFirst => Awaiting::Decomposition,
            Mode::ComputerFirst if !self.visible[3] => Awaiting::W,
            Mode::ComputerFirst => Awaiting::I,
        }
    }

    fn caption(&self, number: usize) -> String {
        let human = self.mode == Mode::HumanFirst;
        match number {
            1 => if human { PROMPT_M } else { DESCRIBE_M }.to_string(),
            2 => if human { DESCRIBE_W } else { PROMPT_W }.to_string(),
            3 if human => match self.lemma.kind() {
                Kind::Regular => "Select decomposition of w into xyz.",
                Kind::ContextFree => "Select decomposition of w into uvxyz.",
            }
            .to_string(),
            3 => DESCRIBE_DECOMPOSITION.to_string(),
            4 => if human { DESCRIBE_I } else { PROMPT_I }.to_string(),
            _ => "Animation".to_string(),
        }
    }

    fn title(&self) -> String {
        let kind = match self.lemma.kind() {
            Kind::Regular => "Regular",
            Kind::ContextFree => "Context-Free",
        };
        format!("L = {{{}}} {kind} Pumping Lemma", self.lemma.title())
    }

    /// A snapshot of everything the window shows.
    pub fn view(&self) -> View {
        let l = &self.lemma;
        let decomposition = if self.visible[3] && !l.parts.iter().all(|p| p.is_empty()) {
            l.decomposition_as_string()
        } else {
            String::new()
        };
        let contents = [
            if l.m() >= 0 {
                l.m().to_string()
            } else {
                String::new()
            },
            l.w().to_string(),
            decomposition,
            if self.visible[4] && l.i() >= 0 {
                l.i().to_string()
            } else {
                String::new()
            },
            self.result
                .as_ref()
                .map(|r| r.message.clone())
                .unwrap_or_default(),
        ];
        let boxes = (1..=5)
            .map(|n| BoxView {
                number: n,
                caption: self.caption(n),
                visible: self.visible[n],
                content: contents[n - 1].clone(),
                message: self.messages[n].clone(),
            })
            .collect();
        let awaiting = self.awaiting();
        let prompt_box = match awaiting {
            Awaiting::M => 1,
            Awaiting::W => 2,
            Awaiting::Decomposition => 3,
            Awaiting::I => 4,
        };
        View {
            title: self.title(),
            html_title: format!("L = {{{}}}", l.html_title()),
            objective: match self.mode {
                Mode::HumanFirst => OBJECTIVE_HUMAN,
                Mode::ComputerFirst => OBJECTIVE_COMPUTER,
            }
            .to_string(),
            notice: self.messages[0].clone(),
            mode: self.mode,
            awaiting,
            prompt: self.caption(prompt_box),
            boxes,
            pumped: self.pumped.clone(),
            attempts: l.attempts().to_vec(),
            result: self.result.clone(),
            cases: self.cases.clone(),
        }
    }

    /// The "Explain" button.
    pub fn explain(&self) -> Explanation {
        let valid = self.lemma.partition_is_valid();
        let attempts = self.lemma.attempts();
        Explanation {
            valid,
            headline: if valid {
                "A valid partition of w exists!"
            } else {
                "Unfortunately no valid partition of w exists."
            }
            .to_string(),
            explanation: self.lemma.explanation().to_string(),
            attempts: attempts
                .iter()
                .enumerate()
                .rev()
                .map(|(i, a)| (i + 1, a.clone()))
                .collect(),
        }
    }

    /// Plays a move.
    pub fn play(&mut self, mv: &Move) -> Reply {
        match mv {
            Move::ChooseM { m } => self.choose_m(*m),
            Move::ChooseW { w } => self.choose_w(w),
            Move::ChooseDecomposition { lengths } => self.choose_decomposition(lengths),
            Move::ChooseI { i } => self.choose_i(*i),
            Move::ClearAll => self.clear_all(),
            Move::AddCase => self.add_case(),
            Move::ReplaceCase { row } => self.replace_case(*row),
            Move::ShowCase { row } => self.show_case(*row),
            Move::DeleteCase { row } => self.delete_case(*row),
            Move::ListCases => self.list_cases(),
            Move::ClearCases => self.clear_cases(),
            Move::CasesDone => self.cases_done(),
        }
    }

    /// `PumpingLemmaInputPane.reset()`.
    fn reset_ui(&mut self) {
        self.lemma.reset();
        self.messages = Default::default();
        self.pumped.clear();
        self.result = None;
        if let Some(panel) = &mut self.cases {
            self.lemma.clear_done_cases();
            panel.message.clear();
            panel.list_enabled = false;
            panel.add_replace_enabled = false;
            panel.selected = None;
        }
        let from = if self.mode == Mode::HumanFirst { 2 } else { 3 };
        for v in &mut self.visible[from..] {
            *v = false;
        }
        self.sync_cases();
    }

    fn sync_cases(&mut self) {
        if let Some(panel) = &mut self.cases {
            panel.rows = self.lemma.done_descriptions();
            panel.total = self.lemma.case_count();
        }
    }

    /// The "Clear All" button (it keeps the list of attempts).
    pub fn clear_all(&mut self) -> Reply {
        self.reset_ui();
        if self.mode == Mode::ComputerFirst {
            self.lemma.choose_m();
        }
        Reply::ok("")
    }

    /// Box 1 of "You go first": enters `m`; the computer answers with a `w`.
    pub fn choose_m(&mut self, m: i32) -> Reply {
        if self.mode != Mode::HumanFirst {
            return Reply::no("The computer picks m in this mode.");
        }
        self.reset_ui();
        let (lo, hi) = self.lemma.range();
        if m < lo || m > hi {
            let message =
                format!("Please enter a positive integer in range [{lo}, {hi}] for best results.");
            self.messages[1] = message.clone();
            return Reply::no(message);
        }
        self.lemma.set_m(m);
        self.visible[2] = true;
        self.visible[3] = true;
        let zero = vec![
            0;
            if self.lemma.kind() == Kind::Regular {
                2
            } else {
                4
            }
        ];
        self.messages[3] = self.violation_text(&zero).unwrap_or_default();
        if let Some(panel) = &mut self.cases {
            panel.list_enabled = true;
        }
        Reply::ok("")
    }

    /// The reason JFLAP's sliders refuse a decomposition (`refresh()`), `None` when fine.
    fn violation_text(&self, lengths: &[i32]) -> Option<String> {
        let m = self.lemma.m();
        match self.lemma.kind() {
            Kind::Regular => {
                let (x, y) = (lengths[0], lengths[1]);
                if x >= x + y {
                    Some("Condition violated: |y| > 0".to_string())
                } else if x + y > m {
                    Some("Condition violated: |xy| <= m".to_string())
                } else {
                    None
                }
            }
            Kind::ContextFree => {
                let (u, v, x, y) = (lengths[0], lengths[1], lengths[2], lengths[3]);
                let (uv, vx, xy) = (u + v, u + v + x, u + v + x + y);
                if u > uv {
                    Some("Condition violated: |v| >= 0".to_string())
                } else if uv > vx {
                    Some("Condition violated: |x| >= 0".to_string())
                } else if vx > xy {
                    Some("Condition violated: |y| >= 0".to_string())
                } else if xy - u > m {
                    Some("Condition violated: |vxy| <= m".to_string())
                } else if (uv - u) + (xy - vx) < 1 {
                    Some("Condition violated: |vy| >= 1".to_string())
                } else {
                    None
                }
            }
        }
    }

    /// Box 3 of "You go first": splits `w` into `x y z` (`[|x|, |y|]`) or `u v x y z`
    /// (`[|u|, |v|, |x|, |y|]`) and presses "Set xyz" / "Set uvxyz": the computer picks
    /// `i` and the round is played.
    pub fn choose_decomposition(&mut self, lengths: &[i32]) -> Reply {
        if self.mode != Mode::HumanFirst {
            return Reply::no("The computer splits w in this mode.");
        }
        if !self.visible[3] {
            return Reply::no("Choose m first (box 1).");
        }
        let cut = if self.lemma.kind() == Kind::Regular {
            2
        } else {
            4
        };
        if lengths.len() != cut {
            return Reply::no(format!("Give {cut} lengths."));
        }
        // moving a slider clears the previous verdict
        self.pumped.clear();
        self.result = None;
        let total: i32 = lengths.iter().sum();
        if lengths.iter().any(|&n| n < 0) || total > java_len(self.lemma.w()) {
            let message = format!(
                "The pieces must be non-negative and fit inside w (|w| = {}).",
                java_len(self.lemma.w())
            );
            self.messages[3] = message.clone();
            return Reply::no(message);
        }
        if let Some(message) = self.violation_text(lengths) {
            self.messages[3] = message.clone();
            return Reply::no(message);
        }
        // "Set xyz": boxes 4 and 5 appear and the computer chooses i
        self.visible[4] = true;
        self.visible[5] = true;
        self.messages = Default::default();
        let mut full = lengths.to_vec();
        full.push(java_len(self.lemma.w()) - total);
        self.lemma.set_decomposition(&full);
        self.lemma.choose_i();
        let i = self.lemma.i();
        if let Some(panel) = &mut self.cases {
            panel.temp_decomposition = Some(full);
            panel.temp_i = i;
        }
        self.show_end();
        let in_language = self.lemma.is_in_lang(&self.lemma.create_pumped_string());
        let attempt = format!(
            "{}; I = {}; <i>{}</i>",
            self.lemma.decomposition_as_string(),
            i,
            if in_language { "Won" } else { "Failed" }
        );
        self.lemma.add_attempt(&attempt);
        if let Some(panel) = &mut self.cases {
            panel.add_replace_enabled = true;
        }
        self.sync_cases();
        Reply::ok(
            self.result
                .as_ref()
                .map(|r| r.message.clone())
                .unwrap_or_default(),
        )
    }

    /// Box 2 of "Computer goes first": enters `w`, which must be in the language and at
    /// least `m` long; the computer splits it.
    pub fn choose_w(&mut self, w: &str) -> Reply {
        if self.mode != Mode::ComputerFirst {
            return Reply::no("The computer picks w in this mode.");
        }
        for v in &mut self.visible[3..] {
            *v = false;
        }
        if self.lemma.is_in_lang(w) && java_len(w) >= self.lemma.m() {
            self.lemma.set_w(w);
            self.visible[3] = true;
            self.visible[4] = true;
            self.pumped.clear();
            self.result = None;
            self.lemma.choose_decomposition();
            self.messages = Default::default();
            Reply::ok(self.lemma.decomposition_as_string())
        } else {
            let message = if java_len(w) >= self.lemma.m() {
                "That string was not in the language.  Please enter another."
            } else {
                "Remember |w| must be >= m"
            };
            self.messages[2] = message.to_string();
            Reply::no(message)
        }
    }

    /// Box 4 of "Computer goes first": enters `i` (0, or 2 to 12) and pumps.
    pub fn choose_i(&mut self, i: i32) -> Reply {
        if self.mode != Mode::ComputerFirst {
            return Reply::no("The computer picks i in this mode.");
        }
        if !self.visible[4] {
            return Reply::no("Enter w first (box 2).");
        }
        if !(0..=12).contains(&i) || i == 1 {
            let message = "Please enter a positive integer in range [0, 2...12] for best results.";
            self.messages[4] = message.to_string();
            return Reply::no(message);
        }
        self.visible[5] = true;
        self.lemma.set_i(i);
        let in_language = self.lemma.is_in_lang(&self.lemma.create_pumped_string());
        let attempt = format!(
            "{}; I = {}; <i>{}</i>",
            self.lemma.decomposition_as_string(),
            i,
            if in_language { "Failed" } else { "Won" }
        );
        self.lemma.add_attempt(&attempt);
        self.messages = Default::default();
        self.show_end();
        Reply::ok(
            self.result
                .as_ref()
                .map(|r| r.message.clone())
                .unwrap_or_default(),
        )
    }

    /// `displayEnd()`: the pumped string and the sentence under the animation.
    fn show_end(&mut self) {
        let pumped = self.lemma.create_pumped_string();
        let in_language = self.lemma.is_in_lang(&pumped);
        let i = self.lemma.i();
        let xyz = match self.lemma.kind() {
            Kind::Regular => format!("<i>xy</i><sup>{i}</sup><i>z</i>"),
            Kind::ContextFree => {
                format!("<i>uv</i><sup>{i}</sup><i>xy</i><sup>{i}</sup><i>z</i>")
            }
        };
        let head = format!("{xyz} = {} = {pumped}", to_html_string(&pumped));
        let (message, user_wins, takeaway) = match self.mode {
            Mode::HumanFirst => {
                let regular = self.lemma.kind() == Kind::Regular;
                if in_language {
                    (
                        format!("{head} is in the language.  YOU WIN!"),
                        true,
                        if regular {
                            YOU_WIN_REGULAR
                        } else {
                            YOU_WIN_CONTEXT_FREE
                        },
                    )
                } else {
                    (
                        format!("{head} is NOT in the language.  Please try again."),
                        false,
                        if regular {
                            I_WIN_REGULAR
                        } else {
                            I_WIN_CONTEXT_FREE
                        },
                    )
                }
            }
            Mode::ComputerFirst => {
                if in_language {
                    (
                        format!("{head} is in the language.  Please try again."),
                        false,
                        "",
                    )
                } else {
                    (
                        format!("{head} is NOT in the language.  YOU WIN!"),
                        true,
                        "",
                    )
                }
            }
        };
        self.pumped = pumped.clone();
        self.result = Some(GameResult {
            pumped,
            in_language,
            user_wins,
            message,
            takeaway: (!takeaway.is_empty()).then(|| takeaway.to_string()),
        });
    }

    // ---- case panel ----

    fn panel(&mut self) -> Result<&mut CasePanel, Reply> {
        self.cases
            .as_mut()
            .ok_or_else(|| Reply::no("This game has no case panel."))
    }

    fn say(&mut self, message: impl Into<String>) -> String {
        let message = message.into();
        if let Some(panel) = &mut self.cases {
            panel.message = message.clone();
        }
        message
    }

    /// "List": shows every case.
    pub fn list_cases(&mut self) -> Reply {
        let (enabled, m) = match self.panel() {
            Ok(p) => (p.list_enabled, self.lemma.m()),
            Err(r) => return r,
        };
        if !enabled {
            return Reply::no("Choose m first (box 1).");
        }
        let message = if self.lemma.done_descriptions().len() == self.lemma.case_count() {
            format!("All cases for m = {m} are already shown.")
        } else {
            self.lemma.do_all();
            format!("All cases for m = {m} shown.")
        };
        self.sync_cases();
        Reply::ok(self.say(message))
    }

    /// "Clear": forgets every case.
    pub fn clear_cases(&mut self) -> Reply {
        if let Err(r) = self.panel() {
            return r;
        }
        if self.lemma.done_descriptions().is_empty() {
            return Reply::no("There are no cases to clear.");
        }
        self.lemma.clear_done_cases();
        self.sync_cases();
        if let Some(p) = &mut self.cases {
            p.selected = None;
        }
        Reply::ok(self.say("All cases cleared."))
    }

    /// "Delete": removes one row.
    pub fn delete_case(&mut self, row: usize) -> Reply {
        if let Err(r) = self.panel() {
            return r;
        }
        if !self.lemma.clear_case(row) {
            return Reply::no(self.say("Please select a case to clear."));
        }
        self.sync_cases();
        if let Some(p) = &mut self.cases {
            p.selected = None;
        }
        Reply::ok(self.say(format!("Case #{} deleted.", row + 1)))
    }

    /// "Show": loads a listed case into the game window.
    pub fn show_case(&mut self, row: usize) -> Reply {
        if let Err(r) = self.panel() {
            return r;
        }
        let Some(case) = self.lemma.case(row) else {
            return Reply::no(self.say("Please select a case to show."));
        };
        let input = case.input;
        self.pumped.clear();
        self.result = None;
        self.messages[3] = self
            .violation_text(&input)
            .unwrap_or_else(|| "Click \"Set uvxyz\" to set decomposition.".to_string());
        let shown_i = case.i != -1;
        if shown_i {
            self.lemma.set_decomposition_with_i(&input, case.i);
            self.show_end();
            self.visible[4] = true;
            self.visible[5] = true;
        } else {
            self.lemma.set_decomposition(&input);
            self.visible[4] = false;
            self.visible[5] = false;
        }
        self.sync_cases();
        if let Some(p) = &mut self.cases {
            p.add_replace_enabled = shown_i;
            p.selected = Some(row);
        }
        Reply::ok(self.say(format!("Showing case #{}.", row + 1)))
    }

    /// "Add": files the last round's decomposition under its case.
    pub fn add_case(&mut self) -> Reply {
        let (enabled, temp, temp_i) = match self.panel() {
            Ok(p) => (
                p.add_replace_enabled,
                p.temp_decomposition.clone(),
                p.temp_i,
            ),
            Err(r) => return r,
        };
        let Some(temp) = temp.filter(|_| enabled) else {
            return Reply::no("Play a decomposition first (box 3).");
        };
        let found = self.lemma.add_case(&temp, temp_i);
        self.sync_cases();
        let shown = self.lemma.done_descriptions().len() as i32;
        if found == -1 {
            return Reply::no(self.say("Illegal decomposition!"));
        }
        let (row, message) = if found >= shown {
            (shown - 1, "Case added.".to_string())
        } else {
            (found, format!("This case is similar to #{}.", found + 1))
        };
        if let Some(p) = &mut self.cases {
            p.selected = Some(row as usize);
        }
        Reply::ok(self.say(message))
    }

    /// "Replace": re-files a listed case with the last round's decomposition.
    pub fn replace_case(&mut self, row: usize) -> Reply {
        let (enabled, temp, temp_i) = match self.panel() {
            Ok(p) => (
                p.add_replace_enabled,
                p.temp_decomposition.clone(),
                p.temp_i,
            ),
            Err(r) => return r,
        };
        let Some(temp) = temp.filter(|_| enabled) else {
            return Reply::no("Play a decomposition first (box 3).");
        };
        if row >= self.lemma.done_descriptions().len() {
            return Reply::no(self.say("Please select a case to replace."));
        }
        if self.lemma.replace_case(&temp, temp_i, row) {
            Reply::ok(self.say(format!("Case #{} replaced.", row + 1)))
        } else {
            Reply::no(self.say("Wrong case selected."))
        }
    }

    /// "Done?": how many cases are left to list.
    pub fn cases_done(&mut self) -> Reply {
        if let Err(r) = self.panel() {
            return r;
        }
        if self.lemma.done_descriptions().is_empty() {
            return Reply::no("There are no cases yet.");
        }
        let left = self.lemma.case_count() - self.lemma.done_descriptions().len();
        let message = match left {
            0 => "All cases done.".to_string(),
            1 => "1 case left.".to_string(),
            n => format!("{n} cases left."),
        };
        Reply::ok(self.say(message))
    }
}
