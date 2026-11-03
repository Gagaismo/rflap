//! Drawing geometry shared by the SVG renderer and the web editor (JFLAP `gui.viewer`:
//! `AutomatonDrawer`, `StateDrawer`, `CurvedArrow`).
//!
//! Every function is pure and works on plain `Point`s so it can be called through WASM.

use std::f64::consts::PI;

use crate::automaton::{Automaton, Label, Point, StateId, Transition};

/// JFLAP `StateDrawer.STATE_RADIUS`.
pub const STATE_RADIUS: f64 = 20.0;
/// Radius of the inner circle of a final state (`STATE_RADIUS - 3`).
pub const FINAL_RING_RADIUS: f64 = STATE_RADIUS - 3.0;
/// Distance a control point moves away from the chord per unit of `curvy`.
pub const CURVE_HEIGHT: f64 = 30.0;
/// Half-opening of the arrowhead (JFLAP `ARROW_ANGLE`).
pub const ARROW_ANGLE: f64 = PI / 10.0;
/// Length of each arrowhead wing (JFLAP `ARROW_LENGTH`).
pub const ARROW_LENGTH: f64 = 15.0;
/// Font size the text metrics below assume.
pub const FONT_SIZE: f64 = 12.0;
/// Approximate font ascent at `FONT_SIZE` (JFLAP uses the real `FontMetrics`).
pub const ASCENT: f64 = 12.0;
/// Padding inside state-label boxes (JFLAP `STATE_LABEL_PAD`).
pub const STATE_LABEL_PAD: f64 = 3.0;

/// Angular gap between the two arrows of a pair of states, in radians.
const PAIR_SPREAD: f64 = 0.125_663_706_143_591_74;
/// Angles on the state circle where a self-loop starts and ends (above the state).
const LOOP_START: f64 = -1.046_150_353_645_401;
const LOOP_END: f64 = -2.095_442_299_944_392_2;

/// An axis-aligned rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// Smallest rectangle containing `self` and the point.
    pub fn include(self, p: Point) -> Rect {
        let (x0, y0) = (self.x.min(p.x), self.y.min(p.y));
        let (x1, y1) = (
            (self.x + self.width).max(p.x),
            (self.y + self.height).max(p.y),
        );
        Rect {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }

    pub fn union(self, other: Rect) -> Rect {
        self.include(Point {
            x: other.x,
            y: other.y,
        })
        .include(Point {
            x: other.x + other.width,
            y: other.y + other.height,
        })
    }
}

/// Where one transition is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeGeometry {
    /// Index of the transition in `Automaton::transitions()`.
    pub index: usize,
    pub from: StateId,
    pub to: StateId,
    /// Start and end lie on the boundaries of the two state circles.
    pub start: Point,
    pub control: Point,
    pub end: Point,
    /// Signed curvature factor JFLAP feeds to `curve_control` (before an explicit control).
    pub curvy: f64,
    /// Middle of the curve; the label is centred here.
    pub label_anchor: Point,
    /// Text rotation in radians, in `(-π, π]`, chosen so the text is never upside down.
    pub label_rotation: f64,
    /// Whether the label goes on the lower side of the curve (JFLAP: `curvy < 0 ^ end.x < start.x`).
    pub label_below: bool,
    /// Position among the transitions sharing the same ordered pair of states (0 = first).
    pub stack_index: usize,
    pub is_loop: bool,
    /// Only the first transition of a stack draws the curve; the others only add a label.
    pub draws_curve: bool,
}

/// Point of the circle of radius `STATE_RADIUS` around `center` at `angle` (radians).
pub fn point_on_state(center: Point, angle: f64) -> Point {
    Point {
        x: center.x + angle.cos() * STATE_RADIUS,
        y: center.y + angle.sin() * STATE_RADIUS,
    }
}

/// Angle of the vector from `a` to `b`.
pub fn angle_between(a: Point, b: Point) -> f64 {
    (b.y - a.y).atan2(b.x - a.x)
}

/// Point where a straight arrow from `from` towards `to` leaves the circle around `from`.
pub fn boundary_point(from: Point, to: Point) -> Point {
    point_on_state(from, angle_between(from, to))
}

/// Control point of the quadratic curve from `start` to `end` for a curvature factor
/// (JFLAP `CurvedArrow.refreshCurve`): `curvy == 0` is a straight line, a positive value bends
/// to the left of the direction of travel.
pub fn curve_control(start: Point, end: Point, curvy: f64) -> Point {
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let len = dx.hypot(dy);
    let (ux, uy) = if len == 0.0 {
        (0.0, 0.0)
    } else {
        (dx / len, dy / len)
    };
    Point {
        x: (start.x + end.x) / 2.0 + curvy * CURVE_HEIGHT * uy,
        y: (start.y + end.y) / 2.0 - curvy * CURVE_HEIGHT * ux,
    }
}

/// Point of the quadratic Bézier curve at parameter `t` in `[0, 1]`.
pub fn quad_point(start: Point, control: Point, end: Point, t: f64) -> Point {
    let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * (1.0 - t) * t, t * t);
    Point {
        x: a * start.x + b * control.x + c * end.x,
        y: a * start.y + b * control.y + c * end.y,
    }
}

/// Wing tips of the arrowhead drawn at `end` for a curve arriving from `control`.
pub fn arrowhead(end: Point, control: Point) -> [Point; 2] {
    let base = (control.x - end.x).atan2(control.y - end.y);
    let wing = |a: f64| Point {
        x: end.x + a.sin() * ARROW_LENGTH,
        y: end.y + a.cos() * ARROW_LENGTH,
    };
    [wing(base + ARROW_ANGLE), wing(base - ARROW_ANGLE)]
}

/// Triangle marking the initial state (tip touches the circle on its left).
pub fn initial_triangle(center: Point) -> [Point; 3] {
    let (x, y, r) = (center.x, center.y, STATE_RADIUS);
    [
        Point { x: x - r, y },
        Point {
            x: x - 2.0 * r,
            y: y - r,
        },
        Point {
            x: x - 2.0 * r,
            y: y + r,
        },
    ]
}

/// Rough width of `text` at `FONT_SIZE` (real metrics need a font; 7 px per character).
pub fn text_width(text: &str) -> f64 {
    text.chars().count() as f64 * 7.0
}

/// Box under a state holding its label lines (JFLAP `StateDrawer.drawStateLabel`).
pub fn state_label_box(center: Point, lines: &[&str]) -> Rect {
    let widest = lines.iter().map(|l| text_width(l)).fold(0.0, f64::max);
    let line = ASCENT + STATE_LABEL_PAD;
    let width = widest + 2.0 * STATE_LABEL_PAD;
    Rect {
        x: center.x - (width / 2.0).trunc(),
        y: center.y + STATE_RADIUS - STATE_LABEL_PAD,
        width,
        height: lines.len() as f64 * line - STATE_LABEL_PAD + 2.0 * STATE_LABEL_PAD,
    }
}

/// Box at the upper right of a state holding its Moore output (JFLAP `MooreStateDrawer`).
pub fn output_box(center: Point, output: &str) -> Rect {
    let width = text_width(output) + 2.0 * STATE_LABEL_PAD;
    let height = ASCENT + 2.0 * STATE_LABEL_PAD;
    Rect {
        x: center.x - (width / 2.0).trunc() + STATE_RADIUS,
        y: center.y - STATE_RADIUS - (height / 2.0).trunc(),
        width,
        height,
    }
}

/// Geometry of every transition, indexed like `Automaton::transitions()`.
///
/// Mirrors `AutomatonDrawer.refreshArrowMap`: transitions between two states in both directions
/// bend apart (`curvy` 0.5), parallel transitions share one curve with stacked labels, and
/// self-loops arch above the state. An explicit `Transition::control` overrides the curve.
pub fn edge_geometries<L: Label>(automaton: &Automaton<L>) -> Vec<EdgeGeometry> {
    let states: Vec<_> = automaton.states().collect();
    let transitions = automaton.transitions();
    let between = |a: StateId, b: StateId| -> Vec<usize> {
        (0..transitions.len())
            .filter(|&i| transitions[i].from == a && transitions[i].to == b)
            .collect()
    };
    let mut out: Vec<Option<EdgeGeometry>> = vec![None; transitions.len()];
    for (i, si) in states.iter().enumerate() {
        for sj in &states[i + 1..] {
            let (forward, backward) = (between(si.id, sj.id), between(sj.id, si.id));
            if forward.is_empty() && backward.is_empty() {
                continue;
            }
            let angle = angle_between(si.point, sj.point);
            let start = point_on_state(si.point, angle - PAIR_SPREAD);
            let end = point_on_state(sj.point, angle + PI + PAIR_SPREAD);
            let base = if backward.is_empty() { 0.0 } else { 0.5 };
            for (n, &t) in forward.iter().enumerate() {
                out[t] = Some(edge(
                    t,
                    &transitions[t],
                    start,
                    end,
                    base + n as f64,
                    n,
                    false,
                ));
            }
            let start = point_on_state(sj.point, angle + PI - PAIR_SPREAD);
            let end = point_on_state(si.point, angle + PAIR_SPREAD);
            let base = if forward.is_empty() { 0.0 } else { 0.5 };
            for (n, &t) in backward.iter().enumerate() {
                out[t] = Some(edge(
                    t,
                    &transitions[t],
                    start,
                    end,
                    base + n as f64,
                    n,
                    false,
                ));
            }
        }
        let start = point_on_state(si.point, LOOP_START);
        let end = point_on_state(si.point, LOOP_END);
        for (n, t) in between(si.id, si.id).into_iter().enumerate() {
            out[t] = Some(edge(
                t,
                &transitions[t],
                start,
                end,
                -2.0 - n as f64,
                n,
                true,
            ));
        }
    }
    out.into_iter().flatten().collect()
}

fn edge<L>(
    index: usize,
    t: &Transition<L>,
    start: Point,
    end: Point,
    curvy: f64,
    stack_index: usize,
    is_loop: bool,
) -> EdgeGeometry {
    let control = t
        .control
        .unwrap_or_else(|| curve_control(start, end, curvy));
    let leftwards = end.x < start.x;
    let mut rotation = (end.y - start.y).atan2(end.x - start.x);
    if leftwards {
        rotation += PI;
    }
    EdgeGeometry {
        index,
        from: t.from,
        to: t.to,
        start,
        control,
        end,
        curvy,
        label_anchor: quad_point(start, control, end, 0.5),
        label_rotation: normalize_angle(rotation),
        label_below: (curvy < 0.0) ^ leftwards,
        stack_index,
        is_loop,
        draws_curve: stack_index == 0,
    }
}

/// Maps an angle to `(-π, π]`.
fn normalize_angle(a: f64) -> f64 {
    let r = a.rem_euclid(2.0 * PI);
    if r > PI { r - 2.0 * PI } else { r }
}
