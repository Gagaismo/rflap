//! SVG rendering of automata (JFLAP `gui.viewer.AutomatonDrawer` look), plus text renderings
//! (Markdown transition table, Graphviz DOT) for chats that cannot show images.

pub mod geometry;
mod text;

use std::collections::BTreeSet;
use std::fmt::Write;

use crate::automaton::{Automaton, Label, Point, StateId};
use geometry::{
    ASCENT, EdgeGeometry, FINAL_RING_RADIUS, FONT_SIZE, Rect, STATE_LABEL_PAD, STATE_RADIUS,
    arrowhead, edge_geometries, initial_triangle, output_box, quad_point, state_label_box,
    text_width,
};
pub use text::{to_dot, transition_table};

/// Options of `to_svg`.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgOptions {
    /// States drawn in the selection color (for example the current configurations).
    pub highlight_states: BTreeSet<StateId>,
    /// Indices into `Automaton::transitions()` drawn with a translucent red halo
    /// (for example nondeterministic or λ transitions).
    pub highlight_transitions: BTreeSet<usize>,
    /// Light-on-dark colors.
    pub dark_mode: bool,
    /// Omit the background rectangle.
    pub transparent_background: bool,
    /// Draw the label boxes under states.
    pub state_labels: bool,
    /// Margin around the drawing, in canvas units.
    pub padding: f64,
    /// Pixel size per canvas unit of the `width`/`height` attributes.
    pub scale: f64,
    /// Accessible title.
    pub title: Option<String>,
}

impl Default for SvgOptions {
    fn default() -> Self {
        SvgOptions {
            highlight_states: BTreeSet::new(),
            highlight_transitions: BTreeSet::new(),
            dark_mode: false,
            transparent_background: false,
            state_labels: true,
            padding: 20.0,
            scale: 1.0,
            title: None,
        }
    }
}

struct Theme {
    background: &'static str,
    ink: &'static str,
    state_fill: &'static str,
    selected_fill: &'static str,
    note: &'static str,
}

const LIGHT: Theme = Theme {
    background: "#ffffff",
    ink: "#000000",
    state_fill: "#ffff96",
    selected_fill: "#64c8c8",
    note: "#444444",
};

const DARK: Theme = Theme {
    background: "#1e1e1e",
    ink: "#e6e6e6",
    state_fill: "#6b6b2e",
    selected_fill: "#2f7f7f",
    note: "#b0b0b0",
};

/// Escapes text for XML content and attribute values.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Compact decimal: at most two decimals, no trailing zeros, no `-0`.
fn n(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

fn pts(points: &[Point]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", n(p.x), n(p.y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Bounding box of everything `to_svg` draws, without padding; `None` for an empty automaton.
pub fn drawing_bounds<L: Label>(
    automaton: &Automaton<L>,
    edges: &[EdgeGeometry],
    options: &SvgOptions,
) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    let mut add = |r: Rect| bounds = Some(bounds.map_or(r, |b| b.union(r)));
    for s in automaton.states() {
        let c = s.point;
        let left = if automaton.initial() == Some(s.id) {
            2.0 * STATE_RADIUS
        } else {
            STATE_RADIUS
        };
        add(Rect {
            x: c.x - left,
            y: c.y - STATE_RADIUS,
            width: left + STATE_RADIUS,
            height: 2.0 * STATE_RADIUS,
        });
        if options.state_labels
            && let Some(label) = s.label.as_deref().filter(|l| !l.is_empty())
        {
            add(state_label_box(c, &label.lines().collect::<Vec<_>>()));
        }
        if let Some(out) = s.output.as_deref() {
            add(output_box(c, out));
        }
    }
    for (e, label) in edges.iter().zip(
        edges
            .iter()
            .map(|e| automaton.transitions()[e.index].label.description()),
    ) {
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let p = quad_point(e.start, e.control, e.end, t);
            add(Rect {
                x: p.x,
                y: p.y,
                width: 0.0,
                height: 0.0,
            });
        }
        let reach = text_width(&label) / 2.0 + ASCENT;
        add(Rect {
            x: e.label_anchor.x - reach,
            y: e.label_anchor.y - reach,
            width: 2.0 * reach,
            height: 2.0 * reach,
        });
    }
    for note in automaton.notes() {
        add(Rect {
            x: note.point.x,
            y: note.point.y - ASCENT,
            width: text_width(&note.text),
            height: ASCENT + 4.0,
        });
    }
    bounds
}

/// Self-contained SVG of the automaton in JFLAP's style, in the automaton's own coordinates.
///
/// States are `<g class="state" data-state-id="…">` and transitions
/// `<g class="transition" data-transition-index="…" data-from="…" data-to="…">`, so a web UI can
/// attach interaction without re-parsing.
pub fn to_svg<L: Label>(automaton: &Automaton<L>, options: &SvgOptions) -> String {
    let theme = if options.dark_mode { &DARK } else { &LIGHT };
    let edges = edge_geometries(automaton);
    let bounds = drawing_bounds(automaton, &edges, options).unwrap_or(Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    });
    let view = Rect {
        x: bounds.x - options.padding,
        y: bounds.y - options.padding,
        width: (bounds.width + 2.0 * options.padding).max(1.0),
        height: (bounds.height + 2.0 * options.padding).max(1.0),
    };
    let mut svg = String::new();
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\" \
         font-family=\"Helvetica, Arial, sans-serif\" font-size=\"{}\" stroke=\"{ink}\" \
         role=\"img\">\n",
        n(view.x),
        n(view.y),
        n(view.width),
        n(view.height),
        n((view.width * options.scale).ceil()),
        n((view.height * options.scale).ceil()),
        n(FONT_SIZE),
        ink = theme.ink,
    );
    if let Some(title) = &options.title {
        let _ = writeln!(svg, "<title>{}</title>", esc(title));
    }
    if !options.transparent_background {
        let _ = writeln!(
            svg,
            "<rect class=\"background\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\"/>",
            n(view.x),
            n(view.y),
            n(view.width),
            n(view.height),
            theme.background
        );
    }
    svg.push_str("<g class=\"transitions\" fill=\"none\">\n");
    for e in &edges {
        draw_transition(&mut svg, automaton, e, options, theme);
    }
    svg.push_str("</g>\n<g class=\"states\">\n");
    for s in automaton.states() {
        draw_state(&mut svg, automaton, s.id, options, theme);
    }
    svg.push_str("</g>\n");
    if !automaton.notes().is_empty() {
        svg.push_str("<g class=\"notes\">\n");
        for note in automaton.notes() {
            let _ = writeln!(
                svg,
                "<text class=\"note\" x=\"{}\" y=\"{}\" fill=\"{}\" stroke=\"none\">{}</text>",
                n(note.point.x),
                n(note.point.y),
                theme.note,
                esc(&note.text)
            );
        }
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>\n");
    svg
}

fn draw_transition<L: Label>(
    svg: &mut String,
    automaton: &Automaton<L>,
    e: &EdgeGeometry,
    options: &SvgOptions,
    theme: &Theme,
) {
    let label = automaton.transitions()[e.index].label.description();
    let highlighted = options.highlight_transitions.contains(&e.index);
    let class = if e.is_loop {
        "transition loop"
    } else {
        "transition"
    };
    let _ = writeln!(
        svg,
        "<g class=\"{class}\" data-transition-index=\"{}\" data-from=\"{}\" data-to=\"{}\">",
        e.index, e.from, e.to
    );
    let width = text_width(&label);
    let dy = if e.label_below { ASCENT } else { -3.0 };
    let label_transform = format!(
        "translate({} {}) rotate({})",
        n(e.label_anchor.x),
        n(e.label_anchor.y),
        n(e.label_rotation.to_degrees())
    );
    if e.draws_curve {
        let d = format!(
            "M {} {} Q {} {} {} {}",
            n(e.start.x),
            n(e.start.y),
            n(e.control.x),
            n(e.control.y),
            n(e.end.x),
            n(e.end.y)
        );
        if highlighted {
            let _ = writeln!(
                svg,
                "<path class=\"halo\" d=\"{d}\" stroke=\"rgba(255,0,0,0.5)\" stroke-width=\"6\"/>"
            );
        }
        let _ = writeln!(svg, "<path class=\"curve\" d=\"{d}\"/>");
        let [a, b] = arrowhead(e.end, e.control);
        let _ = writeln!(
            svg,
            "<path class=\"arrowhead\" d=\"M {} {} L {} {} L {} {}\"/>",
            n(a.x),
            n(a.y),
            n(e.end.x),
            n(e.end.y),
            n(b.x),
            n(b.y)
        );
    }
    if highlighted {
        let top = if e.label_below { 0.0 } else { -ASCENT - 3.0 };
        let _ = writeln!(
            svg,
            "<rect class=\"halo\" transform=\"{label_transform}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
             fill=\"rgba(255,0,0,0.5)\" stroke=\"none\"/>",
            n(-width / 2.0),
            n(top),
            n(width),
            n(ASCENT + 3.0)
        );
    }
    let _ = writeln!(
        svg,
        "<text class=\"label\" transform=\"{label_transform}\" y=\"{}\" text-anchor=\"middle\" \
         fill=\"{}\" stroke=\"none\">{}</text>",
        n(dy),
        theme.ink,
        esc(&label)
    );
    svg.push_str("</g>\n");
}

fn draw_state<L: Label>(
    svg: &mut String,
    automaton: &Automaton<L>,
    id: StateId,
    options: &SvgOptions,
    theme: &Theme,
) {
    let Some(state) = automaton.state(id) else {
        return;
    };
    let c = state.point;
    let fill = if options.highlight_states.contains(&id) {
        theme.selected_fill
    } else {
        theme.state_fill
    };
    let _ = writeln!(svg, "<g class=\"state\" data-state-id=\"{id}\">");
    let _ = writeln!(
        svg,
        "<circle class=\"body\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{fill}\"/>",
        n(c.x),
        n(c.y),
        n(STATE_RADIUS)
    );
    if automaton.is_final(id) {
        let _ = writeln!(
            svg,
            "<circle class=\"final-ring\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\"/>",
            n(c.x),
            n(c.y),
            n(FINAL_RING_RADIUS)
        );
    }
    if automaton.initial() == Some(id) {
        let _ = writeln!(
            svg,
            "<polygon class=\"initial\" points=\"{}\" fill=\"{}\"/>",
            pts(&initial_triangle(c)),
            theme.background
        );
    }
    let _ = writeln!(
        svg,
        "<text class=\"name\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"{}\" stroke=\"none\">{}</text>",
        n(c.x),
        n(c.y + ASCENT / 2.0 - 1.0),
        theme.ink,
        esc(&state.name)
    );
    if let Some(out) = state.output.as_deref() {
        let b = output_box(c, out);
        box_with_lines(svg, "output", b, &[out], theme.state_fill, theme);
    }
    if options.state_labels
        && let Some(label) = state.label.as_deref().filter(|l| !l.is_empty())
    {
        let lines: Vec<&str> = label.lines().collect();
        let b = state_label_box(c, &lines);
        box_with_lines(svg, "state-label", b, &lines, theme.state_fill, theme);
    }
    svg.push_str("</g>\n");
}

/// A filled rectangle with left-aligned text lines (JFLAP label boxes).
fn box_with_lines(
    svg: &mut String,
    class: &str,
    b: Rect,
    lines: &[&str],
    fill: &str,
    theme: &Theme,
) {
    let _ = writeln!(
        svg,
        "<rect class=\"{class}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{fill}\"/>",
        n(b.x),
        n(b.y),
        n(b.width),
        n(b.height)
    );
    for (k, line) in lines.iter().enumerate() {
        let _ = writeln!(
            svg,
            "<text class=\"{class}-text\" x=\"{}\" y=\"{}\" fill=\"{}\" stroke=\"none\">{}</text>",
            n(b.x + STATE_LABEL_PAD),
            n(b.y + (k as f64 + 1.0) * (ASCENT + STATE_LABEL_PAD)),
            theme.ink,
            esc(line)
        );
    }
}
