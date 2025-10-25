use std::cmp::max;

use crate::constants::{DateUnits, EPOCH_1317};

use super::error::ParseError;
use super::model::{
    Color, DateToken, DateTokenKind, NumberPart, NumberToken, Section, SectionToken, StringRule,
    StringToken, Token, TokenKind, TokenValue,
};

pub struct SectionParseResult {
    pub section: Section,
}

struct DateChunkState {
    token_index: usize,
    indeterminate: bool,
    used: bool,
}

pub fn parse_format_section(input_tokens: &[Token]) -> Result<SectionParseResult, ParseError> {
    let mut section = Section::new();
    let mut tokens: Vec<SectionToken> = Vec::new();

    let mut current_pattern = NumberPart::Integer;
    let mut last_number_index: Option<usize> = None;
    let mut date_chunks: Vec<DateChunkState> = Vec::new();
    let mut last_token_kind: Option<TokenKind> = None;
    let mut have_locale = false;
    let mut have_slash = false;
    let mut pattern_source = String::new();

    let mut index: usize = 0;
    let len = input_tokens.len();
    let mut tokens_used = 0usize;

    while index < len {
        let token = &input_tokens[index];
        tokens_used = index;
        pattern_source.push_str(&token.raw);

        match token.kind {
            TokenKind::General => {
                section.general = true;
                tokens.push(SectionToken::Token(token.clone()));
            }
            _ if is_num_op(token, current_pattern) => {
                let value = token_text(token).ok_or_else(|| {
                    ParseError::new("Numeric token missing textual representation")
                })?;
                let pattern_vec = pattern_vec_mut(&mut section, current_pattern);
                if matches!(last_token_kind, Some(TokenKind::Group))
                    || matches!(last_token_kind, Some(kind) if is_num_op_kind(kind, current_pattern))
                {
                    if let Some(last_chunk) = pattern_vec.last_mut() {
                        last_chunk.push_str(value);
                    }
                    if let Some(idx) = last_number_index
                        && let Some(SectionToken::Number(chunk)) = tokens.get_mut(idx)
                    {
                        chunk.pattern.push_str(value);
                    }
                } else {
                    pattern_vec.push(value.to_string());
                    let part = current_pattern;
                    let number_token = NumberToken::new(part, value);
                    tokens.push(SectionToken::Number(number_token));
                    last_number_index = Some(tokens.len() - 1);
                }
            }
            TokenKind::Paren => {
                if matches!(token_value_char(token), Some('(')) {
                    section.parens = true;
                }
                push_string(&mut tokens, token_text(token).unwrap_or_default());
            }
            TokenKind::Digit => {
                push_string(&mut tokens, token_text(token).unwrap_or_default());
            }
            TokenKind::Slash => {
                have_slash = true;
                if !pattern_vec_ref(&section, current_pattern).is_empty() {
                    if last_number_index.is_none() {
                        return Err(ParseError::new("Format pattern is missing a numerator"));
                    }
                    section.fractions = true;
                    let moved = pattern_vec_mut(&mut section, current_pattern)
                        .pop()
                        .unwrap_or_default();
                    section.num_pattern.push(moved.clone());
                    if let Some(idx) = last_number_index
                        && let Some(SectionToken::Number(number)) = tokens.get_mut(idx)
                    {
                        number.part = NumberPart::Numerator;
                        number.pattern = moved;
                    }
                    current_pattern = NumberPart::Denominator;
                    tokens.push(SectionToken::Div);
                    last_number_index = None;
                } else {
                    push_string(&mut tokens, token_text(token).unwrap_or_default());
                }
            }
            TokenKind::Comma => {
                push_string(&mut tokens, ",");
            }
            TokenKind::Scale => {
                // If we're in a date format context, treat scale as a comma character
                if !section.date.is_empty() {
                    push_string(&mut tokens, ",");
                } else {
                    let len = token.raw.chars().count();
                    section.scale = 0.001_f64.powi(len as i32);
                }
            }
            TokenKind::Group => {
                if matches!(current_pattern, NumberPart::Integer) {
                    section.grouping = true;
                } else if matches!(current_pattern, NumberPart::Denominator) {
                    return Err(ParseError::new("Cannot group denominator digits"));
                }
            }
            TokenKind::Space => {
                tokens.push(SectionToken::Token(token.clone()));
            }
            TokenKind::Break => {
                break;
            }
            TokenKind::Text => {
                section.text = true;
                tokens.push(SectionToken::Token(token.clone()));
            }
            TokenKind::Plus | TokenKind::Minus => {
                tokens.push(SectionToken::Token(token.clone()));
            }
            TokenKind::Duration => {
                handle_duration_token(token, &mut section, &mut tokens, &mut date_chunks);
            }
            TokenKind::Point => {
                if !section.date.is_empty() {
                    let mut decimals = 0usize;
                    let mut extra = String::new();
                    let mut look_index = index;
                    while decimals < 3 {
                        if let Some(next) = input_tokens.get(look_index + 1) {
                            if next.kind == TokenKind::Zero {
                                decimals += 1;
                                extra.push('0');
                                look_index += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if decimals > 0 {
                        pattern_source.push_str(&extra);
                        index = look_index;
                        let size = match decimals {
                            1 => DateUnits::DECISECOND,
                            2 => DateUnits::CENTISECOND,
                            3 => DateUnits::MILLISECOND,
                            _ => DateUnits::SECOND,
                        };
                        section.date |= size;
                        section.date_eval = true;
                        section.sec_decimals = max(section.sec_decimals, decimals as u8);
                        let mut token = DateToken::subsecond(decimals as u8);
                        token.unit = size;
                        tokens.push(SectionToken::Date(token));
                        last_number_index = None;
                    } else {
                        tokens.push(SectionToken::Token(token.clone()));
                        if section.date.is_empty() {
                            section.dec_fractions = true;
                            current_pattern = NumberPart::Fraction;
                        }
                    }
                } else {
                    tokens.push(SectionToken::Token(token.clone()));
                    section.dec_fractions = true;
                    current_pattern = NumberPart::Fraction;
                }
            }
            TokenKind::Calendar => {
                if !have_locale && let Some(value) = token_text(token) {
                    if value.eq_ignore_ascii_case("B2") {
                        section.date_system = EPOCH_1317;
                    } else {
                        section.date_system = crate::constants::EPOCH_1900;
                    }
                }
            }
            TokenKind::DateTime => {
                handle_datetime_token(token, &mut section, &mut tokens, &mut date_chunks)?;
            }
            TokenKind::Ampm => {
                section.clock = 12;
                section.date |= DateUnits::HOUR;
                section.date_eval = true;
                let mut tok = token.clone();
                tok.short = token_text(token)
                    .map(|v| v.eq_ignore_ascii_case("A/P"))
                    .unwrap_or(false);
                tokens.push(SectionToken::Token(tok));
            }
            TokenKind::Escaped | TokenKind::String | TokenKind::Char => {
                push_string(&mut tokens, token_display(token));
            }
            TokenKind::Condition => {
                if let TokenValue::Condition(cond) = &token.value {
                    section.condition = Some(cond.clone());
                }
            }
            TokenKind::Locale => {
                handle_locale_token(token, &mut section, &mut tokens);
                have_locale = true;
            }
            TokenKind::Color => {
                if let Some(color) = parse_color(token) {
                    section.color = Some(color);
                }
            }
            TokenKind::Percent => {
                section.scale = 100.0;
                section.percent = true;
                push_string(&mut tokens, "%");
            }
            TokenKind::Exp => {
                section.exponential = true;
                let plus = token_text(token).map(|s| s.contains('+')).unwrap_or(false);
                section.exp_plus = plus;
                current_pattern = NumberPart::Mantissa;
                tokens.push(SectionToken::Exp { plus });
            }
            TokenKind::Skip | TokenKind::Fill => {
                tokens.push(SectionToken::Token(token.clone()));
            }
            TokenKind::DbNum | TokenKind::NatNum => {
                // unsupported but tolerated
            }
            TokenKind::Error => {
                return Err(ParseError::new(format!(
                    "Illegal character: {}",
                    pattern_source
                )));
            }
            TokenKind::Modifier => {
                return Err(ParseError::new(format!(
                    "Unknown token modifier in {}",
                    pattern_source
                )));
            }
            _ => {
                return Err(ParseError::new(format!(
                    "Unknown token {:?} in {}",
                    token.kind, pattern_source
                )));
            }
        }

        last_token_kind = Some(token.kind);
        index += 1;
    }

    section.tokens_used = tokens_used;
    section.pattern = pattern_source;
    section.tokens = tokens;

    finalize_section(&mut section, have_slash)?;

    Ok(SectionParseResult { section })
}

fn pattern_vec_mut(section: &mut Section, part: NumberPart) -> &mut Vec<String> {
    match part {
        NumberPart::Integer => &mut section.int_pattern,
        NumberPart::Fraction => &mut section.frac_pattern,
        NumberPart::Mantissa => &mut section.man_pattern,
        NumberPart::Denominator => &mut section.den_pattern,
        NumberPart::Numerator => &mut section.num_pattern,
    }
}

fn pattern_vec_ref(section: &Section, part: NumberPart) -> &[String] {
    match part {
        NumberPart::Integer => &section.int_pattern,
        NumberPart::Fraction => &section.frac_pattern,
        NumberPart::Mantissa => &section.man_pattern,
        NumberPart::Denominator => &section.den_pattern,
        NumberPart::Numerator => &section.num_pattern,
    }
}

fn is_num_op(token: &Token, current: NumberPart) -> bool {
    is_num_op_kind(token.kind, current)
}

fn is_num_op_kind(kind: TokenKind, current: NumberPart) -> bool {
    matches!(kind, TokenKind::Hash | TokenKind::Zero | TokenKind::Qmark)
        || matches!(kind, TokenKind::Digit) && matches!(current, NumberPart::Denominator)
}

fn token_text(token: &Token) -> Option<&str> {
    match &token.value {
        TokenValue::Text(text) => Some(text.as_str()),
        _ => None,
    }
}

fn token_value_char(token: &Token) -> Option<char> {
    match &token.value {
        TokenValue::Text(text) => text.chars().next(),
        TokenValue::Char(ch) => Some(*ch),
        _ => None,
    }
}

fn token_display(token: &Token) -> String {
    match &token.value {
        TokenValue::Text(text) => text.clone(),
        TokenValue::Char(ch) => ch.to_string(),
        TokenValue::Condition(cond) => cond.operand.to_string(),
        TokenValue::None => token.raw.clone(),
    }
}

fn push_string(tokens: &mut Vec<SectionToken>, value: impl Into<String>) {
    let value = value.into();
    tokens.push(SectionToken::String(StringToken::new(value)));
}

fn handle_duration_token(
    token: &Token,
    section: &mut Section,
    tokens: &mut Vec<SectionToken>,
    date_chunks: &mut Vec<DateChunkState>,
) {
    if let Some(value) = token_text(token) {
        let first = value.chars().next().unwrap_or('h').to_ascii_lowercase();
        let pad = value.chars().count();
        let (kind, unit) = match first {
            'h' => (DateTokenKind::HourElapsed, DateUnits::HOUR),
            'm' => (DateTokenKind::MinuteElapsed, DateUnits::MINUTE),
            _ => (DateTokenKind::SecondElapsed, DateUnits::SECOND),
        };
        section.date |= unit;
        section.date_eval = true;
        let mut dt = DateToken::new(kind, unit);
        dt.width = Some(pad);
        tokens.push(SectionToken::Date(dt));
        date_chunks.push(DateChunkState {
            token_index: tokens.len() - 1,
            indeterminate: false,
            used: false,
        });
    }
}

fn handle_datetime_token(
    token: &Token,
    section: &mut Section,
    tokens: &mut Vec<SectionToken>,
    date_chunks: &mut Vec<DateChunkState>,
) -> Result<(), ParseError> {
    let value = token_text(token)
        .ok_or_else(|| ParseError::new("Date token missing value"))?
        .to_ascii_lowercase();
    let first = value.chars().next().unwrap_or('y');
    let mut dt = DateToken::new(DateTokenKind::Year, DateUnits::YEAR);
    let mut indeterminate = false;
    let mut chunk_used = false;

    match first {
        'y' => {
            if value.len() <= 2 {
                dt.kind = DateTokenKind::YearShort;
            } else {
                dt.kind = DateTokenKind::Year;
            }
            dt.unit = DateUnits::YEAR;
        }
        'e' => {
            dt.kind = DateTokenKind::Year;
            dt.unit = DateUnits::YEAR;
        }
        'b' => {
            dt.unit = DateUnits::YEAR;
            if value.len() <= 2 {
                dt.kind = DateTokenKind::BuddhistYearShort;
            } else {
                dt.kind = DateTokenKind::BuddhistYear;
            }
        }
        'd' => {
            dt.unit = DateUnits::DAY;
            if value.len() == 1 || value.len() == 2 {
                dt.kind = DateTokenKind::Day;
                dt.zero_pad = value.len() == 2;
            } else if value.len() == 3 {
                dt.kind = DateTokenKind::WeekdayShort;
            } else {
                dt.kind = DateTokenKind::Weekday;
            }
        }
        'g' => {
            dt.unit = DateUnits::empty();
            dt.kind = DateTokenKind::Era;
        }
        'h' => {
            dt.unit = DateUnits::HOUR;
            dt.kind = DateTokenKind::Hour;
            dt.zero_pad = value.contains('h') && value.len() >= 2;
        }
        'm' => {
            if value.len() == 3 {
                dt.unit = DateUnits::MONTH;
                dt.kind = DateTokenKind::MonthNameShort;
            } else if value.len() == 5 {
                dt.unit = DateUnits::MONTH;
                dt.kind = DateTokenKind::MonthNameSingle;
            } else if value.len() >= 4 {
                dt.unit = DateUnits::MONTH;
                dt.kind = DateTokenKind::MonthName;
            } else {
                let has_minute = tokens.iter().any(|tok| {
                    matches!(tok, SectionToken::Date(prev) if prev.unit.contains(DateUnits::MINUTE))
                });
                let last = date_chunks.last_mut();
                if let Some(last_chunk) = last {
                    let prev_token = &mut tokens[last_chunk.token_index];
                    if !last_chunk.used
                        && let SectionToken::Date(prev_date) = prev_token
                        && prev_date
                            .unit
                            .intersects(DateUnits::HOUR | DateUnits::SECOND)
                        && (!prev_date.unit.contains(DateUnits::SECOND) || !has_minute)
                    {
                        last_chunk.used = true;
                        dt.unit = DateUnits::MINUTE;
                        dt.kind = DateTokenKind::Minute;
                        dt.zero_pad = value.len() == 2;
                    }
                }
                if dt.kind != DateTokenKind::Minute {
                    dt.unit = DateUnits::MONTH;
                    dt.kind = DateTokenKind::Month;
                    dt.zero_pad = value.len() == 2;
                    indeterminate = true;
                }
            }
        }
        's' => {
            dt.unit = DateUnits::SECOND;
            dt.kind = DateTokenKind::Second;
            dt.zero_pad = value.len() >= 2;
            if let Some(last_chunk) = date_chunks.last_mut() {
                let prev_token = &mut tokens[last_chunk.token_index];
                if let SectionToken::Date(prev_date) = prev_token {
                    if prev_date.unit.contains(DateUnits::MINUTE) {
                        last_chunk.used = true;
                    } else if last_chunk.indeterminate {
                        prev_date.unit = DateUnits::MINUTE;
                        prev_date.kind = DateTokenKind::Minute;
                        prev_date.zero_pad = value.len() >= 2;
                        last_chunk.indeterminate = false;
                        last_chunk.used = true;
                        chunk_used = true;
                    }
                }
            }
        }
        'a' => {
            if value.len() == 3 {
                dt.unit = DateUnits::DAY;
                dt.kind = DateTokenKind::WeekdayShort;
            } else {
                dt.unit = DateUnits::DAY;
                dt.kind = DateTokenKind::Weekday;
            }
        }
        _ => {}
    }

    section.date |= dt.unit;
    section.date_eval = true;
    tokens.push(SectionToken::Date(dt));
    date_chunks.push(DateChunkState {
        token_index: tokens.len() - 1,
        indeterminate,
        used: chunk_used,
    });
    Ok(())
}

fn handle_locale_token(token: &Token, section: &mut Section, tokens: &mut Vec<SectionToken>) {
    if let Some(value) = token_text(token) {
        let mut parts = value.split('-');
        if let Some(currency) = parts.next()
            && !currency.is_empty()
        {
            push_string(tokens, currency);
        }
        let code: String = parts.collect::<Vec<_>>().join("-");
        if !code.is_empty() {
            section.locale = Some(code.clone());
            if let Ok(wincode) = i32::from_str_radix(&code, 16)
                && (wincode & 0xff0000) != 0
            {
                let cal = (wincode >> 16) & 0xff;
                if cal == 6 {
                    section.date_system = EPOCH_1317;
                }
            }
        }
    }
}

fn parse_color(token: &Token) -> Option<Color> {
    let value = token_text(token)?.to_ascii_lowercase();
    if let Some(rest) = value.strip_prefix("color") {
        let digits = rest.trim();
        if let Ok(idx) = digits.parse::<u32>() {
            return Some(Color::Index(idx));
        }
    }
    Some(Color::Named(value))
}

fn finalize_section(section: &mut Section, have_slash: bool) -> Result<(), ParseError> {
    if is_condition_only_pattern(&section.pattern) {
        section.tokens.push(SectionToken::Token(Token::new(
            TokenKind::Text,
            "@",
            TokenValue::Text("@".to_string()),
        )));
    }

    if (section.fractions && section.dec_fractions)
        || (section.grouping && section.int_pattern.is_empty())
        || (section.fractions && section.exponential)
        || (section.fractions && (section.den_pattern.is_empty() || section.num_pattern.is_empty()))
        || (have_slash && !section.fractions && section.date.is_empty())
        || (section.exponential
            && (section.int_pattern.is_empty() && section.frac_pattern.is_empty()
                || section.man_pattern.is_empty()))
    {
        return Err(ParseError::new(format!(
            "Invalid pattern: {}",
            section.pattern
        )));
    }

    compute_number_padding(section);

    if section.den_pattern.len() == 1 {
        let digits: String = section.den_pattern[0]
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();
        if let Ok(value) = digits.parse::<u32>()
            && value != 0
        {
            section.denominator = Some(value);
        }
    }

    section.integer = !section.int_p.is_empty();
    if !section.integer
        && !section.exponential
        && !section.frac_pattern.is_empty()
        && let Some(point_idx) = section
            .tokens
            .iter()
            .position(|t| matches!(t, SectionToken::Token(tok) if tok.kind == TokenKind::Point))
    {
        section.tokens.insert(
            point_idx,
            SectionToken::Number(NumberToken::new(NumberPart::Integer, "#")),
        );
        section.int_pattern = vec!["#".to_string()];
        section.int_p = "#".to_string();
        section.integer = true;
    }

    if section.fractions {
        for i in 0..section.tokens.len().saturating_sub(1) {
            let (prefix, suffix) = section.tokens.split_at_mut(i + 1);
            if let SectionToken::String(tok) = &mut prefix[i] {
                match &suffix[0] {
                    SectionToken::Number(num) if num.part == NumberPart::Numerator => {
                        tok.rule = Some(StringRule::NumPlusInt);
                    }
                    SectionToken::Div => tok.rule = Some(StringRule::Num),
                    SectionToken::Number(num) if num.part == NumberPart::Denominator => {
                        tok.rule = Some(StringRule::Den);
                    }
                    _ => {}
                }
            }
        }
    }

    if section.grouping && section.int_pattern.len() > 1 {
        section.grouping = false;
    }

    Ok(())
}

fn compute_number_padding(section: &mut Section) {
    let int_pattern = section.int_pattern.join("");
    let frac_pattern = section.frac_pattern.join("");
    let man_pattern = section.man_pattern.join("");
    let mut num_pat = section.num_pattern.join("");
    let mut den_pat = section.den_pattern.first().cloned().unwrap_or_default();

    let (int_max, _) = pad_lengths(&int_pattern);
    section.int_max = int_max;
    section.int_min = trailing_required_int(&int_pattern);

    let (frac_max, frac_min) = pad_lengths(&frac_pattern);
    section.frac_max = frac_max;
    section.frac_min = frac_min;

    let (man_max, man_min) = pad_lengths(&man_pattern);
    section.man_max = man_max;
    section.man_min = man_min;

    let enforce = den_pat.contains('?') || num_pat.contains('?');
    if enforce {
        den_pat = den_pat
            .chars()
            .map(|c| if c.is_ascii_digit() { '?' } else { c })
            .collect();
        if den_pat.ends_with('#') {
            den_pat.pop();
            den_pat.push('?');
        }
        if num_pat.ends_with('#') {
            num_pat.pop();
            num_pat.push('?');
        }
    }

    let (num_max, num_min) = pad_lengths(&num_pat);
    let (den_max, den_min) = pad_lengths(&den_pat);
    section.num_max = num_max;
    section.num_min = num_min;
    section.den_max = den_max;
    section.den_min = den_min;

    section.int_p = int_pattern;
    section.man_p = man_pattern;
    section.num_p = num_pat;
    section.den_p = den_pat;
}

fn pad_lengths(pattern: &str) -> (usize, usize) {
    let max_len = pattern.chars().count();
    let min_len = pattern.chars().filter(|&c| c != '#' && c != '?').count();
    (max_len, min_len)
}

fn trailing_required_int(pattern: &str) -> usize {
    let mut min = 0;
    for (i, ch) in pattern.chars().rev().enumerate() {
        if ch.is_ascii_digit() || ch == '?' {
            min = i + 1;
            break;
        }
    }
    min
}

fn is_condition_only_pattern(pattern: &str) -> bool {
    if !pattern.starts_with('[') {
        return false;
    }
    let mut rest = pattern;
    let mut saw_bracket = false;
    while let Some(stripped) = rest.strip_prefix('[') {
        if let Some(end) = stripped.find(']') {
            saw_bracket = true;
            rest = &stripped[end + 1..];
        } else {
            return false;
        }
        if let Some(start) = rest.find('[') {
            rest = &rest[start..];
        } else {
            break;
        }
    }
    if !saw_bracket {
        return false;
    }
    let is_duration = pattern.starts_with("[h")
        || pattern.starts_with("[H")
        || pattern.starts_with("[m")
        || pattern.starts_with("[M")
        || pattern.starts_with("[s")
        || pattern.starts_with("[S");
    if is_duration {
        return false;
    }
    rest.is_empty() || rest.starts_with(';')
}
