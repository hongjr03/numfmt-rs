use super::{to_ymd::to_ymd, value::DateValue};

const DAYSIZE: f64 = 86_400.0;

pub fn date_to_serial(date: &DateValue, _ignore_timezone: bool) -> Option<f64> {
    let month = date.month.unwrap_or(1) as u32;
    let day = date.day.unwrap_or(1) as u32;
    let year = date.year;
    let hour = date.hour.unwrap_or(0) as i64;
    let minute = date.minute.unwrap_or(0) as i64;
    let second = date.second.unwrap_or(0) as i64;
    let millisecond = date.millisecond.unwrap_or(0) as i64;

    let days = days_from_civil(year, month, day);
    let seconds = hour * 3600 + minute * 60 + second;
    let fraction = (seconds as f64 + millisecond as f64 / 1000.0) / DAYSIZE;
    let d = days as f64 + fraction;
    let offset = if d <= -25_509.0 { -25_568.0 } else { -25_569.0 };
    Some(d - offset)
}

pub fn date_from_serial(serial: f64, system: i32, leap1900: bool) -> [i32; 6] {
    let floor = serial.floor();
    let t = DAYSIZE * (serial - floor);
    let mut time = t.floor();
    if (t - time) > 0.9999 {
        time += 1.0;
        if (time - DAYSIZE).abs() < f64::EPSILON {
            time = 0.0;
        }
    }

    let [y, m, d] = to_ymd(serial, system, leap1900);
    let x = if time < 0.0 { DAYSIZE + time } else { time };
    let total_seconds = x as i64;
    let hh = ((total_seconds / 60) / 60) % 60;
    let mm = (total_seconds / 60) % 60;
    let ss = total_seconds % 60;

    [y, m, d, hh as i32, mm as i32, ss as i32]
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year - (month <= 2) as i32;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let month = month as i32;
    let day = day as i32;
    let doy = ((153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5) + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146_097 + doe as i64 - 719_468
}
