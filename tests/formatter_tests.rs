use num_bigint::BigInt;

use numfmt_rs::{
    ColorValue, DateValue, FormatterOptions, format, format_color, format_with_options,
};

#[test]
fn format_basic_number() {
    let output = format("#,##0.00", 1234.56).expect("format succeeded");
    assert_eq!(output, "1,234.56");
}

#[test]
fn format_percentage() {
    let output = format("0.00%", 0.4567).expect("format succeeded");
    assert_eq!(output, "45.67%");
}

#[test]
fn format_text_section() {
    let output = format("\"foo\" @ \"bar\"", "baz").expect("format succeeded");
    assert_eq!(output, "foo baz bar");
}

#[test]
fn format_negative_with_parentheses() {
    let output = format("#,##0;(#,##0)", -1234.0).expect("format succeeded");
    assert_eq!(output, "(1,234)");
}

#[test]
fn format_date_value() {
    let date = DateValue::new(2024).with_month(4).with_day(5);
    let output = format("yyyy-mm-dd", date).expect("format succeeded");
    assert_eq!(output, "2024-04-05");
}

#[test]
fn format_color_named() {
    let options = FormatterOptions::default();
    let color = format_color("[Red]#,##0;[Blue]-#,##0", 42.0, options.clone())
        .expect("color format succeeded");
    assert_eq!(color, Some(ColorValue::String("red".to_string())));

    let color =
        format_color("[Red]#,##0;[Blue]-#,##0", -42.0, options).expect("color format succeeded");
    assert_eq!(color, Some(ColorValue::String("blue".to_string())));
}

#[test]
fn format_bigint_overflow() {
    let big = BigInt::parse_bytes(b"123456789012345678901234567890", 10).unwrap();
    let output = format_with_options("#,##0", big.clone(), FormatterOptions::default())
        .expect("format succeeded");
    assert_eq!(output, "######");

    let mut options = FormatterOptions::default();
    options.bigint_error_number = true;
    let output = format_with_options("#,##0", big, options).expect("format succeeded");
    assert_eq!(output, "123456789012345678901234567890");
}

#[test]
fn format_datetime_x_comma() {
    let output = format("x,0", 1234.5677).expect("format succeeded");
    assert_eq!(output, "x,1235");
}
