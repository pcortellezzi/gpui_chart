use chrono::{Datelike, NaiveTime, TimeZone};
use chrono_tz::Tz;

/// Represents a raw exclusion rule.
#[derive(Debug, Clone, PartialEq)]
pub enum ExclusionRule {
    /// A fixed range defined by [start, end[ in UTC milliseconds.
    Fixed { start: i64, end: i64 },
    /// A weekly recurrence.
    Recurring {
        days: Vec<chrono::Weekday>,
        start_time: NaiveTime,
        end_time: NaiveTime,
        timezone: String,
    },
    /// A numeric recurrence (modulo).
    Numeric {
        modulo: f64,
        offset: f64,
        width: f64,
    },
}

/// A concrete, normalized exclusion segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GapSegment {
    /// Gap start in real time (ms UTC).
    pub start_real: i64,
    /// Gap end in real time (ms UTC).
    pub end_real: i64,
    /// Total duration of ALL gaps preceding this one (excluding this one).
    pub cumulative_before: i64,
}

impl GapSegment {
    pub fn duration(&self) -> i64 {
        self.end_real - self.start_real
    }
}

/// Engine index for real time <-> logical time transformation.
#[derive(Debug, Default, Clone)]
pub struct GapIndex {
    segments: Vec<GapSegment>,
}

impl GapIndex {
    /// Creates a new index from a list of normalized segments.
    pub fn new(mut segments: Vec<GapSegment>) -> Self {
        segments.sort_by_key(|s| s.start_real);

        let mut cumulative = 0;
        for segment in segments.iter_mut() {
            segment.cumulative_before = cumulative;
            cumulative += segment.duration();
        }

        Self { segments }
    }

    /// Converts real time to logical (compressed) time.
    pub fn to_logical(&self, real_ms: i64) -> i64 {
        if self.segments.is_empty() {
            return real_ms;
        }

        match self
            .segments
            .binary_search_by_key(&real_ms, |s| s.start_real)
        {
            Ok(idx) => {
                // Exactly at the start of a gap -> return logical time at gap boundary
                real_ms - self.segments[idx].cumulative_before
            }
            Err(idx) => {
                if idx == 0 {
                    // No gaps before
                    real_ms
                } else {
                    let prev = &self.segments[idx - 1];
                    if real_ms < prev.end_real {
                        // Inside a gap -> compress to gap start
                        prev.start_real - prev.cumulative_before
                    } else {
                        // After the previous gap
                        real_ms - (prev.cumulative_before + prev.duration())
                    }
                }
            }
        }
    }

    /// Converts logical time back to real time.
    pub fn to_real(&self, logical_ms: i64) -> i64 {
        if self.segments.is_empty() {
            return logical_ms;
        }

        // Search by logical time
        // logical_at_start = start_real - cumulative_before
        let mut low = 0;
        let mut high = self.segments.len();

        while low < high {
            let mid = low + (high - low) / 2;
            let mid_segment = &self.segments[mid];
            let logical_start = mid_segment.start_real - mid_segment.cumulative_before;

            if logical_start <= logical_ms {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        // 'low' is the index of the first segment starting AFTER our logical_ms.
        // So the segment impacting us is low - 1.
        if low == 0 {
            logical_ms
        } else {
            let segment = &self.segments[low - 1];
            logical_ms + (segment.cumulative_before + segment.duration())
        }
    }

    /// Checks if a timestamp is inside an exclusion gap.
    pub fn is_inside(&self, real_ms: i64) -> bool {
        match self
            .segments
            .binary_search_by_key(&real_ms, |s| s.start_real)
        {
            Ok(_) => true,
            Err(idx) => {
                if idx == 0 {
                    false
                } else {
                    real_ms < self.segments[idx - 1].end_real
                }
            }
        }
    }

    pub fn segments(&self) -> &[GapSegment] {
        &self.segments
    }

    /// Splits a real-time range [start, end] into sub-ranges that do not overlap any gaps.
    pub fn split_range(&self, start: i64, end: i64) -> Vec<(i64, i64)> {
        if self.segments.is_empty() {
            return vec![(start, end)];
        }

        let mut result = Vec::new();
        let mut current_start = start;

        for seg in &self.segments {
            if seg.end_real <= current_start {
                continue;
            }
            if seg.start_real >= end {
                break;
            }

            if seg.start_real > current_start {
                result.push((current_start, seg.start_real));
            }
            current_start = seg.end_real;
        }

        if current_start < end {
            result.push((current_start, end));
        }

        result
    }

    /// Returns a stateful cursor for optimized sequential access.
    pub fn cursor(&self) -> MappingCursor<'_> {
        MappingCursor::new(self)
    }
}

/// A stateful cursor to optimize sequential transformations (O(1) amortized).
pub struct MappingCursor<'a> {
    index: &'a GapIndex,
    last_seg_idx: usize,
}

impl<'a> MappingCursor<'a> {
    pub fn new(index: &'a GapIndex) -> Self {
        Self {
            index,
            last_seg_idx: 0,
        }
    }

    /// Converts real time to logical time, optimized for increasing `real_ms`.
    pub fn to_logical(&mut self, real_ms: i64) -> i64 {
        let segments = &self.index.segments;
        if segments.is_empty() {
            return real_ms;
        }

        // Advance cursor if needed
        while self.last_seg_idx < segments.len()
            && segments[self.last_seg_idx].end_real <= real_ms
        {
            self.last_seg_idx += 1;
        }

        if self.last_seg_idx >= segments.len() {
            // After all gaps
            if let Some(last) = segments.last() {
                return real_ms - (last.cumulative_before + last.duration());
            } else {
                return real_ms;
            }
        }

        let current = &segments[self.last_seg_idx];
        if real_ms < current.start_real {
            // Between previous and current gap
            real_ms - current.cumulative_before
        } else {
            // Inside current gap -> clamp to start
            current.start_real - current.cumulative_before
        }
    }

    /// Converts logical time back to real time, optimized for increasing `logical_ms`.
    pub fn to_real(&mut self, logical_ms: i64) -> i64 {
        let segments = &self.index.segments;
        if segments.is_empty() {
            return logical_ms;
        }

        // Advance cursor if needed
        // logical_at_start = start_real - cumulative_before
        while self.last_seg_idx < segments.len() {
            let seg = &segments[self.last_seg_idx];
            let logical_start = seg.start_real - seg.cumulative_before;
            if logical_start > logical_ms {
                break;
            }
            self.last_seg_idx += 1;
        }

        // self.last_seg_idx is now the first segment STARTS after logical_ms
        // So the segment impacting us is last_seg_idx - 1 (if > 0)
        if self.last_seg_idx == 0 {
            logical_ms
        } else {
            let prev = &segments[self.last_seg_idx - 1];
            logical_ms + (prev.cumulative_before + prev.duration())
        }
    }

    /// Reset the cursor state (e.g. for a new pass)
    pub fn reset(&mut self) {
        self.last_seg_idx = 0;
    }
}

/// Builder to generate a GapIndex from rules over a time window.
pub struct GapIndexBuilder {
    rules: Vec<ExclusionRule>,
}

impl GapIndexBuilder {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: ExclusionRule) {
        self.rules.push(rule);
    }

    /// Generates the index for a given time range.
    pub fn build(&self, range_start: i64, range_end: i64) -> GapIndex {
        let mut raw_segments = Vec::new();

        for rule in &self.rules {
            match rule {
                ExclusionRule::Fixed { start, end } => {
                    if *start < range_end && *end > range_start {
                        raw_segments.push((*start, *end));
                    }
                }
                ExclusionRule::Recurring {
                    days,
                    start_time,
                    end_time,
                    timezone,
                } => match timezone.parse::<Tz>() {
                    Ok(tz) => {
                        self.generate_recurring_segments(
                            &mut raw_segments,
                            range_start,
                            range_end,
                            days,
                            *start_time,
                            *end_time,
                            tz,
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to parse timezone {}: {}", timezone, e);
                    }
                },
                ExclusionRule::Numeric {
                    modulo,
                    offset,
                    width,
                } => {
                    self.generate_numeric_segments(
                        &mut raw_segments,
                        range_start,
                        range_end,
                        *modulo,
                        *offset,
                        *width,
                    );
                }
            }
        }

        if raw_segments.is_empty() {
            return GapIndex::default();
        }

        raw_segments.sort_by_key(|s| s.0);

        let mut merged = Vec::new();
        let (mut current_start, mut current_end) = raw_segments[0];

        for i in 1..raw_segments.len() {
            let (next_start, next_end) = raw_segments[i];
            if next_start <= current_end {
                // Overlapping or contiguous -> merge
                current_end = current_end.max(next_end);
            } else {
                merged.push(GapSegment {
                    start_real: current_start,
                    end_real: current_end,
                    cumulative_before: 0,
                });
                current_start = next_start;
                current_end = next_end;
            }
        }

        merged.push(GapSegment {
            start_real: current_start,
            end_real: current_end,
            cumulative_before: 0,
        });

        GapIndex::new(merged)
    }

    fn generate_recurring_segments(
        &self,
        segments: &mut Vec<(i64, i64)>,
        range_start: i64,
        range_end: i64,
        days: &[chrono::Weekday],
        start_time: NaiveTime,
        end_time: NaiveTime,
        tz: Tz,
    ) {
        let start_dt = tz.timestamp_millis_opt(range_start).earliest();
        let end_dt = tz.timestamp_millis_opt(range_end).earliest();

        if start_dt.is_none() || end_dt.is_none() {
            return;
        }

        let mut current_dt = start_dt.unwrap().date_naive();
        let end_date = end_dt.unwrap().date_naive();

        // Go back one day to handle gaps spanning midnight
        current_dt = current_dt.pred_opt().unwrap_or(current_dt);

        while current_dt <= end_date {
            if days.contains(&current_dt.weekday()) {
                let s_dt_res = tz
                    .from_local_datetime(&current_dt.and_time(start_time))
                    .earliest();

                if let Some(s_dt) = s_dt_res {
                    let mut e_dt_res = tz
                        .from_local_datetime(&current_dt.and_time(end_time))
                        .earliest();

                    if end_time <= start_time {
                        // Spans to next day
                        if let Some(next_day) = current_dt.succ_opt() {
                            e_dt_res = tz
                                .from_local_datetime(&next_day.and_time(end_time))
                                .earliest();
                        }
                    }

                    if let Some(e_dt) = e_dt_res {
                        let s_ms = s_dt.timestamp_millis();
                        let e_ms = e_dt.timestamp_millis();

                        if s_ms < range_end && e_ms > range_start {
                            segments.push((s_ms, e_ms));
                        }
                    }
                }
            }
            if let Some(next) = current_dt.succ_opt() {
                current_dt = next;
            } else {
                break;
            }
        }
    }

    fn generate_numeric_segments(
        &self,
        segments: &mut Vec<(i64, i64)>,
        range_start: i64,
        range_end: i64,
        modulo: f64,
        offset: f64,
        width: f64,
    ) {
        if modulo <= 0.0 || width <= 0.0 {
            return;
        }

        let first_k = ((range_start as f64 - offset) / modulo).floor() as i64;
        let last_k = ((range_end as f64 - offset) / modulo).ceil() as i64;

        for k in first_k..=last_k {
            let s = (k as f64 * modulo + offset) as i64;
            let e = s + width as i64;

            if s < range_end && e > range_start {
                segments.push((s, e));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    #[test]
    fn test_merge_segments() {
        let mut builder = GapIndexBuilder::new();
        builder.add_rule(ExclusionRule::Fixed {
            start: 100,
            end: 200,
        });
        builder.add_rule(ExclusionRule::Fixed {
            start: 150,
            end: 250,
        });
        builder.add_rule(ExclusionRule::Fixed {
            start: 300,
            end: 400,
        });

        let index = builder.build(0, 1000);
        assert_eq!(index.segments.len(), 2);
        assert_eq!(index.segments[0].start_real, 100);
        assert_eq!(index.segments[0].end_real, 250);
        assert_eq!(index.segments[1].start_real, 300);
        assert_eq!(index.segments[1].end_real, 400);
    }

    #[test]
    fn test_mapping() {
        let mut builder = GapIndexBuilder::new();
        // Gap of 100ms between 100 and 200
        builder.add_rule(ExclusionRule::Fixed {
            start: 100,
            end: 200,
        });
        let index = builder.build(0, 1000);

        assert_eq!(index.to_logical(50), 50);
        assert_eq!(index.to_real(50), 50);

        assert_eq!(index.to_logical(150), 100);

        assert_eq!(index.to_logical(250), 150);
        assert_eq!(index.to_real(150), 250);
    }

    #[test]
    fn test_recurring_night_gap() {
        let mut builder = GapIndexBuilder::new();
        builder.add_rule(ExclusionRule::Recurring {
            days: vec![Weekday::Mon],
            start_time: NaiveTime::from_hms_opt(17, 30, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            timezone: "UTC".to_string(),
        });

        // Monday Jan 12th 2026 at 17:30 UTC
        let start_ms = 1768239000000;
        // Tuesday Jan 13th 2026 at 09:00 UTC
        let end_ms = 1768294800000;

        let index = builder.build(start_ms - 1000, end_ms + 1000);
        assert!(!index.segments.is_empty());
        let seg = &index.segments[0];
        assert_eq!(seg.start_real, start_ms);
        assert_eq!(seg.end_real, end_ms);
    }

    #[test]
    fn test_numeric_gap() {
        let mut builder = GapIndexBuilder::new();
        // Hide 10 units every 100 units, starting at 50.
        builder.add_rule(ExclusionRule::Numeric {
            modulo: 100.0,
            offset: 50.0,
            width: 10.0,
        });

        let index = builder.build(0, 300);
        // Expected gaps: [50, 60], [150, 160], [250, 260]
        assert_eq!(index.segments.len(), 3);
        assert_eq!(index.segments[0].start_real, 50);
        assert_eq!(index.segments[0].end_real, 60);

        assert_eq!(index.to_logical(100), 90); // 100 - 10
        assert_eq!(index.to_logical(200), 180); // 200 - 20
    }

    #[test]
    fn test_mapping_cursor() {
        let mut builder = GapIndexBuilder::new();
        // Gaps: [100, 200], [300, 400]
        builder.add_rule(ExclusionRule::Fixed {
            start: 100,
            end: 200,
        });
        builder.add_rule(ExclusionRule::Fixed {
            start: 300,
            end: 400,
        });
        let index = builder.build(0, 500);
        let mut cursor = index.cursor();

        assert_eq!(cursor.to_logical(50), 50);
        assert_eq!(cursor.to_logical(100), 100);
        assert_eq!(cursor.to_logical(150), 100); // Inside first gap
        assert_eq!(cursor.to_logical(200), 100); // End of first gap
        assert_eq!(cursor.to_logical(250), 150);
        assert_eq!(cursor.to_logical(350), 200); // Inside second gap
        assert_eq!(cursor.to_logical(450), 250);
    }
}