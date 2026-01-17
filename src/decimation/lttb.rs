use crate::data_types::{ColorOp, PlotData, PlotPoint};
use crate::gaps::{GapIndex, MappingCursor};
use rayon::prelude::*;
use super::min_max::decimate_min_max_slice;

pub fn decimate_lttb_arrays(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_lttb_arrays_into(x, y, max_points, &mut output, gaps, reference_logical_range);
    output
}

pub fn decimate_lttb_arrays_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
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

    // Use stable time-based bucketing
    // target_bucket_count is approx max_points - 2.
    // We request buckets based on max_points. The exact number might vary slightly.
    let (_, buckets) = super::bucketing::calculate_stable_buckets(x, gaps, max_points.saturating_sub(2).max(1), 1, reference_logical_range);
    let n_buckets = buckets.len();

    if n_buckets == 0 {
        return;
    }

    // Parallel calculation of averages for LTTB (similar to ILTTB)
    let averages: Vec<(f64, f64)> = buckets
        .par_iter()
        .map(|range| {
            let start = range.start;
            let end = range.end;
            let count = (end - start) as f64;
            if count == 0.0 { return (0.0, 0.0); }

            let sum_x = if let Some(g) = gaps {
                let mut cursor = g.cursor();
                let mut s = 0.0;
                for &val in &x[start..end] { s += cursor.to_logical(val as i64) as f64; }
                s
            } else {
                crate::simd::sum_f64(&x[start..end])
            };
            let sum_y = crate::simd::sum_f64(&y[start..end]);
            (sum_x / count, sum_y / count)
        })
        .collect();

    let mut a_idx = 0;
    output.push(PlotData::Point(PlotPoint {
        x: x[0],
        y: y[0],
        color_op: ColorOp::None,
    }));

    for i in 0..n_buckets {
        let range = &buckets[i];
        let (c_x, c_y) = if i < n_buckets - 1 {
            averages[i + 1]
        } else {
            if let Some(g) = gaps {
                (g.to_logical(x[x.len() - 1] as i64) as f64, y[y.len() - 1])
            } else {
                (x[x.len() - 1], y[y.len() - 1])
            }
        };

        let p_a_x = if let Some(g) = gaps {
            g.to_logical(x[a_idx] as i64) as f64
        } else {
            x[a_idx]
        };
        let p_a_y = y[a_idx];

        let mut max_area = -1.0;
        let mut next_a_idx = range.start;

        let mut cursor = gaps.map(|g| g.cursor());

        for j in range.clone() {
            let p_b_x = if let Some(ref mut c) = cursor {
                c.to_logical(x[j] as i64) as f64
            } else {
                x[j]
            };
            let p_b_y = y[j];

            let area = (p_a_x * (p_b_y - c_y) + p_b_x * (c_y - p_a_y) + c_x * (p_a_y - p_b_y)).abs();

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

pub fn decimate_lttb_slice(
    data: &[PlotData],
    max_points: usize,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) -> Vec<PlotData> {
    if data.is_empty() {
        return vec![];
    }
    if let PlotData::Ohlcv(_) = data[0] {
        return decimate_min_max_slice(data, max_points, gaps, reference_logical_range);
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
        decimate_ilttb_arrays_par_into(&x, &y, max_points, &mut output, gaps, reference_logical_range);
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
        reference_logical_range,
    )
}

pub fn decimate_ilttb_arrays_par(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_ilttb_arrays_par_into(x, y, max_points, &mut output, gaps, reference_logical_range);
    output
}

pub fn decimate_ilttb_arrays_par_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
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

    // Use stable time-based bucketing
    let (_, buckets) = super::bucketing::calculate_stable_buckets(x, gaps, max_points.saturating_sub(2).max(1), 1, reference_logical_range);
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
            if count == 0.0 { return (0.0, 0.0); }

            let sum_x = if let Some(g) = gaps {
                let mut cursor = g.cursor();
                let mut s = 0.0;
                for &val in &x[start..end] { s += cursor.to_logical(val as i64) as f64; }
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

            let mut cursor = gaps.map(|g| g.cursor());
            let local = find_max_area_index_gap_aware(
                &x[start..end],
                &y[start..end],
                a_x,
                a_y,
                c_x,
                c_y,
                &mut cursor,
            );
            let best_idx = start + local;

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
    cursor: &mut Option<MappingCursor>,
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
        
        let vx_logical = if let Some(c) = cursor {
            c.to_logical(vx as i64) as f64
        } else {
            vx
        };
        
        let area = (vx_logical * c1 + vy * c2 + c3).abs();
        if area > max_area {
            max_area = area;
            best_idx = i;
        }
    }
    best_idx
}

pub fn decimate_lttb_generic<T, FX, FY, FC>(
    data: &[T],
    max_points: usize,
    get_x: FX,
    get_y: FY,
    create_point: FC,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) -> Vec<PlotData>
where
    FX: Fn(&T) -> f64,
    FY: Fn(&T) -> f64,
    FC: Fn(&T) -> PlotData,
{
    let n = data.len();
    if n <= max_points || max_points < 3 {
        return data.iter().map(create_point).collect();
    }

    // Use stable time-based bucketing
    // Inner range bucketing: we skip first and last point.
    let inner_len = n.saturating_sub(2);
    if inner_len == 0 {
         return vec![create_point(&data[0]), create_point(&data[n - 1])];
    }
    
    // We want buckets on the inner range.
    // The "x" values are at data[1], data[2]...
    // The offsets are 1..n-1
    
    // We can just use the generic helper on the sub-slice logic.
    // But generic helper takes "n" and "get_x_at(i)".
    // So we pass inner_len, and get_x_at(i) maps to data[i+1].
    
    let (_, buckets_relative) = super::bucketing::calculate_stable_buckets_generic(
        inner_len,
        |i| get_x(&data[i + 1]),
        gaps,
        max_points.saturating_sub(2).max(1),
        1,
        reference_logical_range,
    );

    // Map buckets back to absolute indices
    let buckets: Vec<std::ops::Range<usize>> = buckets_relative
        .into_iter()
        .map(|r| (r.start + 1)..(r.end + 1))
        .collect();

    if buckets.is_empty() {
        return vec![create_point(&data[0]), create_point(&data[n - 1])];
    }

    let averages: Vec<(f64, f64)> = buckets.iter().map(|range| {
        let count = range.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut cursor = gaps.map(|g| g.cursor());
        for j in range.clone() {
            let val_x = get_x(&data[j]);
            sum_x += cursor.as_mut().map(|c| c.to_logical(val_x as i64) as f64).unwrap_or(val_x);
            sum_y += get_y(&data[j]);
        }
        (sum_x / count, sum_y / count)
    }).collect();

    let mut sampled = Vec::with_capacity(buckets.len() + 2);
    sampled.push(create_point(&data[0]));

    let mut a_idx = 0;
    for (i, range) in buckets.iter().enumerate() {
        let (avg_x, avg_y) = if i + 1 < averages.len() {
            averages[i + 1]
        } else {
            let last_x = get_x(&data[n - 1]);
            let last_x_log = gaps.map(|g| g.to_logical(last_x as i64) as f64).unwrap_or(last_x);
            (last_x_log, get_y(&data[n - 1]))
        };

        let p_a_x = get_x(&data[a_idx]);
        let p_a_x_log = gaps.map(|g| g.to_logical(p_a_x as i64) as f64).unwrap_or(p_a_x);
        let p_a_y = get_y(&data[a_idx]);

        let mut max_area = -1.0;
        let mut next_a_idx = range.start;
        let mut cursor = gaps.map(|g| g.cursor());

        for j in range.clone() {
            let p_b_x = get_x(&data[j]);
            let p_b_x_log = cursor.as_mut().map(|c| c.to_logical(p_b_x as i64) as f64).unwrap_or(p_b_x);
            let p_b_y = get_y(&data[j]);
            let area = (p_a_x_log * (p_b_y - avg_y) + p_b_x_log * (avg_y - p_a_y) + avg_x * (p_a_y - p_b_y)).abs();
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
