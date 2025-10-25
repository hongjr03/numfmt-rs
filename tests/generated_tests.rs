use num_bigint::BigInt;
use numfmt_rs::{FormatterOptions, LocaleSettings, add_locale, format, format_with_options};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::str::FromStr;
use std::sync::Once;

fn ensure_test_locales() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let settings = LocaleSettings {
            decimal: Some("·".to_string()),
            positive: Some("ᐩ".to_string()),
            negative: Some("÷".to_string()),
            percent: Some("٪".to_string()),
            exponent: Some("X".to_string()),
            ..Default::default()
        };
        let _ = add_locale(settings, "xx");
    });
}

#[derive(Debug, Deserialize)]
struct TestCase {
    pattern: String,
    value: JsonValue,
    expected: String,
    #[serde(default)]
    options: Option<JsonValue>,
}

fn load_test_cases(file_name: &str) -> Vec<TestCase> {
    let path = format!(
        "{}/tests/fixtures/numfmt/generated/{}",
        env!("CARGO_MANIFEST_DIR"),
        file_name
    );
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read test file {}: {}", path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse test file {}: {}", path, e))
}

fn convert_json_value<'a>(
    value: &'a JsonValue,
    expected_hint: Option<&'a str>,
) -> numfmt_rs::FormatValue<'a> {
    match value {
        JsonValue::Number(n) => {
            if let Some(f) = n.as_f64() {
                numfmt_rs::FormatValue::Number(f)
            } else if let Some(i) = n.as_i64() {
                numfmt_rs::FormatValue::Number(i as f64)
            } else {
                numfmt_rs::FormatValue::Number(0.0)
            }
        }
        JsonValue::String(s) => numfmt_rs::FormatValue::Text(std::borrow::Cow::Borrowed(s)),
        JsonValue::Bool(b) => numfmt_rs::FormatValue::Boolean(*b),
        JsonValue::Null => match expected_hint {
            Some("NaN") => numfmt_rs::FormatValue::Number(f64::NAN),
            Some("∞") => numfmt_rs::FormatValue::Number(f64::INFINITY),
            Some("-∞") => numfmt_rs::FormatValue::Number(f64::NEG_INFINITY),
            _ => numfmt_rs::FormatValue::Null,
        },
        JsonValue::Object(obj) => {
            // Handle special types like BigInt and Date
            if let Some(type_field) = obj.get("type") {
                if type_field.as_str() == Some("BigInt") {
                    if let Some(val) = obj.get("value").and_then(|v| v.as_str()) {
                        let bigint = BigInt::from_str(val)
                            .unwrap_or_else(|e| panic!("Failed to parse BigInt '{}': {}", val, e));
                        return numfmt_rs::FormatValue::BigInt(bigint);
                    }
                } else if type_field.as_str() == Some("Date") {
                    if let Some(serial) = obj.get("serial") {
                        if let Some(n) = serial.as_f64() {
                            return numfmt_rs::FormatValue::Number(n);
                        } else if let Some(i) = serial.as_i64() {
                            return numfmt_rs::FormatValue::Number(i as f64);
                        }
                    }
                }
            }
            panic!("Unsupported JSON object type: {:?}", value)
        }
        JsonValue::Array(_) => {
            panic!("Unsupported JSON value type: {:?}", value)
        }
    }
}

fn parse_options(options_json: Option<&JsonValue>) -> FormatterOptions {
    ensure_test_locales();
    let mut options = FormatterOptions::default();

    if let Some(JsonValue::Object(obj)) = options_json {
        if let Some(JsonValue::Bool(b)) = obj.get("leap1900") {
            options.leap_1900 = *b;
        }
        if let Some(JsonValue::Bool(b)) = obj.get("dateSpanLarge") {
            options.date_span_large = *b;
        }
        if let Some(JsonValue::Bool(b)) = obj.get("dateErrorThrows") {
            options.date_error_throws = *b;
        }
        if let Some(JsonValue::Bool(b)) = obj.get("dateErrorNumber") {
            options.date_error_number = *b;
        }
        if let Some(JsonValue::Bool(b)) = obj.get("bigintErrorNumber") {
            options.bigint_error_number = *b;
        }
        if let Some(JsonValue::Bool(b)) = obj.get("nbsp") {
            options.nbsp = *b;
        }
        if let Some(JsonValue::Bool(b)) = obj.get("throws") {
            options.throws = *b;
        }
        if let Some(JsonValue::String(s)) = obj.get("overflow") {
            options.overflow = s.clone();
        }
        if let Some(JsonValue::String(s)) = obj.get("invalid") {
            options.invalid = s.clone();
        }
        if let Some(locale_value) = obj.get("locale") {
            match locale_value {
                JsonValue::String(s) => options.locale = s.clone(),
                JsonValue::Number(n) => {
                    if let Some(code) = n.as_u64() {
                        options.locale = format!("{:04X}", (code & 0xffff) as u64);
                    }
                }
                _ => {}
            }
        }
    }

    options
}

// Include auto-generated test modules
include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
