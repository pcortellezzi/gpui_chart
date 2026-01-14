use crate::data_types::{ColorOp, Ohlcv, PlotData, PlotPoint};
use crate::gaps::GapIndex;

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

pub fn decimate_m4_arrays_par_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
) {
    output.clear();
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

    // Process chunks in parallel
    let mut chunks: Vec<Vec<PlotData>> = x
        .par_chunks(bin_size)
        .zip(y.par_chunks(bin_size))
        .map(|(x_chunk, y_chunk)| {
            if x_chunk.is_empty() {
                return vec![];
            }

            let first_idx = 0;
            let last_idx = x_chunk.len() - 1;

            let mut min_idx = 0;
            let mut max_idx = 0;
            let mut min_y = y_chunk[0];
            let mut max_y = min_y;

            // Handle NaN start
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
                    x_chunk.len()
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

            let mut indices = vec![first_idx, min_idx, max_idx, last_idx];
            indices.sort_unstable();
            indices.dedup();

            let mut result = Vec::with_capacity(4);
            for idx in indices {
                result.push(PlotData::Point(PlotPoint {
                    x: x_chunk[idx],
                    y: y_chunk[idx],
                    color_op: ColorOp::None,
                }));
            }
            result
        })
        .collect();

    // Flatten results
    for chunk in chunks.drain(..) {
        output.extend(chunk);
    }

    // Ensure strict limit (Safeguard)
    if output.len() > max_points {
        output.truncate(max_points);
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
    output.clear();
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

    let chunks: Vec<PlotData> = time
        .par_chunks(bin_size)
        .enumerate()
        .filter_map(|(chunk_idx, t_chunk)| {
            let start_idx = chunk_idx * bin_size;
            let end_idx = start_idx + t_chunk.len();

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

    if output.len() > max_points {
        output.truncate(max_points);
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

pub fn decimate_min_max_arrays_par_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
) {
    output.clear();
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

    let mut chunks: Vec<Vec<PlotData>> = x
        .par_chunks(bin_size)
        .zip(y.par_chunks(bin_size))
        .map(|(x_chunk, y_chunk)| {
            if x_chunk.is_empty() {
                return vec![];
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
                    x_chunk.len()
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

            let mut result = Vec::with_capacity(2);
            if min_idx == max_idx {
                result.push(PlotData::Point(PlotPoint {
                    x: x_chunk[min_idx],
                    y: y_chunk[min_idx],
                    color_op: ColorOp::None,
                }));
            } else {
                let p1_x = x_chunk[min_idx];
                let p1_y = y_chunk[min_idx];
                let p2_x = x_chunk[max_idx];
                let p2_y = y_chunk[max_idx];

                if p1_x <= p2_x {
                    result.push(PlotData::Point(PlotPoint {
                        x: p1_x,
                        y: p1_y,
                        color_op: ColorOp::None,
                    }));
                    result.push(PlotData::Point(PlotPoint {
                        x: p2_x,
                        y: p2_y,
                        color_op: ColorOp::None,
                    }));
                } else {
                    result.push(PlotData::Point(PlotPoint {
                        x: p2_x,
                        y: p2_y,
                        color_op: ColorOp::None,
                    }));
                    result.push(PlotData::Point(PlotPoint {
                        x: p1_x,
                        y: p1_y,
                        color_op: ColorOp::None,
                    }));
                }
            }
            result
        })
        .collect();

    for chunk in chunks.drain(..) {
        output.extend(chunk);
    }

    if output.len() > max_points {
        output.truncate(max_points);
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
    output.clear();
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
    if data.is_empty() {
        return vec![];
    }

    if let PlotData::Ohlcv(_) = data[0] {
        let target_bins = (max_points / 2).max(1);
        let bin_size = (data.len() as f64 / target_bins as f64).ceil() as usize;
        let mut aggregated = Vec::with_capacity(target_bins.max(1));
        for chunk in data.chunks(bin_size) {
            if let Some(agg) = aggregate_chunk(chunk) {
                aggregated.push(agg);
            }
        }
        return aggregated;
    }

    decimate_min_max_generic(
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

pub fn decimate_m4_slice(
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

    decimate_m4_generic(
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
    _gaps: Option<&GapIndex>,
) {
    output.clear();
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

    let target_bucket_count = max_points - 2;
    let bucket_size = (x.len() - 2) as f64 / target_bucket_count as f64;

    let averages: Vec<(f64, f64)> = (0..target_bucket_count)
        .into_par_iter()
        .map(|i| {
            let start = 1 + (i as f64 * bucket_size).floor() as usize;
            let end = (1 + ((i + 1) as f64 * bucket_size).floor() as usize).min(x.len() - 1);
            if start >= end {
                let idx = start.min(x.len() - 1);
                return (x[idx], y[idx]);
            }

            let count = (end - start) as f64;
            let sum_x = crate::simd::sum_f64(&x[start..end]);
            let sum_y = crate::simd::sum_f64(&y[start..end]);
            (sum_x / count, sum_y / count)
        })
        .collect();

    let mut sampled: Vec<PlotPoint> = (0..target_bucket_count)
        .into_par_iter()
        .map(|i| {
            let start = 1 + (i as f64 * bucket_size).floor() as usize;
            let end = (1 + ((i + 1) as f64 * bucket_size).floor() as usize).min(x.len() - 1);

            let (a_x, a_y) = if i > 0 { averages[i - 1] } else { (x[0], y[0]) };

            let (c_x, c_y) = if i < target_bucket_count - 1 {
                averages[i + 1]
            } else {
                (x[x.len() - 1], y[y.len() - 1])
            };

            let best_local_idx = crate::simd::find_max_area_index(
                &x[start..end],
                &y[start..end],
                a_x,
                a_y,
                c_x,
                c_y,
            );

            let best_idx = start + best_local_idx;

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
    _gaps: Option<&GapIndex>,
) -> Vec<PlotData>
where
    FX: Fn(&T) -> f64,
    FY: Fn(&T) -> f64,
    FC: Fn(&T) -> PlotData,
{
    if data.len() <= max_points || max_points < 3 {
        return data.iter().map(create_point).collect();
    }

    let mut sampled = Vec::with_capacity(max_points);

    let bucket_size = (data.len() - 2) as f64 / (max_points - 2) as f64;

    let mut a_idx = 0;
    sampled.push(create_point(&data[a_idx]));

    for i in 0..(max_points - 2) {
        let bucket_start = 1 + (i as f64 * bucket_size).floor() as usize;
        let bucket_end = 1 + ((i + 1) as f64 * bucket_size).floor() as usize;

        let next_bucket_start = bucket_end;
        let next_bucket_end =
            (1 + ((i + 2) as f64 * bucket_size).floor() as usize).min(data.len() - 1);

        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        let mut avg_count = 0;

        for j in next_bucket_start..next_bucket_end {
            avg_x += get_x(&data[j]);
            avg_y += get_y(&data[j]);
            avg_count += 1;
        }

        if avg_count > 0 {
            avg_x /= avg_count as f64;
            avg_y /= avg_count as f64;
        } else {
            avg_x = get_x(&data[next_bucket_start.min(data.len() - 1)]);
            avg_y = get_y(&data[next_bucket_start.min(data.len() - 1)]);
        }

        let p_a_x = get_x(&data[a_idx]);
        let p_a_y = get_y(&data[a_idx]);

        let mut max_area = -1.0;
        let mut next_a_idx = bucket_start;

        for j in bucket_start..bucket_end {
            let p_b_x = get_x(&data[j]);
            let p_b_y = get_y(&data[j]);

            let area =
                (p_a_x * (p_b_y - avg_y) + p_b_x * (avg_y - p_a_y) + avg_x * (p_a_y - p_b_y)).abs();

            if area > max_area {
                max_area = area;
                next_a_idx = j;
            }
        }

        a_idx = next_a_idx;
        sampled.push(create_point(&data[a_idx]));
    }

    sampled.push(create_point(&data[data.len() - 1]));

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
