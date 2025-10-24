use std::fmt;

use crate::parser::error::ParseError;

#[derive(Debug)]
pub enum FormatterError {
    Parse(ParseError),
    DateOutOfBounds,
    InvalidPattern(String),
    InvalidLocale(String),
    BigIntOverflow,
    Other(String),
}

impl fmt::Display for FormatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatterError::Parse(err) => write!(f, "{}", err),
            FormatterError::DateOutOfBounds => write!(f, "Date out of bounds"),
            FormatterError::InvalidPattern(pat) => write!(f, "Invalid pattern: {pat}"),
            FormatterError::InvalidLocale(tag) => write!(f, "Invalid locale: {tag}"),
            FormatterError::BigIntOverflow => write!(f, "BigInt value out of range"),
            FormatterError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for FormatterError {}

impl From<ParseError> for FormatterError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}
