use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct DateUnits: u16 {
        const YEAR = 1 << 1;
        const MONTH = 1 << 2;
        const DAY = 1 << 3;
        const HOUR = 1 << 4;
        const MINUTE = 1 << 5;
        const SECOND = 1 << 6;
        const DECISECOND = 1 << 7;
        const CENTISECOND = 1 << 8;
        const MILLISECOND = 1 << 9;
    }
}

/// Calendar epoch identifiers. Only a subset is currently supported.
pub const EPOCH_1904: i32 = -1;
pub const EPOCH_1900: i32 = 1;
pub const EPOCH_1317: i32 = 6;

/// Excel date boundaries.
pub const MIN_S_DATE: f64 = 0.0;
pub const MAX_S_DATE: f64 = 2_958_466.0;

/// Google date boundaries.
pub const MIN_L_DATE: f64 = -694_324.0;
pub const MAX_L_DATE: f64 = 35_830_291.0;

/// Known currency symbols supported by the JavaScript implementation.
pub const CURRENCY_SYMBOLS: &[&str] = &[
    "¤", "$", "£", "¥", "֏", "؋", "৳", "฿", "៛", "₡", "₦", "₩", "₪", "₫", "€", "₭", "₮", "₱", "₲",
    "₴", "₸", "₹", "₺", "₼", "₽", "₾", "₿",
];

pub const INDEX_COLORS: &[&str] = &[
    "#000", "#FFF", "#F00", "#0F0", "#00F", "#FF0", "#F0F", "#0FF", "#000", "#FFF", "#F00", "#0F0",
    "#00F", "#FF0", "#F0F", "#0FF", "#800", "#080", "#008", "#880", "#808", "#088", "#CCC", "#888",
    "#99F", "#936", "#FFC", "#CFF", "#606", "#F88", "#06C", "#CCF", "#008", "#F0F", "#FF0", "#0FF",
    "#808", "#800", "#088", "#00F", "#0CF", "#CFF", "#CFC", "#FF9", "#9CF", "#F9C", "#C9F", "#FC9",
    "#36F", "#3CC", "#9C0", "#FC0",
];

/// Characters that Excel considers invalid in format patterns.
pub const INVALID_PATTERN_CHARS: &str = "EÈÉÊËèéêëĒēĔĕĖėĘęĚěȄȅȆȇȨȩNnÑñŃńŅņŇňǸǹ\"*/\\_";
