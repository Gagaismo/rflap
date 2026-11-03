//! Differential checks against the real JFLAP 7.1 jar.
//!
//! `fixtures/t8/LHarness.java` ran JFLAP's own `Expander`, `Renderer` and `XMLCodec`
//! (see its header) and wrote `lsystem_expected.txt` and `lsystem_*.jff`. These tests
//! read the files JFLAP wrote, rewrite them byte for byte and compare expansions,
//! bounds, image sizes and the colours JFLAP painted with what this port produces.

use std::collections::{BTreeMap, BTreeSet};

use rflap_core::lsystem::{DrawOptions, LSystem, Shape, jff};

const EXPECTED: &str = include_str!("fixtures/t8/lsystem_expected.txt");

const KOCH: &str = include_str!("fixtures/t8/lsystem_koch.jff");
const PLANT: &str = include_str!("fixtures/t8/lsystem_plant.jff");
const BUSH: &str = include_str!("fixtures/t8/lsystem_bush.jff");
const COLORS: &str = include_str!("fixtures/t8/lsystem_colors.jff");
const SOLID3D: &str = include_str!("fixtures/t8/lsystem_solid3d.jff");
const VIEWED3D: &str = include_str!("fixtures/t8/lsystem_viewed3d.jff");
const STOCHASTIC: &str = include_str!("fixtures/t8/lsystem_stochastic.jff");
const CONTEXT: &str = include_str!("fixtures/t8/lsystem_context.jff");
const EMPTIES: &str = include_str!("fixtures/t8/lsystem_empties.jff");
const PARAMS: &str = include_str!("fixtures/t8/lsystem_params.jff");

const FIXTURES: [(&str, &str); 10] = [
    ("koch", KOCH),
    ("plant", PLANT),
    ("bush", BUSH),
    ("colors", COLORS),
    ("solid3d", SOLID3D),
    ("viewed3d", VIEWED3D),
    ("stochastic", STOCHASTIC),
    ("context", CONTEXT),
    ("empties", EMPTIES),
    ("params", PARAMS),
];

#[derive(Default, Debug)]
struct Case {
    expansions: Vec<String>,
    image: (i64, i64),
    bounds: [f64; 4],
    colors: BTreeMap<u32, usize>,
    expr: Option<String>,
}

fn parse_expected() -> BTreeMap<String, Case> {
    let mut cases = BTreeMap::new();
    let mut name = String::new();
    for line in EXPECTED.lines() {
        let (key, rest) = line.split_once(' ').unwrap_or((line, ""));
        if key == "CASE" {
            name = rest.to_string();
            cases.insert(name.clone(), Case::default());
            continue;
        }
        let case = cases.get_mut(&name).expect("a CASE line comes first");
        match key {
            k if k.starts_with("expand") => case.expansions.push(rest.to_string()),
            "image" => {
                let v: Vec<i64> = rest.split(' ').map(|s| s.parse().unwrap()).collect();
                case.image = (v[0], v[1]);
            }
            "bounds" => {
                let v: Vec<f64> = rest.split(' ').map(|s| s.parse().unwrap()).collect();
                case.bounds = [v[0], v[1], v[2], v[3]];
            }
            "px" => {
                let argb = u32::from_str_radix(rest.rsplit(' ').next().unwrap(), 16).unwrap();
                if argb >> 24 != 0 {
                    *case.colors.entry(argb & 0xff_ffff).or_default() += 1;
                }
            }
            "expr" => case.expr = Some(rest.to_string()),
            _ => {}
        }
    }
    cases
}

fn seed_of(name: &str) -> i64 {
    match name {
        "stochastic" => 7,
        "context" => 3,
        _ => 0,
    }
}

fn levels_of(case: &Case) -> usize {
    case.expansions.len() - 1
}

fn view_of(name: &str) -> (f64, f64, f64) {
    if name == "viewed3d" {
        (20.0, 10.0, 40.0)
    } else {
        (0.0, 0.0, 0.0)
    }
}

fn lsystem_of(name: &str, case: &Case) -> LSystem {
    match FIXTURES.iter().find(|(n, _)| *n == name) {
        Some((_, text)) => jff::read(text).expect("JFLAP wrote it"),
        None => {
            let expr = case.expr.as_ref().expect("expression cases carry theirs");
            LSystem::new(&format!("g({expr})"), &[], &[("a", "3"), ("distance", "1")])
        }
    }
}

#[test]
fn every_jflap_written_file_is_rewritten_byte_for_byte() {
    for (name, text) in FIXTURES {
        let l = jff::read(text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(jff::write(&l), text, "{name}");
    }
}

#[test]
fn expansions_match_jflap_including_stochastic_and_context_rules() {
    for (name, case) in parse_expected() {
        let l = lsystem_of(&name, &case);
        let mut ex = l.expander(seed_of(&name));
        for (level, want) in case.expansions.iter().enumerate() {
            assert_eq!(
                &ex.expansion_for_level(level).join(" "),
                want,
                "{name} level {level}"
            );
        }
    }
}

#[test]
fn bounds_and_image_size_match_jflap() {
    for (name, case) in parse_expected() {
        let l = lsystem_of(&name, &case);
        let (pitch, roll, yaw) = view_of(&name);
        let options = DrawOptions {
            seed: seed_of(&name),
            pitch,
            roll,
            yaw,
            background: None,
        };
        let d = l.draw(levels_of(&case), &options);
        let got = [d.bounds.x, d.bounds.y, d.bounds.width, d.bounds.height];
        for (g, w) in got.iter().zip(case.bounds) {
            assert!(
                (g - w).abs() < 1e-9,
                "{name}: bounds {got:?} vs {:?}",
                case.bounds
            );
        }
        assert_eq!(
            (d.bounds.width as i64 + 10, d.bounds.height as i64 + 10),
            case.image,
            "{name}"
        );
    }
}

#[test]
fn painted_colours_match_jflap() {
    for (name, case) in parse_expected() {
        if case.colors.is_empty() {
            continue;
        }
        let l = lsystem_of(&name, &case);
        let d = l.draw(levels_of(&case), &DrawOptions::default());
        let mine: BTreeSet<u32> = d
            .shapes
            .iter()
            .map(|s| match s {
                Shape::Line(l) => l.color,
                Shape::Polygon(p) => p.fill,
            })
            .map(|c| (u32::from(c.0) << 16) | (u32::from(c.1) << 8) | u32::from(c.2))
            .collect();
        // antialiased edges blend neighbouring colours into a few stray pixels
        let major: BTreeSet<u32> = case
            .colors
            .iter()
            .filter(|(_, n)| **n >= 5)
            .map(|(c, _)| *c)
            .collect();
        assert_eq!(mine, major, "{name}");
    }
}
