#[derive(Debug, Clone, PartialEq)]
pub struct FormatterOptions {
    pub overflow: String,
    pub date_error_throws: bool,
    pub date_error_number: bool,
    pub bigint_error_number: bool,
    pub date_span_large: bool,
    pub leap_1900: bool,
    pub nbsp: bool,
    pub throws: bool,
    pub invalid: String,
    pub locale: String,
    pub ignore_timezone: bool,
    pub grouping: Vec<u8>,
    pub index_colors: bool,
    pub skip_char: Option<String>,
    pub fill_char: Option<String>,
}

impl Default for FormatterOptions {
    fn default() -> Self {
        Self {
            overflow: "######".to_string(),
            date_error_throws: false,
            date_error_number: true,
            bigint_error_number: false,
            date_span_large: true,
            leap_1900: true,
            nbsp: false,
            throws: true,
            invalid: "######".to_string(),
            locale: String::new(),
            ignore_timezone: false,
            grouping: vec![3, 3],
            index_colors: true,
            skip_char: None,
            fill_char: None,
        }
    }
}

impl FormatterOptions {
    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = locale.into();
        self
    }

    pub fn with_nbsp(mut self, nbsp: bool) -> Self {
        self.nbsp = nbsp;
        self
    }

    pub fn with_grouping<I>(mut self, grouping: I) -> Self
    where
        I: IntoIterator<Item = u8>,
    {
        self.grouping = grouping.into_iter().collect();
        self
    }

    pub fn with_skip_char(mut self, ch: Option<String>) -> Self {
        self.skip_char = ch;
        self
    }

    pub fn with_fill_char(mut self, ch: Option<String>) -> Self {
        self.fill_char = ch;
        self
    }
}
