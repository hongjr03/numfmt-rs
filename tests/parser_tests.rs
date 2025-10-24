use numfmt_rs::constants::DateUnits;
use numfmt_rs::parser::{
    Color, ConditionOperator, NumberPart, Pattern, Section, SectionToken, TokenKind,
    parse_format_section, parse_pattern, tokenize,
};

fn parse_section(pattern: &str) -> Section {
    let tokens = tokenize(pattern).expect("tokenize");
    parse_format_section(&tokens).expect("section").section
}

fn parse_full_pattern(pattern: &str) -> Pattern {
    parse_pattern(pattern).expect("parse")
}

#[test]
fn tokenize_handles_grouping() {
    let tokens = tokenize("#,##0").expect("tokenize");
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Group));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Zero));
}

#[test]
fn parse_pattern_produces_four_partitions() {
    let pattern = parse_full_pattern("#,##0.00");
    assert_eq!(pattern.partitions.len(), 4);

    let positive = &pattern.partitions[0];
    let negative = &pattern.partitions[1];
    let zero = &pattern.partitions[2];
    let text = &pattern.partitions[3];

    let cond_pos = positive.condition.as_ref().expect("positive condition");
    assert_eq!(cond_pos.operator, ConditionOperator::Greater);
    assert_eq!(cond_pos.operand, 0.0);

    let cond_neg = negative.condition.as_ref().expect("negative condition");
    assert_eq!(cond_neg.operator, ConditionOperator::Less);
    assert_eq!(cond_neg.operand, 0.0);

    assert!(zero.condition.is_none());
    assert!(text.text);
}

#[test]
fn parse_pattern_conditional_inserts_minus() {
    let pattern = parse_full_pattern("[>=0]0;0");
    assert_eq!(pattern.partitions.len(), 2);

    let first = &pattern.partitions[0];
    let second = &pattern.partitions[1];

    assert!(matches!(
        first.tokens.first(),
        Some(SectionToken::Token(tok)) if tok.kind == TokenKind::Minus && tok.volatile
    ));
    assert!(matches!(
        second.tokens.first(),
        Some(SectionToken::Token(tok)) if tok.kind == TokenKind::Minus && tok.volatile
    ));
}

#[test]
fn parse_format_section_fraction() {
    let section = parse_section("0/00");
    assert!(section.fractions);
    assert_eq!(section.num_pattern.len(), 1);
    assert_eq!(section.den_pattern.len(), 1);
    assert!(
        section
            .tokens
            .iter()
            .any(|t| matches!(t, SectionToken::Number(num) if num.part == NumberPart::Numerator))
    );
}

#[test]
fn parse_pattern_rejects_excess_sections() {
    assert!(parse_pattern("a;b;c;d;").is_err());
    assert!(parse_pattern("#;#;#;#;#").is_err());
}

#[test]
fn parse_pattern_rejects_mixed_date_numeric_segments() {
    for pattern in [
        "y 0", "yyyy 0", "m #", "mmmm #", "d ?", "dddd ?", "s 0", "h #", "AM/PM 0", "[h] 0",
    ] {
        assert!(
            parse_pattern(pattern).is_err(),
            "pattern should fail: {}",
            pattern
        );
    }
}

#[test]
fn parse_pattern_promotes_text_partition() {
    let pattern = parse_full_pattern("0;@");
    assert_eq!(pattern.partitions.len(), 4);
    assert!(pattern.partitions[3].text);
    assert!(pattern.partitions[2].generated);
    assert!(!pattern.partitions[3].generated);
}

#[test]
fn parse_format_section_injects_integer_when_missing() {
    let section = parse_section(".0");
    assert!(section.integer);
    assert_eq!(section.int_pattern, vec!["#".to_string()]);
    assert!(
        section
            .tokens
            .iter()
            .any(|t| matches!(t, SectionToken::Number(num) if num.part == NumberPart::Integer))
    );
}

#[test]
fn parse_format_section_detects_percent_scaling() {
    let section = parse_section("0%");
    assert!(section.percent);
    assert!((section.scale - 100.0).abs() < f64::EPSILON);
}

#[test]
fn parse_format_section_handles_exponential_mantissa() {
    let section = parse_section("0.00E+00");
    assert!(section.exponential);
    assert_eq!(section.man_pattern, vec!["00".to_string()]);
    assert_eq!(section.man_max, 2);
}

#[test]
fn parse_format_section_handles_duration_and_ampm() {
    let section = parse_section("[h]:mm:ss AM/PM");
    assert!(section.date_eval);
    assert!(section.date.contains(DateUnits::HOUR));
    assert!(section.date.contains(DateUnits::MINUTE));
    assert!(section.date.contains(DateUnits::SECOND));
    assert_eq!(section.clock, 12);
}

#[test]
fn parse_pattern_captures_color_modifiers() {
    let pattern = parse_full_pattern("[Red]0;[Color 5]0");
    assert!(matches!(pattern.partitions[0].color, Some(Color::Named(ref name)) if name == "red"));
    assert!(matches!(pattern.partitions[1].color, Some(Color::Index(5))));
}

#[test]
fn parse_pattern_records_locale_override() {
    let pattern = parse_full_pattern("[$-0401]0");
    assert_eq!(pattern.locale.as_deref(), Some("0401"));
}

#[test]
fn parse_pattern_rejects_forbidden_terminal_b() {
    assert!(parse_pattern("B").is_err());
}
