use super::error::ParseError;
use super::model::{
    Condition, ConditionOperator, Pattern, Section, SectionToken, Token, TokenKind,
};
use super::section::{SectionParseResult, parse_format_section};
use super::tokenizer::tokenize;

fn parse_section_from_str(pattern: &str) -> Result<Section, ParseError> {
    let tokens = tokenize(pattern)?;
    let SectionParseResult { mut section } = parse_format_section(&tokens)?;
    section.generated = true;
    Ok(section)
}

fn clone_part(section: &Section, prefix: Option<SectionToken>) -> Section {
    let mut clone = section.clone();
    if let Some(token) = prefix {
        clone.tokens.insert(0, token);
    }
    clone.generated = true;
    clone
}

fn maybe_add_minus(section: &mut Section) {
    if let Some(cond) = &section.condition
        && cond.operand < 0.0
        && matches!(
            cond.operator,
            ConditionOperator::Less | ConditionOperator::LessEqual | ConditionOperator::Equal
        )
    {
        return;
    }
    section
        .tokens
        .insert(0, SectionToken::Token(Token::minus(true)));
}

fn make_condition(operator: ConditionOperator, operand: f64) -> Condition {
    Condition {
        operator,
        operand,
        raw_operand: operand.to_string(),
    }
}

pub fn parse_pattern(pattern: &str) -> Result<Pattern, ParseError> {
    let tokens = tokenize(pattern)?;
    let total_tokens = tokens.len();
    let mut partitions: Vec<Section> = Vec::new();
    let mut offset = 0usize;
    let mut part_index = 0usize;
    let mut conditions = 0usize;
    let mut conditional = false;
    let mut text_index: Option<usize> = None;
    let mut locale_override: Option<String> = None;
    let mut last_had_break = false;

    while part_index < 4 && conditions < 3 {
        let slice = if offset < total_tokens {
            &tokens[offset..]
        } else {
            &[]
        };
        let SectionParseResult { section } = parse_format_section(slice)?;

        if (!section.date.is_empty() || section.general)
            && (!section.int_pattern.is_empty()
                || !section.frac_pattern.is_empty()
                || (section.scale - 1.0).abs() > f64::EPSILON
                || section.text)
        {
            return Err(ParseError::new("Illegal format"));
        }

        if section.condition.is_some() {
            conditions += 1;
            conditional = true;
        }
        if section.text {
            if text_index.is_some() {
                return Err(ParseError::new("Unexpected partition"));
            }
            text_index = Some(partitions.len());
        }
        if section.locale.is_some() {
            locale_override = section.locale.clone();
        }

        last_had_break = slice
            .get(section.tokens_used)
            .map(|tok| tok.kind == TokenKind::Break)
            .unwrap_or(false);

        partitions.push(section);
        part_index += 1;

        let consumed = if slice.is_empty() {
            0
        } else {
            partitions.last().unwrap().tokens_used + 1
        };
        offset += consumed;

        if !last_had_break {
            break;
        }
    }

    if last_had_break {
        return Err(ParseError::new("Unexpected partition"));
    }

    if conditions > 2 {
        return Err(ParseError::new("Unexpected condition"));
    }

    if partitions.len() > 3 {
        let part3 = &partitions[3];
        if !part3.int_pattern.is_empty() || !part3.frac_pattern.is_empty() || !part3.date.is_empty()
        {
            return Err(ParseError::new("Unexpected partition"));
        }
    }

    if conditional {
        if partitions.len() == 1 {
            partitions.push(parse_section_from_str("General")?);
        }

        if partitions.len() < 3 {
            let cond_first = partitions[0].condition.clone();
            maybe_add_minus(&mut partitions[0]);
            if let Some(second) = partitions.get_mut(1) {
                if second.condition.is_some() {
                    maybe_add_minus(second);
                } else if let Some(cond) = cond_first
                    && (cond.operator == ConditionOperator::Equal
                        || (cond.operand >= 0.0
                            && matches!(
                                cond.operator,
                                ConditionOperator::Greater | ConditionOperator::GreaterEqual
                            )))
                {
                    second
                        .tokens
                        .insert(0, SectionToken::Token(Token::minus(true)));
                }
            }
        } else {
            for part in &mut partitions {
                maybe_add_minus(part);
            }
        }
    } else {
        let mut text_part = text_index.map(|idx| partitions.remove(idx));

        if partitions.is_empty() {
            partitions.push(parse_section_from_str("General")?);
        }

        if partitions.len() < 2 {
            let minus_token = SectionToken::Token(Token::minus(true));
            let clone = clone_part(&partitions[0], Some(minus_token));
            partitions.push(clone);
        }

        if partitions.len() < 3 {
            let clone = clone_part(&partitions[0], None);
            partitions.push(clone);
        }

        if partitions.len() < 4 {
            if let Some(text) = text_part.take() {
                partitions.push(text);
            } else {
                partitions.push(parse_section_from_str("@")?);
            }
        }

        if let Some(part) = partitions.get_mut(0) {
            part.condition = Some(make_condition(ConditionOperator::Greater, 0.0));
        }
        if let Some(part) = partitions.get_mut(1) {
            part.condition = Some(make_condition(ConditionOperator::Less, 0.0));
        }
        if let Some(part) = partitions.get_mut(2) {
            part.condition = None;
        }
    }

    Ok(Pattern {
        pattern: pattern.to_string(),
        partitions,
        locale: locale_override,
    })
}
