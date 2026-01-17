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

pub fn calculate_logical_time_buckets_generic<F>(
    n: usize,
    get_x_at: F,
    gaps: Option<&GapIndex>,
    logical_bin_size: f64,
) -> Vec<Range<usize>>
where
    F: Fn(usize) -> f64,
{
    if n == 0 || logical_bin_size <= 0.0 {
        return Vec::new();
    }

    let mut buckets = Vec::with_capacity(n.min(2048));
    let mut current_idx = 0;
    
    // Helper to get logical time without cursor (for binary search)
    let get_logical_at = |idx: usize| -> f64 {
        let x = get_x_at(idx);
        if let Some(g) = gaps {
            g.to_logical(x as i64) as f64
        } else {
            x
        }
    };

    let first_logical = get_logical_at(0);
    // Align to global grid
    let mut next_boundary = (first_logical / logical_bin_size).floor() * logical_bin_size + logical_bin_size;
    
    while current_idx < n {
        let mut left = current_idx;
        let mut right = n;
        
        if let Some(g) = gaps {
            // Optimization: compute real boundary once per bucket to avoid to_logical in inner loop
            let target_real = g.to_real_first(next_boundary as i64) as f64;
            while left < right {
                let mid = left + (right - left) / 2;
                if get_x_at(mid) < target_real {
                    left = mid + 1;
                }
                else {
                    right = mid;
                }
            }
        } else {
            // Faster binary search without conversion
            while left < right {
                let mid = left + (right - left) / 2;
                if get_x_at(mid) < next_boundary {
                    left = mid + 1;
                }
                else {
                    right = mid;
                }
            }
        }
        
        let end_idx = left;
        
        if end_idx > current_idx {
            buckets.push(current_idx..end_idx);
        }
        
        current_idx = end_idx;
        if current_idx < n {
            // Advance boundary to cover current point's logical time
            let logical = get_logical_at(current_idx);
            while logical >= next_boundary {
                next_boundary += logical_bin_size;
            }
        }
    }

    buckets
}

/// Calculates buckets based on Logical Time rather than index count.
pub fn calculate_logical_time_buckets(
    x: &[f64],
    gaps: Option<&GapIndex>,
    logical_bin_size: f64,
) -> Vec<Range<usize>> {
    calculate_logical_time_buckets_generic(x.len(), |i| x[i], gaps, logical_bin_size)
}

pub fn calculate_logical_time_buckets_data(
    data: &[PlotData],
    gaps: Option<&GapIndex>,
    logical_bin_size: f64,
) -> Vec<Range<usize>> {
    calculate_logical_time_buckets_generic(
        data.len(),
        |i| get_data_x(&data[i]),
        gaps,
        logical_bin_size,
    )
}

/// Helper to calculate stable buckets for any decimation algorithm.
/// Returns (stable_bin_size, buckets).
pub fn calculate_stable_buckets_generic<F>(
    n: usize,
    get_x_at: F,
    gaps: Option<&GapIndex>,
    max_points: usize,
    points_per_bucket: usize,
    reference_logical_range: Option<f64>,
) -> (f64, Vec<Range<usize>>)
where
    F: Fn(usize) -> f64,
{
    if n == 0 {
        return (1.0, Vec::new());
    }

    let logical_range = if let Some(r) = reference_logical_range {
        r
    } else {
        let start_x = get_x_at(0);
        let end_x = get_x_at(n - 1);
        if let Some(g) = gaps {
            (g.to_logical(end_x as i64) - g.to_logical(start_x as i64)) as f64
        } else {
            end_x - start_x
        }
    };
    
    let target_buckets = (max_points / points_per_bucket).max(1);
    // Use fully qualified path or import
    let stable_bin_size = crate::decimation::common::calculate_stable_bin_size(logical_range, target_buckets);

    let buckets = calculate_logical_time_buckets_generic(n, get_x_at, gaps, stable_bin_size);
    (stable_bin_size, buckets)
}

pub fn calculate_stable_buckets(
    x: &[f64],
    gaps: Option<&GapIndex>,
    max_points: usize,
    points_per_bucket: usize,
    reference_logical_range: Option<f64>,
) -> (f64, Vec<Range<usize>>) {
    calculate_stable_buckets_generic(x.len(), |i| x[i], gaps, max_points, points_per_bucket, reference_logical_range)
}

pub fn calculate_stable_buckets_data(
    data: &[PlotData],
    gaps: Option<&GapIndex>,
    max_points: usize,
    points_per_bucket: usize,
    reference_logical_range: Option<f64>,
) -> (f64, Vec<Range<usize>>) {
    calculate_stable_buckets_generic(
        data.len(),
        |i| get_data_x(&data[i]),
        gaps,
        max_points,
        points_per_bucket,
        reference_logical_range,
    )
}
