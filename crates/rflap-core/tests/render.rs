//! SVG, transition table and DOT renderings, and the geometry helpers behind them.

use std::collections::BTreeSet;
use std::f64::consts::PI;

use rflap_core::automaton::{Note, Point, StateId, Transition};
use rflap_core::fa::{Fa, FaLabel};
use rflap_core::render::geometry::{
    ARROW_LENGTH, CURVE_HEIGHT, STATE_RADIUS, arrowhead, boundary_point, curve_control,
    edge_geometries, point_on_state, quad_point,
};
use rflap_core::render::{SvgOptions, to_dot, to_svg, transition_table};

fn at(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn add(fa: &mut Fa, from: StateId, to: StateId, label: &str) {
    assert!(fa.add_transition(Transition::new(from, to, FaLabel::from(label))));
}

/// q0 (initial) and q1 (final): `a` swaps, `b` loops.
fn two_state_dfa() -> Fa {
    let mut fa = Fa::new();
    fa.create_state(at(100.0, 100.0));
    fa.create_state(at(250.0, 100.0));
    fa.set_initial(Some(0));
    fa.set_final(1, true);
    add(&mut fa, 0, 1, "a");
    add(&mut fa, 1, 0, "a");
    add(&mut fa, 0, 0, "b");
    add(&mut fa, 1, 1, "b");
    fa
}

/// Even number of a's: q0 initial and final.
fn even_as() -> Fa {
    let mut fa = two_state_dfa();
    fa.set_final(0, true);
    fa.set_final(1, false);
    fa
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn svg_of_a_two_state_dfa_has_the_jflap_shapes() {
    let svg = to_svg(&two_state_dfa(), &SvgOptions::default());
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.trim_end().ends_with("</svg>"));
    assert_eq!(count(&svg, "data-state-id="), 2);
    assert_eq!(count(&svg, "data-state-id=\"0\""), 1);
    assert_eq!(count(&svg, "data-state-id=\"1\""), 1);
    assert_eq!(count(&svg, "class=\"final-ring\""), 1);
    assert_eq!(count(&svg, "class=\"initial\""), 1);
    assert_eq!(count(&svg, "class=\"transition loop\""), 2);
    assert_eq!(count(&svg, "class=\"transition\""), 2);
    for i in 0..4 {
        assert_eq!(
            count(&svg, &format!("data-transition-index=\"{i}\"")),
            1,
            "{svg}"
        );
    }
    // JFLAP colors: state fill (255, 255, 150), black strokes.
    assert!(svg.contains("fill=\"#ffff96\""));
    assert!(svg.contains("r=\"20\""));
    assert!(svg.contains("r=\"17\""));
    // The initial triangle of state 0 at (100, 100): tip on the circle, base 40 to the left.
    assert!(svg.contains("points=\"80,100 60,80 60,120\""), "{svg}");
}

#[test]
fn svg_escapes_labels_names_notes_and_title() {
    let mut fa = Fa::new();
    fa.create_state(at(50.0, 50.0));
    fa.create_state(at(200.0, 50.0));
    fa.state_mut(0).unwrap().name = "q<0>".to_string();
    add(&mut fa, 0, 1, "x<y&z");
    add(&mut fa, 1, 1, "\"q\"");
    fa.add_note(Note {
        point: at(10.0, 10.0),
        text: "a & <b>".into(),
    });
    let options = SvgOptions {
        title: Some("<DFA> & co".to_string()),
        ..SvgOptions::default()
    };
    let svg = to_svg(&fa, &options);
    assert!(svg.contains(">x&lt;y&amp;z</text>"), "{svg}");
    assert!(svg.contains(">&quot;q&quot;</text>"));
    assert!(svg.contains(">q&lt;0&gt;</text>"));
    assert!(svg.contains(">a &amp; &lt;b&gt;</text>"));
    assert!(svg.contains("<title>&lt;DFA&gt; &amp; co</title>"));
    assert!(!svg.contains("x<y"));
    assert!(!svg.contains("<DFA>"));
}

#[test]
fn svg_highlights_states_and_transitions() {
    let plain = to_svg(&two_state_dfa(), &SvgOptions::default());
    assert_eq!(count(&plain, "#64c8c8"), 0);
    assert_eq!(count(&plain, "class=\"halo\""), 0);
    let options = SvgOptions {
        highlight_states: BTreeSet::from([1]),
        highlight_transitions: BTreeSet::from([0]),
        ..SvgOptions::default()
    };
    let svg = to_svg(&two_state_dfa(), &options);
    assert_eq!(count(&svg, "#64c8c8"), 1);
    // Transition 0 is the first of its stack: curve halo plus label halo.
    assert_eq!(count(&svg, "class=\"halo\""), 2);
}

#[test]
fn svg_dark_mode_and_transparent_background() {
    let dark = to_svg(
        &two_state_dfa(),
        &SvgOptions {
            dark_mode: true,
            ..SvgOptions::default()
        },
    );
    assert!(dark.contains("fill=\"#1e1e1e\""));
    assert!(dark.contains("#e6e6e6"));
    let bare = to_svg(
        &two_state_dfa(),
        &SvgOptions {
            transparent_background: true,
            ..SvgOptions::default()
        },
    );
    assert_eq!(count(&bare, "class=\"background\""), 0);
    assert_eq!(
        count(
            &to_svg(&two_state_dfa(), &SvgOptions::default()),
            "class=\"background\""
        ),
        1
    );
}

#[test]
fn parallel_transitions_share_one_curve_with_stacked_labels() {
    let mut fa = Fa::new();
    fa.create_state(at(50.0, 50.0));
    fa.create_state(at(200.0, 50.0));
    add(&mut fa, 0, 1, "a");
    add(&mut fa, 0, 1, "b");
    add(&mut fa, 0, 1, "c");
    let svg = to_svg(&fa, &SvgOptions::default());
    assert_eq!(count(&svg, "class=\"curve\""), 1);
    assert_eq!(count(&svg, "class=\"arrowhead\""), 1);
    assert_eq!(count(&svg, "class=\"label\""), 3);
    let g = edge_geometries(&fa);
    assert_eq!(g.len(), 3);
    assert!(g[0].label_anchor.y > g[1].label_anchor.y && g[1].label_anchor.y > g[2].label_anchor.y);
    assert_eq!(
        g.iter().map(|e| e.draws_curve).collect::<Vec<_>>(),
        vec![true, false, false]
    );
}

#[test]
fn svg_of_an_empty_automaton_is_a_valid_empty_canvas() {
    let svg = to_svg(&Fa::new(), &SvgOptions::default());
    assert_eq!(count(&svg, "data-state-id="), 0);
    assert!(svg.contains("viewBox="));
}

#[test]
fn svg_shows_moore_output_and_state_labels() {
    let mut fa = two_state_dfa();
    fa.state_mut(0).unwrap().output = Some("1<".to_string());
    fa.state_mut(1).unwrap().label = Some("start\nhere".to_string());
    let svg = to_svg(&fa, &SvgOptions::default());
    assert!(svg.contains(">1&lt;</text>"));
    assert!(svg.contains(">start</text>") && svg.contains(">here</text>"));
    let hidden = to_svg(
        &fa,
        &SvgOptions {
            state_labels: false,
            ..SvgOptions::default()
        },
    );
    assert!(!hidden.contains(">start</text>"));
}

#[test]
fn point_on_state_lies_on_the_circle() {
    let c = at(10.0, 20.0);
    let p = point_on_state(c, 0.0);
    assert_eq!((p.x, p.y), (30.0, 20.0));
    let p = point_on_state(c, PI / 2.0);
    assert!((p.x - 10.0).abs() < 1e-9 && (p.y - 40.0).abs() < 1e-9);
    let b = boundary_point(c, at(110.0, 20.0));
    assert_eq!((b.x, b.y), (30.0, 20.0));
}

#[test]
fn curve_control_is_the_midpoint_when_straight_and_offset_when_curved() {
    let (a, b) = (at(0.0, 0.0), at(100.0, 0.0));
    assert_eq!(curve_control(a, b, 0.0), at(50.0, 0.0));
    // Positive curvy bends towards negative y (up on screen) for a left-to-right arrow.
    assert_eq!(curve_control(a, b, 0.5), at(50.0, -0.5 * CURVE_HEIGHT));
    assert_eq!(curve_control(b, b, 1.0), b);
    assert_eq!(quad_point(a, at(50.0, -30.0), b, 0.5), at(50.0, -15.0));
}

#[test]
fn arrowhead_wings_have_the_jflap_length() {
    let end = at(100.0, 0.0);
    for wing in arrowhead(end, at(50.0, 0.0)) {
        assert!(((wing.x - end.x).hypot(wing.y - end.y) - ARROW_LENGTH).abs() < 1e-9);
        assert!(wing.x < end.x);
    }
}

#[test]
fn edge_geometry_follows_jflap_conventions() {
    let fa = two_state_dfa();
    let g = edge_geometries(&fa);
    assert_eq!(g.len(), 4);
    // Indexed like the transitions.
    assert_eq!(
        g.iter().map(|e| e.index).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    let (forward, backward) = (&g[0], &g[1]);
    // Endpoints on the circle boundaries.
    for (p, c) in [
        (forward.start, at(100.0, 100.0)),
        (forward.end, at(250.0, 100.0)),
    ] {
        assert!(((p.x - c.x).hypot(p.y - c.y) - STATE_RADIUS).abs() < 1e-9);
    }
    // A pair of opposite transitions bends to opposite sides.
    assert!(forward.control.y < 100.0 && backward.control.y > 100.0);
    assert!(forward.label_anchor.y < backward.label_anchor.y);
    // Self-loops arch above their state and keep the label upright.
    let lp = &g[2];
    assert!(
        lp.is_loop
            && lp.control.y < 100.0 - STATE_RADIUS
            && lp.label_anchor.y < 100.0 - STATE_RADIUS
    );
    assert!(lp.label_rotation.abs() < 1e-9 && !lp.label_below);
}

#[test]
fn explicit_control_points_override_the_default_curve() {
    let mut fa = Fa::new();
    fa.create_state(at(0.0, 0.0));
    fa.create_state(at(200.0, 0.0));
    let mut t = Transition::new(0, 1, FaLabel::from("a"));
    t.control = Some(at(100.0, -80.0));
    fa.add_transition(t);
    let g = edge_geometries(&fa);
    assert_eq!(g[0].control, at(100.0, -80.0));
    assert_eq!(
        g[0].label_anchor.y,
        quad_point(g[0].start, g[0].control, g[0].end, 0.5).y
    );
}

#[test]
fn transition_table_of_the_even_as_dfa() {
    assert_eq!(
        transition_table(&even_as()),
        "| State | a | b |\n|---|---|---|\n| →* q0 | q1 | q0 |\n| q1 | q0 | q1 |\n"
    );
}

#[test]
fn transition_table_lists_nondeterministic_targets_empty_cells_and_lambda_last() {
    let mut fa = Fa::new();
    for i in 0..3 {
        fa.create_state(at(f64::from(i) * 100.0, 0.0));
    }
    fa.set_initial(Some(0));
    fa.set_final(2, true);
    add(&mut fa, 0, 2, "a");
    add(&mut fa, 0, 1, "a");
    add(&mut fa, 0, 1, "");
    add(&mut fa, 1, 2, "b|c");
    assert_eq!(
        transition_table(&fa),
        "| State | a | b\\|c | λ |\n|---|---|---|---|\n\
         | → q0 | q1, q2 | ∅ | q1 |\n\
         | q1 | ∅ | q2 | ∅ |\n\
         | * q2 | ∅ | ∅ | ∅ |\n"
    );
}

#[test]
fn transition_table_without_transitions_has_only_states() {
    let mut fa = Fa::new();
    fa.create_state(at(0.0, 0.0));
    fa.set_initial(Some(0));
    assert_eq!(transition_table(&fa), "| State |\n|---|\n| → q0 |\n");
}

#[test]
fn transition_table_shows_moore_outputs() {
    let mut fa = two_state_dfa();
    fa.state_mut(1).unwrap().output = Some("1".to_string());
    assert!(
        transition_table(&fa).contains("| * q1 / 1 |"),
        "{}",
        transition_table(&fa)
    );
}

#[test]
fn dot_of_a_two_state_example() {
    let mut fa = Fa::new();
    fa.create_state(at(0.0, 0.0));
    fa.create_state(at(100.0, 0.0));
    fa.set_initial(Some(0));
    fa.set_final(1, true);
    add(&mut fa, 0, 1, "a");
    add(&mut fa, 0, 1, "b");
    add(&mut fa, 1, 1, "c\"d");
    assert_eq!(
        to_dot(&fa),
        "digraph automaton {\n\
         \x20 rankdir=LR;\n\
         \x20 node [shape=circle];\n\
         \x20 __start [shape=point];\n\
         \x20 s0 [label=\"q0\"];\n\
         \x20 s1 [label=\"q1\", shape=doublecircle];\n\
         \x20 __start -> s0;\n\
         \x20 s0 -> s1 [label=\"a\\nb\"];\n\
         \x20 s1 -> s1 [label=\"c\\\"d\"];\n\
         }\n"
    );
}

#[test]
fn dot_without_initial_state_has_no_start_arrow() {
    let mut fa = Fa::new();
    fa.create_state(at(0.0, 0.0));
    assert_eq!(
        to_dot(&fa),
        "digraph automaton {\n  rankdir=LR;\n  node [shape=circle];\n  s0 [label=\"q0\"];\n}\n"
    );
}

// ---------------------------------------------------------------------------------------------
// labels in the notation of the course
// ---------------------------------------------------------------------------------------------

mod catedra_labels {
    use rflap_core::render::{SvgOptions, to_dot_with, to_svg, to_svg_with, transition_table_with};
    use rflap_core::turing::TuringMachine;
    use rflap_core::turing::catedra::{Notation, format_label, parse_label};

    use super::{at, even_as};

    /// States 2 (initial) and 3 with `(a,a,D)` and `(b,b,D)` looping on 2 and `(Δ,Δ,I)` to 3.
    fn palindrome_corner(tapes: usize) -> TuringMachine {
        let mut m = TuringMachine::new(tapes);
        let a = m.create_state(at(100.0, 100.0));
        let b = m.create_state(at(250.0, 100.0));
        m.state_mut(a).unwrap().name = "2".into();
        m.state_mut(b).unwrap().name = "3".into();
        m.set_initial(Some(a));
        m.set_final(b, true);
        let labels: &[&str] = if tapes == 1 {
            &["(a,a,D)", "(b,b,D)"]
        } else {
            &["a;a,R|□;b,S", "b;b,R|a;□,L"]
        };
        for l in labels {
            m.add_transition(a, a, parse_label(l).unwrap()).unwrap();
        }
        let last = if tapes == 1 {
            "(Δ,Δ,I)"
        } else {
            "□;□,S|□;□,S"
        };
        m.add_transition(a, b, parse_label(last).unwrap()).unwrap();
        m
    }

    fn catedra(l: &rflap_core::turing::TmLabel) -> String {
        format_label(l, Notation::Catedra)
    }

    fn texts(svg: &str) -> Vec<String> {
        svg.split("<text class=\"label\"")
            .skip(1)
            .map(|chunk| {
                let start = chunk.find('>').unwrap() + 1;
                chunk[start..chunk.find("</text>").unwrap()].to_string()
            })
            .collect()
    }

    #[test]
    fn single_tape_labels_are_stacked_with_delta_and_idn() {
        let svg = to_svg_with(
            palindrome_corner(1).automaton(),
            &SvgOptions::default(),
            catedra,
        );
        assert_eq!(texts(&svg), ["(a,a,D)", "(b,b,D)", "(Δ,Δ,I)"]);
    }

    #[test]
    fn single_tape_labels_can_be_joined_with_a_slash() {
        let options = SvgOptions {
            join_labels: true,
            ..SvgOptions::default()
        };
        let svg = to_svg_with(palindrome_corner(1).automaton(), &options, catedra);
        assert_eq!(texts(&svg), ["(a,a,D) / (b,b,D)", "(Δ,Δ,I)"]);
        assert_eq!(svg.matches("data-transition-index=").count(), 3);
    }

    #[test]
    fn multi_tape_labels_keep_the_jflap_form_with_delta() {
        let svg = to_svg_with(
            palindrome_corner(2).automaton(),
            &SvgOptions::default(),
            catedra,
        );
        assert_eq!(texts(&svg), ["a;a,R|Δ;b,S", "b;b,R|a;Δ,L", "Δ;Δ,S|Δ;Δ,S"]);
    }

    #[test]
    fn default_to_svg_still_uses_the_jflap_description() {
        let m = palindrome_corner(1);
        let svg = to_svg(m.automaton(), &SvgOptions::default());
        assert_eq!(texts(&svg), ["a ; a , R", "b ; b , R", "□ ; □ , L"]);
    }

    #[test]
    fn a_finite_automaton_is_drawn_the_same_with_its_own_description() {
        let fa = even_as();
        let options = SvgOptions::default();
        assert_eq!(
            to_svg(&fa, &options),
            to_svg_with(&fa, &options, rflap_core::automaton::Label::description)
        );
    }

    #[test]
    fn dot_and_markdown_take_the_formatter() {
        let m = palindrome_corner(1);
        let dot = to_dot_with(m.automaton(), catedra);
        assert!(
            dot.contains("s0 -> s0 [label=\"(a,a,D)\\n(b,b,D)\"];"),
            "{dot}"
        );
        let md = transition_table_with(m.automaton(), catedra);
        assert!(md.contains("| (Δ,Δ,I) |"), "{md}");
    }
}
