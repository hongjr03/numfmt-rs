use std::borrow::Cow;

use num_bigint::BigInt;

#[derive(Debug, Clone, PartialEq)]
pub enum FormatValue<'a> {
    Number(f64),
    BigInt(BigInt),
    Text(Cow<'a, str>),
    Boolean(bool),
    Null,
    Date(DateValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateValue {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    pub millisecond: Option<u16>,
}

impl DateValue {
    pub fn new(year: i32) -> Self {
        Self {
            year,
            month: None,
            day: None,
            hour: None,
            minute: None,
            second: None,
            millisecond: None,
        }
    }

    pub fn with_month(mut self, month: u8) -> Self {
        self.month = Some(month);
        self
    }

    pub fn with_day(mut self, day: u8) -> Self {
        self.day = Some(day);
        self
    }

    pub fn with_time(mut self, hour: u8, minute: u8, second: u8) -> Self {
        self.hour = Some(hour);
        self.minute = Some(minute);
        self.second = Some(second);
        self
    }

    pub fn with_millisecond(mut self, ms: u16) -> Self {
        self.millisecond = Some(ms);
        self
    }
}

impl<'a> From<f64> for FormatValue<'a> {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl<'a> From<i64> for FormatValue<'a> {
    fn from(value: i64) -> Self {
        Self::Number(value as f64)
    }
}

impl<'a> From<&'a str> for FormatValue<'a> {
    fn from(value: &'a str) -> Self {
        Self::Text(Cow::Borrowed(value))
    }
}

impl<'a> From<String> for FormatValue<'a> {
    fn from(value: String) -> Self {
        Self::Text(Cow::Owned(value))
    }
}

impl<'a> From<bool> for FormatValue<'a> {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl<'a> From<BigInt> for FormatValue<'a> {
    fn from(value: BigInt) -> Self {
        Self::BigInt(value)
    }
}

impl<'a> From<DateValue> for FormatValue<'a> {
    fn from(value: DateValue) -> Self {
        Self::Date(value)
    }
}
