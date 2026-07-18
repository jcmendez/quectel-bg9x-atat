//! Parses the `"yy/MM/dd,hh:mm:ss±zz"` timestamp string shared by `AT+QLTS`
//! and `AT+QNTP` (see `commands::responses::{NitzTimeResponse,
//! NtpTimeResponse}`) into a Unix timestamp.
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

/// Minimum length of a well-formed `"yy/MM/dd,hh:mm:ss±zz"` prefix. Trailing
/// bytes (e.g. NITZ's `,dst` suffix) are ignored.
const PREFIX_LEN: usize = 20;

/// Parses the `"yy/MM/dd,hh:mm:ss±zz"` prefix of `s` into a Unix timestamp
/// (seconds since 1970-01-01T00:00:00Z). Two-digit years are read as
/// 2000-2099, matching the module's actual output (despite the AT command
/// manual's example implying a wider range). Returns `None` if `s` doesn't
/// match the expected layout.
pub(crate) fn parse_timestamp(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < PREFIX_LEN {
        return None;
    }

    let yy = two_digits(b, 0)?;
    if b[2] != b'/' {
        return None;
    }
    let month = two_digits(b, 3)?;
    if b[5] != b'/' {
        return None;
    }
    let day = two_digits(b, 6)?;
    if b[8] != b',' {
        return None;
    }
    let hour = two_digits(b, 9)?;
    if b[11] != b':' {
        return None;
    }
    let minute = two_digits(b, 12)?;
    if b[14] != b':' {
        return None;
    }
    let second = two_digits(b, 15)?;
    if b[17] != b'+' && b[17] != b'-' {
        return None;
    }
    let _tz_quarter_hours = two_digits(b, 18)?;

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }

    let days = days_from_civil(2000 + yy as i32, month as u32, day as u32);
    Some(days * 86_400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64)
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
    fn rejects_malformed_input() {
        assert_eq!(parse_timestamp(""), None);
        assert_eq!(parse_timestamp("25/11/11,19:39:05"), None); // missing tz
        assert_eq!(parse_timestamp("2025/11/11,19:39:05+00"), None); // 4-digit year
        assert_eq!(parse_timestamp("25-11-11,19:39:05+00"), None); // wrong separators
        assert_eq!(parse_timestamp("25/13/11,19:39:05+00"), None); // month out of range
        assert_eq!(parse_timestamp("25/11/32,19:39:05+00"), None); // day out of range
        assert_eq!(parse_timestamp("25/11/11,25:39:05+00"), None); // hour out of range
    }
}
