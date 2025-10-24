use std::fmt;

use crate::constants::{DateUnits, EPOCH_1900};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum TokenKind {
    General,
    Hash,
    Zero,
    Qmark,
    Slash,
    Group,
    Scale,
    Comma,
    Break,
    Text,
    Plus,
    Minus,
    Point,
    Space,
    Percent,
    Digit,
    Calendar,
    Error,
    DateTime,
    Duration,
    Condition,
    DbNum,
    NatNum,
    Locale,
    Color,
    Modifier,
    Ampm,
    Escaped,
    String,
    Skip,
    Exp,
    Fill,
    Paren,
    Char,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum TokenValue {
    #[default]
    None,
    Text(String),
    Char(char),
    Condition(Condition),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub raw: String,
    pub value: TokenValue,
    pub volatile: bool,
    pub short: bool,
}

impl Token {
    pub fn new(kind: TokenKind, raw: impl Into<String>, value: TokenValue) -> Self {
        Self {
            kind,
            raw: raw.into(),
            value,
            volatile: false,
            short: false,
        }
    }

    pub fn minus(volatile: bool) -> Self {
        let mut token = Self::new(TokenKind::Minus, "-", TokenValue::Text("-".to_string()));
        token.volatile = volatile;
        token
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOperator {
    Equal,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
}

impl fmt::Display for ConditionOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ConditionOperator::Equal => "=",
            ConditionOperator::Greater => ">",
            ConditionOperator::GreaterEqual => ">=",
            ConditionOperator::Less => "<",
            ConditionOperator::LessEqual => "<=",
            ConditionOperator::NotEqual => "<>",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub operator: ConditionOperator,
    pub operand: f64,
    pub raw_operand: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Named(String),
    Index(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberPart {
    Integer,
    Fraction,
    Mantissa,
    Denominator,
    Numerator,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberToken {
    pub part: NumberPart,
    pub pattern: String,
}

impl NumberToken {
    pub fn new(part: NumberPart, pattern: impl Into<String>) -> Self {
        Self {
            part,
            pattern: pattern.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringRule {
    NumPlusInt,
    Num,
    Den,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringToken {
    pub value: String,
    pub rule: Option<StringRule>,
}

impl StringToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            rule: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTokenKind {
    Year,
    YearShort,
    BuddhistYear,
    BuddhistYearShort,
    Month,
    MonthName,
    MonthNameShort,
    MonthNameSingle,
    Weekday,
    WeekdayShort,
    Day,
    Hour,
    Minute,
    Second,
    HourElapsed,
    MinuteElapsed,
    SecondElapsed,
    Subsecond,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DateToken {
    pub kind: DateTokenKind,
    pub unit: DateUnits,
    pub zero_pad: bool,
    pub width: Option<usize>,
    pub decimals: u8,
}

impl DateToken {
    pub fn new(kind: DateTokenKind, unit: DateUnits) -> Self {
        Self {
            kind,
            unit,
            zero_pad: false,
            width: None,
            decimals: 0,
        }
    }

    pub fn subsecond(decimals: u8) -> Self {
        let mut token = Self::new(
            DateTokenKind::Subsecond,
            DateUnits::SECOND
                | DateUnits::DECISECOND
                | DateUnits::CENTISECOND
                | DateUnits::MILLISECOND,
        );
        token.decimals = decimals;
        token
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SectionToken {
    Token(Token),
    String(StringToken),
    Number(NumberToken),
    Date(DateToken),
    Div,
    Exp { plus: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub scale: f64,
    pub percent: bool,
    pub text: bool,
    pub date: DateUnits,
    pub date_eval: bool,
    pub date_system: i32,
    pub sec_decimals: u8,
    pub general: bool,
    pub clock: u8,
    pub int_pattern: Vec<String>,
    pub frac_pattern: Vec<String>,
    pub man_pattern: Vec<String>,
    pub den_pattern: Vec<String>,
    pub num_pattern: Vec<String>,
    pub tokens: Vec<SectionToken>,
    pub grouping: bool,
    pub fractions: bool,
    pub dec_fractions: bool,
    pub exponential: bool,
    pub exp_plus: bool,
    pub denominator: Option<u32>,
    pub integer: bool,
    pub int_min: usize,
    pub int_max: usize,
    pub frac_min: usize,
    pub frac_max: usize,
    pub man_min: usize,
    pub man_max: usize,
    pub num_min: usize,
    pub num_max: usize,
    pub den_min: usize,
    pub den_max: usize,
    pub int_p: String,
    pub man_p: String,
    pub num_p: String,
    pub den_p: String,
    pub condition: Option<Condition>,
    pub color: Option<Color>,
    pub locale: Option<String>,
    pub parens: bool,
    pub generated: bool,
    pub pattern: String,
    pub tokens_used: usize,
    pub error: Option<String>,
}

impl Section {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            percent: false,
            text: false,
            date: DateUnits::empty(),
            date_eval: false,
            date_system: EPOCH_1900,
            sec_decimals: 0,
            general: false,
            clock: 24,
            int_pattern: Vec::new(),
            frac_pattern: Vec::new(),
            man_pattern: Vec::new(),
            den_pattern: Vec::new(),
            num_pattern: Vec::new(),
            tokens: Vec::new(),
            grouping: false,
            fractions: false,
            dec_fractions: false,
            exponential: false,
            exp_plus: false,
            denominator: None,
            integer: false,
            int_min: 0,
            int_max: 0,
            frac_min: 0,
            frac_max: 0,
            man_min: 0,
            man_max: 0,
            num_min: 0,
            num_max: 0,
            den_min: 0,
            den_max: 0,
            int_p: String::new(),
            man_p: String::new(),
            num_p: String::new(),
            den_p: String::new(),
            condition: None,
            color: None,
            locale: None,
            parens: false,
            generated: false,
            pattern: String::new(),
            tokens_used: 0,
            error: None,
        }
    }
}

impl Default for Section {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub pattern: String,
    pub partitions: Vec<Section>,
    pub locale: Option<String>,
}
