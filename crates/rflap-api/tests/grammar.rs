mod common;

use common::{EXPRESSION_GRAMMAR, err, ok, strings};
use serde_json::{Value, json};

const ANBN: &str = "S -> aSb | λ";

fn forms(result: &Value) -> Vec<String> {
    strings(&result["derivation_forms"])
}

#[test]
fn grammar_type_of_each_class() {
    for (text, class) in [
        ("S -> aS | bA\nA -> bA | λ", "right_linear"),
        ("S -> Sa | a", "left_linear"),
        ("S -> AB | a\nA -> a\nB -> b", "cnf"),
        ("S -> aSb | λ", "context_free"),
        ("S -> aSbC | ab\naC -> aaC", "context_sensitive"),
        ("S -> aSb | ab\nASb -> b", "unrestricted"),
    ] {
        let r = ok(json!({"op": "grammar_type", "grammar": text}));
        assert_eq!(r["classification"], class, "{text}");
        assert!(r["message"].as_str().unwrap().starts_with("This is"));
    }
    let r = ok(json!({"op": "grammar_type", "grammar": ANBN}));
    assert_eq!(r["report"]["context_free"], true);
    assert_eq!(r["report"]["regular"], false);
    // the grammar can be a document too
    let r = ok(json!({"op": "grammar_type",
                      "grammar": {"type": "grammar", "rules": ["S -> aS | a"]}}));
    assert_eq!(r["classification"], "right_linear");
}

#[test]
fn compact_grammar_forms_agree() {
    let text = ok(json!({"op": "normalize", "document": {"type": "grammar", "text": ANBN}}));
    let rules = ok(json!({"op": "normalize",
        "document": {"type": "grammar", "rules": ["S -> aSb", "S -> λ"]}}));
    assert_eq!(text, rules);
    assert_eq!(text["document"]["start"], "S");
    assert_eq!(text["document"]["productions"].as_array().unwrap().len(), 2);
    let start = ok(json!({"op": "normalize",
        "document": {"type": "grammar", "text": "A -> a\nS -> aS", "start": "S"}}));
    assert_eq!(start["document"]["start"], "S");
}

#[test]
fn brute_force_parse() {
    let r = ok(json!({"op": "parse", "grammar": ANBN, "input": "aabb", "parser": "brute"}));
    assert_eq!(r["parser"], "brute");
    let result = &r["results"][0];
    assert_eq!(result["input"], "aabb");
    assert_eq!(result["outcome"], "accepted");
    let f = forms(result);
    assert_eq!(f.first().unwrap(), "S");
    assert_eq!(f.last().unwrap(), "aabb");
    assert_eq!(result["tree"]["symbol"], "S");
    let r = ok(json!({"op": "parse", "grammar": ANBN, "input": "aab"}));
    assert_eq!(r["parser"], "brute");
    assert_eq!(r["results"][0]["outcome"], "rejected");
}

#[test]
fn brute_force_budget_makes_it_undecided() {
    let r = ok(
        json!({"op": "parse", "grammar": "S -> SS | a | b", "input": "abababababab",
                      "parser": "brute", "budget": 5}),
    );
    assert_eq!(r["results"][0]["outcome"], "undecided");
}

#[test]
fn unrestricted_brute_force() {
    let g = "S -> aSBC | aBC\nCB -> BC\naB -> ab\nbB -> bb\nbC -> bc\ncC -> cc";
    let r = ok(json!({"op": "parse", "grammar": g, "input": "aabbcc",
                      "parser": "brute_unrestricted", "budget": 200000}));
    assert_eq!(r["results"][0]["outcome"], "accepted");
    assert_eq!(r["results"][0]["kind"], "unrestricted");
    let f = forms(&r["results"][0]);
    assert_eq!(f.last().unwrap(), "aabbcc");
}

#[test]
fn ll1_parse() {
    let r = ok(
        json!({"op": "parse", "grammar": ANBN, "inputs": ["aabb", "aab", ""],
                      "parser": "ll1"}),
    );
    let outcomes: Vec<_> = r["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(outcomes, ["accepted", "rejected", "accepted"]);
    assert!(!r["results"][0]["steps"].as_array().unwrap().is_empty());
    assert_eq!(forms(&r["results"][0]).last().unwrap(), "aabb");
}

#[test]
fn ll1_refuses_a_left_recursive_grammar_per_input() {
    let r = ok(
        json!({"op": "parse", "grammar": EXPRESSION_GRAMMAR, "input": "i+i",
                      "parser": "ll1"}),
    );
    let result = &r["results"][0];
    assert_eq!(result["outcome"], "error");
    assert_eq!(result["error"]["code"], "parse_error");
    assert!(
        result["error"]["message"]
            .as_str()
            .unwrap()
            .contains("LL(1)")
    );
}

#[test]
fn slr1_parse() {
    let r = ok(json!({"op": "parse", "grammar": EXPRESSION_GRAMMAR,
                      "inputs": ["i+i*i", "(i+i)*i", "i+", "i)"], "parser": "slr1"}));
    let outcomes: Vec<_> = r["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(outcomes, ["accepted", "accepted", "rejected", "rejected"]);
    assert!(!r["results"][0]["steps"].as_array().unwrap().is_empty());
    assert_eq!(forms(&r["results"][0]).last().unwrap(), "i+i*i");
}

#[test]
fn cyk_parse() {
    let g = "S -> AX | AB\nX -> SB\nA -> a\nB -> b";
    let r = ok(
        json!({"op": "parse", "grammar": g, "inputs": ["aabb", "ab", "aab"],
                      "parser": "cyk"}),
    );
    let outcomes: Vec<_> = r["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(outcomes, ["accepted", "accepted", "rejected"]);
    assert_eq!(r["results"][0]["table"]["length"], 4);
    let r = ok(json!({"op": "parse", "grammar": ANBN, "input": "ab", "parser": "cyk"}));
    assert_eq!(r["results"][0]["outcome"], "error");
    assert_eq!(r["results"][0]["error"]["code"], "parse_error");
}

#[test]
fn a_symbol_outside_the_grammar_is_a_per_input_error() {
    let r = ok(json!({"op": "parse", "grammar": ANBN, "inputs": ["ab", "ax"]}));
    assert_eq!(r["results"][0]["outcome"], "accepted");
    assert_eq!(r["results"][1]["outcome"], "error");
}

#[test]
fn parse_needs_an_input() {
    let e = err(json!({"op": "parse", "grammar": ANBN}));
    assert_eq!(e["code"], "invalid_argument");
}

#[test]
fn first_and_follow_of_the_expression_grammar() {
    let r = ok(json!({"op": "parse_tables", "grammar": EXPRESSION_GRAMMAR}));
    assert_eq!(
        r["first"],
        json!({"E": ["(", "i"], "F": ["(", "i"], "T": ["(", "i"]})
    );
    assert_eq!(
        r["follow"],
        json!({
            "E": ["$", ")", "+"],
            "F": ["$", ")", "*", "+"],
            "T": ["$", ")", "*", "+"]
        })
    );
}

#[test]
fn first_and_follow_with_lambda() {
    let r = ok(json!({"op": "parse_tables", "grammar": "S -> AB\nA -> a | λ\nB -> b | λ"}));
    assert_eq!(r["first"]["S"], json!(["a", "b", "λ"]));
    assert_eq!(r["first"]["A"], json!(["a", "λ"]));
    assert_eq!(r["follow"]["A"], json!(["$", "b"]));
    assert_eq!(r["follow"]["S"], json!(["$"]));
}

#[test]
fn ll1_table() {
    let r = ok(json!({"op": "parse_tables", "grammar": ANBN}));
    let ll1 = &r["ll1"];
    assert_eq!(ll1["is_ll1"], true);
    assert_eq!(strings(&ll1["terminals"]), ["a", "b", "$"]);
    assert_eq!(ll1["table"]["S"]["a"], json!(["S -> aSb"]));
    assert_eq!(ll1["table"]["S"]["b"], json!(["S -> λ"]));
    assert_eq!(ll1["table"]["S"]["$"], json!(["S -> λ"]));
    assert_eq!(ll1["conflicts"], json!([]));

    let r = ok(json!({"op": "parse_tables", "grammar": EXPRESSION_GRAMMAR}));
    assert_eq!(r["ll1"]["is_ll1"], false);
    assert!(!r["ll1"]["conflicts"].as_array().unwrap().is_empty());
}

#[test]
fn slr1_table_of_the_expression_grammar() {
    let r = ok(json!({"op": "parse_tables", "grammar": EXPRESSION_GRAMMAR}));
    let slr = &r["slr1"];
    assert_eq!(slr["is_slr1"], true);
    assert_eq!(slr["states"].as_array().unwrap().len(), 12);
    assert_eq!(slr["productions"][0], "E' -> E");
    assert_eq!(slr["productions"].as_array().unwrap().len(), 7);
    assert_eq!(strings(&slr["terminals"]), ["(", ")", "*", "+", "i", "$"]);
    assert_eq!(strings(&slr["variables"]), ["E", "F", "T"]);
    // the first state: closure of E' -> .E, shifts on ( and i, gotos on E, T, F
    assert_eq!(slr["states"][0]["items"].as_array().unwrap().len(), 7);
    assert_eq!(slr["actions"][0]["i"], json!(["s5"]));
    assert_eq!(slr["actions"][0]["("], json!(["s1"]));
    assert_eq!(slr["gotos"][0]["E"], 2);
    assert_eq!(slr["actions"][2]["$"], json!(["acc"]));
    assert_eq!(slr["conflicts"], json!([]));

    let r = ok(json!({"op": "parse_tables", "grammar": "S -> aS | Sa | a"}));
    assert_eq!(r["slr1"]["is_slr1"], false);
    assert!(!r["slr1"]["conflicts"].as_array().unwrap().is_empty());
}

#[test]
fn user_controlled_derivation() {
    let moves = json!([{"production": 0}, {"production": 0}, {"production": 1}]);
    let r = ok(json!({"op": "user_parse", "grammar": ANBN, "target": "aabb", "moves": moves}));
    assert_eq!(r["sentential"], "aabb");
    assert_eq!(r["finished"], true);
    assert_eq!(
        strings(&r["derivation_forms"]),
        ["S", "aSb", "aaSbb", "aabb"]
    );
    assert_eq!(r["tree"]["symbol"], "S");

    let r = ok(
        json!({"op": "user_parse", "grammar": ANBN, "target": "aabb",
                      "moves": [{"production": 0}, "undo"]}),
    );
    assert_eq!(r["sentential"], "S");
    assert_eq!(r["finished"], false);
    let applicable = r["applicable"].as_array().unwrap();
    assert_eq!(applicable.len(), 2);
    assert_eq!(applicable[0]["production"], 0);
    assert_eq!(applicable[0]["text"], "S -> aSb");
    assert_eq!(applicable[0]["positions"], json!([0]));

    let r = ok(
        json!({"op": "user_parse", "grammar": "S -> SS | a", "target": "aa",
                      "moves": [{"production": 0}, {"production": 1, "position": 1}]}),
    );
    assert_eq!(r["sentential"], "Sa");
}

#[test]
fn user_parse_reports_the_invalid_move() {
    let e = err(json!({"op": "user_parse", "grammar": ANBN, "target": "ab",
                       "moves": [{"production": 0}, {"production": 7}]}));
    assert_eq!(e["code"], "invalid_move");
    assert_eq!(e["details"]["move_index"], 1);
    let e = err(json!({"op": "user_parse", "grammar": ANBN, "target": "ab", "moves": ["undo"]}));
    assert_eq!(e["code"], "invalid_move");
    let e = err(json!({"op": "user_parse", "grammar": ANBN, "target": "ax"}));
    assert_eq!(e["code"], "parse_error");
}
