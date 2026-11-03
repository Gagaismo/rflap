//! Integration tests for JFLAP-style regular expressions.

use rflap_core::regex::{ParseError, Regex};

fn sym(c: char) -> Regex {
    Regex::Symbol(c)
}

fn err(input: &str) -> ParseError {
    Regex::parse(input).expect_err(input)
}

#[test]
fn parses_and_prints_a_plus_b_star_abb() {
    let re = Regex::parse("(a+b)*abb").unwrap();
    assert_eq!(
        re,
        Regex::Concat(vec![
            Regex::Star(Box::new(Regex::Union(vec![sym('a'), sym('b')]))),
            sym('a'),
            sym('b'),
            sym('b'),
        ])
    );
    assert_eq!(re.to_string(), "(a+b)*abb");
}

#[test]
fn lambda_is_written_as_lambda_or_bang() {
    let expected = Regex::Union(vec![Regex::Star(Box::new(sym('a'))), Regex::Lambda]);
    assert_eq!(Regex::parse("a*+λ").unwrap(), expected);
    assert_eq!(Regex::parse("a*+!").unwrap(), expected);
    assert_eq!(expected.to_string(), "a*+λ");
    assert_eq!(expected.to_jflap_string(), "a*+!");
}

#[test]
fn precedence_is_star_then_concat_then_union() {
    assert_eq!(
        Regex::parse("a+bc*").unwrap(),
        Regex::Union(vec![
            sym('a'),
            Regex::Concat(vec![sym('b'), Regex::Star(Box::new(sym('c')))]),
        ])
    );
}

#[test]
fn explicit_groups_survive_a_print_parse_round_trip() {
    for text in [
        "a+(b+c)",
        "a(bc)",
        "(ab)*",
        "a**",
        "(a+b)(c+d)",
        "ø",
        "(λ)a",
    ] {
        let re = Regex::parse(text).unwrap();
        assert_eq!(re.to_string(), text, "{text}");
        assert_eq!(Regex::parse(&re.to_string()).unwrap(), re, "{text}");
    }
}

#[test]
fn whitespace_is_ignored() {
    assert_eq!(
        Regex::parse(" ( a + b ) * ").unwrap(),
        Regex::parse("(a+b)*").unwrap()
    );
}

#[test]
fn errors_carry_the_character_position() {
    assert_eq!(err("(a+").position, 3);
    assert_eq!(err("").position, 0);
    assert_eq!(err("a++b").position, 2);
    assert_eq!(err("*a").position, 0);
    assert_eq!(err("a)").position, 1);
    assert_eq!(err("()").position, 1);
    assert_eq!(err("(a").position, 2);
    assert_eq!(err("a+").position, 2);
    assert_eq!(err("(+a)").position, 1);
    assert_eq!(err("aλ").position, 1);
    assert_eq!(err("λb").position, 0);
    assert!(err("(a+").to_string().contains("position 3"));
}

#[test]
fn matches_agrees_with_the_language() {
    let re = Regex::parse("(a+b)*abb").unwrap();
    for ok in ["abb", "aabb", "babb", "abababb"] {
        assert!(re.matches(ok), "{ok}");
    }
    for bad in ["", "ab", "abba", "bb", "abbc"] {
        assert!(!re.matches(bad), "{bad}");
    }
    let opt = Regex::parse("a*+λ").unwrap();
    assert!(opt.matches("") && opt.matches("aaa") && !opt.matches("b"));
    assert!(!Regex::parse("ø").unwrap().matches(""));
    assert!(Regex::parse("λ").unwrap().matches(""));
    assert!(!Regex::parse("λ").unwrap().matches("a"));
}

#[test]
fn serde_round_trip() {
    let re = Regex::parse("(a+b)*abb").unwrap();
    let json = serde_json::to_string(&re).unwrap();
    assert_eq!(serde_json::from_str::<Regex>(&json).unwrap(), re);
}
