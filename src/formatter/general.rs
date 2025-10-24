use crate::parser::model::Section;

use super::{
    locale::Locale,
    math::{get_exponent, get_significand, numdec, round},
};

fn fix_locale(input: &str, locale: &Locale) -> String {
    if locale.decimal == "." {
        input.to_string()
    } else {
        input.replace('.', &locale.decimal)
    }
}

fn exponent_string(n: f64, exp: i32, locale: &Locale) -> String {
    let abs_exp = exp.abs();
    let mut out = String::new();
    let mut mantissa = round(n, 5);
    if mantissa == 1.0 && n != 1.0 {
        mantissa = n;
    }
    out.push_str(&fix_locale(&mantissa.to_string(), locale));
    out.push_str(&locale.exponent);
    out.push_str(if exp < 0 {
        &locale.negative
    } else {
        &locale.positive
    });
    if abs_exp < 10 {
        out.push('0');
    }
    out.push_str(&abs_exp.to_string());
    out
}

pub fn format_general(buffer: &mut String, value: f64, _part: &Section, locale: &Locale) {
    let int = value.trunc() as i64;

    if (value - int as f64).abs() < f64::EPSILON {
        buffer.push_str(&int.abs().to_string());
        return;
    }

    let v = value.abs();
    let mut exp = get_exponent(v, 0);
    let mut n = get_significand(v, exp);
    if (n - 10.0).abs() < f64::EPSILON {
        n = 1.0;
        exp += 1;
    }

    let num_dig = numdec(v, true);

    if (-4..=-1).contains(&exp) {
        let o = format!("{:.width$}", v, width = (10 + exp) as usize);
        let trimmed = o.trim_end_matches('0').trim_end_matches('.');
        buffer.push_str(&fix_locale(trimmed, locale));
    } else if exp == 10 {
        let mut o = format!("{:.10}", v);
        if o.len() > 12 {
            o.truncate(12);
        }
        if o.ends_with('.') {
            o.pop();
        }
        buffer.push_str(&fix_locale(&o, locale));
    } else if exp.abs() <= 9 {
        if num_dig.total <= 11 {
            let o = round(v, 9);
            let formatted = format!("{o:.prec$}", prec = num_dig.frac);
            buffer.push_str(&fix_locale(&formatted, locale));
        } else if exp == 9 {
            buffer.push_str(&v.floor().to_string());
        } else if (0..9).contains(&exp) {
            let o = round(v, (9 - exp) as usize);
            buffer.push_str(&fix_locale(&o.to_string(), locale));
        } else {
            buffer.push_str(&exponent_string(n, exp, locale));
        }
    } else if num_dig.total >= 12 {
        buffer.push_str(&exponent_string(n, exp, locale));
    } else {
        let o = round(v, 9);
        let formatted = format!("{o:.prec$}", prec = num_dig.frac);
        buffer.push_str(&fix_locale(&formatted, locale));
    }
}
