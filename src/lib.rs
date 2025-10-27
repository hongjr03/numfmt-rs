pub mod constants;
pub mod formatter;
pub mod parser;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "typst-plugin")]
pub mod typst_plugin;

pub use formatter::{
    ColorValue, DateValue, FormatValue, FormatterError, FormatterOptions, LocaleSettings,
    add_locale, format, format_color, format_with_options,
};
pub use parser::{parse_format_section, parse_pattern, tokenize};
