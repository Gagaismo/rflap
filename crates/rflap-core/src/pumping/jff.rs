//! `.jff` files of type `regular pumping lemma` and `context-free pumping lemma`
//! (JFLAP `RegPumpingLemmaTransducer`, `CFPumpingLemmaTransducer`).
//!
//! A file stores the language by its title, the first player, `m`, `w`, `i`, the lengths
//! of the pieces of the decomposition, the attempts and (context-free) the cases.

use super::{Kind, Language, Lemma};
use crate::jff::{Element, JffError, parse_document, structure_type_of, write_document};

/// The first element named `name` below the root (`getElementsByTagName(name).item(0)`).
fn first<'a>(root: &'a Element, name: &str) -> Result<&'a Element, JffError> {
    root.descendant(name)
        .ok_or_else(|| JffError::missing(name, &root.name))
}

/// `getTextContent` of a leaf.
fn text<'a>(root: &'a Element, name: &str) -> Result<&'a str, JffError> {
    Ok(first(root, name)?.text.as_str())
}

/// `Integer.parseInt(text)`.
fn int(text: &str, what: &str) -> Result<i32, JffError> {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(JffError::bad_number(what, text));
    }
    text.parse().map_err(|_| JffError::bad_number(what, text))
}

fn all<'a>(el: &'a Element, name: &str, out: &mut Vec<&'a Element>) {
    for child in el.elements() {
        if child.name == name {
            out.push(child);
        }
        all(child, name, out);
    }
}

/// Reads a game from `.jff` text; the file says which lemma it is for. Randomness
/// (should the game continue) is seeded with 0.
pub fn read(xml: &str) -> Result<Lemma, JffError> {
    read_with_seed(xml, 0)
}

/// Like [`read`], seeding the generator behind the computer's choices.
pub fn read_with_seed(xml: &str, seed: i64) -> Result<Lemma, JffError> {
    let root = parse_document(xml)?;
    let found = structure_type_of(&root)?;
    let kind = match found.as_str() {
        "regular pumping lemma" => Kind::Regular,
        "context-free pumping lemma" => Kind::ContextFree,
        _ => {
            return Err(JffError::WrongType {
                expected: "regular pumping lemma or context-free pumping lemma".to_string(),
                found,
            });
        }
    };
    let title = text(&root, "name")?;
    let language = Language::from_title(kind, title)
        .ok_or_else(|| JffError::BadXml(format!("unknown pumping lemma \"{title}\"")))?;
    let mut lemma = Lemma::new(language, seed);
    lemma.set_m(int(text(&root, "m")?, "m")?);
    lemma.set_w(text(&root, "w")?);
    lemma.set_i(int(text(&root, "i")?, "i")?);
    if kind == Kind::ContextFree {
        let mut cases = Vec::new();
        all(&root, "case", &mut cases);
        for case in cases {
            let d = [
                int(text(case, "caseULength")?, "caseULength")?,
                int(text(case, "caseVLength")?, "caseVLength")?,
                int(text(case, "caseXLength")?, "caseXLength")?,
                int(text(case, "caseYLength")?, "caseYLength")?,
            ];
            let i = int(text(case, "caseI")?, "caseI")?;
            lemma.add_case(&d, i);
        }
    }
    let mut attempts = Vec::new();
    all(&root, "attempt", &mut attempts);
    for attempt in attempts {
        lemma.add_attempt(&attempt.text);
    }
    lemma.set_first_player(text(&root, "first_player")?);
    let lengths: &[(&str, &str)] = match kind {
        Kind::Regular => &[("xLength", "xLength"), ("yLength", "yLength")],
        Kind::ContextFree => &[
            ("uLength", "uLength"),
            ("vLength", "vLength"),
            ("xLength", "xLength"),
            ("yLength", "yLength"),
        ],
    };
    let mut d = Vec::new();
    for (name, what) in lengths {
        d.push(int(text(&root, name)?, what)?);
    }
    lemma.set_decomposition(&d);
    Ok(lemma)
}

/// Writes a game as `.jff` text in the layout JFLAP 7.1 writes.
pub fn write(lemma: &Lemma) -> String {
    let kind = lemma.kind();
    let mut root = Element::new("structure");
    root.push(Element::with_text("type", kind.jff_type()));
    root.push(Element::with_text("name", lemma.title()));
    root.push(Element::with_text("first_player", lemma.first_player()));
    root.push(Element::with_text("m", &lemma.m().to_string()));
    root.push(Element::with_text("w", lemma.w()));
    root.push(Element::with_text("i", &lemma.i().to_string()));
    let len = |s: &str| s.encode_utf16().count().to_string();
    if kind == Kind::ContextFree {
        root.push(Element::with_text("uLength", &len(lemma.u())));
        root.push(Element::with_text("vLength", &len(lemma.v())));
    }
    root.push(Element::with_text("xLength", &len(lemma.x())));
    root.push(Element::with_text("yLength", &len(lemma.y())));
    for attempt in lemma.attempts() {
        root.push(Element::with_text("attempt", attempt));
    }
    if kind == Kind::ContextFree {
        for case in lemma.done_cases() {
            let mut el = Element::new("case");
            let names = ["caseULength", "caseVLength", "caseXLength", "caseYLength"];
            for (name, value) in names.iter().zip(case.input) {
                el.push(Element::with_text(name, &value.to_string()));
            }
            el.push(Element::with_text("caseI", &case.i.to_string()));
            root.push(el);
        }
    }
    write_document(&root)
}
