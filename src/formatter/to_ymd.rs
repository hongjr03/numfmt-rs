use crate::constants::{EPOCH_1317, EPOCH_1904};

fn to_ymd_1900(ord: i32, leap1900: bool) -> [i32; 3] {
    if leap1900 && ord >= 0 {
        if ord == 0 {
            return [1900, 1, 0];
        }
        if ord == 60 {
            return [1900, 2, 29];
        }
        if ord < 60 {
            return [1900, if ord < 32 { 1 } else { 2 }, ((ord - 1) % 31) + 1];
        }
    }

    let mut l = ord as i64 + 68_569 + 2_415_019;
    let n = (4 * l) / 146_097;
    l = l - ((146_097 * n + 3) / 4);
    let i = (4_000 * (l + 1)) / 1_461_001;
    l = l - ((1_461 * i) / 4) + 31;
    let j = (80 * l) / 2_447;
    let n_day = l - ((2_447 * j) / 80);
    l = j / 11;
    let n_month = j + 2 - (12 * l);
    let n_year = 100 * (n - 49) + i + l;

    [n_year as i32, n_month as i32, n_day as i32]
}

fn to_ymd_1904(ord: i32) -> [i32; 3] {
    to_ymd_1900(ord + 1_462, false)
}

fn to_ymd_1317(ord: i32) -> [i32; 3] {
    if ord == 60 {
        panic!("#VALUE!");
    }
    if ord <= 1 {
        return [1317, 8, 29];
    }
    if ord < 60 {
        return [1317, if ord < 32 { 9 } else { 10 }, 1 + ((ord - 2) % 30)];
    }

    let y = 10_631_f64 / 30.0;
    let shift1 = 8.01 / 60.0;
    let mut z = ord as f64 + 466_935.0;
    let cyc = (z / 10_631.0).floor();
    z = z - 10_631.0 * cyc;
    let j = ((z - shift1) / y).floor();
    z = z - (j * y + shift1).floor();
    let m = ((z + 28.5001) / 29.5).floor();
    if (m as i32) == 13 {
        return [30 * cyc as i32 + j as i32, 12, 30];
    }
    [
        30 * cyc as i32 + j as i32,
        m as i32,
        (z - (29.5001 * m - 29.0).floor()).round() as i32,
    ]
}

pub fn to_ymd(ord: f64, system: i32, leap1900: bool) -> [i32; 3] {
    let int = ord.floor() as i32;
    if system == EPOCH_1317 {
        return to_ymd_1317(int);
    }
    if system == EPOCH_1904 {
        return to_ymd_1904(int);
    }
    to_ymd_1900(int, leap1900)
}
