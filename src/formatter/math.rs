pub fn round(number: f64, places: usize) -> f64 {
    if !number.is_finite() {
        return number;
    }
    if number < 0.0 {
        return -round(-number, places);
    }
    if places > 0 {
        let p = 10_f64.powi(places as i32);
        return round(number * p, 0) / p;
    }
    number.round()
}

pub fn clamp(number: f64) -> f64 {
    if number == 0.0 {
        return number;
    }
    let abs = number.abs();
    let d = abs.log10().ceil();
    let mag_exp = 16.0 - d.floor();
    let mag = if mag_exp.is_finite() {
        10_f64.powi(mag_exp as i32)
    } else {
        f64::INFINITY
    };
    if mag.is_finite() {
        (number * mag).round() / mag
    } else {
        0.0
    }
}

pub fn get_exponent(num: f64, int_max: usize) -> i32 {
    if num == 0.0 {
        return 0;
    }
    let exp = num.log10().floor() as i32;
    if int_max > 1 {
        let step = int_max as i32;
        let adjusted = (exp as f64 / step as f64).floor() as i32;
        adjusted * step
    } else {
        exp
    }
}

pub fn get_significand(n: f64, exp: i32) -> f64 {
    if exp < -300 {
        let repr = format!("{:e}", n);
        if let Some((mantissa, _)) = repr.split_once('e') {
            return mantissa.parse::<f64>().unwrap_or(n);
        }
    }
    n * 10_f64.powi(-exp)
}

const PRECISION: f64 = 1e-13;

pub fn dec2frac(
    number: f64,
    // infinity
    _numerator_max_digits: Option<usize>,
    denominator_max_digits: Option<usize>,
) -> (i64, i64) {
    if number.is_nan() || number.is_infinite() {
        return (0, 1);
    }

    let sign = if number < 0.0 { -1 } else { 1 };
    let number = number.abs();

    let maxdigits_d = 10_f64.powi(denominator_max_digits.unwrap_or(2) as i32);

    if number.fract() == 0.0 {
        return ((number * sign as f64) as i64, 1);
    } else if number < 1e-19 {
        return (sign, 1e19 as i64);
    } else if number > 1e19 {
        return ((1e19 * sign as f64) as i64, 1);
    }

    let mut z = number;
    let mut last_d = 0_f64;
    let mut last_n;
    let mut curr_n = 0_f64;
    let mut curr_d = 1_f64;

    loop {
        let floor_z = z.floor();
        z = 1.0 / (z - floor_z);

        let tmp_d = curr_d;
        curr_d = curr_d * z.floor() + last_d;
        last_d = tmp_d;

        last_n = curr_n;
        curr_n = (number * curr_d + 0.5).floor();

        if curr_d >= maxdigits_d {
            return ((sign as f64 * last_n).round() as i64, last_d.round() as i64);
        }

        if (number - curr_n / curr_d).abs() < PRECISION || z == z.floor() {
            break;
        }
    }

    ((sign as f64 * curr_n).round() as i64, curr_d.round() as i64)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NumDecInfo {
    pub total: usize,
    pub digits: usize,
    pub sign: usize,
    pub period: usize,
    pub int: usize,
    pub frac: usize,
}

pub fn numdec(value: f64, include_sign: bool) -> NumDecInfo {
    let v = value.abs();
    if v == 0.0 {
        return NumDecInfo {
            total: 1,
            digits: 0,
            sign: 0,
            period: 0,
            int: 1,
            frac: 0,
        };
    }

    let sign_size = if include_sign && value < 0.0 { 1 } else { 0 };
    let int_size = (v.log10() + 1.0).floor() as i32;
    let mut period_size = 0usize;
    let mut frac_size = 0isize;

    if v.fract() != 0.0 {
        period_size = 1;
        let scale = 10_f64.powi(-int_size);
        let scaled = round(v * scale, 15);
        let n = scaled.to_string();
        let chars: Vec<char> = n.chars().collect();
        let mut f = chars.len() as isize;
        let mut leading = true;
        for ch in chars.iter() {
            match ch {
                '.' => {
                    f -= 1;
                    break;
                }
                '0' if leading => {
                    f -= 1;
                }
                '-' => {}
                _ => {
                    leading = false;
                }
            }
        }
        frac_size = f - int_size as isize;
        if frac_size < 0 {
            frac_size = 0;
            period_size = 0;
        }
    }

    let int_digits = int_size.max(1) as usize;
    let frac_digits = frac_size.max(0) as usize;

    NumDecInfo {
        total: sign_size + int_digits + period_size + frac_digits,
        digits: int_digits + frac_digits,
        sign: sign_size,
        period: period_size,
        int: int_digits,
        frac: frac_digits,
    }
}
