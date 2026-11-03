//! The turtle (JFLAP `gui.lsystem.Turtle` and `gui.transform.Matrix`).
//!
//! The turtle's pose is a 3x4 matrix that maps the world into the turtle's frame; the
//! position is `R^T t`. Turning pre-multiplies a rotation, moving pre-multiplies a
//! translation. The sums are written in the same order as JFLAP so results agree bit
//! for bit apart from `sin`/`cos`.

use std::collections::HashMap;

use super::Rgb;

const DEGREES_TO_RADIANS: f64 = 0.017453292519943295;

/// A 3x4 pose; the fourth row is always `(0, 0, 0, 1)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    e: [[f64; 4]; 3],
}

impl Default for Matrix {
    fn default() -> Self {
        Matrix {
            e: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
        }
    }
}

impl Matrix {
    fn premultiply(&mut self, m: &[[f64; 4]; 3]) {
        let row3 = [0.0, 0.0, 0.0, 1.0];
        let mut out = [[0.0; 4]; 3];
        for (row, mrow) in out.iter_mut().zip(m) {
            for (j, slot) in row.iter_mut().enumerate() {
                let mut acc = 0.0;
                for (k, factor) in mrow.iter().enumerate() {
                    let b = if k == 3 { row3[j] } else { self.e[k][j] };
                    acc += factor * b;
                }
                *slot = acc;
            }
        }
        self.e = out;
    }

    /// Rotation about the x axis (`Matrix.pitch`).
    pub fn pitch(&mut self, degrees: f64) {
        let (s, c) = (degrees * DEGREES_TO_RADIANS).sin_cos();
        self.premultiply(&[[1.0, 0.0, 0.0, 0.0], [0.0, c, -s, 0.0], [0.0, s, c, 0.0]]);
    }

    /// Rotation about the y axis, the heading (`Matrix.roll`).
    pub fn roll(&mut self, degrees: f64) {
        let (s, c) = (degrees * DEGREES_TO_RADIANS).sin_cos();
        self.premultiply(&[[c, 0.0, s, 0.0], [0.0, 1.0, 0.0, 0.0], [-s, 0.0, c, 0.0]]);
    }

    /// Rotation about the z axis, a turn in the drawing plane (`Matrix.yaw`).
    pub fn yaw(&mut self, degrees: f64) {
        let (s, c) = (degrees * DEGREES_TO_RADIANS).sin_cos();
        self.premultiply(&[[c, -s, 0.0, 0.0], [s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]]);
    }

    pub fn translate(&mut self, x: f64, y: f64, z: f64) {
        self.premultiply(&[[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]);
    }

    /// The turtle's position in the world, `R^T t` (`Matrix.origin`).
    pub fn origin(&self) -> [f64; 3] {
        let mut out = [0.0; 3];
        for (n, slot) in out.iter_mut().enumerate() {
            let mut acc = 0.0;
            for i in 0..3 {
                acc += self.e[i][n] * self.e[i][3];
            }
            acc += 0.0 * 1.0;
            *slot = acc;
        }
        out
    }
}

/// `Math.IEEEremainder(x, 360.0)`.
pub(crate) fn ieee_remainder_360(x: f64) -> f64 {
    let y = 360.0;
    if x.is_nan() || x.is_infinite() {
        return f64::NAN;
    }
    let mut r = x % y;
    let half = y / 2.0;
    if r.abs() > half {
        r -= y.copysign(r);
    } else if r.abs() == half {
        let n = ((x - r) / y).round();
        if n % 2.0 != 0.0 {
            r -= y.copysign(r);
        }
    }
    r
}

/// `java.awt.Color.RGBtoHSB`.
fn rgb_to_hsb(r: u8, g: u8, b: u8) -> [f32; 3] {
    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    let brightness = cmax as f32 / 255.0;
    let saturation = if cmax != 0 {
        (cmax - cmin) as f32 / cmax as f32
    } else {
        0.0
    };
    let hue = if saturation == 0.0 {
        0.0
    } else {
        let span = (cmax - cmin) as f32;
        let redc = (cmax - r) as f32 / span;
        let greenc = (cmax - g) as f32 / span;
        let bluec = (cmax - b) as f32 / span;
        let mut h = if r == cmax {
            bluec - greenc
        } else if g == cmax {
            2.0 + redc - bluec
        } else {
            4.0 + greenc - redc
        };
        h /= 6.0;
        if h < 0.0 {
            h += 1.0;
        }
        h
    };
    [hue, saturation, brightness]
}

/// `java.awt.Color.HSBtoRGB` (`getHSBColor`).
fn hsb_to_rgb(hue: f32, saturation: f32, brightness: f32) -> Rgb {
    let scale = |v: f32| (v * 255.0 + 0.5) as i32 as u8;
    if saturation == 0.0 {
        let v = scale(brightness);
        return Rgb(v, v, v);
    }
    let h = (hue - hue.floor()) * 6.0;
    let f = h - h.floor();
    let p = brightness * (1.0 - saturation);
    let q = brightness * (1.0 - saturation * f);
    let t = brightness * (1.0 - saturation * (1.0 - f));
    let (r, g, b) = match h as i32 {
        0 => (brightness, t, p),
        1 => (q, brightness, p),
        2 => (p, brightness, t),
        3 => (p, q, brightness),
        4 => (t, p, brightness),
        _ => (brightness, p, q),
    };
    Rgb(scale(r), scale(g), scale(b))
}

/// `Turtle.COLORS`, the extra names JFLAP adds to the `java.awt.Color` constants.
fn named_color(name: &str) -> Option<Rgb> {
    Some(match name {
        // public static fields of java.awt.Color
        "white" | "WHITE" => Rgb(255, 255, 255),
        "lightGray" | "LIGHT_GRAY" => Rgb(192, 192, 192),
        "gray" | "GRAY" => Rgb(128, 128, 128),
        "darkGray" | "DARK_GRAY" => Rgb(64, 64, 64),
        "black" | "BLACK" => Rgb(0, 0, 0),
        "red" | "RED" => Rgb(255, 0, 0),
        "pink" | "PINK" => Rgb(255, 175, 175),
        "orange" | "ORANGE" => Rgb(255, 200, 0),
        "yellow" | "YELLOW" => Rgb(255, 255, 0),
        "green" | "GREEN" => Rgb(0, 255, 0),
        "magenta" | "MAGENTA" => Rgb(255, 0, 255),
        "cyan" | "CYAN" => Rgb(0, 255, 255),
        "blue" | "BLUE" => Rgb(0, 0, 255),
        // Turtle.COLORS
        "dukeBlue" => Rgb(0, 0, 156),
        "brown" => Rgb(129, 0, 0),
        "oliveDrab" => Rgb(114, 93, 0),
        "darkOliveGreen" => Rgb(109, 111, 0),
        "orangeRed" => Rgb(252, 118, 0),
        "maroon" => Rgb(190, 0, 0),
        "forestGreen" => Rgb(0, 127, 0),
        "purple" => Rgb(209, 0, 255),
        "springGreen" => Rgb(193, 255, 157),
        "violetRed" => Rgb(210, 0, 205),
        "goldenrod" => Rgb(255, 214, 0),
        "darkOliveGreen2" => Rgb(10, 127, 0),
        _ => return None,
    })
}

/// `Float.parseFloat` for the numbers a colour triple holds.
fn parse_float(s: &str) -> Option<f32> {
    java_parse_double(s).map(|d| d as f32)
}

/// `Turtle.colorForString`: a colour name or a `r,g,b` triple (0-255, or HSB when all are <= 1).
pub(crate) fn color_for_string(s: &str) -> Option<Rgb> {
    if let Some(c) = named_color(s) {
        return Some(c);
    }
    let mut parts = s.split(',').filter(|p| !p.is_empty());
    let r = parse_float(parts.next()?)?;
    let g = parse_float(parts.next()?)?;
    let b = parse_float(parts.next()?)?;
    if !(0.0..=255.0).contains(&r) || !(0.0..=255.0).contains(&g) || !(0.0..=255.0).contains(&b) {
        return None;
    }
    if r <= 1.0 && g <= 1.0 && b <= 1.0 {
        return Some(hsb_to_rgb(r, g, b));
    }
    Some(Rgb(r as u8, g as u8, b as u8))
}

/// `Double.parseDouble`.
pub(crate) fn java_parse_double(s: &str) -> Option<f64> {
    let t = s.trim();
    let t = t
        .strip_suffix(['f', 'F', 'd', 'D'])
        .filter(|rest| !rest.is_empty() && !rest.ends_with(|c: char| c.is_ascii_alphabetic()))
        .unwrap_or(t);
    match t {
        "NaN" | "+NaN" | "-NaN" => return Some(f64::NAN),
        "Infinity" | "+Infinity" => return Some(f64::INFINITY),
        "-Infinity" => return Some(f64::NEG_INFINITY),
        _ => {}
    }
    if t.is_empty()
        || !t
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E'))
    {
        return None;
    }
    t.parse::<f64>().ok()
}

/// `Double.toString`.
pub(crate) fn java_double_to_string(d: f64) -> String {
    if d.is_nan() {
        return "NaN".into();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if d == 0.0 {
        return if d.is_sign_negative() { "-0.0" } else { "0.0" }.into();
    }
    let abs = d.abs();
    if (1e-3..1e7).contains(&abs) {
        let s = format!("{d}");
        if s.contains('.') { s } else { format!("{s}.0") }
    } else {
        // shortest digits in scientific form: d.dddE[-]n
        let s = format!("{d:e}");
        let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let mantissa = if mantissa.contains('.') {
            mantissa.to_string()
        } else {
            format!("{mantissa}.0")
        };
        format!("{mantissa}E{exp}")
    }
}

/// One token of `Turtle.valueOf`'s tokenizer.
#[derive(Clone, Copy)]
enum Tok {
    Num(f64),
    Op(char),
}

/// The tokens `java.io.StreamTokenizer` yields for the expression after JFLAP put a space
/// before every `-`, with unknown names counted as 0 and an implicit `+` between operands.
fn tokenize_expression(text: &str, params: &HashMap<String, f64>) -> Vec<Tok> {
    let spaced = text.replace('-', " -");
    let chars: Vec<char> = spaced.chars().collect();
    let mut out = Vec::new();
    let mut after_operand = false;
    let mut i = 0;
    let is_digitish = |c: char| c.is_ascii_digit() || c == '.' || c == '-';
    while i < chars.len() {
        let c = chars[i];
        if (c as u32) <= 32 {
            i += 1;
        } else if c.is_ascii_alphabetic() || (160..=255).contains(&(c as u32)) {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_alphabetic()
                    || (160..=255).contains(&(chars[i] as u32))
                    || is_digitish(chars[i]))
            {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            if after_operand {
                out.push(Tok::Op('+'));
            }
            after_operand = true;
            out.push(Tok::Num(params.get(&name).copied().unwrap_or(0.0)));
        } else if is_digitish(c) {
            let mut neg = false;
            let mut j = i;
            if c == '-' {
                j += 1;
                if !(j < chars.len() && (chars[j] == '.' || chars[j].is_ascii_digit())) {
                    // a lone minus sign is an operator
                    after_operand = false;
                    out.push(Tok::Op('-'));
                    i += 1;
                    continue;
                }
                neg = true;
            }
            let mut v = 0.0f64;
            let mut decexp = 0;
            let mut seen_dot = false;
            while j < chars.len() {
                if chars[j] == '.' && !seen_dot {
                    seen_dot = true;
                } else if chars[j].is_ascii_digit() {
                    v = v * 10.0 + f64::from(chars[j] as u8 - b'0');
                    if seen_dot {
                        decexp += 1;
                    }
                } else {
                    break;
                }
                j += 1;
            }
            if decexp != 0 {
                let mut denom = 10.0;
                decexp -= 1;
                while decexp > 0 {
                    denom *= 10.0;
                    decexp -= 1;
                }
                v /= denom;
            }
            if after_operand {
                out.push(Tok::Op('+'));
            }
            after_operand = true;
            out.push(Tok::Num(if neg { -v } else { v }));
            i = j;
        } else {
            after_operand = false;
            out.push(Tok::Op(c));
            i += 1;
        }
    }
    out
}

fn apply(op: char, left: f64, right: f64) -> f64 {
    match op {
        '^' => left.powf(right),
        '*' => left * right,
        '/' => left / right,
        '+' => left + right,
        '-' => left - right,
        _ => left,
    }
}

fn eval_tokens(it: &mut std::iter::Peekable<std::vec::IntoIter<Tok>>) -> Result<f64, ()> {
    let mut nums: Vec<f64> = vec![0.0];
    let mut ops: Vec<char> = Vec::new();
    while let Some(tok) = it.next() {
        let c2 = match tok {
            Tok::Num(n) => {
                nums.push(n);
                continue;
            }
            Tok::Op(c) => c,
        };
        if c2 == ')' {
            break;
        }
        if c2 == '(' {
            nums.push(eval_tokens(it)?);
            continue;
        }
        while let Some(&c3) = ops.last() {
            let reduce = match c2 {
                '+' | '-' => matches!(c3, '+' | '-' | '*' | '/' | '^'),
                '*' | '/' => matches!(c3, '*' | '/' | '^'),
                '^' => c3 == '^',
                _ => return Err(()),
            };
            if !reduce {
                break;
            }
            let right = nums.pop().ok_or(())?;
            let left = nums.pop().ok_or(())?;
            ops.pop();
            nums.push(apply(c3, left, right));
        }
        ops.push(c2);
    }
    while let Some(op) = ops.pop() {
        let right = nums.pop().ok_or(())?;
        let left = nums.pop().ok_or(())?;
        nums.push(apply(op, left, right));
    }
    nums.pop().ok_or(())
}

/// `Turtle.valueOf(String)`: `+ - * / ^`, parentheses and parameter names, no precedence
/// beyond what JFLAP's two-stack scheme gives. `Err` is any exception JFLAP would throw.
pub(crate) fn evaluate(text: &str, params: &HashMap<String, f64>) -> Result<f64, ()> {
    let tokens = tokenize_expression(text, params);
    eval_tokens(&mut tokens.into_iter().peekable())
}

/// The state of the drawing pen, copied on `[` and restored on `]`.
#[derive(Clone, Debug)]
pub struct Turtle {
    pub matrix: Matrix,
    pub distance: f64,
    pub line_width: f64,
    pub width_increment: f64,
    pub color: Rgb,
    pub polygon_color: Rgb,
    pub hue_change: f64,
    pub angle_change: f64,
    pub params: HashMap<String, f64>,
}

impl Turtle {
    /// `new Turtle()` followed by the view matrix.
    pub fn new(matrix: Matrix) -> Self {
        let mut params = HashMap::new();
        params.insert("distance".to_string(), 15.0);
        params.insert("angle".to_string(), 15.0);
        params.insert("hueChange".to_string(), 10.0);
        params.insert("lineWidth".to_string(), 1.0);
        params.insert("lineIncrement".to_string(), 1.0);
        Turtle {
            matrix,
            distance: 15.0,
            line_width: 1.0,
            width_increment: 1.0,
            color: Rgb(0, 0, 0),
            polygon_color: Rgb(255, 0, 0),
            hue_change: 10.0,
            angle_change: 15.0,
            params,
        }
    }

    /// The position in the drawing plane (the z coordinate is dropped).
    pub fn position(&self) -> (f64, f64) {
        let o = self.matrix.origin();
        (o[0], o[1])
    }

    pub fn value_of(&self, text: &str) -> Result<f64, ()> {
        evaluate(text, &self.params)
    }

    /// `turn(boolean)`: `+` is clockwise, a negative yaw.
    pub fn turn_dir(&mut self, clockwise: bool) {
        self.turn(if clockwise {
            -self.angle_change
        } else {
            self.angle_change
        });
    }

    pub fn turn(&mut self, degrees: f64) {
        self.matrix.yaw(degrees);
    }

    pub fn pitch_dir(&mut self, down: bool) {
        self.matrix.pitch(if down {
            self.angle_change
        } else {
            -self.angle_change
        });
    }

    pub fn pitch(&mut self, degrees: f64) {
        self.matrix.pitch(degrees);
    }

    pub fn roll_dir(&mut self, right: bool) {
        self.matrix.roll(if right {
            -self.angle_change
        } else {
            self.angle_change
        });
    }

    pub fn roll(&mut self, degrees: f64) {
        self.matrix.roll(degrees);
    }

    pub fn go(&mut self, d: f64) {
        self.matrix.translate(0.0, -d, 0.0);
    }

    pub fn go_dir(&mut self, forward: bool) {
        self.go(if forward {
            self.distance
        } else {
            -self.distance
        });
    }

    pub fn set_angle_change(&mut self, d: f64) {
        self.angle_change = ieee_remainder_360(d);
        self.params.insert("angle".into(), d);
    }

    pub fn set_hue_change(&mut self, d: f64) {
        self.hue_change = ieee_remainder_360(d);
        self.params.insert("hueChange".into(), d);
    }

    pub fn set_line_increment(&mut self, d: f64) {
        self.width_increment = d;
        self.params.insert("lineIncrement".into(), d);
    }

    pub fn set_line_width(&mut self, d: f64) {
        self.line_width = d;
        self.params.insert("lineWidth".into(), d);
    }

    pub fn change_line_width(&mut self, d: f64) {
        self.set_line_width(self.line_width + d);
    }

    pub fn set_color(&mut self, name: &str) -> Result<(), ()> {
        self.color = color_for_string(name).ok_or(())?;
        Ok(())
    }

    pub fn set_polygon_color(&mut self, name: &str) -> Result<(), ()> {
        self.polygon_color = color_for_string(name).ok_or(())?;
        Ok(())
    }

    fn shifted_hue(c: Rgb, d: f64) -> Rgb {
        let mut hsb = rgb_to_hsb(c.0, c.1, c.2);
        hsb[0] += d as f32 / 360.0;
        hsb_to_rgb(hsb[0], hsb[1], hsb[2])
    }

    pub fn change_hue(&mut self, d: f64) {
        self.color = Self::shifted_hue(self.color, d);
    }

    pub fn change_polygon_hue(&mut self, d: f64) {
        self.polygon_color = Self::shifted_hue(self.polygon_color, d);
    }

    /// `Turtle.assign`: evaluates `text` and stores it under `name`.
    pub fn assign(&mut self, name: &str, text: &str) -> Result<f64, ()> {
        let v = self.value_of(text)?;
        self.params.insert(name.to_string(), v);
        Ok(v)
    }
}

/// `String.hashCode` over UTF-16 code units.
fn java_string_hash(s: &str) -> i32 {
    s.encode_utf16()
        .fold(0i32, |h, u| h.wrapping_mul(31).wrapping_add(i32::from(u)))
}

/// The order a `java.util.HashMap<String, _>` iterates in when its keys were inserted in
/// the given order (JFLAP keeps an L-system's parameters in one).
pub(crate) fn java_hash_order<T>(items: &[(String, T)]) -> Vec<usize> {
    let mut capacity = 16usize;
    while items.len() * 4 > capacity * 3 {
        capacity *= 2;
    }
    let mut order: Vec<usize> = (0..items.len()).collect();
    let bucket = |k: &str| {
        let h = java_string_hash(k);
        ((h ^ ((h as u32) >> 16) as i32) as u32 as usize) & (capacity - 1)
    };
    order.sort_by_key(|&i| bucket(&items[i].0));
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(s: &str) -> f64 {
        evaluate(s, &HashMap::new()).unwrap()
    }

    #[test]
    fn expressions_follow_jflap_scheme() {
        assert_eq!(eval("2*3"), 6.0);
        assert_eq!(eval("10-4"), 6.0);
        assert_eq!(eval("10 - 4"), 6.0);
        assert_eq!(eval("2^3"), 8.0);
        assert_eq!(eval("(1+2)*4"), 12.0);
        assert_eq!(eval("-5"), -5.0);
        assert_eq!(eval("7/2"), 3.5);
        assert_eq!(eval("zzz"), 0.0);
    }

    #[test]
    fn java_number_text_round_trips() {
        assert_eq!(java_double_to_string(90.0), "90.0");
        assert_eq!(java_double_to_string(0.001), "0.001");
        assert_eq!(java_double_to_string(1.0e7), "1.0E7");
        assert_eq!(java_double_to_string(1.5e-5), "1.5E-5");
        assert_eq!(java_double_to_string(-2.5), "-2.5");
        assert_eq!(java_parse_double("45"), Some(45.0));
        assert_eq!(java_parse_double(" 2.5f "), Some(2.5));
        assert_eq!(java_parse_double("red"), None);
    }

    #[test]
    fn remainder_wraps_like_ieee() {
        assert_eq!(ieee_remainder_360(90.0), 90.0);
        assert_eq!(ieee_remainder_360(270.0), -90.0);
        assert_eq!(ieee_remainder_360(360.0), 0.0);
        assert_eq!(ieee_remainder_360(180.0), 180.0);
        assert_eq!(ieee_remainder_360(540.0), -180.0);
    }

    #[test]
    fn hash_map_order_of_parameter_names() {
        let items: Vec<(String, ())> = ["angle", "distance", "color", "lineWidth"]
            .iter()
            .map(|k| (k.to_string(), ()))
            .collect();
        let order = java_hash_order(&items);
        assert_eq!(order.len(), 4);
    }
}
