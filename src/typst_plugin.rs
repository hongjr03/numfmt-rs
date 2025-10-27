use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str;
use typst_wasm_protocol::wasm_export;

/// Typst entry point for the `format` function.
#[wasm_export(export_rename = "format")]
pub fn typst_format(
    format_string_bytes: &[u8],
    value_bytes: &[u8],
    options_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    let format_str = str::from_utf8(format_string_bytes)
        .map_err(|e| format!("Format string UTF-8 error: {}", e))?;

    let value_str = str::from_utf8(value_bytes).map_err(|e| format!("Value UTF-8 error: {}", e))?;

    // Parse options, use default if empty
    let formatter_options = parse_formatter_options(options_bytes)?;

    // Try to parse as number, otherwise treat as text
    let format_value = match value_str.parse::<f64>() {
        Ok(num) => crate::FormatValue::Number(num),
        Err(_) => crate::FormatValue::Text(std::borrow::Cow::Borrowed(value_str)),
    };

    let result = crate::format_with_options(format_str, format_value, formatter_options)
        .map_err(|e| format!("Format error: {}", e))?;

    Ok(result.into_bytes())
}

/// Typst entry point for the `formatColor` function.
#[wasm_export(export_rename = "format-color")]
pub fn typst_format_color(
    format_string_bytes: &[u8],
    value_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    let format_str = str::from_utf8(format_string_bytes)
        .map_err(|e| format!("Format string UTF-8 error: {}", e))?;

    let value_str = str::from_utf8(value_bytes).map_err(|e| format!("Value UTF-8 error: {}", e))?;

    // Try to parse as number, otherwise treat as text
    let format_value = match value_str.parse::<f64>() {
        Ok(num) => crate::FormatValue::Number(num),
        Err(_) => crate::FormatValue::Text(std::borrow::Cow::Borrowed(value_str)),
    };

    // Call format_color with default options
    let color_value =
        crate::format_color(format_str, format_value, crate::FormatterOptions::default())
            .map_err(|e| format!("Format error: {}", e))?;

    match color_value {
        Some(color) => {
            // Convert ColorValue to JSON
            let response = match color {
                crate::ColorValue::String(s) => serde_json::json!({
                    "type": "string",
                    "value": s
                }),
                crate::ColorValue::Index(idx) => serde_json::json!({
                    "type": "index",
                    "value": idx
                }),
            };

            serde_json::to_vec(&response).map_err(|e| format!("JSON serialization error: {}", e))
        }
        None => {
            // No color information
            Ok("null".to_string().into_bytes())
        }
    }
}

/// Typst entry point for the `getFormatInfo` function.
/// Parse format pattern and return detailed information
/// Args: format_string (bytes), currency_symbol (bytes, optional)
/// Returns: JSON format parse result
#[wasm_export(export_rename = "get-format-info")]
pub fn typst_get_format_info(
    format_string_bytes: &[u8],
    currency_symbol_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    let format_str = str::from_utf8(format_string_bytes)
        .map_err(|e| format!("Format string UTF-8 error: {}", e))?;

    // Optional currency symbol
    let _currency_symbol = if !currency_symbol_bytes.is_empty() {
        str::from_utf8(currency_symbol_bytes).ok()
    } else {
        None
    };

    let parsed =
        crate::parser::parse_pattern(format_str).map_err(|e| format!("Parse error: {}", e))?;

    let sections: Vec<SectionInfo> = parsed
        .partitions
        .iter()
        .enumerate()
        .map(|(i, section)| {
            let tokens: Vec<TokenInfo> = section
                .tokens
                .iter()
                .map(|token| match token {
                    crate::parser::SectionToken::Token(t) => TokenInfo {
                        kind: format!("{:?}", t.kind),
                        value: match &t.value {
                            crate::parser::TokenValue::Text(s) => s.clone(),
                            crate::parser::TokenValue::Char(c) => c.to_string(),
                            crate::parser::TokenValue::None => String::new(),
                            _ => format!("{:?}", t.value),
                        },
                    },
                    crate::parser::SectionToken::String(s) => TokenInfo {
                        kind: "String".to_string(),
                        value: s.value.clone(),
                    },
                    crate::parser::SectionToken::Number(n) => TokenInfo {
                        kind: format!("Number({:?})", n.part),
                        value: n.pattern.clone(),
                    },
                    crate::parser::SectionToken::Date(d) => TokenInfo {
                        kind: format!("Date({:?})", d.unit),
                        value: format!("{:?}", d.kind),
                    },
                    crate::parser::SectionToken::Div => TokenInfo {
                        kind: "Div".to_string(),
                        value: "/".to_string(),
                    },
                    crate::parser::SectionToken::Exp { plus } => TokenInfo {
                        kind: "Exp".to_string(),
                        value: if *plus { "E+" } else { "E" }.to_string(),
                    },
                })
                .collect();

            SectionInfo {
                index: i,
                content: format!("{:?}", section),
                tokens,
            }
        })
        .collect();

    let response = ParseResponse {
        success: true,
        sections,
        error: None,
    };

    serde_json::to_vec(&response).map_err(|e| format!("JSON serialization error: {}", e))
}

/// Typst entry point for the `getLocale` function.
/// Get list of supported locales
/// Args: none
/// Returns: JSON format locale list (as string)
#[wasm_export(export_rename = "get-locale")]
pub fn typst_get_locale() -> Result<Vec<u8>, String> {
    // Dynamically read all supported locales from locales.json file
    let locales_json = include_str!("formatter/locales.json");

    let data = serde_json::from_str::<serde_json::Value>(locales_json)
        .map_err(|e| format!("Failed to parse locales.json: {}", e))?;

    let locales_obj = data
        .get("locales")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "Invalid locales.json format".to_string())?;

    let locales: Vec<String> = locales_obj.keys().cloned().collect();
    let response = LocaleResponse { locales };

    serde_json::to_vec(&response).map_err(|e| format!("JSON serialization error: {}", e))
}

// Response structs
#[derive(Serialize, Deserialize)]
struct ParseResponse {
    success: bool,
    sections: Vec<SectionInfo>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SectionInfo {
    index: usize,
    content: String,
    tokens: Vec<TokenInfo>,
}

#[derive(Serialize, Deserialize)]
struct TokenInfo {
    kind: String,
    value: String,
}

#[derive(Serialize, Deserialize)]
struct LocaleResponse {
    locales: Vec<String>,
}

/// Parse formatter options
/// If options is empty, return default options
fn parse_formatter_options(options: &[u8]) -> Result<crate::FormatterOptions, String> {
    // If options is empty, use default values
    if options.is_empty() {
        return Ok(crate::FormatterOptions::default());
    }

    let options_str = str::from_utf8(options).map_err(|e| format!("Options UTF-8 error: {}", e))?;

    // If options is an empty string, use default values
    if options_str.trim().is_empty() {
        return Ok(crate::FormatterOptions::default());
    }

    // Try to parse JSON
    let json_value: Value = serde_json::from_str(options_str)
        .map_err(|e| format!("Options JSON parse error: {}", e))?;

    // Create FormatterOptions from JSON
    let mut formatter_options = crate::FormatterOptions::default();

    if let Value::Object(map) = json_value {
        for (key, value) in map {
            match key.as_str() {
                "locale" => {
                    if let Some(s) = value.as_str() {
                        formatter_options.locale = s.to_string();
                    }
                }
                "overflow" => {
                    if let Some(s) = value.as_str() {
                        formatter_options.overflow = s.to_string();
                    }
                }
                "invalid" => {
                    if let Some(s) = value.as_str() {
                        formatter_options.invalid = s.to_string();
                    }
                }
                "date_error_throws" => {
                    if let Some(b) = value.as_bool() {
                        formatter_options.date_error_throws = b;
                    }
                }
                "date_error_number" => {
                    if let Some(b) = value.as_bool() {
                        formatter_options.date_error_number = b;
                    }
                }
                "bigint_error_number" => {
                    if let Some(b) = value.as_bool() {
                        formatter_options.bigint_error_number = b;
                    }
                }
                "date_span_large" => {
                    if let Some(b) = value.as_bool() {
                        formatter_options.date_span_large = b;
                    }
                }
                "leap_1900" => {
                    if let Some(b) = value.as_bool() {
                        formatter_options.leap_1900 = b;
                    }
                }
                "nbsp" => {
                    if let Some(b) = value.as_bool() {
                        formatter_options.nbsp = b;
                    }
                }
                "throws" => {
                    if let Some(b) = value.as_bool() {
                        formatter_options.throws = b;
                    }
                }
                "ignore_timezone" => {
                    if let Some(b) = value.as_bool() {
                        formatter_options.ignore_timezone = b;
                    }
                }
                "index_colors" => {
                    if let Some(b) = value.as_bool() {
                        formatter_options.index_colors = b;
                    }
                }
                "grouping" => {
                    if let Some(arr) = value.as_array() {
                        let mut grouping = Vec::new();
                        for item in arr {
                            if let Some(n) = item.as_u64() {
                                if n <= u8::MAX as u64 {
                                    grouping.push(n as u8);
                                }
                            }
                        }
                        if !grouping.is_empty() {
                            formatter_options.grouping = grouping;
                        }
                    }
                }
                "skip_char" => {
                    if let Some(s) = value.as_str() {
                        formatter_options.skip_char = Some(s.to_string());
                    }
                }
                "fill_char" => {
                    if let Some(s) = value.as_str() {
                        formatter_options.fill_char = Some(s.to_string());
                    }
                }
                _ => {
                    // Ignore unknown options
                }
            }
        }
    }

    Ok(formatter_options)
}
