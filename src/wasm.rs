use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[derive(Serialize, Deserialize)]
pub struct ParseResult {
    pub success: bool,
    pub error: Option<String>,
    pub sections: Vec<SectionInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct SectionInfo {
    pub index: usize,
    pub content: String,
    pub tokens: Vec<TokenInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct TokenInfo {
    pub kind: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct FormatResult {
    pub success: bool,
    pub error: Option<String>,
    pub result: Option<String>,
    pub color: Option<String>,
}

#[wasm_bindgen]
pub fn parse_format(pattern: &str) -> JsValue {
    let result = match crate::parser::parse_pattern(pattern) {
        Ok(parsed) => {
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

            ParseResult {
                success: true,
                error: None,
                sections,
            }
        }
        Err(e) => ParseResult {
            success: false,
            error: Some(format!("{}", e)),
            sections: vec![],
        },
    };

    serde_wasm_bindgen::to_value(&result).unwrap()
}

#[wasm_bindgen]
pub fn format_number(pattern: &str, value: f64) -> JsValue {
    format_value_internal(pattern, crate::FormatValue::Number(value))
}

#[wasm_bindgen]
pub fn format_text(pattern: &str, value: &str) -> JsValue {
    format_value_internal(
        pattern,
        crate::FormatValue::Text(std::borrow::Cow::Borrowed(value)),
    )
}

#[wasm_bindgen]
pub fn format_boolean(pattern: &str, value: bool) -> JsValue {
    format_value_internal(pattern, crate::FormatValue::Boolean(value))
}

#[wasm_bindgen]
pub fn format_null(pattern: &str) -> JsValue {
    format_value_internal(pattern, crate::FormatValue::Null)
}

fn format_value_internal(pattern: &str, value: crate::FormatValue) -> JsValue {
    let options = crate::FormatterOptions::default();

    let color_result = crate::format_color(pattern, value.clone(), options.clone());
    let format_result = crate::format_with_options(pattern, value, options);

    let result = match (format_result, color_result) {
        (Ok(formatted), Ok(color)) => FormatResult {
            success: true,
            error: None,
            result: Some(formatted),
            color: color.map(|c| format!("{:?}", c)),
        },
        (Err(e), _) | (_, Err(e)) => FormatResult {
            success: false,
            error: Some(format!("{}", e)),
            result: None,
            color: None,
        },
    };

    serde_wasm_bindgen::to_value(&result).unwrap()
}
