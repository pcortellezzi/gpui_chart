use crate::data_types::{ColorOp, PlotData, PlotPoint};
use crate::gaps::GapIndex;
use rayon::prelude::*;
use super::common::{find_extrema_indices_generic, get_data_x, get_data_y};

pub fn decimate_min_max_arrays_par(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_min_max_arrays_par_into(x, y, max_points, &mut output, gaps, reference_logical_range);
    output
}

#[inline(always)]
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

    let (min_idx, max_idx) = super::common::find_extrema_indices_f64(y_chunk);
    if y_chunk[min_idx].is_nan() {
        return ([PlotPoint::default(); 2], 0);
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
    reference_logical_range: Option<f64>,
) {
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

    // Use stable time-based bucketing
    let (stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets(x, gaps, max_points, 2, reference_logical_range);

    let chunks: Vec<([PlotPoint; 2], usize)> = buckets
        .into_par_iter()
        .map(|range| {
            let x_chunk = &x[range.start..range.end];
            let y_chunk = &y[range.start..range.end];
            let (mut pts, n) = aggregate_min_max_bucket_to_array(x_chunk, y_chunk);
            for i in 0..n {
                pts[i].x = super::common::snap_to_grid(pts[i].x, stable_bin_size, gaps);
            }
            (pts, n)
        })
        .collect();

    for (pts, n) in chunks {
        for i in 0..n {
            output.push(PlotData::Point(pts[i]));
        }
    }
}

pub fn decimate_min_max_slice(
    data: &[PlotData],
    max_points: usize,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_min_max_slice_into(data, max_points, &mut output, gaps, reference_logical_range);
    output
}

pub fn decimate_min_max_slice_into(
    data: &[PlotData],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) {
    if data.is_empty() {
        return;
    }

    if let PlotData::Ohlcv(_) = data[0] {
        use crate::decimation::ohlcv::decimate_ohlcv_slice_into; // Resolve circular dependency or move ohlcv call
        decimate_ohlcv_slice_into(data, max_points, output, gaps, reference_logical_range);
        return;
    }

    if data.len() <= max_points {
        output.extend_from_slice(data);
        return;
    }

    let (_stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets_data(data, gaps, max_points, 2, reference_logical_range);

    // Process buckets in parallel
    let chunks: Vec<([PlotData; 2], usize)> = buckets
        .into_par_iter()
        .map(|range| {
            let chunk = &data[range.start..range.end];
            aggregate_min_max_bucket_generic(chunk)
        })
        .collect();

    for (pts, n) in chunks {
        for i in 0..n {
            output.push(pts[i].clone());
        }
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

    let (min_idx, max_idx) = find_extrema_indices_generic(chunk, &get_data_y);

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

pub fn decimate_min_max_generic<T, FX, FY, FC>(
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
    if n <= max_points {
        return data.iter().map(create_point).collect();
    }

    let (_stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets_generic(
        n,
        |i| get_x(&data[i]),
        gaps,
        max_points,
        2,
        reference_logical_range,
    );

    let mut aggregated = Vec::with_capacity(max_points);

    for range in buckets {
        let chunk = &data[range];
        if chunk.is_empty() {
            continue;
        }

        let (min_idx, max_idx) = find_extrema_indices_generic(chunk, &get_y);

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

    aggregated
}
