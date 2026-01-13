use crate::data_types::TimeUnit;
use chrono::{TimeZone, Utc};

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

/// Formats a timestamp according to the specified format and unit.
pub fn format_timestamp(value: f64, format: SmartDateFormat, unit: TimeUnit) -> String {
    let seconds = match unit {
        TimeUnit::Seconds => value as i64,
        TimeUnit::Milliseconds => (value / 1000.0) as i64,
        TimeUnit::Microseconds => (value / 1_000_000.0) as i64,
        TimeUnit::Nanoseconds => (value / 1_000_000_000.0) as i64,
    };

    let dt = match Utc.timestamp_opt(seconds, 0) {
        chrono::LocalResult::Single(d) => d,
        chrono::LocalResult::Ambiguous(d, _) => d,
        chrono::LocalResult::None => return format!("{:.2}", value),
    };

    match format {
        SmartDateFormat::Year => dt.format("%Y").to_string(),
        SmartDateFormat::MonthYear => dt.format("%b %Y").to_string(),
        SmartDateFormat::DayMonth => dt.format("%d %b").to_string(),
        SmartDateFormat::HourMin => dt.format("%H:%M").to_string(),
        SmartDateFormat::HourMinSec => dt.format("%H:%M:%S").to_string(),
    }
}