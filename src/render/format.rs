//! Presentation helpers. Locale-independent on purpose — `format!` with a `.`
//! decimal separator is sufficient because Rust's float formatting is not
//! locale-aware. Kept in one module so widgets stay declarative.

use jiff::{Timestamp, tz::TimeZone};

/// `8` → `[..........]`, `21` → `[##........]`, `100` → `[##########]`.
/// `width` is the inner cell count; the wrapper `[]` is added by callers when
/// they want it (or use [`format_bar_wrapped`]).
pub fn format_bar(percent: f64, width: usize, filled: char, empty: char) -> String {
    let clamped = clamp_percent(percent);
    let fill = ((clamped / 10.0) as usize * width / 10).min(width);
    // The default spec is width=10 with `floor(pct/10)` cells. Express that
    // directly to avoid the rounding ambiguity of a per-width formula.
    let cells = if width == 10 {
        (clamped.trunc() as usize / 10).min(10)
    } else {
        fill
    };
    let mut s = String::with_capacity(width);
    for _ in 0..cells {
        s.push(filled);
    }
    for _ in cells..width {
        s.push(empty);
    }
    s
}

pub fn format_bar_wrapped(percent: f64, width: usize, filled: char, empty: char) -> String {
    let mut out = String::with_capacity(width + 2);
    out.push('[');
    out.push_str(&format_bar(percent, width, filled, empty));
    out.push(']');
    out
}

/// `(21%)`. Floor toward zero for non-negative input.
pub fn format_percent_paren(percent: f64) -> String {
    let p = clamp_percent(percent).trunc() as u64;
    format!("({p}%)")
}

/// `80000` → `80.0K`, `85_300` → `85.3K`, `1_000_000` → `1.0M`. Always one
/// decimal, even on round thousands.
pub fn abbreviate_tokens(value: u64) -> String {
    if value >= 1_000_000 {
        format_tenth(value as f64 / 1_000_000.0, 'M')
    } else if value >= 1_000 {
        format_tenth(value as f64 / 1_000.0, 'K')
    } else {
        format!("{value}")
    }
}

fn format_tenth(v: f64, suffix: char) -> String {
    let truncated = (v * 10.0).floor() / 10.0;
    format!("{truncated:.1}{suffix}")
}

/// `$2.55`. Two decimals, always.
pub fn format_cost_usd(value: f64) -> String {
    let v = if value.is_finite() { value } else { 0.0 };
    format!("${v:.2}")
}

/// `HH:mm` in 24-hour notation.
pub fn format_clock(ts: Timestamp, tz: &TimeZone) -> String {
    let z = ts.to_zoned(tz.clone());
    format!("{:02}:{:02}", z.hour(), z.minute())
}

/// `M/d HH:mm`, no zero-pad on month/day, no year.
pub fn format_weekly_reset(ts: Timestamp, tz: &TimeZone) -> String {
    let z = ts.to_zoned(tz.clone());
    format!(
        "{}/{} {:02}:{:02}",
        z.month(),
        z.day(),
        z.hour(),
        z.minute(),
    )
}

fn clamp_percent(v: f64) -> f64 {
    // NaN must collapse to 0; `f64::clamp` would propagate it.
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 100.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kst() -> TimeZone {
        TimeZone::get("Asia/Seoul").unwrap()
    }

    #[test]
    fn bar_zero_is_all_empty() {
        assert_eq!(format_bar(0.0, 10, '#', '.'), "..........");
    }

    #[test]
    fn bar_eight_percent_floors_to_zero() {
        assert_eq!(format_bar(8.0, 10, '#', '.'), "..........");
    }

    #[test]
    fn bar_twenty_one_percent_fills_two() {
        assert_eq!(format_bar(21.0, 10, '#', '.'), "##........");
    }

    #[test]
    fn bar_one_hundred_fills_all() {
        assert_eq!(format_bar(100.0, 10, '#', '.'), "##########");
    }

    #[test]
    fn bar_wrapped_includes_brackets() {
        assert_eq!(format_bar_wrapped(21.0, 10, '#', '.'), "[##........]");
    }

    #[test]
    fn percent_paren_truncates() {
        assert_eq!(format_percent_paren(8.9), "(8%)");
        assert_eq!(format_percent_paren(21.4), "(21%)");
    }

    #[test]
    fn percent_paren_clamps() {
        assert_eq!(format_percent_paren(120.0), "(100%)");
        assert_eq!(format_percent_paren(-5.0), "(0%)");
    }

    #[test]
    fn abbreviate_85_3k() {
        assert_eq!(abbreviate_tokens(85_300), "85.3K");
    }

    #[test]
    fn abbreviate_80_0k() {
        assert_eq!(abbreviate_tokens(80_000), "80.0K");
    }

    #[test]
    fn abbreviate_one_million_keeps_decimal() {
        assert_eq!(abbreviate_tokens(1_000_000), "1.0M");
    }

    #[test]
    fn abbreviate_below_1k() {
        assert_eq!(abbreviate_tokens(742), "742");
    }

    #[test]
    fn cost_two_decimals() {
        assert_eq!(format_cost_usd(2.55), "$2.55");
        assert_eq!(format_cost_usd(0.0), "$0.00");
    }

    #[test]
    fn clock_kst_renders_12_00() {
        // 2026-05-14 12:00 KST = 2026-05-14 03:00 UTC.
        let epoch = jiff::civil::date(2026, 5, 14)
            .at(12, 0, 0, 0)
            .to_zoned(kst())
            .unwrap()
            .timestamp();
        assert_eq!(format_clock(epoch, &kst()), "12:00");
    }

    #[test]
    fn weekly_reset_no_zero_padding_on_month_day() {
        // 2026-05-19 06:00 KST.
        let epoch = jiff::civil::date(2026, 5, 19)
            .at(6, 0, 0, 0)
            .to_zoned(kst())
            .unwrap()
            .timestamp();
        assert_eq!(format_weekly_reset(epoch, &kst()), "5/19 06:00");
    }
}
