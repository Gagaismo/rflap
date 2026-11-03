//! Differential checks against the real JFLAP 7.1 jar.
//!
//! `fixtures/t8/PHarness.java` ran JFLAP's own pumping lemma classes with a seeded
//! `java.util.Random` (see its header) and wrote `pumping_expected.txt`. These tests run the
//! same calls on this port and compare titles, descriptions, cases, membership of every
//! string up to length 6 over `abcd`, and the computer's choices of `w`, decomposition and
//! `i` for three seeds, line by line.

use rflap_core::lsystem::JavaRandom;
use rflap_core::pumping::{Kind, Lemma, catalogue};

const EXPECTED: &str = include_str!("fixtures/t8/pumping_expected.txt");

fn show(a: &[i32]) -> String {
    a.iter().map(i32::to_string).collect::<Vec<_>>().join(",")
}

fn strings(acc: &mut Vec<String>, prefix: &str, left: usize) {
    acc.push(prefix.to_string());
    if left == 0 {
        return;
    }
    for c in "abcd".chars() {
        strings(acc, &format!("{prefix}{c}"), left - 1);
    }
}

fn parts(l: &Lemma) -> String {
    match l.kind() {
        Kind::Regular => format!("{}|{}|{}", l.x(), l.y(), l.z()),
        Kind::ContextFree => format!("{}|{}|{}|{}|{}", l.u(), l.v(), l.x(), l.y(), l.z()),
    }
}

/// What `PHarness.lemma` prints for one language.
fn dump(kind: Kind, index: usize) -> String {
    let language = catalogue(kind)[index];
    let mut l = Lemma::new(language, 1);
    let tag = match kind {
        Kind::Regular => "reg",
        Kind::ContextFree => "cf",
    };
    let mut out = vec![format!("LEMMA {tag} {index}")];
    out.push(format!("title {}", l.title()));
    out.push(format!("html {}", l.html_title()));
    let (lo, hi) = l.range();
    out.push(format!("range {lo} {hi}"));
    out.push(format!("valid {}", l.partition_is_valid()));
    out.push(format!("explanation {}", l.explanation()));
    out.push(format!("ncases {}", l.case_count()));
    l.do_all();
    for (i, d) in l.done_descriptions().iter().enumerate() {
        out.push(format!("casedesc {i} {d}"));
    }
    l.clear_done_cases();

    let mut all = Vec::new();
    strings(&mut all, "", 6);
    let mut bits = String::new();
    let (mut nib, mut count) = (0u32, 0);
    for s in &all {
        nib = nib * 2 + u32::from(l.is_in_lang(s));
        count += 1;
        if count % 4 == 0 {
            bits.push_str(&format!("{nib:x}"));
            nib = 0;
        }
    }
    while count % 4 != 0 {
        nib *= 2;
        count += 1;
        if count % 4 == 0 {
            bits.push_str(&format!("{nib:x}"));
        }
    }
    out.push(format!("lang {bits}"));

    for seed in 1..=3 {
        l.set_seed(seed);
        out.push(format!("SEED {seed}"));
        l.reset();
        l.choose_m();
        out.push(format!("chooseM {}", l.m()));
        for m in lo..=hi {
            l.set_m(m);
            out.push(format!("m {m} w {}", l.w()));
            l.do_all();
            let presets: String = l
                .done_cases()
                .iter()
                .map(|c| format!("{};", show(&c.input)))
                .collect();
            l.clear_done_cases();
            if !presets.is_empty() {
                out.push(format!("presets {presets}"));
            }
            l.choose_decomposition();
            let d = l.decomposition().map_or("null".to_string(), show);
            out.push(format!("decomp {d} parts {}", parts(&l)));
            l.choose_i();
            out.push(format!("i {}", l.i()));
            let pumped = l.create_pumped_string();
            out.push(format!("pumped {pumped} {}", l.is_in_lang(&pumped)));
            out.push(format!("decompstr {}", l.decomposition_as_string()));
            out.push(format!("winlang {}", l.is_in_lang(l.w())));
        }
    }

    if kind == Kind::ContextFree && l.case_count() > 0 {
        let mut rnd = JavaRandom::new(99);
        l.clear_done_cases();
        l.reset();
        l.set_seed(5);
        l.set_m(lo);
        let len = l.w().len() as i32;
        out.push(format!("CASES m {lo} w {}", l.w()));
        for _ in 0..60 {
            let n1 = rnd.next_int(len + 1);
            let cap = len.min(lo) + 1;
            let d = [n1, rnd.next_int(cap), rnd.next_int(cap), rnd.next_int(cap)];
            let i = rnd.next_int(3);
            // JFLAP throws when a legal-looking decomposition runs past the end of w
            let legal = d[1] + d[2] + d[3] <= lo && d[1] + d[3] >= 1;
            let ret = if legal && d.iter().sum::<i32>() > len {
                "EXC".to_string()
            } else {
                l.add_case(&d, i).to_string()
            };
            let done = if ret == "EXC" {
                format!("{}", l.done_cases().len())
            } else {
                l.done_cases().len().to_string()
            };
            if ret == "EXC" {
                out.push(format!("addcase {} i {i} -> EXC", show(&d)));
            } else {
                out.push(format!("addcase {} i {i} -> {ret} done {done}", show(&d)));
            }
        }
        for (i, c) in l.done_cases().iter().enumerate() {
            out.push(format!(
                "done {i} {} input {} i {}",
                c.description,
                show(&c.input),
                c.i
            ));
        }
        l.clear_done_cases();
    }
    out.join("\n")
}

/// The block JFLAP printed for a language.
fn expected(kind: Kind, index: usize) -> String {
    let tag = match kind {
        Kind::Regular => "reg",
        Kind::ContextFree => "cf",
    };
    let header = format!("LEMMA {tag} {index}");
    let mut lines = Vec::new();
    let mut inside = false;
    for line in EXPECTED.lines() {
        if line.starts_with("LEMMA ") {
            if inside {
                break;
            }
            inside = line == header;
        }
        if inside {
            lines.push(line);
        }
    }
    lines.join("\n")
}

fn compare(kind: Kind) {
    for index in 0..catalogue(kind).len() {
        let (got, want) = (dump(kind, index), expected(kind, index));
        assert!(!want.is_empty(), "no expected block for {kind:?} {index}");
        for (n, (g, w)) in got.lines().zip(want.lines()).enumerate() {
            assert_eq!(g, w, "{kind:?} {index}, line {} of the block", n + 1);
        }
        assert_eq!(
            got.lines().count(),
            want.lines().count(),
            "{kind:?} {index}: number of lines"
        );
    }
}

#[test]
fn regular_languages_behave_like_jflap() {
    compare(Kind::Regular);
}

#[test]
fn context_free_languages_behave_like_jflap() {
    compare(Kind::ContextFree);
}
