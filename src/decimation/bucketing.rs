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
    logical_range: f64,
) -> Vec<Range<usize>>
where
    F: Fn(usize) -> f64,
{
    if n == 0 || logical_bin_size <= 0.0 {
        return Vec::new();
    }

    let mut current_idx = 0;
    
    if gaps.is_none() {
        // FAST PATH: No gaps, use direct index stepping
        let avg_pts_per_bin = (n as f64 * (logical_bin_size / logical_range)).round() as usize;
        let bin_size = avg_pts_per_bin.max(1);
        let mut buckets = Vec::with_capacity(n / bin_size + 1);
        for i in (0..n).step_by(bin_size) {
            buckets.push(i..(i + bin_size).min(n));
        }
        return buckets;
    }

    let mut buckets = Vec::with_capacity(n.min(2048));
    
    let mut buckets_to_find = (logical_range / logical_bin_size).ceil() as usize;
    buckets_to_find = buckets_to_find.max(1);

    let first_x = get_x_at(0);
    let first_logical = if let Some(g) = gaps {
        g.to_logical(first_x as i64) as f64
    } else {
        first_x
    };

    // Align to global grid
    let mut next_boundary = (first_logical / logical_bin_size).floor() * logical_bin_size + logical_bin_size;
    
    while current_idx < n {
        let target_real = if let Some(g) = gaps {
            g.to_real_first(next_boundary as i64) as f64
        } else {
            next_boundary
        };

        // Heuristic: the next boundary is likely around remaining_n / remaining_buckets from here.
        let remaining_n = n - current_idx;
        let step_guess = (remaining_n / buckets_to_find.max(1)) * 2;
        let mut right = (current_idx + step_guess).min(n);
        
        // If our guess was too short, fallback to full range
        if right < n && get_x_at(right - 1) < target_real {
            right = n;
        }

        let mut left = current_idx;
        while left < right {
            let mid = left + (right - left) / 2;
            if get_x_at(mid) < target_real {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        let end_idx = left;
        if end_idx > current_idx {
            buckets.push(current_idx..end_idx);
        }
        
        current_idx = end_idx;
        if current_idx < n {
            let x = get_x_at(current_idx);
            let logical = if let Some(g) = gaps {
                g.to_logical(x as i64) as f64
            } else {
                x
            };
            while logical >= next_boundary {
                next_boundary += logical_bin_size;
                if buckets_to_find > 1 { buckets_to_find -= 1; }
            }
        }
    }

    buckets
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

    let buckets = calculate_logical_time_buckets_generic(n, get_x_at, gaps, stable_bin_size, logical_range);
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
