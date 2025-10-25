use std::borrow::Cow;
use std::ptr;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::constants::{DateUnits, MAX_L_DATE, MAX_S_DATE, MIN_L_DATE, MIN_S_DATE};
use crate::parser::model::{
    DateToken, DateTokenKind, NumberPart, NumberToken, Section, SectionToken, StringRule, Token,
    TokenKind,
};

use super::{
    error::FormatterError,
    general::format_general,
    locale::{Locale, default_locale},
    math::{clamp, dec2frac, get_exponent, get_significand, round},
    options::FormatterOptions,
    pad::pad,
    serial::date_from_serial,
};

const DAYSIZE: f64 = 86_400.0;
const MAX_SAFE_INTEGER: i128 = 9_007_199_254_740_991;
const MIN_SAFE_INTEGER: i128 = -9_007_199_254_740_991;

#[derive(Debug, Clone)]
pub enum RunValue<'a> {
    Number(f64),
    BigInt(&'a BigInt),
    Text(Cow<'a, str>),
}

impl<'a> From<f64> for RunValue<'a> {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl<'a> From<&'a BigInt> for RunValue<'a> {
    fn from(value: &'a BigInt) -> Self {
        Self::BigInt(value)
    }
}

impl<'a> From<&'a str> for RunValue<'a> {
    fn from(value: &'a str) -> Self {
        Self::Text(Cow::Borrowed(value))
    }
}

impl<'a> From<String> for RunValue<'a> {
    fn from(value: String) -> Self {
        Self::Text(Cow::Owned(value))
    }
}

pub fn run_part(
    value: RunValue<'_>,
    part: &Section,
    opts: &FormatterOptions,
    locale: &Locale,
) -> Result<String, FormatterError> {
    let mut numeric_value = match value {
        RunValue::Number(n) => Some(n),
        RunValue::BigInt(big) => {
            if BigInt::from(MIN_SAFE_INTEGER) <= *big && *big <= BigInt::from(MAX_SAFE_INTEGER) {
                big.to_f64()
            } else {
                return Ok(if opts.bigint_error_number {
                    big.to_string()
                } else {
                    opts.overflow.clone()
                });
            }
        }
        RunValue::Text(_) => None,
    };

    let text_value = match &value {
        RunValue::Text(cow) => Some(cow.as_ref()),
        _ => None,
    };

    let mut mantissa = String::new();
    let mut mantissa_sign = String::new();
    let mut numerator = String::new();
    let mut denominator = String::new();
    let mut fraction = String::new();
    let mut integer = String::new();
    let mut exponent = 0i32;
    let mut date = numeric_value.map(|n| n.trunc()).unwrap_or(0.0);
    let mut time = 0.0;
    let mut year = 0i32;
    let mut month = 1u8;
    let mut day = 0i32;
    let mut weekday = 0usize;
    let mut hour = 0i32;
    let mut minute = 0i32;
    let mut second = 0i32;
    let mut subsec = 0.0f64;

    if !part.text && part.scale.is_finite() && (part.scale - 1.0).abs() > f64::EPSILON {
        if let Some(num) = numeric_value {
            numeric_value = Some(clamp(num * part.scale));
        }
    }

    if part.exponential {
        if let Some(mut val) = numeric_value {
            let mut abs_val = val.abs();
            if abs_val != 0.0 {
                exponent = get_exponent(abs_val, part.int_max);
            }
            if val != 0.0 && !part.integer {
                exponent += 1;
            }
            abs_val = get_significand(abs_val, exponent);
            if part.int_max == 1 && round(abs_val, part.frac_max) == 10.0 {
                abs_val = 1.0;
                exponent += 1;
            }
            val = if val < 0.0 { -abs_val } else { abs_val };
            numeric_value = Some(val);
            mantissa = exponent.abs().to_string();
        }
    }

    if part.integer {
        if let Some(num) = numeric_value {
            let rounded = round(num, if part.fractions { 1 } else { part.frac_max });
            let abs_rounded = rounded.abs();
            if abs_rounded >= 1.0 {
                integer = abs_rounded.floor().to_string();
            }
        }
    }

    let frac_full = part.frac_pattern.join("");

    if part.dec_fractions && part.frac_max > 0 {
        if let Some(num) = numeric_value {
            let rounded = round(num, part.frac_max);
            let repr = rounded.to_string();
            if let Some(idx) = repr.find('.') {
                let frac_part = &repr[idx + 1..];
                fraction = frac_part.to_string();
                let mut frac_chars: Vec<char> = fraction.chars().collect();
                let pattern_chars: Vec<char> = frac_full.chars().collect();
                let mut pattern_idx = pattern_chars.len();
                let mut digit_idx = frac_chars.len();
                while pattern_idx > 0 && digit_idx > 0 {
                    pattern_idx -= 1;
                    let placeholder = pattern_chars[pattern_idx];
                    let current_digit = digit_idx - 1;
                    if (placeholder == '#' || placeholder == '?')
                        && frac_chars.get(current_digit) == Some(&'0')
                        && frac_chars.len() > part.frac_min
                        && current_digit + 1 == frac_chars.len()
                    {
                        frac_chars.pop();
                        digit_idx -= 1;
                        continue;
                    }
                    digit_idx -= 1;
                }
                fraction = frac_chars.into_iter().collect();
            }
        }
    }

    let fixed_slash =
        part.error.is_none() && (part.num_p.contains('0') || part.den_p.contains('0'));

    let mut have_fraction = fixed_slash;
    if part.fractions {
        if let Some(num) = numeric_value {
            have_fraction = fixed_slash || (num % 1.0 != 0.0);
            let fractional = if part.integer {
                (num % 1.0).abs()
            } else {
                num.abs()
            };
            if fractional != 0.0 {
                have_fraction = true;
                if let Some(den) = part.denominator {
                    denominator = den.to_string();
                    let num_val = round(fractional * den as f64, 0).round() as i64;
                    numerator = num_val.to_string();
                    if numerator == "0" {
                        numerator.clear();
                        denominator.clear();
                        have_fraction = fixed_slash;
                    }
                } else {
                    let (num_val, den_val) = dec2frac(fractional, None, Some(part.den_max));
                    numerator = num_val.to_string();
                    denominator = den_val.to_string();
                    if part.integer && numerator == "0" {
                        numerator.clear();
                        denominator.clear();
                        have_fraction = fixed_slash;
                    }
                }
            } else if num == 0.0 && !part.integer {
                have_fraction = true;
                numerator = "0".to_string();
                denominator = "1".to_string();
            }
            if part.integer && !have_fraction && num.trunc() == 0.0 {
                integer = "0".to_string();
            }
        }
    }

    let group_pri_raw = opts.grouping.get(0).copied().unwrap_or(3);
    let group_sec_raw = opts.grouping.get(1).copied().unwrap_or(group_pri_raw);
    let group_pri = group_pri_raw as usize;
    let group_sec = group_sec_raw as usize;

    if !part.date.is_empty() {
        if let Some(num) = numeric_value {
            date = num.trunc();
            let t = DAYSIZE * (num - date);
            time = t.floor();
            subsec = t - time;
            if subsec.abs() < 1e-6 {
                subsec = 0.0;
            } else if subsec > 0.9999 {
                subsec = 0.0;
                time += 1.0;
                if (time - DAYSIZE).abs() < f64::EPSILON {
                    time = 0.0;
                    date += 1.0;
                }
            }
            if subsec != 0.0 {
                let has_msec = part.date.contains(DateUnits::MILLISECOND);
                let has_csec = part.date.contains(DateUnits::CENTISECOND);
                let has_dsec = part.date.contains(DateUnits::DECISECOND);
                let should_round = if has_msec {
                    subsec > 0.9995
                } else if has_csec {
                    subsec > 0.995
                } else if has_dsec {
                    subsec > 0.95
                } else {
                    subsec >= 0.5
                };
                if should_round {
                    time += 1.0;
                    subsec = 0.0;
                }
            }
            if date != 0.0 || part.date_system != 0 {
                let dt = date_from_serial(num, part.date_system, opts.leap_1900);
                year = dt[0];
                month = dt[1] as u8;
                day = dt[2];
            }
            if time != 0.0 {
                let x = if time < 0.0 { DAYSIZE + time } else { time };
                second = (x as i64 % 60) as i32;
                minute = ((x as i64 / 60) % 60) as i32;
                hour = (((x as i64 / 60) / 60) % 60) as i32;
            }
            weekday = ((6.0 + date).rem_euclid(7.0)) as usize;

            let overflow_val = date + (time / DAYSIZE);
            if date_overflows(num, overflow_val, opts.date_span_large) {
                if opts.date_error_throws {
                    return Err(FormatterError::DateOutOfBounds);
                }
                if opts.date_error_number {
                    let mut buffer = String::new();
                    if num < 0.0 {
                        buffer.push_str(&locale.negative);
                    }
                    format_general(&mut buffer, num, part, locale);
                    return Ok(buffer);
                }
                return Ok(opts.overflow.clone());
            }
        }
    }

    let pad_q = pad('?', opts.nbsp);

    if exponent < 0 {
        mantissa_sign = "-".to_string();
    } else if part.exp_plus {
        mantissa_sign = "+".to_string();
    }

    let mut output = String::new();
    let mut counter_int = 0usize;
    let mut counter_frac = 0usize;
    let mut counter_man = 0usize;
    let mut counter_num = 0usize;
    let mut counter_den = 0usize;
    let mut denominator_fixed = false;

    let integer_chars: Vec<char> = integer.chars().collect();
    let fraction_chars: Vec<char> = fraction.chars().collect();
    let mantissa_chars: Vec<char> = mantissa.chars().collect();
    let numerator_chars: Vec<char> = numerator.chars().collect();
    let denominator_chars: Vec<char> = denominator.chars().collect();

    let negative_value = numeric_value.map_or(false, |n| n.is_sign_negative());
    let has_integer_digit = integer_chars.iter().any(|c| *c != '0');
    let has_fraction_digit = fraction_chars.iter().any(|c| *c != '0');
    let has_numerator_digit = numerator_chars.iter().any(|c| *c != '0')
        || (part.fractions && numeric_value.map_or(false, |n| n != 0.0));
    let uses_general = part.tokens.iter().any(|tok| {
        matches!(
            tok,
            SectionToken::Token(token) if token.kind == TokenKind::General
        )
    });
    let general_has_value = uses_general && numeric_value.map(|n| n != 0.0).unwrap_or(false);
    let has_value_digits =
        has_integer_digit || has_fraction_digit || has_numerator_digit || general_has_value;
    let show_negative_sign = negative_value && has_value_digits;

    for (idx, token) in part.tokens.iter().enumerate() {
        match token {
            SectionToken::String(tok) => {
                let value = match tok.rule {
                    Some(StringRule::Num) => {
                        if have_fraction {
                            tok.value.replace(' ', pad_q)
                        } else if part.num_min > 0 || part.den_min > 0 {
                            tok.value.chars().map(|_| pad_q).collect()
                        } else {
                            tok.value.replace(' ', pad_q)
                        }
                    }
                    Some(StringRule::NumPlusInt) => {
                        if have_fraction && !integer.is_empty() {
                            tok.value.replace(' ', pad_q)
                        } else if part.den_min > 0 && (!integer.is_empty() || part.num_min > 0) {
                            tok.value.chars().map(|_| pad_q).collect()
                        } else {
                            tok.value.replace(' ', pad_q)
                        }
                    }
                    Some(StringRule::Den) => {
                        if have_fraction {
                            tok.value.replace(' ', pad_q)
                        } else if part.den_min > 0 {
                            tok.value.chars().map(|_| pad_q).collect()
                        } else {
                            tok.value.replace(' ', pad_q)
                        }
                    }
                    None => tok.value.replace(' ', pad_q),
                };
                output.push_str(&value);
            }
            SectionToken::Token(tok) => match tok.kind {
                TokenKind::Space => {
                    if !should_skip_fraction_space(part, have_fraction, idx) {
                        output.push_str(pad_q);
                    }
                }
                TokenKind::Error => output.push_str(&opts.invalid),
                TokenKind::Point => {
                    if part.date.is_empty() {
                        output.push_str(&locale.decimal);
                    } else {
                        output.push_str(&token_raw(tok));
                    }
                }
                TokenKind::General => {
                    if let Some(num) = numeric_value {
                        format_general(&mut output, num, part, locale);
                    } else if let Some(text) = text_value {
                        output.push_str(text);
                    }
                }
                TokenKind::Minus => {
                    if tok.volatile && !part.date.is_empty() {
                        // no-op
                    } else if tok.volatile && numeric_value.map_or(true, |n| n >= 0.0) {
                        // skip volatile minus for non-negative numeric values or non-numeric inputs
                    } else if tok.volatile
                        && !part.fractions
                        && (part.integer || part.dec_fractions)
                    {
                        if show_negative_sign
                            && ((!integer.is_empty() && integer != "0") || !fraction.is_empty())
                        {
                            output.push_str(&locale.negative);
                        }
                    } else {
                        output.push_str(&locale.negative);
                    }
                }
                TokenKind::Plus => output.push_str(&locale.positive),
                TokenKind::Text => {
                    if let Some(text) = text_value {
                        output.push_str(text);
                    } else if let Some(num) = numeric_value {
                        output.push_str(&num.to_string());
                    }
                }
                TokenKind::Fill => {
                    if let Some(fill) = &opts.fill_char {
                        output.push_str(fill);
                        output.push_str(&token_raw(tok));
                    }
                }
                TokenKind::Skip => {
                    if let Some(skip) = &opts.skip_char {
                        output.push_str(skip);
                        output.push_str(&token_raw(tok));
                    } else {
                        output.push_str(if opts.nbsp { "\u{00A0}" } else { " " });
                    }
                }
                TokenKind::Ampm => {
                    let idx = if hour < 12 { 0 } else { 1 };
                    if tok.short && ptr::eq(locale, default_locale()) {
                        output.push(if idx == 0 { 'A' } else { 'P' });
                    } else if let Some(val) = locale.ampm.get(idx) {
                        output.push_str(val);
                    }
                }
                TokenKind::Percent => output.push('%'),
                TokenKind::Digit | TokenKind::Char | TokenKind::String | TokenKind::Escaped => {
                    output.push_str(&token_raw(tok));
                }
                TokenKind::Locale
                | TokenKind::Color
                | TokenKind::Modifier
                | TokenKind::Condition
                | TokenKind::NatNum
                | TokenKind::DbNum
                | TokenKind::Scale
                | TokenKind::Comma
                | TokenKind::Break
                | TokenKind::Calendar
                | TokenKind::Duration
                | TokenKind::DateTime
                | TokenKind::Hash
                | TokenKind::Zero
                | TokenKind::Qmark
                | TokenKind::Slash
                | TokenKind::Group => {}
                _ => {
                    output.push_str(&token_raw(tok));
                }
            },
            SectionToken::Div => {
                if have_fraction {
                    output.push('/');
                } else if part.num_min > 0
                    || part.den_min > 0
                    || part.num_p.contains('?')
                    || part.den_p.contains('?')
                {
                    output.push_str(pad_q);
                } else {
                    output.push_str(pad('#', opts.nbsp));
                }
            }
            SectionToken::Number(NumberToken {
                part: number_part,
                pattern,
            }) => match number_part {
                NumberPart::Integer => {
                    if part.int_pattern.len() == 1 {
                        let pt_chars: Vec<char> = part.int_p.chars().collect();
                        let pt_len = pt_chars.len();
                        let l = usize::max(pt_len.max(part.int_min), integer_chars.len());
                        let mut digits_str = String::new();

                        for i in (1..=l).rev() {
                            let digit = if i <= integer_chars.len() {
                                Some(integer_chars[integer_chars.len() - i])
                            } else {
                                None
                            };

                            let placeholder = if digit.is_some() {
                                None
                            } else if i <= pt_len {
                                Some(pt_chars[pt_len - i])
                            } else {
                                pt_chars.first().copied()
                            };

                            let value_piece = if let Some(ch) = digit {
                                ch.to_string()
                            } else {
                                let ph = placeholder.unwrap_or('#');
                                pad(ph, opts.nbsp).to_string()
                            };

                            let mut separator = String::new();
                            if part.grouping {
                                if let Some(base) = i.checked_sub(1) {
                                    if base >= group_pri {
                                        let n = base - group_pri;
                                        if group_sec > 0 && n % group_sec == 0 {
                                            if digit.is_some() || placeholder == Some('0') {
                                                separator.push_str(&locale.group);
                                            } else if placeholder == Some('?') {
                                                separator.push_str(pad('?', opts.nbsp));
                                            }
                                        }
                                    }
                                }
                            }

                            digits_str.push_str(&value_piece);
                            digits_str.push_str(&separator);
                        }

                        output.push_str(&digits_str);
                        counter_int += l;
                    } else {
                        counter_int += append_digit_sequence(
                            &mut output,
                            &integer_chars,
                            &part.int_p,
                            pattern,
                            counter_int,
                            opts.nbsp,
                            false,
                        );
                    }
                }
                NumberPart::Fraction => {
                    counter_frac += append_digit_sequence(
                        &mut output,
                        &fraction_chars,
                        &frac_full,
                        pattern,
                        counter_frac,
                        opts.nbsp,
                        true,
                    );
                }
                NumberPart::Mantissa => {
                    if counter_man == 0 {
                        output.push_str(&mantissa_sign);
                    }
                    counter_man += append_digit_sequence(
                        &mut output,
                        &mantissa_chars,
                        &part.man_p,
                        pattern,
                        counter_man,
                        opts.nbsp,
                        false,
                    );
                }
                NumberPart::Numerator => {
                    counter_num += append_digit_sequence(
                        &mut output,
                        &numerator_chars,
                        &part.num_p,
                        pattern,
                        counter_num,
                        opts.nbsp,
                        false,
                    );
                }
                NumberPart::Denominator => {
                    counter_den += append_fraction_denominator(
                        &mut output,
                        &denominator_chars,
                        pattern,
                        counter_den,
                        opts.nbsp,
                        &mut denominator_fixed,
                    );
                }
            },
            SectionToken::Date(date_token) => append_date_token(
                &mut output,
                date_token,
                part,
                locale,
                year,
                month,
                day,
                weekday,
                hour,
                minute,
                second,
                subsec,
                date,
                time,
                numeric_value.unwrap_or(0.0),
            ),
            SectionToken::Exp { .. } => {
                output.push_str(&locale.exponent);
            }
        }
    }

    Ok(output)
}

fn should_skip_fraction_space(part: &Section, have_fraction: bool, idx: usize) -> bool {
    if !part.fractions || have_fraction {
        return false;
    }
    let requires_padding = part.num_min > 0
        || part.den_min > 0
        || part.num_p.contains('?')
        || part.den_p.contains('?');
    if requires_padding {
        return false;
    }
    space_adjacent_to_fraction(&part.tokens, idx)
}

fn space_adjacent_to_fraction(tokens: &[SectionToken], idx: usize) -> bool {
    let prev = tokens
        .get(..idx)
        .and_then(|slice| slice.iter().rfind(|tok| !token_is_space(tok)));
    if prev.map_or(false, is_fraction_component) {
        return true;
    }
    let next = tokens
        .get(idx + 1..)
        .and_then(|slice| slice.iter().find(|tok| !token_is_space(tok)));
    next.map_or(false, is_fraction_component)
}

fn token_is_space(token: &SectionToken) -> bool {
    matches!(token, SectionToken::Token(tok) if tok.kind == TokenKind::Space)
}

fn is_fraction_component(token: &SectionToken) -> bool {
    match token {
        SectionToken::Number(num) => {
            matches!(num.part, NumberPart::Numerator | NumberPart::Denominator)
        }
        SectionToken::Div => true,
        SectionToken::String(tok) => matches!(
            tok.rule,
            Some(StringRule::Num | StringRule::NumPlusInt | StringRule::Den)
        ),
        _ => false,
    }
}

fn token_raw(token: &Token) -> String {
    match &token.value {
        crate::parser::model::TokenValue::Text(text) => text.clone(),
        crate::parser::model::TokenValue::Char(ch) => ch.to_string(),
        _ => token.raw.clone(),
    }
}

fn append_digit_sequence(
    output: &mut String,
    digits: &[char],
    full_pattern: &str,
    chunk_pattern: &str,
    offset: usize,
    nbsp: bool,
    align_left: bool,
) -> usize {
    let chunk_chars: Vec<char> = chunk_pattern.chars().collect();
    let full_len = full_pattern.chars().count();
    let chunk_len = chunk_chars.len();
    let digits_len = digits.len();

    let length = if offset == 0 && digits_len > full_len {
        chunk_len + digits_len - full_len
    } else {
        chunk_len
    };

    let mut local_offset = offset as isize;
    if !align_left && digits_len < full_len {
        local_offset += digits_len as isize - full_len as isize;
    }

    for i in 0..length {
        let idx = local_offset + i as isize;
        if idx >= 0 {
            if let Some(ch) = digits.get(idx as usize) {
                output.push(*ch);
                continue;
            }
        }
        let placeholder = chunk_chars.get(i).copied().unwrap_or('#');
        output.push_str(pad(placeholder, nbsp));
    }

    length
}

fn append_fraction_denominator(
    output: &mut String,
    digits: &[char],
    chunk_pattern: &str,
    offset: usize,
    nbsp: bool,
    denominator_fixed: &mut bool,
) -> usize {
    let chunk_chars: Vec<char> = chunk_pattern.chars().collect();
    let chunk_len = chunk_chars.len();

    for i in 0..chunk_len {
        let idx = offset + i;
        if let Some(ch) = digits.get(idx) {
            output.push(*ch);
        } else {
            let placeholder = chunk_chars.get(i).copied().unwrap_or('#');
            if "123456789".contains(placeholder) || (*denominator_fixed && placeholder == '0') {
                *denominator_fixed = true;
                output.push(if nbsp { '\u{00A0}' } else { ' ' });
            } else if !*denominator_fixed
                && i == chunk_len - 1
                && placeholder == '0'
                && digits.is_empty()
            {
                output.push('1');
            } else {
                output.push_str(pad(placeholder, nbsp));
            }
        }
    }

    chunk_len
}

fn append_date_token(
    output: &mut String,
    token: &DateToken,
    part: &Section,
    locale: &Locale,
    year: i32,
    month: u8,
    day: i32,
    weekday: usize,
    hour: i32,
    minute: i32,
    second: i32,
    subsec: f64,
    date: f64,
    time: f64,
    numeric_value: f64,
) {
    match token.kind {
        DateTokenKind::Year => {
            if year < 0 {
                output.push_str(&locale.negative);
            }
            output.push_str(&format!("{:04}", year.abs()));
        }
        DateTokenKind::YearShort => {
            let y = year % 100;
            output.push_str(&format!("{:02}", y.abs()));
        }
        DateTokenKind::Era => {}
        DateTokenKind::BuddhistYear => {
            output.push_str(&(year + 543).to_string());
        }
        DateTokenKind::BuddhistYearShort => {
            let y = (year + 543) % 100;
            output.push_str(&format!("{:02}", y));
        }
        DateTokenKind::Month => {
            if token.zero_pad && month < 10 {
                output.push('0');
            }
            output.push_str(&month.to_string());
        }
        DateTokenKind::MonthNameSingle => {
            let source = if part.date_system == crate::constants::EPOCH_1317 {
                &locale.mmmm6
            } else {
                &locale.mmmm
            };
            if let Some(ch) = source
                .get((month as usize).saturating_sub(1))
                .and_then(|s| s.chars().next())
            {
                output.push(ch);
            }
        }
        DateTokenKind::MonthNameShort => {
            let source = if part.date_system == crate::constants::EPOCH_1317 {
                &locale.mmm6
            } else {
                &locale.mmm
            };
            if let Some(name) = source.get((month as usize).saturating_sub(1)) {
                output.push_str(name);
            }
        }
        DateTokenKind::MonthName => {
            let source = if part.date_system == crate::constants::EPOCH_1317 {
                &locale.mmmm6
            } else {
                &locale.mmmm
            };
            if let Some(name) = source.get((month as usize).saturating_sub(1)) {
                output.push_str(name);
            }
        }
        DateTokenKind::WeekdayShort => {
            if let Some(name) = locale.ddd.get(weekday) {
                output.push_str(name);
            }
        }
        DateTokenKind::Weekday => {
            if let Some(name) = locale.dddd.get(weekday) {
                output.push_str(name);
            }
        }
        DateTokenKind::Day => {
            if token.zero_pad && day < 10 {
                output.push('0');
            }
            output.push_str(&day.to_string());
        }
        DateTokenKind::Hour => {
            let mut h = hour % part.clock as i32;
            if h == 0 && part.clock < 24 {
                h = part.clock as i32;
            }
            if token.zero_pad && h < 10 {
                output.push('0');
            }
            output.push_str(&h.to_string());
        }
        DateTokenKind::Minute => {
            if token.zero_pad && minute < 10 {
                output.push('0');
            }
            output.push_str(&minute.to_string());
        }
        DateTokenKind::Second => {
            if token.zero_pad && second < 10 {
                output.push('0');
            }
            output.push_str(&second.to_string());
        }
        DateTokenKind::Subsecond => {
            output.push_str(&locale.decimal);
            let frac = format!("{:.prec$}", subsec, prec = part.sec_decimals as usize);
            if let Some(fragment) = frac.split('.').nth(1) {
                let len = token.decimals as usize;
                output.push_str(&fragment[..len.min(fragment.len())]);
            }
        }
        DateTokenKind::HourElapsed => {
            if numeric_value < 0.0 {
                output.push_str(&locale.negative);
            }
            let hh = (date * 24.0) + (time / 3600.0).trunc();
            output.push_str(&format!(
                "{:0width$}",
                hh.abs() as i64,
                width = token.width.unwrap_or(2)
            ));
        }
        DateTokenKind::MinuteElapsed => {
            if numeric_value < 0.0 {
                output.push_str(&locale.negative);
            }
            let mm = (date * 1440.0) + (time / 60.0).floor();
            output.push_str(&format!(
                "{:0width$}",
                mm.abs() as i64,
                width = token.width.unwrap_or(2)
            ));
        }
        DateTokenKind::SecondElapsed => {
            if numeric_value < 0.0 {
                output.push_str(&locale.negative);
            }
            let ss = (date * DAYSIZE) + time;
            output.push_str(&format!(
                "{:0width$}",
                ss.abs() as i64,
                width = token.width.unwrap_or(2)
            ));
        }
    }
}

fn date_overflows(value: f64, rounded: f64, big_range: bool) -> bool {
    if big_range {
        value < MIN_L_DATE || rounded >= MAX_L_DATE
    } else {
        value < MIN_S_DATE || rounded >= MAX_S_DATE
    }
}
