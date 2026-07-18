//! Parses the `"yy/MM/dd,hh:mm:ss±zz"` (or `"yyyy/MM/dd,hh:mm:ss±zz"`)
//! timestamp string used by `AT+QLTS` and `AT+QNTP` (see
//! `commands::responses::{NitzTimeResponse, NtpTimeResponse}`) into a Unix
//! timestamp.
//!
//! Both a 2-digit and a 4-digit year are accepted: `AT+QLTS` reports a
//! 2-digit year, but `AT+QNTP` was confirmed on real BG95-M3 hardware to
//! report a *4-digit* year for the same field (`"2026/07/18,16:32:18-20"`)
//! despite the AT command manual documenting the same 2-digit layout for
//! both — see quectel-bg9x-atat#5's follow-up. Which one `s` uses is
//! detected from where the first `/` lands.
//!
//! Deliberately hand-rolled instead of pulling in the `time` crate: this
//! crate is `no_std` and otherwise allocation-free (see the hex encoder in
//! `SslCipherSuiteEnum::to_bytes`), and a `format_description`/`parse`
//! pipeline is a lot of weight for one fixed-width field.
//!
//! The trailing `±zz` timezone field (quarter-hour offset from GMT) is
//! parsed for well-formedness but not applied: both source commands are used
//! in modes that already report GMT (`AT+QLTS` mode 1, and `AT+QNTP`'s
//! NTP-synced result), so the offset is informational rather than something
//! to add back in.

/// Length of the fixed-width `"MM/dd,hh:mm:ss±zz"` suffix that follows the
/// year (and its `/`), regardless of which year width was used.
const SUFFIX_LEN: usize = 17;

/// Parses the `"yy/MM/dd,hh:mm:ss±zz"` or `"yyyy/MM/dd,hh:mm:ss±zz"` prefix
/// of `s` into a Unix timestamp (seconds since 1970-01-01T00:00:00Z).
/// Two-digit years are read as 2000-2099. Returns `None` if `s` doesn't
/// match either expected layout.
pub(crate) fn parse_timestamp(s: &str) -> Option<i64> {
    let b = s.as_bytes();

    let (year, off) = if b.len() >= 5 && b[4] == b'/' {
        (four_digits(b, 0)? as i32, 5)
    } else if b.len() >= 3 && b[2] == b'/' {
        (2000 + two_digits(b, 0)? as i32, 3)
    } else {
        return None;
    };

    if b.len() < off + SUFFIX_LEN {
        return None;
    }

    let month = two_digits(b, off)?;
    if b[off + 2] != b'/' {
        return None;
    }
    let day = two_digits(b, off + 3)?;
    if b[off + 5] != b',' {
        return None;
    }
    let hour = two_digits(b, off + 6)?;
    if b[off + 8] != b':' {
        return None;
    }
    let minute = two_digits(b, off + 9)?;
    if b[off + 11] != b':' {
        return None;
    }
    let second = two_digits(b, off + 12)?;
    if b[off + 14] != b'+' && b[off + 14] != b'-' {
        return None;
    }
    let _tz_quarter_hours = two_digits(b, off + 15)?;

    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }

    if day < 1 || day > days_in_month(year, month) {
        return None;
    }

    let days = days_from_civil(year, month as u32, day as u32);
    Some(days * 86_400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64)
}

/// Number of days in `month` (1-12) of `year`, so [`parse_timestamp`] can
/// reject e.g. day 31 of a 30-day month or Feb 29 in a non-leap year instead
/// of letting [`days_from_civil`] silently roll it into the following month.
fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 31,
    }
}

/// Reads a two-ASCII-digit number at `b[at..at + 2]`.
fn two_digits(b: &[u8], at: usize) -> Option<u8> {
    let tens = b[at].checked_sub(b'0')?;
    let ones = b[at + 1].checked_sub(b'0')?;
    if tens > 9 || ones > 9 {
        return None;
    }
    Some(tens * 10 + ones)
}

/// Reads a four-ASCII-digit number at `b[at..at + 4]`.
fn four_digits(b: &[u8], at: usize) -> Option<u16> {
    let mut n: u16 = 0;
    for &digit in &b[at..at + 4] {
        let d = digit.checked_sub(b'0')?;
        if d > 9 {
            return None;
        }
        n = n * 10 + d as u16;
    }
    Some(n)
}

/// Days since 1970-01-01 for a proleptic-Gregorian civil date. Howard
/// Hinnant's `days_from_civil` algorithm — public domain,
/// <http://howardhinnant.github.io/date_algorithms.html>.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (month as i64 + 9) % 12; // [0, 11]
    let doy = (153 * mp + 2) / 5 + day as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_epoch_boundary() {
        assert_eq!(parse_timestamp("00/01/01,00:00:00+00"), Some(946_684_800));
    }

    #[test]
    fn parses_ordinary_date() {
        assert_eq!(parse_timestamp("25/11/11,19:39:05+00"), Some(1_762_889_945));
    }

    #[test]
    fn ignores_timezone_offset() {
        // AT+QLTS mode 1 / AT+QNTP already report GMT; the ±zz field is
        // informational and shouldn't shift the parsed timestamp.
        assert_eq!(parse_timestamp("25/11/11,19:39:05+04"), Some(1_762_889_945));
        assert_eq!(parse_timestamp("25/11/11,19:39:05-04"), Some(1_762_889_945));
    }

    #[test]
    fn ignores_trailing_dst_field() {
        // AT+QLTS response is "yy/MM/dd,hh:mm:ss±zz,dst".
        assert_eq!(
            parse_timestamp("25/11/11,19:39:05+00,1"),
            Some(1_762_889_945)
        );
    }

    #[test]
    fn handles_leap_day() {
        assert_eq!(parse_timestamp("24/02/29,00:00:00+00"), Some(1_709_164_800));
    }

    #[test]
    fn accepts_four_digit_year() {
        // AT+QNTP on real BG95-M3 hardware reports a 4-digit year for this
        // same field, despite the AT command manual documenting a 2-digit
        // year for both AT+QLTS and AT+QNTP — see quectel-bg9x-atat#5.
        assert_eq!(
            parse_timestamp("2025/11/11,19:39:05+00"),
            Some(1_762_889_945)
        );
        // Exact string captured from a live +QNTP URC.
        assert_eq!(
            parse_timestamp("2026/07/18,16:32:18-20"),
            Some(1_784_392_338)
        );
    }

    #[test]
    fn rejects_malformed_input() {
        assert_eq!(parse_timestamp(""), None);
        assert_eq!(parse_timestamp("25/11/11,19:39:05"), None); // missing tz
        assert_eq!(parse_timestamp("25-11-11,19:39:05+00"), None); // wrong separators
        assert_eq!(parse_timestamp("25/13/11,19:39:05+00"), None); // month out of range
        assert_eq!(parse_timestamp("25/11/32,19:39:05+00"), None); // day out of range
        assert_eq!(parse_timestamp("25/11/11,25:39:05+00"), None); // hour out of range
    }

    #[test]
    fn rejects_day_invalid_for_month() {
        assert_eq!(parse_timestamp("25/04/31,00:00:00+00"), None); // April has 30 days
        assert_eq!(parse_timestamp("25/02/29,00:00:00+00"), None); // 2025 isn't a leap year
        assert_eq!(parse_timestamp("25/02/30,00:00:00+00"), None);
        assert_eq!(parse_timestamp("00/02/29,00:00:00+00"), Some(951_782_400)); // 2000 is a leap year
    }
}
