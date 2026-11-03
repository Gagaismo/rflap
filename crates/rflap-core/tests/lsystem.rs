//! L-systems: rewriting, turtle interpretation and SVG (JFLAP `grammar.lsystem`, `gui.lsystem`).

use rflap_core::lsystem::{DrawOptions, Expander, LSystem, Rgb, Shape, tokenify};

fn koch() -> LSystem {
    LSystem::new(
        "g - - g - - g",
        &[("g", "g + g - - g + g")],
        &[("angle", "60"), ("distance", "5")],
    )
}

fn words(s: &str) -> Vec<String> {
    tokenify(s)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn koch_expands_to_the_exact_strings() {
    let l = koch();
    assert_eq!(l.expand(0), words("g - - g - - g"));
    assert_eq!(
        l.expand(1),
        words("g + g - - g + g - - g + g - - g + g - - g + g - - g + g")
    );
    let two = l.expand(2);
    assert_eq!(two.iter().filter(|s| *s == "g").count(), 48);
    // the first eight symbols of level 1 are `g + g - - g + g`, each `g` becomes that block
    let p = "g + g - - g + g";
    assert_eq!(two[..36].join(" "), format!("{p} + {p} - - {p} + {p}"));
}

#[test]
fn koch_turtle_draws_one_segment_per_g() {
    let l = koch();
    assert_eq!(l.draw(0, &DrawOptions::default()).segments().count(), 3);
    assert_eq!(l.draw(1, &DrawOptions::default()).segments().count(), 12);
    assert_eq!(l.draw(2, &DrawOptions::default()).segments().count(), 48);
}

#[test]
fn turtle_heads_up_and_plus_turns_clockwise() {
    let l = LSystem::new("g + g", &[], &[("angle", "90"), ("distance", "10")]);
    let d = l.draw(0, &DrawOptions::default());
    let s: Vec<_> = d.segments().collect();
    assert_eq!(s.len(), 2);
    assert!(close(s[0].x1, 0.0) && close(s[0].y1, 0.0));
    assert!(close(s[0].x2, 0.0) && close(s[0].y2, -10.0));
    assert!(close(s[1].x1, 0.0) && close(s[1].y1, -10.0));
    assert!(close(s[1].x2, 10.0) && close(s[1].y2, -10.0));
    // bounds are the visited positions
    assert!(close(d.bounds.x, 0.0) && close(d.bounds.y, -10.0));
    assert!(close(d.bounds.width, 10.0) && close(d.bounds.height, 10.0));
}

#[test]
fn minus_turns_counter_clockwise_and_f_moves_without_drawing() {
    let l = LSystem::new("f - g", &[], &[("angle", "90"), ("distance", "10")]);
    let d = l.draw(0, &DrawOptions::default());
    let s: Vec<_> = d.segments().collect();
    assert_eq!(s.len(), 1);
    assert!(close(s[0].x1, 0.0) && close(s[0].y1, -10.0));
    assert!(close(s[0].x2, -10.0) && close(s[0].y2, -10.0));
}

#[test]
fn bracketed_plant_pushes_and_pops_the_turtle() {
    let l = LSystem::new("g [ + g ] g", &[], &[("angle", "90"), ("distance", "10")]);
    let d = l.draw(0, &DrawOptions::default());
    let s: Vec<_> = d.segments().collect();
    assert_eq!(s.len(), 3);
    let end = |i: usize| (s[i].x2, s[i].y2);
    assert!(close(end(0).0, 0.0) && close(end(0).1, -10.0));
    assert!(close(end(1).0, 10.0) && close(end(1).1, -10.0));
    // after `]` the turtle is back at (0,-10) heading up
    assert!(close(s[2].x1, 0.0) && close(s[2].y1, -10.0));
    assert!(close(end(2).0, 0.0) && close(end(2).1, -20.0));
    assert!(close(d.bounds.x, 0.0) && close(d.bounds.y, -20.0));
    assert!(close(d.bounds.width, 10.0) && close(d.bounds.height, 20.0));
}

#[test]
fn parenthesised_arguments_and_assignments_change_the_turtle() {
    // g(2*3) moves 6 units, distance=4 then g moves 4, angle(180) + turns back
    let l = LSystem::new(
        "g(2*3) distance=4 angle(180) + g",
        &[],
        &[("distance", "10")],
    );
    let d = l.draw(0, &DrawOptions::default());
    let s: Vec<_> = d.segments().collect();
    assert_eq!(s.len(), 2);
    assert!(close(s[0].y2, -6.0));
    assert!(close(s[1].y1, -6.0) && close(s[1].y2, -2.0));
}

#[test]
fn colors_and_line_widths_follow_the_turtle() {
    let l = LSystem::new(
        "g color=red lineWidth=3 g [ color(0,0,255) ! g ] g",
        &[],
        &[("distance", "10")],
    );
    let d = l.draw(0, &DrawOptions::default());
    let s: Vec<_> = d.segments().collect();
    assert_eq!(s.len(), 4);
    assert_eq!((s[0].color, s[0].width), (Rgb(0, 0, 0), 1.0));
    assert_eq!((s[1].color, s[1].width), (Rgb(255, 0, 0), 3.0));
    assert_eq!((s[2].color, s[2].width), (Rgb(0, 0, 255), 4.0));
    // popping restores colour and width
    assert_eq!((s[3].color, s[3].width), (Rgb(255, 0, 0), 3.0));
}

#[test]
fn polygons_fill_instead_of_stroking() {
    let l = LSystem::new(
        "{ g + g + g } g",
        &[],
        &[
            ("angle", "120"),
            ("distance", "10"),
            ("polygonColor", "blue"),
        ],
    );
    let d = l.draw(0, &DrawOptions::default());
    assert_eq!(d.segments().count(), 1);
    let polys: Vec<_> = d.polygons().collect();
    assert_eq!(polys.len(), 1);
    assert_eq!(polys[0].fill, Rgb(0, 0, 255));
    assert_eq!(polys[0].points.len(), 4);
    assert!(matches!(d.shapes[0], Shape::Polygon(_)));
}

#[test]
fn pitch_roll_and_yaw_project_a_3d_turtle_onto_the_plane() {
    // pitching 90 degrees sends the heading into the screen: the move projects to nothing
    let l = LSystem::new("& g", &[], &[("angle", "90"), ("distance", "10")]);
    let d = l.draw(0, &DrawOptions::default());
    let s: Vec<_> = d.segments().collect();
    assert_eq!(s.len(), 1);
    assert!(close(s[0].x2, 0.0) && close(s[0].y2, 0.0));
    // rolling about the heading changes nothing, but a roll followed by a pitch swings it sideways
    let l = LSystem::new("/ & g", &[], &[("angle", "90"), ("distance", "10")]);
    let d = l.draw(0, &DrawOptions::default());
    let s: Vec<_> = d.segments().collect();
    assert!(close(s[0].x2, 10.0) && close(s[0].y2, 0.0));
    let l = LSystem::new("/ g", &[], &[("angle", "90"), ("distance", "10")]);
    let s: Vec<_> = l
        .draw(0, &DrawOptions::default())
        .segments()
        .cloned()
        .collect();
    assert!(close(s[0].x2, 0.0) && close(s[0].y2, -10.0));
}

#[test]
fn svg_has_one_line_element_per_segment_and_jflap_sized_view_box() {
    let l = koch();
    let svg = l.to_svg(1);
    assert!(svg.starts_with("<svg "));
    assert_eq!(svg.matches("<line ").count(), 12);
    assert_eq!(svg.matches("<polygon ").count(), 0);
    let d = l.draw(1, &DrawOptions::default());
    // JFLAP renders into an image of the bounds plus 5 pixels on every side
    let w = d.bounds.width as i64 + 10;
    let h = d.bounds.height as i64 + 10;
    assert!(
        svg.contains(&format!("width=\"{w}\" height=\"{h}\"")),
        "{svg}"
    );
    assert!(svg.ends_with("</svg>"));
}

#[test]
fn expansion_is_deterministic_for_a_deterministic_system() {
    let l = koch();
    assert!(!l.is_nondeterministic());
    let a = l.expander(1).expansion_for_level(3).to_vec();
    let b = l.expander(99).expansion_for_level(3).to_vec();
    assert_eq!(a, b);
}

#[test]
fn stochastic_rules_follow_java_util_random_with_the_seed() {
    let l = LSystem::new("a", &[("a", "a b a"), ("a", "b a")], &[]);
    assert!(l.is_nondeterministic());
    let one = |seed| l.expander(seed).expansion_for_level(8).to_vec();
    assert_eq!(one(7), one(7));
    assert_ne!(one(7), one(8));
    // java.util.Random(0).nextInt(2) is 1, so level 1 picks the second replacement
    assert_eq!(l.expander(0).expansion_for_level(1), ["b", "a"]);
}

#[test]
fn context_sensitive_rules_use_the_leading_number_as_centre() {
    // `1 x y`: the symbol at index 1 of the pattern (the `y`) is replaced when preceded by x
    let l = LSystem::new("x y y", &[("1 x y", "Z")], &[]);
    assert_eq!(l.expand(1), words("x Z y"));
    assert_eq!(l.expand(2), words("x Z y"));
}

#[test]
fn replacements_are_listed_per_symbol_in_sorted_order() {
    let l = LSystem::new(
        "b",
        &[("b", "a  a"), ("a", "b"), ("a", "b"), ("a", "b b")],
        &[],
    );
    let syms: Vec<_> = l.symbols_with_replacements().collect();
    assert_eq!(syms, ["a", "b"]);
    // identical productions are stored once (the grammar drops duplicates)
    assert_eq!(l.replacements("a"), [words("b"), words("b b")]);
    assert_eq!(l.replacements("b"), [words("a a")]);
    assert!(l.replacements("zz").is_empty());
    assert!(l.is_nondeterministic());
}

#[test]
fn an_expander_caches_its_levels() {
    let l = koch();
    let mut e = Expander::new(&l, 0);
    assert_eq!(e.expansion_for_level(0), words("g - - g - - g"));
    let first = e.expansion_for_level(2).to_vec();
    assert_eq!(e.expansion_for_level(2), first);
}
