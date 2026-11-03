//! Layout algorithms (JFLAP `automata.graph.layout`) through the public API.

use rflap_core::automaton::{Point, StateId};
use rflap_core::fa::{Fa, FaLabel};
use rflap_core::layout::{Algorithm, LayoutOptions, apply};

const RADIUS: f64 = 20.0;

fn fa(n: u32, edges: &[(StateId, StateId)]) -> Fa {
    let mut fa = Fa::new();
    for _ in 0..n {
        fa.create_state(Point { x: 0.0, y: 0.0 });
    }
    for &(from, to) in edges {
        fa.add_transition(rflap_core::automaton::Transition::new(
            from,
            to,
            FaLabel::from("a"),
        ));
    }
    fa
}

fn points(fa: &Fa) -> Vec<(f64, f64)> {
    fa.states().map(|s| (s.point.x, s.point.y)).collect()
}

/// Ten states: a cycle with chords, a self-loop and an isolated state.
fn ten_states() -> Fa {
    let mut edges: Vec<(StateId, StateId)> = (0..8).map(|i| (i, (i + 1) % 8)).collect();
    edges.extend([(0, 4), (2, 6), (1, 5), (3, 3), (8, 0)]);
    fa(10, &edges)
}

fn assert_fits(fa: &Fa, options: &LayoutOptions, what: &str) {
    let pts = points(fa);
    for &(x, y) in &pts {
        assert!(
            x >= RADIUS
                && x <= options.width - RADIUS
                && y >= RADIUS
                && y <= options.height - RADIUS,
            "{what}: ({x}, {y}) outside the box"
        );
    }
    for (i, a) in pts.iter().enumerate() {
        for b in &pts[i + 1..] {
            let d = (a.0 - b.0).hypot(a.1 - b.1);
            assert!(
                d >= 2.0 * RADIUS,
                "{what}: states {a:?} and {b:?} are only {d} apart"
            );
        }
    }
}

#[test]
fn circle_layout_of_a_path_of_four_states_has_exact_coordinates() {
    // radius = hypot(30, 30) + 30 = 72.43; the circle is shifted to the margin of 30.
    let mut a = fa(4, &[(0, 1), (1, 2), (2, 3)]);
    apply(&mut a, Algorithm::Circle, &LayoutOptions::default());
    assert_eq!(
        points(&a),
        vec![(174.0, 102.0), (102.0, 174.0), (30.0, 102.0), (102.0, 30.0)]
    );
}

#[test]
fn circle_layout_honours_vertex_buffer() {
    // radius = hypot(10, 10) + 5 = 19.14, margin 10: the extent is 2 * radius.
    let options = LayoutOptions {
        vertex_width: 10.0,
        vertex_height: 10.0,
        vertex_buffer: 5.0,
        min_distance: 0.0,
        ..LayoutOptions::default()
    };
    let mut a = fa(4, &[(0, 1), (1, 2), (2, 3)]);
    apply(&mut a, Algorithm::Circle, &options);
    assert_eq!(
        points(&a),
        vec![(48.0, 29.0), (29.0, 48.0), (10.0, 29.0), (29.0, 10.0)]
    );
}

#[test]
fn single_state_goes_to_the_margin() {
    for algorithm in [
        Algorithm::Circle,
        Algorithm::Spiral,
        Algorithm::Tree,
        Algorithm::TwoCircle,
    ] {
        let mut a = fa(1, &[]);
        apply(&mut a, algorithm, &LayoutOptions::default());
        assert_eq!(points(&a), vec![(30.0, 30.0)], "{algorithm:?}");
    }
}

#[test]
fn empty_automaton_is_left_alone() {
    for algorithm in Algorithm::ALL {
        let mut a = fa(0, &[]);
        apply(&mut a, algorithm, &LayoutOptions::default());
        assert_eq!(a.states().count(), 0);
    }
}

#[test]
fn tree_layout_stacks_a_chain_by_level() {
    let mut a = fa(3, &[(0, 1), (1, 2)]);
    apply(&mut a, Algorithm::Tree, &LayoutOptions::default());
    assert_eq!(points(&a), vec![(30.0, 30.0), (30.0, 90.0), (30.0, 150.0)]);
}

#[test]
fn tree_layout_puts_children_on_the_next_row() {
    let mut a = fa(3, &[(0, 1), (0, 2)]);
    apply(&mut a, Algorithm::Tree, &LayoutOptions::default());
    let p = points(&a);
    assert!(p[0].1 < p[1].1 && p[1].1 == p[2].1, "{p:?}");
    assert_eq!((p[2].0 - p[1].0).abs(), 60.0);
}

#[test]
fn random_layout_is_identical_for_the_same_seed_and_differs_for_another() {
    let run = |seed| {
        let mut a = ten_states();
        apply(
            &mut a,
            Algorithm::Random,
            &LayoutOptions {
                seed,
                ..LayoutOptions::default()
            },
        );
        points(&a)
    };
    assert_eq!(run(7), run(7));
    assert_ne!(run(7), run(8));
}

#[test]
fn every_algorithm_is_deterministic_for_a_seed() {
    for algorithm in Algorithm::ALL {
        let run = || {
            let mut a = ten_states();
            apply(
                &mut a,
                algorithm,
                &LayoutOptions {
                    seed: 3,
                    ..LayoutOptions::default()
                },
            );
            points(&a)
        };
        assert_eq!(run(), run(), "{algorithm:?}");
    }
}

#[test]
fn every_algorithm_keeps_ten_states_inside_the_box_and_apart() {
    let options = LayoutOptions::default();
    for algorithm in Algorithm::ALL {
        for seed in 0..5 {
            let mut a = ten_states();
            apply(
                &mut a,
                algorithm,
                &LayoutOptions {
                    seed,
                    ..LayoutOptions::default()
                },
            );
            assert_fits(&a, &options, &format!("{algorithm:?} seed {seed}"));
        }
    }
}

#[test]
fn every_algorithm_separates_unconnected_states_in_a_small_box() {
    let options = LayoutOptions {
        width: 400.0,
        height: 300.0,
        ..LayoutOptions::default()
    };
    for algorithm in Algorithm::ALL {
        let mut a = fa(10, &[]);
        apply(&mut a, algorithm, &options);
        assert_fits(&a, &options, &format!("{algorithm:?}"));
    }
}

#[test]
fn coordinates_are_integral_like_jflap() {
    let mut a = ten_states();
    apply(&mut a, Algorithm::Gem, &LayoutOptions::default());
    assert!(
        points(&a)
            .iter()
            .all(|&(x, y)| x.fract() == 0.0 && y.fract() == 0.0)
    );
}

#[test]
fn algorithm_names_round_trip() {
    for algorithm in Algorithm::ALL {
        assert_eq!(algorithm.name().parse::<Algorithm>(), Ok(algorithm));
    }
    assert_eq!("Two-Circle".parse::<Algorithm>(), Ok(Algorithm::TwoCircle));
    assert!("hexagon".parse::<Algorithm>().is_err());
}

#[test]
fn larger_pseudo_random_graphs_still_fit_and_stay_apart() {
    let options = LayoutOptions::default();
    for algorithm in Algorithm::ALL {
        for round in 0..4u64 {
            let mut x = 0x2545_F491_4F6C_DD1D_u64.wrapping_mul(round + 1);
            let mut next = move |n: u64| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x % n) as u32
            };
            let edges: Vec<(StateId, StateId)> = (0..30).map(|_| (next(25), next(25))).collect();
            let mut a = fa(25, &edges);
            apply(
                &mut a,
                algorithm,
                &LayoutOptions {
                    seed: round,
                    ..LayoutOptions::default()
                },
            );
            assert_fits(&a, &options, &format!("{algorithm:?} round {round}"));
        }
    }
}

#[test]
fn algorithm_names_agree_between_serde_and_from_str() {
    for algorithm in Algorithm::ALL {
        let json = serde_json::to_string(&algorithm).unwrap();
        assert_eq!(json, format!("\"{}\"", algorithm.name()));
        assert_eq!(algorithm.name().parse::<Algorithm>(), Ok(algorithm));
        assert_eq!(serde_json::from_str::<Algorithm>(&json).unwrap(), algorithm);
    }
    // The old serde spelling is still accepted on input.
    assert_eq!(
        serde_json::from_str::<Algorithm>("\"tree-by-degree\"").unwrap(),
        Algorithm::TreeByDegree
    );
}
