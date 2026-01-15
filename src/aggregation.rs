use crate::data_types::{ColorOp, Ohlcv, PlotData, PlotPoint};
use crate::gaps::{GapIndex, MappingCursor};

use rayon::prelude::*;

/// Calculates a stable bin size (power of 10 or 2) that is just above the ideal resolution.
fn calculate_stable_bin_size(range: f64, max_points: usize) -> f64 {
    if range <= 0.0 || max_points == 0 {
        return 1.0;
    }
    let ideal = range / max_points as f64;
    let exponent = ideal.log10().floor();
    let base = 10.0f64.powf(exponent);
    let rel = ideal / base;

    let stable_rel = if rel <= 1.0 {
        1.0
    } else if rel <= 2.0 {
        2.0
    } else if rel <= 5.0 {
        5.0
    } else {
        10.0
    };

    base * stable_rel
}

use std::ops::Range;

fn get_data_x(p: &PlotData) -> f64 {
    match p {
        PlotData::Point(pt) => pt.x,
        PlotData::Ohlcv(o) => o.time,
    }
}

pub fn calculate_gap_aware_buckets_data(
    data: &[PlotData],
    gap_index: Option<&GapIndex>,
    bin_size: usize,
) -> Vec<Range<usize>> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    let bin_size = bin_size.max(1);
    let mut buckets = Vec::with_capacity(n / bin_size + 1);
    let mut current_start = 0;

    if let Some(gi) = gap_index {
        let segments = gi.segments();
        let mut seg_idx = 0;

        while current_start < n {
            let next_target = (current_start + bin_size).min(n);

            // Advance seg_idx to find the first gap that could potentially split this bin.
            while seg_idx < segments.len()
                && segments[seg_idx].end_real <= get_data_x(&data[current_start]) as i64
            {
                seg_idx += 1;
            }

            if seg_idx < segments.len()
                && segments[seg_idx].start_real < get_data_x(&data[next_target - 1]) as i64
            {
                // A gap starts within this bin. Find the exact split point.
                let gap_start_real = segments[seg_idx].start_real;
                let split_idx = data[current_start..next_target]
                    .partition_point(|val| (get_data_x(val) as i64) < gap_start_real);
                let actual_split_idx = current_start + split_idx;

                if actual_split_idx > current_start {
                    buckets.push(current_start..actual_split_idx);
                    current_start = actual_split_idx;
                } else {
                    // current_start is already in or after the gap. Skip it.
                    let gap_end_real = segments[seg_idx].end_real;
                    let skip_idx = data[current_start..n]
                        .partition_point(|val| (get_data_x(val) as i64) < gap_end_real);
                    current_start += skip_idx;
                    seg_idx += 1;
                }
            } else {
                buckets.push(current_start..next_target);
                current_start = next_target;
            }
        }
    } else {
        for i in (0..n).step_by(bin_size) {
            buckets.push(i..(i + bin_size).min(n));
        }
        return buckets;
    }

    buckets
}

/// Calculates index ranges (buckets) that respect gaps.
/// A bucket ends if it reaches `bin_size` OR if a gap is encountered.
pub fn calculate_gap_aware_buckets(
    x: &[f64],
    gap_index: Option<&GapIndex>,
    bin_size: usize,
) -> Vec<Range<usize>> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }

    let bin_size = bin_size.max(1);
    let mut buckets = Vec::with_capacity(n / bin_size + 1);
    let mut current_start = 0;

    if let Some(gi) = gap_index {
        let segments = gi.segments();
        let mut seg_idx = 0;

        while current_start < n {
            let next_target = (current_start + bin_size).min(n);

            // Advance seg_idx to find the first gap that could potentially split this bin.
            while seg_idx < segments.len() && segments[seg_idx].end_real <= x[current_start] as i64 {
                seg_idx += 1;
            }

            if seg_idx < segments.len() && segments[seg_idx].start_real < x[next_target - 1] as i64 {
                // A gap starts within this bin. Find the exact split point.
                let gap_start_real = segments[seg_idx].start_real;
                let split_idx = x[current_start..next_target]
                    .partition_point(|&val| (val as i64) < gap_start_real);
                let actual_split_idx = current_start + split_idx;

                if actual_split_idx > current_start {
                    buckets.push(current_start..actual_split_idx);
                    current_start = actual_split_idx;
                } else {
                    // x[current_start] is already in or after the gap. Skip it.
                    let gap_end_real = segments[seg_idx].end_real;
                    let skip_idx = x[current_start..n]
                        .partition_point(|&val| (val as i64) < gap_end_real);
                    current_start += skip_idx;
                    seg_idx += 1;
                }
            } else {
                buckets.push(current_start..next_target);
                current_start = next_target;
            }
        }
    } else {
        for i in (0..n).step_by(bin_size) {
            buckets.push(i..(i + bin_size).min(n));
        }
        return buckets;
    }

    buckets
}

/// Decimates parallel arrays (Structure of Arrays) using M4 and Rayon.
pub fn decimate_m4_arrays_par(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_m4_arrays_par_into(x, y, max_points, &mut output, gaps);
    output
}

fn aggregate_m4_bucket_to_array(x_chunk: &[f64], y_chunk: &[f64]) -> ([PlotPoint; 4], usize) {
    let n = x_chunk.len();
    if n == 0 {
        return ([PlotPoint::default(); 4], 0);
    }
    if n == 1 {
        return (
            [
                PlotPoint {
                    x: x_chunk[0],
                    y: y_chunk[0],
                    color_op: ColorOp::None,
                },
                PlotPoint::default(),
                PlotPoint::default(),
                PlotPoint::default(),
            ],
            1,
        );
    }

    let first_idx = 0;
    let last_idx = n - 1;
    let mut min_idx = 0;
    let mut max_idx = 0;
    let mut min_y = y_chunk[0];
    let mut max_y = min_y;

    let start_search = if min_y.is_nan() {
        let mut found = false;
        let mut idx = 0;
        for (i, val) in y_chunk.iter().enumerate().skip(1) {
            if !val.is_nan() {
                min_y = *val;
                max_y = *val;
                min_idx = i;
                max_idx = i;
                idx = i;
                found = true;
                break;
            }
        }
        if found {
            idx + 1
        } else {
            n
        }
    } else {
        1
    };

    for (i, val) in y_chunk.iter().enumerate().skip(start_search) {
        if val.is_nan() {
            continue;
        }
        if *val < min_y {
            min_y = *val;
            min_idx = i;
        }
        if *val > max_y {
            max_y = *val;
            max_idx = i;
        }
    }

    let mut idxs = [first_idx, min_idx, max_idx, last_idx];
    idxs.sort_unstable();

    let mut result = [PlotPoint::default(); 4];
    result[0] = PlotPoint {
        x: x_chunk[idxs[0]],
        y: y_chunk[idxs[0]],
        color_op: ColorOp::None,
    };
    let mut count = 1;
    for i in 1..4 {
        if idxs[i] != idxs[i - 1] {
            result[count] = PlotPoint {
                x: x_chunk[idxs[i]],
                y: y_chunk[idxs[i]],
                color_op: ColorOp::None,
            };
            count += 1;
        }
    }
    (result, count)
}

pub fn decimate_m4_arrays_par_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
) {
    let initial_len = output.len();
    if x.is_empty() || y.is_empty() || x.len() != y.len() {
        return;
    }

    if x.len() <= max_points {
        output.extend(x.iter().zip(y.iter()).map(|(x_val, y_val)| {
            PlotData::Point(PlotPoint {
                x: *x_val,
                y: *y_val,
                color_op: ColorOp::None,
            })
        }));
        return;
    }

    // Use stable binning to prevent jitter during pan
    let real_range = x[x.len() - 1] - x[0];

    // If gaps are present, calculate logical range for stable bin size
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(x[x.len() - 1] as i64) - g.to_logical(x[0] as i64)) as f64
    } else {
        real_range
    };

    let stable_bin_size = calculate_stable_bin_size(logical_range, (max_points / 4).max(1));

    // Calculate stable index-based bin size based on value range
    let avg_items_per_bin = (x.len() as f64 * (stable_bin_size / logical_range)).ceil() as usize;
    let bin_size = avg_items_per_bin.max(1);

    // Pre-calculate buckets that respect gaps
    let buckets = calculate_gap_aware_buckets(x, gaps, bin_size);

    // Process buckets in parallel, returning fixed-size arrays to avoid allocations.
    let chunks: Vec<([PlotPoint; 4], usize)> = if gaps.is_some() {
        buckets
            .into_par_iter()
            .map(|range| {
                let x_chunk = &x[range.start..range.end];
                let y_chunk = &y[range.start..range.end];
                aggregate_m4_bucket_to_array(x_chunk, y_chunk)
            })
            .collect()
    } else {
        x.par_chunks(bin_size)
            .zip(y.par_chunks(bin_size))
            .map(|(x_chunk, y_chunk)| aggregate_m4_bucket_to_array(x_chunk, y_chunk))
            .collect()
    };

    // Flatten results
    for (pts, n) in chunks {
        for i in 0..n {
            output.push(PlotData::Point(pts[i]));
        }
    }

    // Ensure strict limit (Safeguard)
    let final_len = output.len();
    if final_len > initial_len + max_points {
        output.truncate(initial_len + max_points);
    }
}

pub fn decimate_ohlcv_arrays_par(
    time: &[f64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    max_points: usize,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_ohlcv_arrays_par_into(time, open, high, low, close, max_points, &mut output, gaps);
    output
}

pub fn decimate_ohlcv_arrays_par_into(
    time: &[f64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
) {
    let initial_len = output.len();
    if time.is_empty() {
        return;
    }

    if time.len() <= max_points {
        for i in 0..time.len() {
            output.push(PlotData::Ohlcv(Ohlcv {
                time: time[i],
                span: 0.0,
                open: open.get(i).copied().unwrap_or(0.0),
                high: high.get(i).copied().unwrap_or(0.0),
                low: low.get(i).copied().unwrap_or(0.0),
                close: close.get(i).copied().unwrap_or(0.0),
                volume: 0.0,
            }));
        }
        return;
    }

    let real_range = time[time.len() - 1] - time[0];
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(time[time.len() - 1] as i64) - g.to_logical(time[0] as i64)) as f64
    } else {
        real_range
    };

    let stable_bin_size = calculate_stable_bin_size(logical_range, max_points.max(1));
    let avg_frequency = time.len() as f64 / logical_range;
    let items_per_stable_bin = (stable_bin_size * avg_frequency).ceil() as usize;
    let bin_size = items_per_stable_bin.max(1);

    // Pre-calculate buckets that respect gaps
    let buckets = calculate_gap_aware_buckets(time, gaps, bin_size);

    let chunks: Vec<PlotData> = buckets
        .into_par_iter()
        .filter_map(|range| {
            let start_idx = range.start;
            let end_idx = range.end;
            let t_chunk = &time[start_idx..end_idx];

            if t_chunk.is_empty() {
                return None;
            }

            let o_chunk = &open[start_idx..end_idx.min(open.len())];
            let h_chunk = &high[start_idx..end_idx.min(high.len())];
            let l_chunk = &low[start_idx..end_idx.min(low.len())];
            let c_chunk = &close[start_idx..end_idx.min(close.len())];

            if o_chunk.is_empty() {
                return None;
            }

            let mut agg_open = 0.0;
            for &v in o_chunk {
                if !v.is_nan() {
                    agg_open = v;
                    break;
                }
            }

            let mut agg_close = 0.0;
            for &v in c_chunk.iter().rev() {
                if !v.is_nan() {
                    agg_close = v;
                    break;
                }
            }

            let agg_high = crate::simd::max_f64(h_chunk);
            let agg_low = crate::simd::min_f64(l_chunk);

            let first_time = t_chunk[0];

            Some(PlotData::Ohlcv(Ohlcv {
                time: (first_time / stable_bin_size).floor() * stable_bin_size,
                span: stable_bin_size,
                open: agg_open,
                high: if agg_high.is_nan() { 0.0 } else { agg_high },
                low: if agg_low.is_nan() { 0.0 } else { agg_low },
                close: agg_close,
                volume: 0.0,
            }))
        })
        .collect();

    output.extend(chunks);

    if output.len() > initial_len + max_points {
        output.truncate(initial_len + max_points);
    }
}

pub fn decimate_min_max_arrays_par(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_min_max_arrays_par_into(x, y, max_points, &mut output, gaps);
    output
}

fn aggregate_min_max_bucket_to_array(x_chunk: &[f64], y_chunk: &[f64]) -> ([PlotPoint; 2], usize) {
    let n = x_chunk.len();
    if n == 0 {
        return ([PlotPoint::default(); 2], 0);
    }
    if n == 1 {
        return (
            [
                PlotPoint {
                    x: x_chunk[0],
                    y: y_chunk[0],
                    color_op: ColorOp::None,
                },
                PlotPoint::default(),
            ],
            1,
        );
    }

    let mut min_idx = 0;
    let mut max_idx = 0;
    let mut min_y = y_chunk[0];
    let mut max_y = min_y;

    let start_search = if min_y.is_nan() {
        let mut found = false;
        let mut idx = 0;
        for (i, val) in y_chunk.iter().enumerate().skip(1) {
            if !val.is_nan() {
                min_y = *val;
                max_y = *val;
                min_idx = i;
                max_idx = i;
                idx = i;
                found = true;
                break;
            }
        }
        if found {
            idx + 1
        } else {
            n
        }
    } else {
        1
    };

    for (i, val) in y_chunk.iter().enumerate().skip(start_search) {
        if val.is_nan() {
            continue;
        }
        if *val < min_y {
            min_y = *val;
            min_idx = i;
        }
        if *val > max_y {
            max_y = *val;
            max_idx = i;
        }
    }

    let mut result = [PlotPoint::default(); 2];
    if min_idx == max_idx {
        result[0] = PlotPoint {
            x: x_chunk[min_idx],
            y: y_chunk[min_idx],
            color_op: ColorOp::None,
        };
        (result, 1)
    } else {
        let p1_x = x_chunk[min_idx];
        let p1_y = y_chunk[min_idx];
        let p2_x = x_chunk[max_idx];
        let p2_y = y_chunk[max_idx];

        if p1_x <= p2_x {
            result[0] = PlotPoint {
                x: p1_x,
                y: p1_y,
                color_op: ColorOp::None,
            };
            result[1] = PlotPoint {
                x: p2_x,
                y: p2_y,
                color_op: ColorOp::None,
            };
        } else {
            result[0] = PlotPoint {
                x: p2_x,
                y: p2_y,
                color_op: ColorOp::None,
            };
            result[1] = PlotPoint {
                x: p1_x,
                y: p1_y,
                color_op: ColorOp::None,
            };
        }
        (result, 2)
    }
}

pub fn decimate_min_max_arrays_par_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
) {
    let initial_len = output.len();
    if x.is_empty() || y.is_empty() || x.len() != y.len() {
        return;
    }

    if x.len() <= max_points {
        output.extend(x.iter().zip(y.iter()).map(|(x_val, y_val)| {
            PlotData::Point(PlotPoint {
                x: *x_val,
                y: *y_val,
                color_op: ColorOp::None,
            })
        }));
        return;
    }

    let real_range = x[x.len() - 1] - x[0];
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(x[x.len() - 1] as i64) - g.to_logical(x[0] as i64)) as f64
    } else {
        real_range
    };

    let stable_bin_size = calculate_stable_bin_size(logical_range, (max_points / 2).max(1));
    let avg_items_per_bin = (x.len() as f64 * (stable_bin_size / logical_range)).ceil() as usize;
    let bin_size = avg_items_per_bin.max(1);

    // Pre-calculate buckets that respect gaps
    let buckets = calculate_gap_aware_buckets(x, gaps, bin_size);

    let chunks: Vec<([PlotPoint; 2], usize)> = if gaps.is_some() {
        buckets
            .into_par_iter()
            .map(|range| {
                let x_chunk = &x[range.start..range.end];
                let y_chunk = &y[range.start..range.end];
                aggregate_min_max_bucket_to_array(x_chunk, y_chunk)
            })
            .collect()
    } else {
        x.par_chunks(bin_size)
            .zip(y.par_chunks(bin_size))
            .map(|(x_chunk, y_chunk)| aggregate_min_max_bucket_to_array(x_chunk, y_chunk))
            .collect()
    };

    for (pts, n) in chunks {
        for i in 0..n {
            output.push(PlotData::Point(pts[i]));
        }
    }

    if output.len() > initial_len + max_points {
        output.truncate(initial_len + max_points);
    }
}

pub fn decimate_lttb_arrays(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    _gaps: Option<&GapIndex>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_lttb_arrays_into(x, y, max_points, &mut output, _gaps);
    output
}

pub fn decimate_lttb_arrays_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    _gaps: Option<&GapIndex>,
) {
    if x.len() <= max_points || max_points < 3 {
        output.extend(x.iter().zip(y.iter()).map(|(x_val, y_val)| {
            PlotData::Point(PlotPoint {
                x: *x_val,
                y: *y_val,
                color_op: ColorOp::None,
            })
        }));
        return;
    }

    let range = x[x.len() - 1] - x[0];
    let stable_bin_size = calculate_stable_bin_size(range, max_points - 2);
    let target_bucket_count = (range / stable_bin_size).round() as usize;
    let target_bucket_count = target_bucket_count.clamp(1, max_points - 2);

    let bucket_size = (x.len() - 2) as f64 / target_bucket_count as f64;

    let mut a_idx = 0;
    output.push(PlotData::Point(PlotPoint {
        x: x[0],
        y: y[0],
        color_op: ColorOp::None,
    }));

    for i in 0..target_bucket_count {
        let bucket_start = 1 + (i as f64 * bucket_size).floor() as usize;
        let bucket_end = (1 + ((i + 1) as f64 * bucket_size).floor() as usize).min(x.len() - 1);

        if bucket_start >= bucket_end {
            continue;
        }

        let next_bucket_start = bucket_end;
        let next_bucket_end =
            (1 + ((i + 2) as f64 * bucket_size).floor() as usize).min(x.len() - 1);

        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        let mut avg_count = 0;

        for j in next_bucket_start..next_bucket_end {
            let val_x = x[j];
            let val_y = y[j];
            if !val_x.is_nan() && !val_y.is_nan() {
                avg_x += val_x;
                avg_y += val_y;
                avg_count += 1;
            }
        }

        if avg_count > 0 {
            avg_x /= avg_count as f64;
            avg_y /= avg_count as f64;
        } else {
            let idx = next_bucket_start.min(x.len() - 1);
            avg_x = x[idx];
            avg_y = y[idx];
        }

        let p_a_x = x[a_idx];
        let p_a_y = y[a_idx];

        let mut max_area = -1.0;
        let mut next_a_idx = bucket_start;

        for j in bucket_start..bucket_end {
            let p_b_x = x[j];
            let p_b_y = y[j];

            if p_b_x.is_nan() || p_b_y.is_nan() {
                continue;
            }

            let area =
                (p_a_x * (p_b_y - avg_y) + p_b_x * (avg_y - p_a_y) + avg_x * (p_a_y - p_b_y)).abs();

            if area > max_area {
                max_area = area;
                next_a_idx = j;
            }
        }

        a_idx = next_a_idx;
        output.push(PlotData::Point(PlotPoint {
            x: x[a_idx],
            y: y[a_idx],
            color_op: ColorOp::None,
        }));
    }

    output.push(PlotData::Point(PlotPoint {
        x: x[x.len() - 1],
        y: y[y.len() - 1],
        color_op: ColorOp::None,
    }));
}

pub fn decimate_min_max_slice(
    data: &[PlotData],
    max_points: usize,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_min_max_slice_into(data, max_points, &mut output, gaps);
    output
}

pub fn decimate_min_max_slice_into(
    data: &[PlotData],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
) {
    let initial_len = output.len();
    if data.is_empty() {
        return;
    }

    if let PlotData::Ohlcv(_) = data[0] {
        decimate_ohlcv_slice_into(data, max_points, output, gaps);
        return;
    }

    if data.len() <= max_points {
        output.extend_from_slice(data);
        return;
    }

    let real_range = get_data_x(&data[data.len() - 1]) - get_data_x(&data[0]);
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(get_data_x(&data[data.len() - 1]) as i64)
            - g.to_logical(get_data_x(&data[0]) as i64)) as f64
    } else {
        real_range
    };

    let stable_bin_size = calculate_stable_bin_size(logical_range, (max_points / 2).max(1));
    let avg_items_per_bin = (data.len() as f64 * (stable_bin_size / logical_range)).ceil() as usize;
    let bin_size = avg_items_per_bin.max(1);

    // Pre-calculate buckets that respect gaps
    let buckets = calculate_gap_aware_buckets_data(data, gaps, bin_size);

    // Using parallel iteration if possible, but for slice of PlotData (AoS), Rayon might be less effective due to cache locality of fields?
    // But we can parallelize buckets.
    let chunks: Vec<([PlotData; 2], usize)> = if gaps.is_some() {
        buckets
            .into_par_iter()
            .map(|range| {
                let chunk = &data[range.start..range.end];
                aggregate_min_max_bucket_generic(chunk)
            })
            .collect()
    } else {
        data.par_chunks(bin_size)
            .map(|chunk| aggregate_min_max_bucket_generic(chunk))
            .collect()
    };

    for (pts, n) in chunks {
        for i in 0..n {
            output.push(pts[i].clone());
        }
    }

    if output.len() > initial_len + max_points {
        output.truncate(initial_len + max_points);
    }
}

fn aggregate_min_max_bucket_generic(chunk: &[PlotData]) -> ([PlotData; 2], usize) {
    let n = chunk.len();
    if n == 0 {
        return ([PlotData::Point(PlotPoint::default()), PlotData::Point(PlotPoint::default())], 0);
    }
    if n == 1 {
        return ([chunk[0].clone(), PlotData::Point(PlotPoint::default())], 1);
    }

    let mut min_idx = 0;
    let mut max_idx = 0;
    let mut min_y = get_data_y(&chunk[0]);
    let mut max_y = min_y;

    let start_search = if min_y.is_nan() {
        let mut found = false;
        let mut idx = 0;
        for (i, item) in chunk.iter().enumerate().skip(1) {
            let val = get_data_y(item);
            if !val.is_nan() {
                min_y = val;
                max_y = val;
                min_idx = i;
                max_idx = i;
                idx = i;
                found = true;
                break;
            }
        }
        if found {
            idx + 1
        } else {
            n
        }
    } else {
        1
    };

    for (i, item) in chunk.iter().enumerate().skip(start_search) {
        let val = get_data_y(item);
        if val.is_nan() {
            continue;
        }
        if val < min_y {
            min_y = val;
            min_idx = i;
        }
        if val > max_y {
            max_y = val;
            max_idx = i;
        }
    }

    if min_idx == max_idx {
        ([chunk[min_idx].clone(), PlotData::Point(PlotPoint::default())], 1)
    } else {
        let p1 = &chunk[min_idx];
        let p2 = &chunk[max_idx];
        if get_data_x(p1) <= get_data_x(p2) {
            ([p1.clone(), p2.clone()], 2)
        } else {
            ([p2.clone(), p1.clone()], 2)
        }
    }
}

fn get_data_y(p: &PlotData) -> f64 {
    match p {
        PlotData::Point(pt) => pt.y,
        PlotData::Ohlcv(o) => o.close, // or handle differently? usually MinMax is for lines (y)
    }
}

pub fn decimate_ohlcv_slice_into(
    data: &[PlotData],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
) {
    let initial_len = output.len();
    if data.is_empty() {
        return;
    }

    if data.len() <= max_points {
        output.extend_from_slice(data);
        return;
    }

    let real_range = get_data_x(&data[data.len() - 1]) - get_data_x(&data[0]);
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(get_data_x(&data[data.len() - 1]) as i64)
            - g.to_logical(get_data_x(&data[0]) as i64)) as f64
    } else {
        real_range
    };

    let stable_bin_size = calculate_stable_bin_size(logical_range, max_points.max(1));
    let avg_items_per_bin = (data.len() as f64 * (stable_bin_size / logical_range)).ceil() as usize;
    let bin_size = avg_items_per_bin.max(1);

    let buckets = calculate_gap_aware_buckets_data(data, gaps, bin_size);

    let chunks: Vec<PlotData> = buckets
        .into_par_iter()
        .filter_map(|range| {
            let chunk = &data[range.start..range.end];
            aggregate_chunk(chunk)
        })
        .collect();
    
    output.extend(chunks);

    if output.len() > initial_len + max_points {
        output.truncate(initial_len + max_points);
    }
}

pub fn decimate_m4_slice(
    data: &[PlotData],
    max_points: usize,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_m4_slice_into(data, max_points, &mut output, gaps);
    output
}

pub fn decimate_m4_slice_into(
    data: &[PlotData],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
) {
    let initial_len = output.len();
    if data.is_empty() { return; }

    if let PlotData::Ohlcv(_) = data[0] {
        decimate_ohlcv_slice_into(data, max_points, output, gaps);
        return;
    }

    if data.len() <= max_points {
        output.extend_from_slice(data);
        return;
    }

    let real_range = get_data_x(&data[data.len() - 1]) - get_data_x(&data[0]);
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(get_data_x(&data[data.len() - 1]) as i64)
            - g.to_logical(get_data_x(&data[0]) as i64)) as f64
    } else {
        real_range
    };

    let stable_bin_size = calculate_stable_bin_size(logical_range, (max_points / 4).max(1));
    let avg_items_per_bin = (data.len() as f64 * (stable_bin_size / logical_range)).ceil() as usize;
    let bin_size = avg_items_per_bin.max(1);

    let buckets = calculate_gap_aware_buckets_data(data, gaps, bin_size);

    let chunks: Vec<([PlotData; 4], usize)> = if gaps.is_some() {
        buckets
            .into_par_iter()
            .map(|range| {
                let chunk = &data[range.start..range.end];
                aggregate_m4_bucket_generic(chunk)
            })
            .collect()
    } else {
        data.par_chunks(bin_size)
            .map(|chunk| aggregate_m4_bucket_generic(chunk))
            .collect()
    };

    for (pts, n) in chunks {
        for i in 0..n {
            output.push(pts[i].clone());
        }
    }

    if output.len() > initial_len + max_points {
        output.truncate(initial_len + max_points);
    }
}

fn aggregate_m4_bucket_generic(chunk: &[PlotData]) -> ([PlotData; 4], usize) {
    let n = chunk.len();
    if n == 0 {
        return ([
            PlotData::Point(PlotPoint::default()),
            PlotData::Point(PlotPoint::default()),
            PlotData::Point(PlotPoint::default()),
            PlotData::Point(PlotPoint::default()),
        ], 0);
    }
    if n == 1 {
        return ([chunk[0].clone(), PlotData::Point(PlotPoint::default()), PlotData::Point(PlotPoint::default()), PlotData::Point(PlotPoint::default())], 1);
    }

    let first = &chunk[0];
    
    let mut min_idx = 0;
    let mut max_idx = 0;
    let mut min_y = get_data_y(first);
    let mut max_y = min_y;

    let start_search = if min_y.is_nan() {
        let mut found = false;
        let mut idx = 0;
        for (i, item) in chunk.iter().enumerate().skip(1) {
            let val = get_data_y(item);
            if !val.is_nan() {
                min_y = val;
                max_y = val;
                min_idx = i;
                max_idx = i;
                idx = i;
                found = true;
                break;
            }
        }
        if found {
            idx + 1
        } else {
            n
        }
    } else {
        1
    };

    for (i, item) in chunk.iter().enumerate().skip(start_search) {
        let val = get_data_y(item);
        if val.is_nan() {
            continue;
        }
        if val < min_y {
            min_y = val;
            min_idx = i;
        }
        if val > max_y {
            max_y = val;
            max_idx = i;
        }
    }
    
    let mut indices = [0, min_idx, max_idx, n - 1];
    indices.sort_unstable();
    
    let mut result = [
        PlotData::Point(PlotPoint::default()),
        PlotData::Point(PlotPoint::default()),
        PlotData::Point(PlotPoint::default()),
        PlotData::Point(PlotPoint::default()),
    ];
    result[0] = chunk[indices[0]].clone();
    let mut count = 1;
    for i in 1..4 {
        if indices[i] != indices[i-1] {
            result[count] = chunk[indices[i]].clone();
            count += 1;
        }
    }
    (result, count)
}

pub fn decimate_lttb_slice(
    data: &[PlotData],
    max_points: usize,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData> {
    if data.is_empty() {
        return vec![];
    }
    if let PlotData::Ohlcv(_) = data[0] {
        return decimate_min_max_slice(data, max_points, gaps);
    }

    let mut all_points = true;
    for p in data.iter().take(100) {
        if !matches!(p, PlotData::Point(_)) {
            all_points = false;
            break;
        }
    }

    if all_points && data.len() > 1000 {
        let x: Vec<f64> = data
            .iter()
            .map(|p| match p {
                PlotData::Point(pt) => pt.x,
                _ => 0.0,
            })
            .collect();
        let y: Vec<f64> = data
            .iter()
            .map(|p| match p {
                PlotData::Point(pt) => pt.y,
                _ => 0.0,
            })
            .collect();
        let mut output = Vec::with_capacity(max_points);
        decimate_ilttb_arrays_par_into(&x, &y, max_points, &mut output, gaps);
        return output;
    }

    decimate_lttb_generic(
        data,
        max_points,
        |p| match p {
            PlotData::Point(pt) => pt.x,
            _ => 0.0,
        },
        |p| match p {
            PlotData::Point(pt) => pt.y,
            _ => 0.0,
        },
        |p| p.clone(),
        gaps,
    )
}

pub fn decimate_ilttb_arrays_par(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_ilttb_arrays_par_into(x, y, max_points, &mut output, gaps);
    output
}

pub fn decimate_ilttb_arrays_par_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
) {
    if x.is_empty() || y.is_empty() || x.len() != y.len() {
        return;
    }

    if x.len() <= max_points || max_points < 3 {
        output.extend(x.iter().zip(y.iter()).map(|(x_val, y_val)| {
            PlotData::Point(PlotPoint {
                x: *x_val,
                y: *y_val,
                color_op: ColorOp::None,
            })
        }));
        return;
    }

    let real_range = x[x.len() - 1] - x[0];
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(x[x.len() - 1] as i64) - g.to_logical(x[0] as i64)) as f64
    } else {
        real_range
    };

    let target_bucket_count = max_points - 2;
    let stable_bin_size = calculate_stable_bin_size(logical_range, target_bucket_count);
    let avg_items_per_bin = (x.len() as f64 * (stable_bin_size / logical_range)).ceil() as usize;
    let bin_size = avg_items_per_bin.max(1);

    // Pre-calculate buckets that respect gaps
    let buckets = calculate_gap_aware_buckets(x, gaps, bin_size);
    let n_buckets = buckets.len();

    if n_buckets == 0 {
        return;
    }

    let averages: Vec<(f64, f64)> = buckets
        .par_iter()
        .map(|range| {
            let start = range.start;
            let end = range.end;
            let count = (end - start) as f64;

            if count == 0.0 {
                return (0.0, 0.0);
            }

            // Calculate sum of LOGICAL X
            let sum_x = if let Some(g) = gaps {
                let mut cursor = g.cursor();
                let mut s = 0.0;
                for &val in &x[start..end] {
                    s += cursor.to_logical(val as i64) as f64;
                }
                s
            } else {
                crate::simd::sum_f64(&x[start..end])
            };

            let sum_y = crate::simd::sum_f64(&y[start..end]);
            (sum_x / count, sum_y / count)
        })
        .collect();

    let mut sampled: Vec<PlotPoint> = buckets
        .par_iter()
        .enumerate()
        .map(|(i, range)| {
            let start = range.start;
            let end = range.end;

            let (a_x, a_y) = if i > 0 {
                averages[i - 1]
            } else {
                if let Some(g) = gaps {
                    (g.to_logical(x[0] as i64) as f64, y[0])
                } else {
                    (x[0], y[0])
                }
            };

            let (c_x, c_y) = if i < n_buckets - 1 {
                averages[i + 1]
            } else {
                if let Some(g) = gaps {
                    (g.to_logical(x[x.len() - 1] as i64) as f64, y[y.len() - 1])
                } else {
                    (x[x.len() - 1], y[y.len() - 1])
                }
            };

            let best_idx = if let Some(g) = gaps {
                let mut cursor = g.cursor();
                let local = find_max_area_index_gap_aware(
                    &x[start..end],
                    &y[start..end],
                    a_x,
                    a_y,
                    c_x,
                    c_y,
                    &mut cursor,
                );
                start + local
            } else {
                let local = crate::simd::find_max_area_index(
                    &x[start..end],
                    &y[start..end],
                    a_x,
                    a_y,
                    c_x,
                    c_y,
                );
                start + local
            };

            PlotPoint {
                x: x[best_idx],
                y: y[best_idx],
                color_op: ColorOp::None,
            }
        })
        .collect();

    output.push(PlotData::Point(PlotPoint {
        x: x[0],
        y: y[0],
        color_op: ColorOp::None,
    }));
    for pt in sampled.drain(..) {
        output.push(PlotData::Point(pt));
    }
    output.push(PlotData::Point(PlotPoint {
        x: x[x.len() - 1],
        y: y[y.len() - 1],
        color_op: ColorOp::None,
    }));
}

fn find_max_area_index_gap_aware(
    x: &[f64],
    y: &[f64],
    ax_logical: f64,
    ay: f64,
    cx_logical: f64,
    cy: f64,
    cursor: &mut MappingCursor,
) -> usize {
    let len = x.len().min(y.len());
    if len == 0 {
        return 0;
    }

    let c1 = ay - cy;
    let c2 = cx_logical - ax_logical;
    let c3 = ax_logical * cy - cx_logical * ay;

    let mut max_area = -1.0;
    let mut best_idx = 0;

    for (i, (&vx, &vy)) in x.iter().zip(y.iter()).enumerate() {
        if vx.is_nan() || vy.is_nan() { continue; }
        
        let vx_logical = cursor.to_logical(vx as i64) as f64;
        
        let area = (vx_logical * c1 + vy * c2 + c3).abs();
        if area > max_area {
            max_area = area;
            best_idx = i;
        }
    }
    best_idx
}

pub fn decimate_min_max_generic<T, FX, FY, FC>(
    data: &[T],
    max_points: usize,
    get_x: FX,
    get_y: FY,
    create_point: FC,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData>
where
    FX: Fn(&T) -> f64,
    FY: Fn(&T) -> f64,
    FC: Fn(&T) -> PlotData,
{
    if data.len() <= max_points {
        return data.iter().map(create_point).collect();
    }

    let real_range = get_x(&data[data.len() - 1]) - get_x(&data[0]);
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(get_x(&data[data.len() - 1]) as i64) - g.to_logical(get_x(&data[0]) as i64))
            as f64
    } else {
        real_range
    };

    let stable_bin_size = calculate_stable_bin_size(logical_range, (max_points / 2).max(1));
    let avg_items_per_bin = (data.len() as f64 * (stable_bin_size / logical_range)).ceil() as usize;
    let bin_size = avg_items_per_bin.max(1);

    let mut aggregated = Vec::with_capacity(max_points);

    for chunk in data.chunks(bin_size) {
        if chunk.is_empty() {
            continue;
        }

        let mut min_idx = 0;
        let mut max_idx = 0;
        let mut min_y = get_y(&chunk[0]);
        let mut max_y = min_y;

        let start_search_idx = if min_y.is_nan() {
            let mut found = false;
            let mut first_non_nan = 0;
            for (i, item) in chunk.iter().enumerate().skip(1) {
                let y = get_y(item);
                if !y.is_nan() {
                    min_y = y;
                    max_y = y;
                    min_idx = i;
                    max_idx = i;
                    first_non_nan = i;
                    found = true;
                    break;
                }
            }
            if found {
                first_non_nan + 1
            } else {
                chunk.len()
            }
        } else {
            1
        };

        for (i, item) in chunk.iter().enumerate().skip(start_search_idx) {
            let y = get_y(item);
            if y.is_nan() {
                continue;
            }
            if y < min_y {
                min_y = y;
                min_idx = i;
            }
            if y > max_y {
                max_y = y;
                max_idx = i;
            }
        }

        if min_idx == max_idx {
            aggregated.push(create_point(&chunk[min_idx]));
        } else {
            let p1 = &chunk[min_idx];
            let p2 = &chunk[max_idx];

            if get_x(p1) <= get_x(p2) {
                aggregated.push(create_point(p1));
                aggregated.push(create_point(p2));
            } else {
                aggregated.push(create_point(p2));
                aggregated.push(create_point(p1));
            }
        }
    }

    if aggregated.len() > max_points {
        aggregated.truncate(max_points);
    }

    aggregated
}

pub fn decimate_m4_generic<T, FX, FY, FC>(
    data: &[T],
    max_points: usize,
    get_x: FX,
    get_y: FY,
    create_point: FC,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData>
where
    FX: Fn(&T) -> f64,
    FY: Fn(&T) -> f64,
    FC: Fn(&T) -> PlotData,
{
    if data.len() <= max_points {
        return data.iter().map(create_point).collect();
    }

    let real_range = get_x(&data[data.len() - 1]) - get_x(&data[0]);
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(get_x(&data[data.len() - 1]) as i64) - g.to_logical(get_x(&data[0]) as i64))
            as f64
    } else {
        real_range
    };

    let stable_bin_size = calculate_stable_bin_size(logical_range, (max_points / 4).max(1));
    let avg_items_per_bin = (data.len() as f64 * (stable_bin_size / logical_range)).ceil() as usize;
    let bin_size = avg_items_per_bin.max(1);

    let mut aggregated = Vec::with_capacity(max_points);

    for chunk in data.chunks(bin_size) {
        if chunk.is_empty() {
            continue;
        }

        let first_idx = 0;
        let last_idx = chunk.len() - 1;
        let mut min_idx = 0;
        let mut max_idx = 0;
        let mut min_y = get_y(&chunk[0]);
        let mut max_y = min_y;

        let start_search_idx = if min_y.is_nan() {
            let mut found = false;
            let mut first_non_nan = 0;
            for (i, item) in chunk.iter().enumerate().skip(1) {
                let y = get_y(item);
                if !y.is_nan() {
                    min_y = y;
                    max_y = y;
                    min_idx = i;
                    max_idx = i;
                    first_non_nan = i;
                    found = true;
                    break;
                }
            }
            if found {
                first_non_nan + 1
            } else {
                chunk.len()
            }
        } else {
            1
        };

        for (i, item) in chunk.iter().enumerate().skip(start_search_idx) {
            let y = get_y(item);
            if y.is_nan() {
                continue;
            }
            if y < min_y {
                min_y = y;
                min_idx = i;
            }
            if y > max_y {
                max_y = y;
                max_idx = i;
            }
        }

        let mut indices = vec![first_idx, min_idx, max_idx, last_idx];
        indices.sort_unstable();
        indices.dedup();

        for idx in indices {
            aggregated.push(create_point(&chunk[idx]));
        }
    }

    if aggregated.len() > max_points {
        aggregated.truncate(max_points);
    }

    aggregated
}

pub fn decimate_lttb_generic<T, FX, FY, FC>(
    data: &[T],
    max_points: usize,
    get_x: FX,
    get_y: FY,
    create_point: FC,
    gaps: Option<&GapIndex>,
) -> Vec<PlotData>
where
    FX: Fn(&T) -> f64,
    FY: Fn(&T) -> f64,
    FC: Fn(&T) -> PlotData,
{
    if data.len() <= max_points || max_points < 3 {
        return data.iter().map(create_point).collect();
    }

    let real_range = get_x(&data[data.len() - 1]) - get_x(&data[0]);
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(get_x(&data[data.len() - 1]) as i64) - g.to_logical(get_x(&data[0]) as i64))
            as f64
    } else {
        real_range
    };

    let target_bucket_count = max_points - 2;
    let stable_bin_size = calculate_stable_bin_size(logical_range, target_bucket_count);
    let avg_items_per_bin = (data.len() as f64 * (stable_bin_size / logical_range)).ceil() as usize;
    let bin_size = avg_items_per_bin.max(1);

    // This is generic, so we don't have calculate_gap_aware_buckets_data for generic T
    // but we can implement a local version or use indices.
    let mut current_start = 1;
    let n = data.len();
    let mut buckets = Vec::new();

    if let Some(gi) = gaps {
        let segments = gi.segments();
        let mut seg_idx = 0;

        while current_start < n - 1 {
            let next_target = (current_start + bin_size).min(n - 1);

            while seg_idx < segments.len()
                && segments[seg_idx].end_real <= get_x(&data[current_start]) as i64
            {
                seg_idx += 1;
            }

            if seg_idx < segments.len()
                && segments[seg_idx].start_real < get_x(&data[next_target - 1]) as i64
            {
                let gap_start_real = segments[seg_idx].start_real;
                let mut split_idx = current_start;
                for j in current_start..next_target {
                    if (get_x(&data[j]) as i64) >= gap_start_real {
                        break;
                    }
                    split_idx = j + 1;
                }

                if split_idx > current_start {
                    buckets.push(current_start..split_idx);
                    current_start = split_idx;
                } else {
                    let gap_end_real = segments[seg_idx].end_real;
                    let mut skip_idx = current_start;
                    for j in current_start..n - 1 {
                        if (get_x(&data[j]) as i64) >= gap_end_real {
                            break;
                        }
                        skip_idx = j + 1;
                    }
                    current_start = skip_idx;
                    seg_idx += 1;
                }
            } else {
                buckets.push(current_start..next_target);
                current_start = next_target;
            }
        }
    } else {
        for i in (1..n - 1).step_by(bin_size) {
            buckets.push(i..(i + bin_size).min(n - 1));
        }
    }

    if buckets.is_empty() {
        return vec![create_point(&data[0]), create_point(&data[data.len() - 1])];
    }

    let mut sampled = Vec::with_capacity(max_points);
    sampled.push(create_point(&data[0]));

    let averages: Vec<(f64, f64)> = buckets
        .iter()
        .map(|range| {
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let count = range.len() as f64;
            if let Some(g) = gaps {
                let mut cursor = g.cursor();
                for j in range.clone() {
                    sum_x += cursor.to_logical(get_x(&data[j]) as i64) as f64;
                    sum_y += get_y(&data[j]);
                }
            } else {
                for j in range.clone() {
                    sum_x += get_x(&data[j]);
                    sum_y += get_y(&data[j]);
                }
            }
            (sum_x / count, sum_y / count)
        })
        .collect();

    let mut a_idx = 0;
    let n_buckets = buckets.len();

    for i in 0..n_buckets {
        let range = &buckets[i];
        let (c_x, c_y) = if i < n_buckets - 1 {
            averages[i + 1]
        } else {
            if let Some(g) = gaps {
                (
                    g.to_logical(get_x(&data[n - 1]) as i64) as f64,
                    get_y(&data[n - 1]),
                )
            } else {
                (get_x(&data[n - 1]), get_y(&data[n - 1]))
            }
        };

        let p_a_x = if let Some(g) = gaps {
            g.to_logical(get_x(&data[a_idx]) as i64) as f64
        } else {
            get_x(&data[a_idx])
        };
        let p_a_y = get_y(&data[a_idx]);

        let mut max_area = -1.0;
        let mut next_a_idx = range.start;

        let mut cursor = gaps.map(|g| g.cursor());

        for j in range.clone() {
            let p_b_x = if let Some(ref mut c) = cursor {
                c.to_logical(get_x(&data[j]) as i64) as f64
            } else {
                get_x(&data[j])
            };
            let p_b_y = get_y(&data[j]);

            let area =
                (p_a_x * (p_b_y - c_y) + p_b_x * (c_y - p_a_y) + c_x * (p_a_y - p_b_y)).abs();

            if area > max_area {
                max_area = area;
                next_a_idx = j;
            }
        }

        a_idx = next_a_idx;
        sampled.push(create_point(&data[a_idx]));
    }

    sampled.push(create_point(&data[n - 1]));
    sampled
}

pub fn aggregate_chunk(chunk: &[PlotData]) -> Option<PlotData> {
    if chunk.is_empty() {
        return None;
    }

    if let PlotData::Point(_) = chunk[0] {
        let mut sum_y = 0.0;
        let mut sum_x = 0.0;
        let len = chunk.len() as f64;

        for p in chunk {
            if let PlotData::Point(pt) = p {
                sum_x += pt.x;
                sum_y += pt.y;
            }
        }

        Some(PlotData::Point(PlotPoint {
            x: sum_x / len,
            y: sum_y / len,
            color_op: ColorOp::None,
        }))
    } else if let PlotData::Ohlcv(_) = chunk[0] {
        let mut open = 0.0;
        let mut close = 0.0;
        let mut high = f64::NEG_INFINITY;
        let mut low = f64::INFINITY;
        let mut volume = 0.0;
        let mut first_time = 0.0;
        let mut last_time_end = 0.0;

        for (i, p) in chunk.iter().enumerate() {
            if let PlotData::Ohlcv(o) = p {
                if i == 0 {
                    open = o.open;
                    first_time = o.time;
                }
                if i == chunk.len() - 1 {
                    close = o.close;
                    last_time_end = o.time + o.span;
                }
                high = high.max(o.high);
                low = low.min(o.low);
                volume += o.volume;
            }
        }

        Some(PlotData::Ohlcv(Ohlcv {
            time: first_time,
            span: last_time_end - first_time,
            open,
            high,
            low,
            close,
            volume,
        }))
    } else {
        None
    }
}
