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
    // Constants in seconds
    const YEAR: f64 = 365.0 * 24.0 * 3600.0;
    const MONTH: f64 = 30.0 * 24.0 * 3600.0;
    const DAY: f64 = 24.0 * 3600.0;
    const HOUR: f64 = 3600.0;

    if visible_range_sec > YEAR * 2.0 {
        SmartDateFormat::Year
    } else if visible_range_sec > MONTH * 6.0 {
        SmartDateFormat::MonthYear
    } else if visible_range_sec > DAY * 2.0 {
        SmartDateFormat::DayMonth
    } else if visible_range_sec > HOUR {
        SmartDateFormat::HourMin
    } else {
        SmartDateFormat::HourMinSec
    }
}

/// Formats a timestamp according to the specified format.
/// Handles auto-detection of Milliseconds vs Seconds based on magnitude (> 1e11 is ms).
pub fn format_timestamp(value: f64, format: SmartDateFormat) -> String {
    // Heuristic: If value > 100 billion, it's likely milliseconds (valid for dates after 1973)
    let (seconds, _) = if value.abs() > 100_000_000_000.0 {
        ((value / 1000.0) as i64, 0)
    } else {
        (value as i64, 0)
    };

    // Use single to avoid panic on ambiguity, fallback to default if invalid
    let dt = match Utc.timestamp_opt(seconds, 0) {
        chrono::LocalResult::Single(d) => d,
        chrono::LocalResult::Ambiguous(d, _) => d,
        chrono::LocalResult::None => return format!("{:.2}", value), // Fallback for non-date values
    };

    match format {
        SmartDateFormat::Year => dt.format("%Y").to_string(),
        SmartDateFormat::MonthYear => dt.format("%b %Y").to_string(),
        SmartDateFormat::DayMonth => dt.format("%d %b").to_string(),
        SmartDateFormat::HourMin => dt.format("%H:%M").to_string(),
        SmartDateFormat::HourMinSec => dt.format("%H:%M:%S").to_string(),
    }
}
