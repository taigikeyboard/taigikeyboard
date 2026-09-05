//! The `yyyy-MM-dd HH:mm:ss` UTC text the custom dictionary stores its
//! timestamps as — the same string SQLite's `CURRENT_TIMESTAMP` writes and
//! the macOS formatter binds, so the stored text sorts the way the query
//! orders by it regardless of locale or time zone.

use std::time::{SystemTime, UNIX_EPOCH};

/// Now, as `yyyy-MM-dd HH:mm:ss` in UTC.
pub fn utc_timestamp_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    format_utc_timestamp(seconds)
}

/// `seconds` since the Unix epoch as `yyyy-MM-dd HH:mm:ss`. Proleptic
/// Gregorian, civil-from-days (Howard Hinnant's algorithm).
pub fn format_utc_timestamp(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let remainder = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        remainder / 3600,
        (remainder % 3600) / 60,
        remainder % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_instants() {
        assert_eq!(format_utc_timestamp(0), "1970-01-01 00:00:00");
        assert_eq!(
            format_utc_timestamp(951_782_400),
            "2000-02-29 00:00:00",
            "leap day"
        );
        assert_eq!(format_utc_timestamp(1_756_425_599), "2025-08-28 23:59:59");
        assert_eq!(utc_timestamp_now().len(), 19);
    }
}
