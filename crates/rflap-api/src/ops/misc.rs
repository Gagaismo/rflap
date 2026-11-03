//! `design`, `lsystem_render` and the pumping lemma games.

use rflap_core::design::{self, DesignAutomaton};
use rflap_core::pumping::{Game, Kind, Language, Mode, catalogue};
use serde_json::{Value, json};

use super::{object, to_value, wrong_type};
use crate::document::Document;
use crate::error::{ApiError, code};
use crate::request::{LanguageRef, LsystemRender, PumpKind, PumpingMove, PumpingNew};

// ---------------------------------------------------------------------------------------------
// design
// ---------------------------------------------------------------------------------------------

pub fn design(r: crate::request::DesignArgs) -> Result<Value, ApiError> {
    let options = r.options.unwrap_or_default();
    let built = design::design(&r.spec, &options)
        .map_err(|e| ApiError::new(code::DESIGN_ERROR, e.to_string()).with_details(to_value(&e)))?;
    let mut out = object(&built);
    out.remove("automaton");
    let document = match built.automaton {
        DesignAutomaton::Fa(fa) => Document::Fa(fa),
        DesignAutomaton::Pda(pda) => Document::Pda(pda),
    };
    out.insert("document".into(), document.to_value());
    Ok(Value::Object(out))
}

// ---------------------------------------------------------------------------------------------
// lsystem_render
// ---------------------------------------------------------------------------------------------

const MAX_DEPTH: usize = 12;
const MAX_SYMBOLS: usize = 1_000_000;
const SHOWN_SYMBOLS: usize = 4_000;

pub fn lsystem_render(r: LsystemRender) -> Result<Value, ApiError> {
    let Document::Lsystem(l) = &r.document else {
        return Err(wrong_type(
            "lsystem_render",
            "an lsystem document",
            &r.document,
        ));
    };
    let depth = r.depth.unwrap_or(3);
    if depth > MAX_DEPTH {
        return Err(ApiError::new(
            code::LIMIT_EXCEEDED,
            format!("`depth` may be at most {MAX_DEPTH}"),
        ));
    }
    let options = r.options.unwrap_or_default();
    // Level by level, so a rule set that explodes is stopped before it eats the memory.
    let mut expander = l.expander(options.seed);
    let mut symbols = 0;
    for level in 0..=depth {
        symbols = expander.expansion_for_level(level).len();
        if symbols > MAX_SYMBOLS {
            return Err(ApiError::new(
                code::LIMIT_EXCEEDED,
                format!(
                    "level {level} already has more than {MAX_SYMBOLS} symbols; use a smaller `depth`"
                ),
            ));
        }
    }
    let expansion =
        (symbols <= SHOWN_SYMBOLS).then(|| expander.expansion_for_level(depth).join(" "));
    Ok(json!({
        "depth": depth,
        "svg": l.to_svg_with(depth, &options),
        "expansion_length": symbols,
        "expansion": expansion,
    }))
}

// ---------------------------------------------------------------------------------------------
// pumping lemma
// ---------------------------------------------------------------------------------------------

pub fn pumping_languages() -> Value {
    let mut languages = Vec::new();
    for (kind, name) in [
        (Kind::Regular, "regular"),
        (Kind::ContextFree, "context_free"),
    ] {
        for (index, language) in catalogue(kind).into_iter().enumerate() {
            let mut entry = object(&language.info());
            entry.insert("kind".into(), json!(name));
            entry.insert("index".into(), json!(index));
            languages.push(Value::Object(entry));
        }
    }
    json!({"languages": languages})
}

fn state_of(game: &Game, document: Document) -> serde_json::Map<String, Value> {
    let lemma = game.lemma();
    let mut out = serde_json::Map::new();
    out.insert("document".into(), document.to_value());
    out.insert("view".into(), to_value(&game.view()));
    out.insert(
        "lemma".into(),
        json!({
            "m": lemma.m(),
            "w": lemma.w(),
            "i": lemma.i(),
            "u": lemma.u(),
            "v": lemma.v(),
            "x": lemma.x(),
            "y": lemma.y(),
            "z": lemma.z(),
            "decomposition": lemma.decomposition(),
            "attempts": lemma.attempts(),
        }),
    );
    out.insert("result".into(), to_value(&game.result()));
    out.insert(
        "conclusion".into(),
        game.conclusion().map_or(Value::Null, |c| to_value(&c)),
    );
    out
}

fn wrap(kind: Kind, game: Game) -> Document {
    match kind {
        Kind::Regular => Document::RegularPumpingLemma(game),
        Kind::ContextFree => Document::ContextFreePumpingLemma(game),
    }
}

pub fn pumping_new(r: PumpingNew) -> Result<Value, ApiError> {
    let kind: Kind = match r.kind {
        PumpKind::Regular => Kind::Regular,
        PumpKind::ContextFree => Kind::ContextFree,
    };
    let language = match &r.language {
        LanguageRef::Title(title) => Language::from_title(kind, title),
        LanguageRef::Index(i) => Language::from_index(kind, *i),
    }
    .ok_or_else(|| {
        ApiError::new(
            code::INVALID_ARGUMENT,
            format!(
                "unknown pumping lemma language; use an index (0..{}) or one of the titles from `pumping_languages`",
                catalogue(kind).len() - 1
            ),
        )
    })?;
    let game = Game::new(
        language,
        r.mode.unwrap_or(Mode::ComputerFirst),
        r.seed.unwrap_or(1),
    );
    let document = wrap(kind, game.clone());
    Ok(Value::Object(state_of(&game, document)))
}

pub fn pumping_move(r: PumpingMove) -> Result<Value, ApiError> {
    let (kind, mut game) = match r.game {
        Document::RegularPumpingLemma(g) => (Kind::Regular, g),
        Document::ContextFreePumpingLemma(g) => (Kind::ContextFree, g),
        other => {
            return Err(wrong_type(
                "pumping_move",
                "a regular_pumping_lemma or context_free_pumping_lemma document",
                &other,
            ));
        }
    };
    let reply = game.play(&r.mv);
    let document = wrap(kind, game.clone());
    let mut out = state_of(&game, document);
    out.insert("reply".into(), to_value(&reply));
    Ok(Value::Object(out))
}
