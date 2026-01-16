use std::ops::Range;
use crate::data_types::PlotData;
use crate::gaps::GapIndex;
use super::common::get_data_x;

/// Core logic for calculating gap-aware buckets.
/// Generic over how we retrieve the x-value for a given index.
pub fn calculate_gap_aware_buckets_generic<F>(
    n: usize,
    get_x_at: F,
    gap_index: Option<&GapIndex>,
    bin_size: usize,
    offset: usize,
) -> Vec<Range<usize>>
where
    F: Fn(usize) -> f64,
{
    if n == 0 {
        return Vec::new();
    }

    let bin_size = bin_size.max(1);
    let mut buckets = Vec::with_capacity(n / bin_size + 2);
    let mut current_start = 0;

    // Align the first bucket to the global grid to ensure stability
    let first_bin_remaining = bin_size - (offset % bin_size);
    let mut next_target = first_bin_remaining.min(n);

    if let Some(gi) = gap_index {
        let segments = gi.segments();
        let mut seg_idx = 0;

        while current_start < n {
            // Advance seg_idx to find the first gap that could potentially split this bin.
            while seg_idx < segments.len()
                && segments[seg_idx].end_real <= get_x_at(current_start) as i64
            {
                seg_idx += 1;
            }

            if seg_idx < segments.len()
                && segments[seg_idx].start_real < get_x_at(next_target - 1) as i64
            {
                // A gap starts within this bin. Find the exact split point.
                let gap_start_real = segments[seg_idx].start_real;
                
                // Find split index relative to current_start without allocation.
                // We search in the range [0 .. (next_target - current_start)]
                let len = next_target - current_start;
                let mut left = 0;
                let mut right = len;
                while left < right {
                    let mid = left + (right - left) / 2;
                    if (get_x_at(current_start + mid) as i64) < gap_start_real {
                        left = mid + 1;
                    } else {
                        right = mid;
                    }
                }
                let split_offset = left;
                
                let actual_split_idx = current_start + split_offset;

                if actual_split_idx > current_start {
                    buckets.push(current_start..actual_split_idx);
                    current_start = actual_split_idx;
                    next_target = (current_start + bin_size).min(n);
                } else {
                    // current_start is already in or after the gap. Skip it.
                    let gap_end_real = segments[seg_idx].end_real;
                    
                    // Search for where to resume without allocation
                    let len_rest = n - current_start;
                    let mut left = 0;
                    let mut right = len_rest;
                    while left < right {
                        let mid = left + (right - left) / 2;
                        if (get_x_at(current_start + mid) as i64) < gap_end_real {
                            left = mid + 1;
                        } else {
                            right = mid;
                        }
                    }
                    let skip_offset = left;

                    current_start += skip_offset;
                    seg_idx += 1;
                    next_target = (current_start + bin_size).min(n);
                }
            } else {
                buckets.push(current_start..next_target);
                current_start = next_target;
                next_target = (current_start + bin_size).min(n);
            }
        }
    } else {
        while current_start < n {
            buckets.push(current_start..next_target);
            current_start = next_target;
            next_target = (current_start + bin_size).min(n);
        }
    }

    buckets
}

pub fn calculate_gap_aware_buckets_data(
    data: &[PlotData],
    gap_index: Option<&GapIndex>,
    bin_size: usize,
    offset: usize,
) -> Vec<Range<usize>> {
    calculate_gap_aware_buckets_generic(
        data.len(),
        |i| get_data_x(&data[i]),
        gap_index,
        bin_size,
        offset,
    )
}

/// Calculates index ranges (buckets) that respect gaps.
/// A bucket ends if it reaches `bin_size` OR if a gap is encountered.
pub fn calculate_gap_aware_buckets(
    x: &[f64],
    gap_index: Option<&GapIndex>,
    bin_size: usize,
    offset: usize,
) -> Vec<Range<usize>> {
    calculate_gap_aware_buckets_generic(x.len(), |i| x[i], gap_index, bin_size, offset)
}
