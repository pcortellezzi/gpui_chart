use crate::data_types::TimeUnit;
use chrono::{TimeZone, Utc};
use chrono_tz::Tz;

#[derive(Debug, Clone, Copy)]
pub enum SmartDateFormat {
    Year,       // 2024
    MonthYear,  // Jan 2024
    DayMonth,   // 12 Jan
    HourMin,    // 10:30
    HourMinSec, // 10:30:15
}

/// Determines the best date format based on the visible time range (in seconds).
pub fn determine_date_format(visible_range_sec: f64) -> SmartDateFormat {
    const MINUTE: f64 = 60.0;
    const HOUR: f64 = 3600.0;
    const DAY: f64 = 24.0 * HOUR;
    const MONTH: f64 = 30.0 * DAY;
    const YEAR: f64 = 365.0 * DAY;

    if visible_range_sec > YEAR * 2.0 {
        SmartDateFormat::Year
    } else if visible_range_sec > MONTH * 2.0 {
        SmartDateFormat::MonthYear
    } else if visible_range_sec > DAY * 1.5 {
        SmartDateFormat::DayMonth
    } else if visible_range_sec > MINUTE * 5.0 {
        SmartDateFormat::HourMin
    } else {
        SmartDateFormat::HourMinSec
    }
}

/// Formats a timestamp according to the specified format, unit and optional timezone.
pub fn format_timestamp(value: f64, format: SmartDateFormat, unit: TimeUnit, tz: Option<Tz>) -> String {
    let raw_ms = match unit {
        TimeUnit::Seconds => value * 1000.0,
        TimeUnit::Milliseconds => value,
        TimeUnit::Microseconds => value / 1000.0,
        TimeUnit::Nanoseconds => value / 1_000_000.0,
    };

    let seconds = (raw_ms / 1000.0) as i64;
    let ms_remainder = (raw_ms as i64 % 1000).abs() as u32;

    let dt_utc = match Utc.timestamp_opt(seconds, ms_remainder * 1_000_000) {
        chrono::LocalResult::Single(d) => d,
        chrono::LocalResult::Ambiguous(d, _) => d,
        chrono::LocalResult::None => return format!("{:.2}", value),
    };

    if let Some(tz) = tz {
        let dt_local = dt_utc.with_timezone(&tz);
        match format {
            SmartDateFormat::Year => dt_local.format("%Y").to_string(),
            SmartDateFormat::MonthYear => dt_local.format("%b %Y").to_string(),
            SmartDateFormat::DayMonth => dt_local.format("%d %b").to_string(),
            SmartDateFormat::HourMin => dt_local.format("%H:%M").to_string(),
            SmartDateFormat::HourMinSec => dt_local.format("%H:%M:%S").to_string(),
        }
    } else {
        match format {
            SmartDateFormat::Year => dt_utc.format("%Y").to_string(),
            SmartDateFormat::MonthYear => dt_utc.format("%b %Y").to_string(),
            SmartDateFormat::DayMonth => dt_utc.format("%d %b").to_string(),
            SmartDateFormat::HourMin => dt_utc.format("%H:%M").to_string(),
            SmartDateFormat::HourMinSec => dt_utc.format("%H:%M:%S").to_string(),
        }
    }
}
