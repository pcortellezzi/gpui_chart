use crate::data_types::{ColorOp, PlotData, PlotPoint};
use crate::gaps::GapIndex;
use rayon::prelude::*;
use super::common::{find_extrema_indices_generic, get_data_y};

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

    // Use stable time-based bucketing to prevent jitter
    let (_stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets(x, gaps, max_points, 4);

    // Process buckets in parallel, returning fixed-size arrays to avoid allocations.
    let chunks: Vec<([PlotPoint; 4], usize)> = buckets
        .into_par_iter()
        .map(|range| {
            let x_chunk = &x[range.start..range.end];
            let y_chunk = &y[range.start..range.end];
            aggregate_m4_bucket_to_array(x_chunk, y_chunk)
        })
        .collect();

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
        use crate::decimation::ohlcv::decimate_ohlcv_slice_into; // Resolve circular dep
        decimate_ohlcv_slice_into(data, max_points, output, gaps);
        return;
    }

    if data.len() <= max_points {
        output.extend_from_slice(data);
        return;
    }

    let (_stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets_data(data, gaps, max_points, 4);

    let chunks: Vec<([PlotData; 4], usize)> = buckets
        .into_par_iter()
        .map(|range| {
            let chunk = &data[range.start..range.end];
            aggregate_m4_bucket_generic(chunk)
        })
        .collect();

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

    let last_idx = n - 1;
    let (min_idx, max_idx) = find_extrema_indices_generic(chunk, &get_data_y);

    let mut indices = [0, min_idx, max_idx, last_idx];
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
    let n = data.len();
    if n <= max_points {
        return data.iter().map(create_point).collect();
    }

    let (_stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets_generic(
        n,
        |i| get_x(&data[i]),
        gaps,
        max_points,
        4,
    );

    let mut aggregated = Vec::with_capacity(max_points);

    for range in buckets {
        let chunk = &data[range];
        if chunk.is_empty() {
            continue;
        }

        let first_idx = 0;
        let last_idx = chunk.len() - 1;
        let (min_idx, max_idx) = find_extrema_indices_generic(chunk, &get_y);

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
