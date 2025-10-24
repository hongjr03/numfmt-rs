use std::str::FromStr;

use winnow::ascii::Caseless;
use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::{any, take_until, take_while};

use crate::constants::INVALID_PATTERN_CHARS;

use super::error::ParseError;
use super::model::{Condition, ConditionOperator, Token, TokenKind, TokenValue};

pub fn tokenize(pattern: &str) -> Result<Vec<Token>, ParseError> {
    let mut input = pattern;
    let mut tokens: Vec<Token> = Vec::new();
    let mut unresolved_commas: Vec<usize> = Vec::new();
    let mut prev_char: Option<char> = None;

    while !input.is_empty() {
        // Special handling for commas - needs context from prev and next chars
        if input.starts_with(',') {
            let comma_count = input.chars().take_while(|&c| c == ',').count();
            let raw = &input[..comma_count];
            let look_ahead = input.chars().nth(comma_count);

            let mut maybe_group = false;
            let mut maybe_scale = false;

            // Check what comes before
            if prev_char.map_or(false, is_num_op_char) {
                maybe_group = true;
                maybe_scale = true;
            } else if prev_char == Some('.') {
                maybe_scale = true;
            }

            // Check what comes after
            if maybe_group && (look_ahead.is_none() || look_ahead == Some(';')) {
                maybe_group = false;
            }
            if maybe_scale && look_ahead.map_or(false, is_num_op_char) {
                maybe_scale = false;
            }

            let (kind, unresolved) = if maybe_group && !maybe_scale {
                (TokenKind::Group, false)
            } else if !maybe_group && maybe_scale {
                (TokenKind::Scale, false)
            } else if maybe_group && maybe_scale {
                (TokenKind::Scale, true)
            } else {
                (TokenKind::Comma, false)
            };

            let token = Token::new(kind, raw, TokenValue::Text(",".to_string()));
            if unresolved {
                unresolved_commas.push(tokens.len());
            }

            prev_char = Some(',');
            tokens.push(token);
            input = &input[comma_count..];
            continue;
        }

        let (token, unresolved, last_char) = next_token
            .parse_next(&mut input)
            .map_err(|_err: ErrMode<ContextError>| {
                ParseError::new("Unexpected character in pattern")
            })
            .map(|(tok, unres)| {
                let last = tok.raw.chars().last();
                (tok, unres, last)
            })?;

        if unresolved {
            unresolved_commas.push(tokens.len());
        }

        if matches!(token.kind, TokenKind::Break) {
            unresolved_commas.clear();
        }

        if is_numeric_token(&token) {
            for idx in unresolved_commas.drain(..) {
                let t = tokens.get_mut(idx).expect("comma index");
                if matches!(t.kind, TokenKind::Scale) {
                    t.kind = TokenKind::Group;
                }
            }
        }

        prev_char = last_char.or(prev_char);
        tokens.push(token);
    }

    Ok(tokens)
}

fn next_token(input: &mut &str) -> PResult<(Token, bool)> {
    // Split into multiple alt calls to avoid tuple size limits
    alt((
        general_parser,
        simple_char_parser('#', TokenKind::Hash),
        simple_char_parser('0', TokenKind::Zero),
        simple_char_parser('?', TokenKind::Qmark),
        simple_char_parser('/', TokenKind::Slash),
        simple_char_parser(';', TokenKind::Break),
        simple_char_parser('@', TokenKind::Text),
    ))
    .parse_next(input)
    .or_else(|_: ErrMode<ContextError>| {
        alt((
            simple_char_parser('+', TokenKind::Plus),
            simple_char_parser('-', TokenKind::Minus),
            simple_char_parser('.', TokenKind::Point),
            simple_char_parser(' ', TokenKind::Space),
            simple_char_parser('%', TokenKind::Percent),
            digit_parser,
            calendar_parser,
            single_b_parser,
        ))
        .parse_next(input)
    })
    .or_else(|_: ErrMode<ContextError>| {
        alt((
            datetime_parser,
            duration_parser,
            condition_parser,
            dbnum_parser,
            natnum_parser,
            locale_parser,
            color_parser,
            modifier_parser,
        ))
        .parse_next(input)
    })
    .or_else(|_: ErrMode<ContextError>| {
        alt((
            ampm_parser,
            escaped_parser,
            string_literal_parser,
            skip_parser,
            exponent_parser,
            fill_parser,
            paren_parser,
            error_char_parser,
            fallback_char_parser,
        ))
        .parse_next(input)
    })
}

type PResult<T> = Result<T, ErrMode<ContextError>>;

fn is_numeric_token(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Hash | TokenKind::Zero | TokenKind::Qmark | TokenKind::Digit
    )
}

fn is_num_op_char(ch: char) -> bool {
    matches!(ch, '?' | '#' | '0'..='9')
}

// Parsers using winnow combinators

fn general_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    Caseless("General").parse_next(input).map(|_| {
        let raw = &start[..7];
        let token = Token::new(TokenKind::General, raw, TokenValue::Text(raw.to_string()));
        (token, false)
    })
}

fn simple_char_parser(
    ch: char,
    kind: TokenKind,
) -> impl FnMut(&mut &str) -> PResult<(Token, bool)> {
    move |input: &mut &str| {
        let c = ch;
        any.verify(move |&x| x == c).parse_next(input)?;
        let raw = c.to_string();
        let token = Token::new(kind, raw.clone(), TokenValue::Text(raw));
        Ok((token, false))
    }
}

fn digit_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let ch = any
        .verify(|c: &char| c.is_ascii_digit() && *c != '0')
        .parse_next(input)?;
    let raw = ch.to_string();
    let token = Token::new(TokenKind::Digit, raw.clone(), TokenValue::Text(raw));
    Ok((token, false))
}

fn calendar_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    alt((Caseless("B1"), Caseless("B2")))
        .parse_next(input)
        .map(|_| {
            let raw = &start[..2];
            let token = Token::new(TokenKind::Calendar, raw, TokenValue::Text(raw.to_string()));
            (token, false)
        })
}

fn single_b_parser(input: &mut &str) -> PResult<(Token, bool)> {
    if *input == "B" {
        let token = Token::new(TokenKind::Error, "B", TokenValue::Text("B".to_string()));
        *input = "";
        Ok((token, false))
    } else {
        Err(ErrMode::Backtrack(ContextError::new()))
    }
}

fn datetime_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    let first = any.parse_next(input)?;

    match first {
        'h' | 'H' | 'm' | 'M' | 's' | 'S' | 'y' | 'Y' | 'b' | 'B' | 'd' | 'D' | 'g' | 'G' => {
            let additional =
                take_while(0.., move |c: char| c.eq_ignore_ascii_case(&first)).parse_next(input)?;
            let len = first.len_utf8() + additional.len();
            let raw = &start[..len];
            let token = Token::new(TokenKind::DateTime, raw, TokenValue::Text(raw.to_string()));
            Ok((token, false))
        }
        'a' | 'A' => {
            let additional =
                take_while(0.., move |c: char| c.eq_ignore_ascii_case(&first)).parse_next(input)?;
            let count = 1 + additional.chars().count();
            if count < 3 {
                return Err(ErrMode::Backtrack(ContextError::new()));
            }
            let len = first.len_utf8() + additional.len();
            let raw = &start[..len];
            let token = Token::new(TokenKind::DateTime, raw, TokenValue::Text(raw.to_string()));
            Ok((token, false))
        }
        'e' => {
            let additional = take_while(0.., 'e').parse_next(input)?;
            let len = 1 + additional.len();
            let raw = &start[..len];
            let token = Token::new(TokenKind::DateTime, raw, TokenValue::Text(raw.to_string()));
            Ok((token, false))
        }
        'E' => Err(ErrMode::Backtrack(ContextError::new())),
        _ => Err(ErrMode::Backtrack(ContextError::new())),
    }
}

fn duration_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '['.parse_next(input)?;
    let inner = take_until(1.., ']').parse_next(input)?;
    ']'.parse_next(input)?;

    let first = inner
        .chars()
        .next()
        .ok_or_else(|| ErrMode::Backtrack(ContextError::new()))?;

    if !matches!(first, 'h' | 'H' | 'm' | 'M' | 's' | 'S') {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    if inner.chars().any(|c| !c.eq_ignore_ascii_case(&first)) {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    let raw_len = 1 + inner.len() + 1;
    let raw = &start[..raw_len];
    let token = Token::new(
        TokenKind::Duration,
        raw,
        TokenValue::Text(inner.to_string()),
    );
    Ok((token, false))
}

fn condition_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '['.parse_next(input)?;
    let inner = take_until(1.., ']').parse_next(input)?;
    ']'.parse_next(input)?;

    let mut inner_input = inner;
    let op = condition_operator_parser.parse_next(&mut inner_input)?;

    let value_str = inner_input.trim_start();
    if value_str.is_empty() {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    // Validate number format
    let mut chars = value_str.chars();
    let first = chars
        .next()
        .ok_or_else(|| ErrMode::Backtrack(ContextError::new()))?;

    if !(first == '-' || first == '.' || first.is_ascii_digit()) {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    if first == '-' {
        if let Some(second) = chars.next() {
            if !(second == '.' || second.is_ascii_digit()) {
                return Err(ErrMode::Backtrack(ContextError::new()));
            }
        } else {
            return Err(ErrMode::Backtrack(ContextError::new()));
        }
    }

    if chars.any(|c| !(c == '.' || c.is_ascii_digit())) {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    let operand = f64::from_str(value_str).map_err(|_| ErrMode::Backtrack(ContextError::new()))?;

    let condition = Condition {
        operator: op,
        operand,
        raw_operand: value_str.to_string(),
    };

    let raw_len = 1 + inner.len() + 1;
    let raw = &start[..raw_len];
    let token = Token::new(TokenKind::Condition, raw, TokenValue::Condition(condition));
    Ok((token, false))
}

fn condition_operator_parser(input: &mut &str) -> PResult<ConditionOperator> {
    alt((
        "<=".map(|_| ConditionOperator::LessEqual),
        "<>".map(|_| ConditionOperator::NotEqual),
        "<".map(|_| ConditionOperator::Less),
        ">=".map(|_| ConditionOperator::GreaterEqual),
        ">".map(|_| ConditionOperator::Greater),
        "=".map(|_| ConditionOperator::Equal),
    ))
    .parse_next(input)
}

fn dbnum_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '['.parse_next(input)?;
    let inner = take_until(1.., ']').parse_next(input)?;

    if !inner.to_ascii_lowercase().starts_with("dbnum") {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    ']'.parse_next(input)?;

    let raw_len = 1 + inner.len() + 1;
    let raw = &start[..raw_len];
    let token = Token::new(TokenKind::DbNum, raw, TokenValue::Text(inner.to_string()));
    Ok((token, false))
}

fn natnum_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '['.parse_next(input)?;
    let inner = take_until(1.., ']').parse_next(input)?;

    if !inner.to_ascii_lowercase().starts_with("natnum") {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    ']'.parse_next(input)?;

    let raw_len = 1 + inner.len() + 1;
    let raw = &start[..raw_len];
    let token = Token::new(TokenKind::NatNum, raw, TokenValue::Text(inner.to_string()));
    Ok((token, false))
}

fn locale_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    "[$".parse_next(input)?;
    let inner = take_until(1.., ']').parse_next(input)?;
    ']'.parse_next(input)?;

    let raw_len = 2 + inner.len() + 1;
    let raw = &start[..raw_len];
    let token = Token::new(TokenKind::Locale, raw, TokenValue::Text(inner.to_string()));
    Ok((token, false))
}

fn color_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '['.parse_next(input)?;
    let inner = take_until(1.., ']').parse_next(input)?;
    ']'.parse_next(input)?;

    let inner_lower = inner.trim().to_ascii_lowercase();
    let is_named = matches!(
        inner_lower.as_str(),
        "black" | "blue" | "cyan" | "green" | "magenta" | "red" | "white" | "yellow"
    );

    let is_color_index = inner_lower.starts_with("color")
        && inner_lower
            .strip_prefix("color")
            .map(|rest| rest.trim().chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(false);

    if !is_named && !is_color_index {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    let value = if is_color_index {
        inner.to_string()
    } else {
        inner_lower
    };

    let raw_len = 1 + inner.len() + 1;
    let raw = &start[..raw_len];
    let token = Token::new(TokenKind::Color, raw, TokenValue::Text(value));
    Ok((token, false))
}

fn modifier_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '['.parse_next(input)?;
    let inner = take_until(1.., ']').parse_next(input)?;

    if inner.is_empty() {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    ']'.parse_next(input)?;

    let raw_len = 1 + inner.len() + 1;
    let raw = &start[..raw_len];
    let token = Token::new(
        TokenKind::Modifier,
        raw,
        TokenValue::Text(inner.to_string()),
    );
    Ok((token, false))
}

fn ampm_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    let matched = alt(("AM/PM", "am/pm", "A/P", "a/p")).parse_next(input)?;

    let raw = &start[..matched.len()];
    let token = Token::new(TokenKind::Ampm, raw, TokenValue::Text(matched.to_string()));
    Ok((token, false))
}

fn escaped_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '\\'.parse_next(input)?;
    let next = any.parse_next(input)?;
    let len = 1 + next.len_utf8();
    let raw = &start[..len];
    let token = Token::new(TokenKind::Escaped, raw, TokenValue::Char(next));
    Ok((token, false))
}

fn string_literal_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '"'.parse_next(input)?;

    let mut len = 0;
    let mut found_close = false;

    for (idx, ch) in input.char_indices() {
        if ch == '"' {
            found_close = true;
            len = idx;
            break;
        }
    }

    if !found_close {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    let value = &input[..len];
    *input = &input[len..];
    '"'.parse_next(input)?;

    let total_len = 1 + len + 1;
    let raw = &start[..total_len];
    let token = Token::new(TokenKind::String, raw, TokenValue::Text(value.to_string()));
    Ok((token, false))
}

fn skip_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '_'.parse_next(input)?;

    let (value, len) = if input.starts_with('\\') {
        let mut temp = *input;
        '\\'.parse_next(&mut temp)?;
        let next = any.parse_next(&mut temp)?;
        let l = 1 + next.len_utf8();
        *input = temp;
        (format!("\\{}", next), l)
    } else {
        let ch = any.parse_next(input)?;
        (ch.to_string(), ch.len_utf8())
    };

    let total_len = 1 + len;
    let raw = &start[..total_len];
    let token = Token::new(TokenKind::Skip, raw, TokenValue::Text(value));
    Ok((token, false))
}

fn exponent_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;

    let first = any.parse_next(input)?;
    if first != 'E' && first != 'e' {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    let sign = any.parse_next(input)?;
    if sign != '+' && sign != '-' {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    let raw = &start[..2];
    let token = Token::new(TokenKind::Exp, raw, TokenValue::Text(sign.to_string()));
    Ok((token, false))
}

fn fill_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let start = *input;
    '*'.parse_next(input)?;

    let (value, len) = if input.starts_with('\\') {
        let mut temp = *input;
        '\\'.parse_next(&mut temp)?;
        let next = any.parse_next(&mut temp)?;
        let l = 1 + next.len_utf8();
        *input = temp;
        (format!("\\{}", next), l)
    } else {
        let ch = any.parse_next(input)?;
        (ch.to_string(), ch.len_utf8())
    };

    let total_len = 1 + len;
    let raw = &start[..total_len];
    let token = Token::new(TokenKind::Fill, raw, TokenValue::Text(value));
    Ok((token, false))
}

fn paren_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let ch = any.verify(|&c| c == '(' || c == ')').parse_next(input)?;
    let raw = ch.to_string();
    let token = Token::new(TokenKind::Paren, raw.clone(), TokenValue::Text(raw));
    Ok((token, false))
}

fn error_char_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let ch = any
        .verify(|&c| INVALID_PATTERN_CHARS.contains(c))
        .parse_next(input)?;
    let raw = ch.to_string();
    let token = Token::new(TokenKind::Error, raw.clone(), TokenValue::Text(raw));
    Ok((token, false))
}

fn fallback_char_parser(input: &mut &str) -> PResult<(Token, bool)> {
    let ch = any.parse_next(input)?;
    let raw = ch.to_string();
    let token = Token::new(TokenKind::Char, raw.clone(), TokenValue::Text(raw));
    Ok((token, false))
}
