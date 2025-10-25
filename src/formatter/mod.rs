use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::constants::INDEX_COLORS;
use crate::parser::model::{
    Color, ConditionOperator, Pattern, Section, SectionToken, Token, TokenKind, TokenValue,
};
use crate::parser::parse_pattern;
use num_traits::{Signed, ToPrimitive};

pub mod error;
mod general;
mod locale;
mod math;
pub mod options;
mod pad;
mod run_part;
mod serial;
mod to_ymd;
pub mod value;

pub use error::FormatterError;
pub use locale::{LocaleError, LocaleSettings, add_locale, default_locale};
pub use options::FormatterOptions;
pub use run_part::RunValue;
pub use value::{DateValue, FormatValue};

use locale::get_locale_or_default;
use run_part::run_part;
use serial::date_to_serial;

#[derive(Debug, Clone, PartialEq)]
pub enum ColorValue {
    String(String),
    Index(u32),
}

struct CacheEntry {
    value: CachedPattern,
}

enum CachedPattern {
    Valid(Arc<Pattern>),
    Invalid {
        message: String,
        fallback: Arc<Pattern>,
    },
}

static PATTERN_CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
static DEFAULT_TEXT_SECTION: OnceLock<Arc<Section>> = OnceLock::new();

fn pattern_cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    PATTERN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn default_text_section() -> Arc<Section> {
    DEFAULT_TEXT_SECTION
        .get_or_init(|| {
            let mut section = Section::new();
            section.text = true;
            let token = Token::new(TokenKind::Text, "@", TokenValue::Text("@".to_string()));
            section.tokens.push(SectionToken::Token(token));
            Arc::new(section)
        })
        .clone()
}

fn build_error_pattern(pattern: &str, error: &str) -> Arc<Pattern> {
    let mut section = Section::new();
    section.error = Some(error.to_string());
    let token = Token::new(TokenKind::Error, "", TokenValue::Text(error.to_string()));
    section.tokens.push(SectionToken::Token(token));
    let mut partitions = Vec::with_capacity(4);
    for _ in 0..4 {
        partitions.push(section.clone());
    }
    Arc::new(Pattern {
        pattern: pattern.to_string(),
        partitions,
        locale: None,
    })
}

fn prepare_pattern(pattern: &str, should_throw: bool) -> Result<Arc<Pattern>, FormatterError> {
    let mut cache = pattern_cache().lock().expect("pattern cache poisoned");
    if let Some(entry) = cache.get(pattern) {
        return match &entry.value {
            CachedPattern::Valid(pat) => Ok(pat.clone()),
            CachedPattern::Invalid { message, fallback } => {
                if should_throw {
                    Err(FormatterError::InvalidPattern(message.clone()))
                } else {
                    Ok(fallback.clone())
                }
            }
        };
    }

    match parse_pattern(pattern) {
        Ok(parsed) => {
            let arc = Arc::new(parsed);
            cache.insert(
                pattern.to_string(),
                CacheEntry {
                    value: CachedPattern::Valid(arc.clone()),
                },
            );
            Ok(arc)
        }
        Err(err) => {
            let message = err.to_string();
            let fallback = build_error_pattern(pattern, &message);
            cache.insert(
                pattern.to_string(),
                CacheEntry {
                    value: CachedPattern::Invalid {
                        message: message.clone(),
                        fallback: fallback.clone(),
                    },
                },
            );
            if should_throw {
                Err(FormatterError::Parse(err))
            } else {
                Ok(fallback)
            }
        }
    }
}

fn resolve_locale_tag<'a>(pattern: &'a Pattern, opts: &'a FormatterOptions) -> Option<&'a str> {
    pattern.locale.as_deref().or({
        if opts.locale.is_empty() {
            None
        } else {
            Some(opts.locale.as_str())
        }
    })
}

fn get_part(value: f64, parts: &[Section]) -> Option<&Section> {
    for part in parts.iter().take(3) {
        if let Some(cond) = &part.condition {
            let operand = cond.operand;
            let result = match cond.operator {
                ConditionOperator::Equal => value == operand,
                ConditionOperator::Greater => value > operand,
                ConditionOperator::GreaterEqual => value >= operand,
                ConditionOperator::Less => value < operand,
                ConditionOperator::LessEqual => value <= operand,
                ConditionOperator::NotEqual => value != operand,
            };
            if result {
                return Some(part);
            }
        } else {
            return Some(part);
        }
    }
    None
}

fn bigint_condition_value(value: &num_bigint::BigInt) -> f64 {
    if let Some(f) = value.to_f64() {
        f
    } else if value.is_negative() {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    }
}

fn resolve_color_from_section(section: &Section, opts: &FormatterOptions) -> Option<ColorValue> {
    let color = section.color.as_ref()?;
    match color {
        Color::Named(name) => Some(ColorValue::String(name.clone())),
        Color::Index(idx) => {
            if opts.index_colors {
                let index = (*idx).saturating_sub(1) as usize;
                let value = INDEX_COLORS.get(index).unwrap_or(&"#000");
                Some(ColorValue::String((*value).to_string()))
            } else {
                Some(ColorValue::Index(*idx))
            }
        }
    }
}

fn locale_for(pattern: &Pattern, opts: &FormatterOptions) -> &'static locale::Locale {
    let tag = resolve_locale_tag(pattern, opts);
    get_locale_or_default(tag)
}

pub fn format<'a, V>(pattern: &str, value: V) -> Result<String, FormatterError>
where
    V: Into<FormatValue<'a>>,
{
    format_with_options(pattern, value, FormatterOptions::default())
}

pub fn format_with_options<'a, V>(
    pattern: &str,
    value: V,
    options: FormatterOptions,
) -> Result<String, FormatterError>
where
    V: Into<FormatValue<'a>>,
{
    let parse_data = prepare_pattern(pattern, options.throws)?;
    let locale = locale_for(&parse_data, &options);
    let parts = &parse_data.partitions;
    let default_text = default_text_section();
    let text_section = parts.get(3).unwrap_or(default_text.as_ref());

    let value = value.into();
    match value {
        FormatValue::Null => Ok(String::new()),
        FormatValue::Boolean(flag) => {
            let text = if flag {
                locale.bool_true().to_string()
            } else {
                locale.bool_false().to_string()
            };
            run_part(
                run_part::RunValue::Text(Cow::Owned(text)),
                text_section,
                &options,
                locale,
            )
        }
        FormatValue::Text(text) => run_part(
            run_part::RunValue::Text(text),
            text_section,
            &options,
            locale,
        ),
        FormatValue::Number(num) => format_number(num, parts, &options, locale),
        FormatValue::BigInt(big) => format_bigint(big, parts, &options, locale),
        FormatValue::Date(date) => {
            if let Some(serial) = date_to_serial(&date, options.ignore_timezone) {
                format_number(serial, parts, &options, locale)
            } else {
                run_part(
                    run_part::RunValue::Text(Cow::Owned("".to_string())),
                    text_section,
                    &options,
                    locale,
                )
            }
        }
    }
}

fn format_number(
    value: f64,
    parts: &[Section],
    options: &FormatterOptions,
    locale: &locale::Locale,
) -> Result<String, FormatterError> {
    if !value.is_finite() {
        if value.is_nan() {
            return Ok(locale.nan.clone());
        }
        let mut result = String::new();
        if value.is_sign_negative() {
            result.push_str(&locale.negative);
        }
        result.push_str(&locale.infinity);
        return Ok(result);
    }

    let part = get_part(value, parts);
    if let Some(section) = part {
        run_part(run_part::RunValue::Number(value), section, options, locale)
    } else {
        Ok(options.overflow.clone())
    }
}

fn format_bigint(
    value: num_bigint::BigInt,
    parts: &[Section],
    options: &FormatterOptions,
    locale: &locale::Locale,
) -> Result<String, FormatterError> {
    let condition_value = bigint_condition_value(&value);
    let part = get_part(condition_value, parts);
    if let Some(section) = part {
        run_part(run_part::RunValue::BigInt(&value), section, options, locale)
    } else {
        Ok(options.overflow.clone())
    }
}

pub fn format_color<'a, V>(
    pattern: &str,
    value: V,
    options: FormatterOptions,
) -> Result<Option<ColorValue>, FormatterError>
where
    V: Into<FormatValue<'a>>,
{
    let value = value.into();
    let parse_data = prepare_pattern(pattern, options.throws)?;
    let parts = &parse_data.partitions;
    let default_text = default_text_section();
    let mut part: Option<&Section> = parts.get(3).or_else(|| Some(default_text.as_ref()));

    match &value {
        FormatValue::Number(num) if num.is_finite() => {
            part = get_part(*num, parts);
        }
        FormatValue::BigInt(big) => {
            let num = bigint_condition_value(big);
            part = get_part(num, parts);
        }
        _ => {}
    }

    let section = match part {
        Some(section) => section,
        None => return Ok(None),
    };

    Ok(resolve_color_from_section(section, &options))
}
