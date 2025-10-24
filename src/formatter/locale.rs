use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Locale {
    pub group: String,
    pub decimal: String,
    pub positive: String,
    pub negative: String,
    pub percent: String,
    pub exponent: String,
    pub nan: String,
    pub infinity: String,
    pub ampm: Vec<String>,
    pub mmmm6: Vec<String>,
    pub mmm6: Vec<String>,
    pub mmmm: Vec<String>,
    pub mmm: Vec<String>,
    pub dddd: Vec<String>,
    pub ddd: Vec<String>,
    pub bool_values: Vec<String>,
    pub prefer_mdy: bool,
}

impl Locale {
    pub fn bool_true(&self) -> &str {
        self.bool_values
            .get(0)
            .map(|s| s.as_str())
            .unwrap_or("TRUE")
    }

    pub fn bool_false(&self) -> &str {
        self.bool_values
            .get(1)
            .map(|s| s.as_str())
            .unwrap_or("FALSE")
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LocaleFile {
    default: LocaleRaw,
    locales: HashMap<String, LocaleRaw>,
}

#[derive(Debug, Clone, Deserialize)]
struct LocaleRaw {
    #[serde(default)]
    group: String,
    #[serde(default)]
    decimal: String,
    #[serde(default)]
    positive: String,
    #[serde(default)]
    negative: String,
    #[serde(default)]
    percent: String,
    #[serde(default)]
    exponent: String,
    #[serde(default)]
    nan: String,
    #[serde(default)]
    infinity: String,
    #[serde(default)]
    ampm: Vec<String>,
    #[serde(default)]
    mmmm6: Vec<String>,
    #[serde(default)]
    mmm6: Vec<String>,
    #[serde(default)]
    mmmm: Vec<String>,
    #[serde(default)]
    mmm: Vec<String>,
    #[serde(default)]
    dddd: Vec<String>,
    #[serde(default)]
    ddd: Vec<String>,
    #[serde(default, rename = "bool")]
    bool_values: Vec<String>,
    #[serde(default, rename = "preferMDY")]
    prefer_mdy: bool,
}

#[derive(Debug, Clone)]
struct LocaleId {
    lang: String,
    language: String,
}

struct LocaleRegistry {
    default: Locale,
    locales: HashMap<String, Locale>,
}

static REGISTRY: OnceLock<LocaleRegistry> = OnceLock::new();
static CODE_MAP: OnceLock<HashMap<u32, String>> = OnceLock::new();

pub fn default_locale() -> &'static Locale {
    &REGISTRY.get_or_init(LocaleRegistry::load).default
}

pub fn get_locale(tag: Option<&str>) -> Option<&'static Locale> {
    tag.and_then(|t| lookup_locale(t))
}

pub fn get_locale_or_default(tag: Option<&str>) -> &'static Locale {
    get_locale(tag).unwrap_or_else(|| default_locale())
}

#[allow(dead_code)]
pub fn resolve_locale(tag: &str) -> Option<String> {
    resolve_code(tag).or_else(|| parse_locale_tag(tag).map(|id| id.lang))
}

fn lookup_locale(tag: &str) -> Option<&'static Locale> {
    let registry = REGISTRY.get_or_init(LocaleRegistry::load);
    if tag.trim().is_empty() {
        return None;
    }
    if let Some(code) = resolve_code(tag) {
        if let Some(loc) = registry.locales.get(&code) {
            return Some(loc);
        }
        if let Some(parsed) = parse_locale_tag(&code) {
            if let Some(loc) = registry.locales.get(&parsed.language) {
                return Some(loc);
            }
        }
    }
    if let Some(parsed) = parse_locale_tag(tag) {
        if let Some(loc) = registry.locales.get(&parsed.lang) {
            return Some(loc);
        }
        if let Some(loc) = registry.locales.get(&parsed.language) {
            return Some(loc);
        }
    }
    None
}

impl LocaleRegistry {
    fn load() -> Self {
        let raw: LocaleFile =
            serde_json::from_str(include_str!("./locales.json")).expect("invalid locale data");

        let default = Locale::from_raw(raw.default);
        let mut locales = HashMap::new();
        for (key, value) in raw.locales {
            let canonical = canonicalize_key(&key);
            locales.insert(canonical, Locale::from_raw(value));
        }
        Self { default, locales }
    }
}

impl Locale {
    fn from_raw(raw: LocaleRaw) -> Self {
        Self {
            group: if raw.group.is_empty() {
                "\u{00A0}".to_string()
            } else {
                raw.group
            },
            decimal: if raw.decimal.is_empty() {
                ".".to_string()
            } else {
                raw.decimal
            },
            positive: if raw.positive.is_empty() {
                "+".to_string()
            } else {
                raw.positive
            },
            negative: if raw.negative.is_empty() {
                "-".to_string()
            } else {
                raw.negative
            },
            percent: if raw.percent.is_empty() {
                "%".to_string()
            } else {
                raw.percent
            },
            exponent: if raw.exponent.is_empty() {
                "E".to_string()
            } else {
                raw.exponent
            },
            nan: if raw.nan.is_empty() {
                "NaN".to_string()
            } else {
                raw.nan
            },
            infinity: if raw.infinity.is_empty() {
                "∞".to_string()
            } else {
                raw.infinity
            },
            ampm: ensure_pair(raw.ampm, ["AM", "PM"]),
            mmmm6: ensure_list(raw.mmmm6, 12),
            mmm6: ensure_list(raw.mmm6, 12),
            mmmm: ensure_list(raw.mmmm, 12),
            mmm: ensure_list(raw.mmm, 12),
            dddd: ensure_list(raw.dddd, 7),
            ddd: ensure_list(raw.ddd, 7),
            bool_values: ensure_pair(raw.bool_values, ["TRUE", "FALSE"]),
            prefer_mdy: raw.prefer_mdy,
        }
    }
}

fn ensure_pair(values: Vec<String>, fallback: [&str; 2]) -> Vec<String> {
    match values.len() {
        0 => vec![fallback[0].to_string(), fallback[1].to_string()],
        1 => vec![values.into_iter().next().unwrap(), fallback[1].to_string()],
        _ => values,
    }
}

fn ensure_list(values: Vec<String>, min_len: usize) -> Vec<String> {
    if values.len() >= min_len {
        values
    } else {
        Vec::from(values)
    }
}

fn canonicalize_key(key: &str) -> String {
    parse_locale_tag(key)
        .map(|id| id.lang)
        .unwrap_or_else(|| key.to_ascii_lowercase())
}

fn parse_locale_tag(input: &str) -> Option<LocaleId> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let head = trimmed.split('@').next().unwrap_or(trimmed);
    let head = head.split('.').next().unwrap_or(head);
    let mut parts = head
        .split(|c| c == '-' || c == '_')
        .filter(|part| !part.is_empty());

    let language = parts.next()?.to_ascii_lowercase();
    if !language.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let territory = parts.next().map(|part| part.to_ascii_uppercase());
    if parts.next().is_some() {
        return None;
    }

    let lang = if let Some(region) = &territory {
        format!("{}_{}", language, region)
    } else {
        language.clone()
    };

    Some(LocaleId { lang, language })
}

fn resolve_code(tag: &str) -> Option<String> {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        return None;
    }
    // drop leading currency or dash markers as needed
    let cleaned = trimmed.trim_start_matches('$').trim_start_matches('-');
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return None;
    }
    if cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        if let Ok(value) = u32::from_str_radix(cleaned, 16) {
            let code = value & 0xffff;
            if let Some(locale) = code_map().get(&code) {
                return Some(locale.clone());
            }
        }
    }
    None
}

fn code_map() -> &'static HashMap<u32, String> {
    CODE_MAP.get_or_init(|| {
        let raw: HashMap<String, String> =
            serde_json::from_str(include_str!("./code_to_locale.json"))
                .expect("invalid codeToLocale data");
        raw.into_iter()
            .filter_map(|(key, value)| key.parse::<u32>().ok().map(|num| (num, value)))
            .collect()
    })
}
