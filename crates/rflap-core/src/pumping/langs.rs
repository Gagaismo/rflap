//! The per-language behaviour of JFLAP's predefined languages: the `w` the computer picks
//! for an `m`, its choice of decomposition and of `i`, and the membership test.
//!
//! Every function is a transcription of the Java method of the same name in
//! `pumping/reg/*.java` and `pumping/cf/*.java`, including the order in which the random
//! generator is drawn from.

use super::lemma::{Lemma, fetch_rand_int, pump_string};
use super::{CfLang, Language, RegLang};

fn count(s: &str, c: char) -> i32 {
    s.chars().filter(|&x| x == c).count() as i32
}

/// `LemmaMath.isMixture`: whether the letters of `s` do not follow the order of `chars`
/// (or are not among them).
fn is_mixture(s: &str, chars: &[char]) -> bool {
    let mut n = 0;
    for ch in s.chars() {
        if ch == ' ' {
            continue;
        }
        while n < chars.len() && ch != chars[n] {
            n += 1;
        }
        if n != chars.len() {
            continue;
        }
        return true;
    }
    false
}

/// `LemmaMath.otherCharactersFound`.
fn other_characters_found(s: &str, chars: &[char]) -> bool {
    s.chars().any(|c| !chars.contains(&c))
}

/// `String.indexOf(char)`, `-1` when absent.
fn index_of(s: &str, c: char) -> i32 {
    s.find(c).map_or(-1, |i| i as i32)
}

fn tail(s: &str, from: usize) -> &str {
    s.get(from..).unwrap_or("")
}

/// The three `c`-separated words of `getWs` and the rest.
fn get_ws(s: &str) -> Option<[String; 4]> {
    let mut rest = s;
    let mut ws: Vec<String> = Vec::new();
    for _ in 0..3 {
        let n = rest.find('c')?;
        ws.push(rest[..n].to_string());
        rest = &rest[n + 1..];
    }
    ws.push(rest.to_string());
    ws.try_into().ok()
}

/// `getVVr` of `w1 v v^R w2`: the positions of a palindromic `v v^R` of length >= 6 with
/// as many `a`s before as after.
fn get_vvr(s: &str) -> Option<[i32; 2]> {
    let b = s.as_bytes();
    let len = b.len() as i32;
    if len < 6 {
        return None;
    }
    for i in (5..len).rev() {
        for j in 0..=(i - 5) {
            if (i - j) % 2 != 1 || b[j as usize] != b[i as usize] {
                continue;
            }
            let mut ok = true;
            for k in 0..=((i - j) / 2) {
                if b[(j + k) as usize] != b[(i - k) as usize] {
                    ok = false;
                }
            }
            if !ok || count(&s[..j as usize], 'a') != count(&s[(i + 1) as usize..], 'a') {
                continue;
            }
            return Some([j, i]);
        }
    }
    None
}

impl Lemma {
    fn flip_coin(&mut self) -> i32 {
        if self.random.next_boolean() { 0 } else { 2 }
    }

    fn rand(&mut self, lo: i32, hi: i32) -> i32 {
        fetch_rand_int(&mut self.random, lo, hi)
    }

    /// `chooseW()`: the `w` the computer picks for the current `m`.
    pub(super) fn choose_w(&mut self) {
        let m = self.m;
        let p = pump_string;
        self.w = match self.language {
            Language::Regular(l) => match l {
                RegLang::AnBn => p("a", m) + &p("b", m),
                RegLang::NaNb => p("a", m) + &p("b", m + 1),
                RegLang::Palindrome => p("a", m) + "bb" + &p("a", m),
                RegLang::ABnAk => p("ab", m + 1) + &p("a", m),
                RegLang::AnBkCnk => p("a", m) + &p("b", m) + &p("c", m * 2),
                RegLang::AnBlAk => {
                    if m <= 3 {
                        p("a", 6) + &p("b", 4) + &p("a", 4)
                    } else {
                        p("a", 6) + &p("b", m) + &p("a", m)
                    }
                }
                RegLang::AnEven => {
                    if m % 2 == 0 {
                        p("a", m)
                    } else {
                        p("a", m + 1)
                    }
                }
                RegLang::AnBk => {
                    if m % 2 == 0 {
                        p("a", m - 2) + "bb"
                    } else {
                        "a".to_string() + &p("b", m)
                    }
                }
                RegLang::BBABAnAn => "bba".to_string() + &p("ba", m) + &p("a", m - 1),
                RegLang::B5W => {
                    let n = m - 5;
                    "bbbbb".to_string() + &p("b", 2 * n) + &p("a", 3 * n)
                }
                RegLang::B5Wmod => {
                    let mut w = "bbbbb".to_string();
                    let mut i = 5;
                    while i < m || (i - 5) % 3 != 0 {
                        w.push(if self.flip_coin() == 2 { 'a' } else { 'b' });
                        i += 1;
                    }
                    w
                }
                RegLang::BkABnBAn => {
                    let n = m / 2;
                    "bbbb".to_string() + &p("ab", n) + &p("ba", n)
                }
                RegLang::AB2n => {
                    if m % 2 == 0 {
                        p("ab", m)
                    } else {
                        p("ab", m + 1)
                    }
                }
            },
            Language::ContextFree(l) => match l {
                CfLang::AnBnCn => p("a", m) + &p("b", m) + &p("c", m),
                CfLang::WW | CfLang::AnBjAnBj => p("a", m) + &p("b", m) + &p("a", m) + &p("b", m),
                CfLang::NaNbNc => p("a", m) + &p("b", m + 1) + &p("c", m + 2),
                CfLang::NagNbeNc | CfLang::AiBjCk => p("a", m + 1) + &p("b", m) + &p("c", m),
                CfLang::AnBn => p("a", m) + &p("b", m),
                CfLang::AkBnCnDj => p("a", m) + &p("b", m) + &p("c", m) + &p("d", m + 1),
                CfLang::WW1WrGrtrThanEq => p("a", m) + "babab" + &p("a", m),
                CfLang::WW1WrEquals => p("a", m) + &p("b", m) + &p("a", m),
                CfLang::W1BnW2 => p("a", m) + &p("b", m + 1) + &p("a", m + 1),
                CfLang::W1CW2CW3CW4 => {
                    p("a", m) + &p("b", m) + "c" + &p("a", m) + &p("b", m) + "cacb"
                }
                CfLang::W1VVrW2 => {
                    let n = m / 2;
                    p("ab", n) + "abbbba" + &p("ab", n)
                }
            },
        };
    }

    /// `chooseI()`: the `i` the computer picks to break (or keep) the pumped string.
    pub fn choose_i(&mut self) {
        self.i = match self.language {
            Language::Regular(l) => match l {
                RegLang::ABnAk | RegLang::AnBlAk | RegLang::B5Wmod | RegLang::BkABnBAn => 0,
                RegLang::NaNb => 2,
                _ => self.flip_coin(),
            },
            Language::ContextFree(l) => match l {
                CfLang::AnBn => 2,
                CfLang::WW | CfLang::AnBjAnBj | CfLang::W1CW2CW3CW4 => 0,
                CfLang::W1VVrW2 => 3,
                CfLang::NaNbNc => {
                    let n = count(self.v(), 'c') + count(self.y(), 'c');
                    if n > 0 { 0 } else { 2 }
                }
                CfLang::NagNbeNc | CfLang::AiBjCk => {
                    if !self.v().contains('a') && !self.y().contains('a') {
                        2
                    } else {
                        0
                    }
                }
                CfLang::AkBnCnDj => {
                    let n = count(self.v(), 'a') + count(self.y(), 'a');
                    if n == 1 { 2 } else { 0 }
                }
                CfLang::W1BnW2 => {
                    if (self.u().len() as i32) < self.m {
                        2
                    } else {
                        0
                    }
                }
                CfLang::AnBnCn | CfLang::WW1WrGrtrThanEq | CfLang::WW1WrEquals => self.flip_coin(),
            },
        };
    }

    /// `chooseDecomposition()`: the decomposition the computer picks for the current `w`.
    pub fn choose_decomposition(&mut self) {
        let m = self.m;
        let len = self.w.len() as i32;
        match self.language {
            Language::Regular(l) => self.choose_regular_decomposition(l),
            Language::ContextFree(l) => self.choose_cf_decomposition(l, m, len),
        }
    }

    /// The default `RegularPumpingLemma.chooseDecomposition`.
    fn default_regular_decomposition(&mut self) {
        let n = self.rand(0, self.m - 1);
        let m = self.m;
        self.set_decomposition(&[n, m - n]);
    }

    fn choose_regular_decomposition(&mut self, l: RegLang) {
        let m = self.m;
        match l {
            RegLang::AnBn | RegLang::AnBkCnk => self.default_regular_decomposition(),
            RegLang::NaNb => {
                let n = (m - 1).min(index_of(&self.w, 'b'));
                self.set_decomposition(&[n, 1]);
            }
            RegLang::Palindrome => {
                let n = (self.w.len() as i32 / 2 - 1).min(m - 2);
                self.set_decomposition(&[n, 2]);
            }
            RegLang::ABnAk => {
                self.set_decomposition(&[0, 2]);
            }
            RegLang::AnBlAk => {
                let n = index_of(&self.w, 'b');
                if n > 6 {
                    self.set_decomposition(&[0, 1]);
                } else {
                    self.set_decomposition(&[n.min(m - 1), 1]);
                }
            }
            RegLang::AnEven => {
                self.set_decomposition(&[0, 2]);
            }
            RegLang::AnBk => {
                let n = index_of(&self.w, 'b');
                if n == -1 || n >= 2 {
                    self.set_decomposition(&[0, 2]);
                } else {
                    self.set_decomposition(&[n, 2]);
                }
            }
            RegLang::BBABAnAn => {
                self.set_decomposition(&[1, 2]);
            }
            RegLang::B5W => {
                let w = self.w.as_bytes();
                let mut n = 5usize;
                let mut found = 0;
                while found < 3 {
                    match w.get(n) {
                        Some(b'a') => found += 1,
                        Some(_) => {}
                        // JFLAP throws StringIndexOutOfBoundsException
                        None => return,
                    }
                    n += 1;
                }
                let n = n as i32;
                self.set_decomposition(&[(n - 5).min(m - 5), 5]);
            }
            RegLang::B5Wmod => {
                self.set_decomposition(&[5, 3]);
            }
            RegLang::BkABnBAn => {
                let n = index_of(&self.w, 'a');
                let len = self.w.len() as i32;
                let n2 = n + (len - n) / 2 - 2;
                if n > 4 {
                    self.set_decomposition(&[0, 1]);
                } else if n2 + 4 <= m {
                    self.set_decomposition(&[n2, 4]);
                } else {
                    self.default_regular_decomposition();
                }
            }
            RegLang::AB2n => {
                self.set_decomposition(&[0, 4]);
            }
        }
    }

    /// The default `ContextFreePumpingLemma.chooseDecomposition`.
    fn default_cf_decomposition(&mut self) {
        let len = self.w.len() as i32;
        let m = self.m;
        let mut d = [0i32; 4];
        d[0] = self.rand(0, len - 1);
        let mut n = d[0];
        let mut n2 = (len - n).min(m);
        d[1] = self.rand(0, n2);
        if d[1] == len - n {
            d[2] = 0;
            d[3] = 0;
        } else {
            n += d[1];
            n2 = (len - n - 1).min(m - 1);
            d[2] = self.rand(0, n2);
            n += d[2];
            n2 = (len - n).min(m - d[2]);
            d[3] = if d[1] > 0 {
                self.rand(0, n2)
            } else {
                self.rand(1, n2)
            };
        }
        self.set_decomposition(&d);
    }

    fn choose_cf_decomposition(&mut self, l: CfLang, m: i32, len: i32) {
        match l {
            CfLang::AnBnCn => self.default_cf_decomposition(),
            CfLang::AnBn => {
                self.set_decomposition(&[len / 2 - 1, 1, 0, 1]);
            }
            CfLang::WW => self.choose_ww_decomposition(m, len),
            CfLang::AnBjAnBj => {
                let mut n = len / 2 - 1;
                if n > m - 2 {
                    n = m - 2;
                }
                if self.w.contains('a') {
                    self.set_decomposition(&[0, 1, n, 1]);
                } else {
                    let b = index_of(&self.w, 'b');
                    self.set_decomposition(&[b, 1, n, 1]);
                }
            }
            CfLang::NaNbNc => {
                let na = count(&self.w, 'a');
                let nb = count(&self.w, 'b');
                let nc = count(&self.w, 'c');
                if nc > nb + 1 {
                    let c = index_of(&self.w, 'c');
                    self.set_decomposition(&[c, 1, 0, 0]);
                } else if nb > na + 1 {
                    let b = index_of(&self.w, 'b');
                    self.set_decomposition(&[b, 1, 0, 0]);
                } else {
                    self.default_cf_decomposition();
                }
            }
            CfLang::NagNbeNc => {
                let na = count(&self.w, 'a');
                let nb = count(&self.w, 'b');
                if na > nb + 1 {
                    self.set_decomposition(&[0, 1, 0, 0]);
                } else {
                    self.default_cf_decomposition();
                }
            }
            CfLang::AiBjCk => {
                let na = count(&self.w, 'a');
                let nb = count(&self.w, 'b');
                let nc = count(&self.w, 'c');
                if na > nb + 1 && na > nc + 1 {
                    self.set_decomposition(&[0, 1, 0, 0]);
                } else {
                    self.default_cf_decomposition();
                }
            }
            CfLang::AkBnCnDj => {
                let a = index_of(&self.w, 'a');
                let b = index_of(&self.w, 'b');
                let c = index_of(&self.w, 'c');
                let d = index_of(&self.w, 'd');
                if b > -1 && c > -1 {
                    self.set_decomposition(&[c - 1, 1, 0, 1]);
                } else if a > -1 && d > -1 {
                    self.set_decomposition(&[d - 1, 1, 0, 1]);
                } else {
                    self.default_cf_decomposition();
                }
            }
            CfLang::WW1WrGrtrThanEq => {
                self.set_decomposition(&[len / 2, 1, 0, 0]);
            }
            CfLang::WW1WrEquals => {
                self.set_decomposition(&[0, 1, 0, 0]);
            }
            CfLang::W1BnW2 => {
                for i in (0..len).rev() {
                    let mut without = self.w.clone();
                    without.remove(i as usize);
                    if self.is_in_lang(&without) {
                        self.set_decomposition(&[i, 1, 0, 0]);
                        return;
                    }
                }
                self.default_cf_decomposition();
            }
            CfLang::W1CW2CW3CW4 => {
                let Some(ws) = get_ws(&self.w) else {
                    // JFLAP throws NullPointerException
                    return;
                };
                if let Some(d) = self.check_if_possibility(&ws[0], &ws[1], 0) {
                    self.set_decomposition(&d);
                    return;
                }
                let offset = (ws[0].len() + ws[1].len() + 2) as i32;
                if let Some(d) = self.check_if_possibility(&ws[2], &ws[3], offset) {
                    self.set_decomposition(&d);
                    return;
                }
                self.default_cf_decomposition();
            }
            CfLang::W1VVrW2 => {
                let Some([j, i]) = get_vvr(&self.w) else {
                    // JFLAP throws NullPointerException
                    return;
                };
                let s1 = &self.w[..j as usize];
                let s2 = &self.w[(i + 1) as usize..];
                let d = if i - j > 5 || (count(s1, 'b') == 0 && count(s2, 'b') == 0) {
                    [j + (i - j) / 2, 1, 0, 1]
                } else if s1.contains('b') {
                    [index_of(s1, 'b'), 1, 0, 0]
                } else {
                    [index_of(s2, 'b') + i + 1, 1, 0, 0]
                };
                self.set_decomposition(&d);
            }
        }
    }

    /// `WW.chooseDecomposition`.
    fn choose_ww_decomposition(&mut self, m: i32, len: i32) {
        let n = len / 2;
        if m > n {
            self.set_decomposition(&[0, 1, n - 1, 1]);
            return;
        }
        let w = self.w.clone();
        let b = w.as_bytes();
        let doubled = |at: usize| -> String {
            let mut s = w[..at].to_string();
            s.push(b[at] as char);
            s.push(b[at] as char);
            s.push_str(&w[at + 1..n as usize]);
            s
        };
        for i in (n - m + 1)..n {
            for j in 0..i {
                if b[i as usize] != b[j as usize] {
                    continue;
                }
                let s1 = doubled(i as usize);
                let s2 = doubled(j as usize);
                let n2 = n - i + j - 1;
                if s1 != s2 || n2 > m - 2 {
                    continue;
                }
                self.set_decomposition(&[i, 1, n2, 1]);
                return;
            }
        }
        self.default_cf_decomposition();
    }

    /// `W1CW2CW3CW4.checkIfPossibility`.
    fn check_if_possibility(&self, s: &str, s2: &str, n: i32) -> Option<[i32; 4]> {
        let (l1, l2) = (s.len() as i32, s2.len() as i32);
        if l1 == 1 && l2 == 1 {
            return None;
        }
        if l1 == 1 {
            return Some([n + l1 + 1, 1, 0, 0]);
        }
        if l2 == 1 || s != s2 {
            return Some([n, 1, 0, 0]);
        }
        if self.m >= l2 + 2 {
            return Some([n, 1, l1, 1]);
        }
        let b = s2.as_bytes();
        for i in (l2 - self.m + 2)..l2 {
            for j in 0..i {
                if b[i as usize] != b[j as usize] {
                    continue;
                }
                let double = |at: usize| {
                    let mut t = s2[..at].to_string();
                    t.push(b[at] as char);
                    t.push(b[at] as char);
                    t.push_str(&s2[at + 1..]);
                    t
                };
                let s3 = double(i as usize);
                let s4 = double(j as usize);
                let n2 = l2 - i + j;
                if s3 != s4 || n2 > self.m - 2 {
                    continue;
                }
                return Some([n + i, 1, n2, 1]);
            }
        }
        None
    }

    /// `isInLang(s)`: membership in the language. (For `a^n : n even` the empty string is
    /// judged by the current pumped string, as JFLAP does.)
    pub fn is_in_lang(&self, s: &str) -> bool {
        match self.language {
            Language::Regular(l) => self.regular_in_lang(l, s),
            Language::ContextFree(l) => self.cf_in_lang(l, s),
        }
    }

    fn regular_in_lang(&self, l: RegLang, s: &str) -> bool {
        match l {
            RegLang::AnBn => !is_mixture(s, &['a', 'b']) && count(s, 'a') == count(s, 'b'),
            RegLang::NaNb => {
                !other_characters_found(s, &['a', 'b']) && count(s, 'a') < count(s, 'b')
            }
            RegLang::Palindrome => {
                let b = s.as_bytes();
                let n = b.len();
                if n == 0 {
                    return true;
                }
                if n % 2 == 1 {
                    return false;
                }
                if other_characters_found(s, &['a', 'b']) {
                    return false;
                }
                (0..=n / 2).all(|i| b[i] == b[n - i - 1])
            }
            RegLang::ABnAk => {
                let mut rest = s;
                while let Some(r) = rest.strip_prefix("ab") {
                    rest = r;
                }
                if rest == "ab" {
                    return true;
                }
                if is_mixture(rest, &['a']) {
                    return false;
                }
                count(s, 'a') < 2 * count(s, 'b')
            }
            RegLang::AnBkCnk => {
                !is_mixture(s, &['a', 'b', 'c']) && count(s, 'a') + count(s, 'b') == count(s, 'c')
            }
            RegLang::AnBlAk => {
                if is_mixture(s, &['a', 'b', 'a']) {
                    return false;
                }
                let nb = count(s, 'b');
                if nb == 0 {
                    return false;
                }
                let tail_b = tail(s, s.find('b').unwrap_or(0));
                let n2 = (s.len() - tail_b.len()) as i32;
                let n3 = count(tail_b, 'a');
                n2 > 5 && nb > 3 && n3 <= nb
            }
            RegLang::AnEven => {
                if other_characters_found(s, &['a']) {
                    return false;
                }
                let t = if s.is_empty() {
                    self.create_pumped_string()
                } else {
                    s.to_string()
                };
                t.len() % 2 == 0
            }
            RegLang::AnBk => {
                if is_mixture(s, &['a', 'b']) {
                    return false;
                }
                count(s, 'a') % 2 == 1 || count(s, 'b') % 2 == 0
            }
            RegLang::BBABAnAn => {
                let Some(mut rest) = s.strip_prefix("bba") else {
                    return false;
                };
                let mut n = 0;
                while let Some(r) = rest.strip_prefix("ba") {
                    rest = r;
                    n += 1;
                }
                while let Some(r) = rest.strip_prefix('a') {
                    rest = r;
                    n -= 1;
                }
                n == 1 && rest.is_empty()
            }
            RegLang::B5W => {
                if other_characters_found(s, &['a', 'b']) {
                    return false;
                }
                let Some(rest) = s.strip_prefix("bbbbb") else {
                    return false;
                };
                2 * count(rest, 'a') == 3 * count(rest, 'b')
            }
            RegLang::B5Wmod => {
                if other_characters_found(s, &['a', 'b']) {
                    return false;
                }
                let Some(rest) = s.strip_prefix("bbbbb") else {
                    return false;
                };
                (2 * count(rest, 'a') + 5 * count(rest, 'b')) % 3 == 0
            }
            RegLang::BkABnBAn => {
                let n = index_of(s, 'a');
                if n < 4 {
                    return false;
                }
                let mut rest = &s[n as usize..];
                if !rest.starts_with("ab") {
                    return false;
                }
                let mut n2 = 0;
                while let Some(r) = rest.strip_prefix("ab") {
                    rest = r;
                    n2 += 1;
                }
                while let Some(r) = rest.strip_prefix("ba") {
                    rest = r;
                    n2 -= 1;
                }
                n2 == 0 && rest.is_empty()
            }
            RegLang::AB2n => {
                let mut rest = s;
                let mut n = 0;
                while let Some(r) = rest.strip_prefix("ab") {
                    rest = r;
                    n += 1;
                }
                n > 0 && n % 2 == 0 && rest.is_empty()
            }
        }
    }

    fn cf_in_lang(&self, l: CfLang, s: &str) -> bool {
        match l {
            CfLang::AnBnCn => {
                if is_mixture(s, &['a', 'b', 'c']) {
                    return false;
                }
                let (a, b, c) = (count(s, 'a'), count(s, 'b'), count(s, 'c'));
                a == b && a == c
            }
            CfLang::WW => {
                if other_characters_found(s, &['a', 'b']) {
                    return false;
                }
                let n = s.len() / 2;
                s[..n] == s[n..]
            }
            CfLang::AnBjAnBj => {
                let n = s.len() / 2;
                let (Some(left), Some(right)) = (s.get(..n), s.get(n..)) else {
                    return false;
                };
                if left != right {
                    return false;
                }
                !is_mixture(left, &['a', 'b']) && !is_mixture(right, &['a', 'b'])
            }
            CfLang::NaNbNc => {
                if other_characters_found(s, &['a', 'b', 'c']) {
                    return false;
                }
                let (a, b, c) = (count(s, 'a'), count(s, 'b'), count(s, 'c'));
                a < b && b < c
            }
            CfLang::NagNbeNc => {
                if other_characters_found(s, &['a', 'b', 'c']) {
                    return false;
                }
                let (a, b, c) = (count(s, 'a'), count(s, 'b'), count(s, 'c'));
                a > b && b == c
            }
            CfLang::AiBjCk => {
                if is_mixture(s, &['a', 'b', 'c']) {
                    return false;
                }
                let (a, b, c) = (count(s, 'a'), count(s, 'b'), count(s, 'c'));
                a > b && a > c
            }
            CfLang::AnBn => !is_mixture(s, &['a', 'b']) && count(s, 'a') == count(s, 'b'),
            CfLang::AkBnCnDj => {
                let letters = ['a', 'b', 'c', 'd'];
                if is_mixture(s, &letters) {
                    return false;
                }
                let mut runs = [0i32; 4];
                let mut at = 0;
                for ch in s.chars() {
                    while at < 4 && ch != letters[at] {
                        at += 1;
                    }
                    if at == 4 {
                        return false;
                    }
                    runs[at] += 1;
                }
                runs[1] == runs[2] && runs[0] != runs[3]
            }
            CfLang::WW1WrGrtrThanEq => {
                if other_characters_found(s, &['a', 'b']) {
                    return false;
                }
                s.len() >= 5
            }
            CfLang::WW1WrEquals => {
                if other_characters_found(s, &['a', 'b']) || s.is_empty() {
                    return false;
                }
                let b = s.as_bytes();
                let mut n: i32 = 0;
                let mut n2: i32 = b.len() as i32 - 1;
                while b[n as usize] == b[n2 as usize] && n < n2 {
                    n += 1;
                    n2 -= 1;
                    if n == n2 - n + 1 {
                        return true;
                    }
                }
                false
            }
            CfLang::W1BnW2 => {
                if other_characters_found(s, &['a', 'b']) {
                    return false;
                }
                let b = s.as_bytes();
                let len = b.len();
                let mut i = 0;
                while i < len {
                    if b[i] != b'b' {
                        i += 1;
                        continue;
                    }
                    let mut run = 0;
                    while i < len && b[i] == b'b' {
                        run += 1;
                        i += 1;
                    }
                    let before = &s[..i - run];
                    let after = if i != len { &s[i..] } else { "" };
                    let n2 = count(before, 'a') as usize;
                    let n = count(after, 'a') as usize;
                    if !(n2 >= n || run <= n2) {
                        return true;
                    }
                    i += 1;
                }
                false
            }
            CfLang::W1CW2CW3CW4 => {
                let Some(ws) = get_ws(s) else {
                    return false;
                };
                for w in &ws {
                    if other_characters_found(w, &['a', 'b']) || w.is_empty() {
                        return false;
                    }
                }
                ws[0] == ws[1] || ws[2] == ws[3]
            }
            CfLang::W1VVrW2 => {
                if other_characters_found(s, &['a', 'b']) {
                    return false;
                }
                get_vvr(s).is_some()
            }
        }
    }
}
