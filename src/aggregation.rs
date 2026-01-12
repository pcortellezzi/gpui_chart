use crate::data_types::{ColorOp, Ohlcv, PlotData, PlotPoint};

use rayon::prelude::*;

/// Decimates parallel arrays (Structure of Arrays) using M4 and Rayon.
pub fn decimate_m4_arrays_par(
    x: &[f64],
    y: &[f64],
    max_points: usize,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_m4_arrays_par_into(x, y, max_points, &mut output);
    output
}

/// Decimates parallel arrays (Structure of Arrays) using M4 and Rayon into an existing buffer.
/// 
/// This method is the core of the "Zero-Alloc" rendering pipeline. By reusing the `output`
/// vector, we avoid repeated allocations during the render loop.
///
/// **Algorithm**: M4 (Min, Max, First, Last)
/// **Parallelism**: Rayon (Chunk-based)
pub fn decimate_m4_arrays_par_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
) {
    output.clear();
    if x.is_empty() || y.is_empty() || x.len() != y.len() {
        return;
    }
    
    if x.len() <= max_points {
        output.extend(x.iter().zip(y.iter()).map(|(x_val, y_val)| {
            PlotData::Point(PlotPoint { x: *x_val, y: *y_val, color_op: ColorOp::None })
        }));
        return;
    }

    // M4 targets 4 points per bin
    let target_bins = max_points / 4;
    let bin_size = (x.len() as f64 / target_bins.max(1) as f64).ceil() as usize;
    
    // Process chunks in parallel
    let mut chunks: Vec<Vec<PlotData>> = x.par_chunks(bin_size)
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
                if found { idx + 1 } else { x_chunk.len() }
            } else {
                1
            };

            for (i, val) in y_chunk.iter().enumerate().skip(start_search) {
                if val.is_nan() { continue; }
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
    
    // Ensure strict limit
    if output.len() > max_points {
        output.truncate(max_points);
    }
}

/// Decimates parallel arrays for OHLCV data.
pub fn decimate_ohlcv_arrays_par(
    time: &[f64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    max_points: usize,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_ohlcv_arrays_par_into(time, open, high, low, close, max_points, &mut output);
    output
}

/// Decimates parallel arrays for OHLCV data into an existing buffer.
pub fn decimate_ohlcv_arrays_par_into(
    time: &[f64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
) {
    output.clear();
    if time.is_empty() {
        return;
    }
    
    // If we have fewer points than the target, return 1:1
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

    // Target 1 point per bin
    let target_bins = max_points;
    let bin_size = (time.len() as f64 / target_bins.max(1) as f64).ceil() as usize;
    
    // Process chunks in parallel
    let mut chunks: Vec<Option<PlotData>> = time.par_chunks(bin_size)
        .enumerate()
        .map(|(chunk_idx, t_chunk)| {
            let start_idx = chunk_idx * bin_size;
            let end_idx = start_idx + t_chunk.len();
            
            if t_chunk.is_empty() {
                return None;
            }

            let o_chunk = &open[start_idx..end_idx.min(open.len())];
            let h_chunk = &high[start_idx..end_idx.min(high.len())];
            let l_chunk = &low[start_idx..end_idx.min(low.len())];
            let c_chunk = &close[start_idx..end_idx.min(close.len())];

            if o_chunk.is_empty() { return None; }

            let mut agg_open = f64::NAN;
            let mut agg_close = f64::NAN;
            let mut agg_high = f64::NEG_INFINITY;
            let mut agg_low = f64::INFINITY;
            let first_time = t_chunk[0];
            let last_time = t_chunk[t_chunk.len() - 1];

            // Find first valid open
            for &v in o_chunk {
                if !v.is_nan() {
                    agg_open = v;
                    break;
                }
            }
            if agg_open.is_nan() { agg_open = 0.0; }

            // Find last valid close
            for &v in c_chunk.iter().rev() {
                if !v.is_nan() {
                    agg_close = v;
                    break;
                }
            }
            if agg_close.is_nan() { agg_close = 0.0; }

            // Min/Max
            for &v in h_chunk {
                if !v.is_nan() {
                    agg_high = agg_high.max(v);
                }
            }
            if agg_high == f64::NEG_INFINITY { agg_high = 0.0; }

            for &v in l_chunk {
                if !v.is_nan() {
                    agg_low = agg_low.min(v);
                }
            }
            if agg_low == f64::INFINITY { agg_low = 0.0; }

            Some(PlotData::Ohlcv(Ohlcv {
                time: first_time,
                span: last_time - first_time,
                open: agg_open,
                high: agg_high,
                low: agg_low,
                close: agg_close,
                volume: 0.0,
            }))
        })
        .collect();

    for chunk in chunks.drain(..) {
        if let Some(c) = chunk {
            output.push(c);
        }
    }
    
    if output.len() > max_points {
        output.truncate(max_points);
    }
}

/// Decimates parallel arrays using Min/Max and Rayon.
pub fn decimate_min_max_arrays_par(
    x: &[f64],
    y: &[f64],
    max_points: usize,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_min_max_arrays_par_into(x, y, max_points, &mut output);
    output
}

/// Decimates parallel arrays using Min/Max and Rayon into an existing buffer.
pub fn decimate_min_max_arrays_par_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
) {
    output.clear();
    if x.is_empty() || y.is_empty() || x.len() != y.len() {
        return;
    }
    
    if x.len() <= max_points {
        output.extend(x.iter().zip(y.iter()).map(|(x_val, y_val)| {
            PlotData::Point(PlotPoint { x: *x_val, y: *y_val, color_op: ColorOp::None })
        }));
        return;
    }

    // Min/Max returns 2 points per bin.
    let target_bins = max_points / 2;
    let bin_size = (x.len() as f64 / target_bins.max(1) as f64).ceil() as usize;

    let mut chunks: Vec<Vec<PlotData>> = x.par_chunks(bin_size)
        .zip(y.par_chunks(bin_size))
        .map(|(x_chunk, y_chunk)| {
            if x_chunk.is_empty() {
                return vec![];
            }

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
                if found { idx + 1 } else { x_chunk.len() }
            } else {
                1
            };

            for (i, val) in y_chunk.iter().enumerate().skip(start_search) {
                if val.is_nan() { continue; }
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
                    result.push(PlotData::Point(PlotPoint { x: p1_x, y: p1_y, color_op: ColorOp::None }));
                    result.push(PlotData::Point(PlotPoint { x: p2_x, y: p2_y, color_op: ColorOp::None }));
                } else {
                    result.push(PlotData::Point(PlotPoint { x: p2_x, y: p2_y, color_op: ColorOp::None }));
                    result.push(PlotData::Point(PlotPoint { x: p1_x, y: p1_y, color_op: ColorOp::None }));
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

/// Decimates arrays using LTTB (Sequential but optimized for arrays).
pub fn decimate_lttb_arrays(
    x: &[f64],
    y: &[f64],
    max_points: usize,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_lttb_arrays_into(x, y, max_points, &mut output);
    output
}

/// Decimates arrays using LTTB (Sequential but optimized for arrays) into an existing buffer.
pub fn decimate_lttb_arrays_into(
    x: &[f64],
    y: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
) {
    output.clear();
    if x.len() <= max_points || max_points < 3 {
         output.extend(x.iter().zip(y.iter()).map(|(x_val, y_val)| {
            PlotData::Point(PlotPoint { x: *x_val, y: *y_val, color_op: ColorOp::None })
        }));
        return;
    }

    let bucket_size = (x.len() - 2) as f64 / (max_points - 2) as f64;
    
    // 1. First point
    let mut a_idx = 0;
    output.push(PlotData::Point(PlotPoint { x: x[0], y: y[0], color_op: ColorOp::None }));

    for i in 0..(max_points - 2) {
        let bucket_start = 1 + (i as f64 * bucket_size).floor() as usize;
        let bucket_end = 1 + ((i + 1) as f64 * bucket_size).floor() as usize;
        
        let next_bucket_start = bucket_end;
        let next_bucket_end = (1 + ((i + 2) as f64 * bucket_size).floor() as usize).min(x.len() - 1);

        // Average for next bucket
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
             // Fallback
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
            
            if p_b_x.is_nan() || p_b_y.is_nan() { continue; }

            let area = (p_a_x * (p_b_y - avg_y) + p_b_x * (avg_y - p_a_y) + avg_x * (p_a_y - p_b_y)).abs();
            
            if area > max_area {
                max_area = area;
                next_a_idx = j;
            }
        }
        
        a_idx = next_a_idx;
        output.push(PlotData::Point(PlotPoint { x: x[a_idx], y: y[a_idx], color_op: ColorOp::None }));
    }

    // Last point
    output.push(PlotData::Point(PlotPoint { x: x[x.len() - 1], y: y[y.len() - 1], color_op: ColorOp::None }));
}

/// Decimates a slice of PlotData using the Min/Max algorithm.
/// This is the standard entry point for most data sources.
pub fn decimate_min_max_slice(data: &[PlotData], max_points: usize) -> Vec<PlotData> {
    if data.is_empty() {
        return vec![];
    }

    // Special case for OHLCV: they already represent aggregates, 
    // so we typically want 1 point per bin (the aggregate of the bin).
    // To stay consistent with standard Point decimation (which returns 2 points per bin),
    // we use target_bins = max_points / 2.
    if let PlotData::Ohlcv(_) = data[0] {
        let target_bins = max_points / 2;
        let bin_size = (data.len() as f64 / target_bins.max(1) as f64).ceil() as usize;
        let mut aggregated = Vec::with_capacity(target_bins.max(1));
        for chunk in data.chunks(bin_size) {
            if let Some(agg) = aggregate_chunk(chunk) {
                aggregated.push(agg);
            }
        }
        return aggregated;
    }

    // For standard points, use the generic Min/Max (2 points per bin)
    decimate_min_max_generic(
        data,
        max_points,
        |p| match p { PlotData::Point(pt) => pt.x, _ => 0.0 },
        |p| match p { PlotData::Point(pt) => pt.y, _ => 0.0 },
        |p| p.clone()
    )
}

/// Decimates a slice of PlotData using the M4 algorithm.
/// M4 extracts 4 points per bin: First, Min, Max, Last.
pub fn decimate_m4_slice(data: &[PlotData], max_points: usize) -> Vec<PlotData> {
    if data.is_empty() {
        return vec![];
    }

    if let PlotData::Ohlcv(_) = data[0] {
        // For OHLCV, we keep the existing aggregation for now as it's already "M4-like"
        return decimate_min_max_slice(data, max_points);
    }

    decimate_m4_generic(
        data,
        max_points,
        |p| match p { PlotData::Point(pt) => pt.x, _ => 0.0 },
        |p| match p { PlotData::Point(pt) => pt.y, _ => 0.0 },
        |p| p.clone()
    )
}

/// Decimates a slice of PlotData using the LTTB algorithm.
/// LTTB preserves visual shape better than Min/Max for line charts.
pub fn decimate_lttb_slice(data: &[PlotData], max_points: usize) -> Vec<PlotData> {
    if data.is_empty() {
        return vec![];
    }
    // Fallback for OHLCV
    if let PlotData::Ohlcv(_) = data[0] {
        return decimate_min_max_slice(data, max_points);
    }

    decimate_lttb_generic(
        data,
        max_points,
        |p| match p { PlotData::Point(pt) => pt.x, _ => 0.0 },
        |p| match p { PlotData::Point(pt) => pt.y, _ => 0.0 },
        |p| p.clone()
    )
}

/// Generic Min/Max aggregator.
/// Takes any slice and returns a decimated Vec of PlotData.
pub fn decimate_min_max_generic<T, FX, FY, FC>(
    data: &[T],
    max_points: usize,
    get_x: FX,
    get_y: FY,
    create_point: FC,
) -> Vec<PlotData>
where
    FX: Fn(&T) -> f64,
    FY: Fn(&T) -> f64,
    FC: Fn(&T) -> PlotData,
{
    if data.len() <= max_points {
        return data.iter().map(create_point).collect();
    }

    // Min/Max returns 2 points per bin.
    let target_bins = max_points / 2;
    let bin_size = (data.len() as f64 / target_bins.max(1) as f64).ceil() as usize;
    let mut aggregated = Vec::with_capacity(max_points);

    for chunk in data.chunks(bin_size) {
        if chunk.is_empty() {
            continue;
        }

        let mut min_idx = 0;
        let mut max_idx = 0;
        let mut min_y = get_y(&chunk[0]);
        let mut max_y = min_y;

        // If the first point is NaN, try to find the first non-NaN point to initialize min/max
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
            if found { first_non_nan + 1 } else { chunk.len() }
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

    // Ensure we respect max_points exactly
    if aggregated.len() > max_points {
        aggregated.truncate(max_points);
    }

    aggregated
}

/// Generic M4 aggregator.
/// Takes any slice and returns a decimated Vec of PlotData.
pub fn decimate_m4_generic<T, FX, FY, FC>(
    data: &[T],
    max_points: usize,
    _get_x: FX,
    get_y: FY,
    create_point: FC,
) -> Vec<PlotData>
where
    FX: Fn(&T) -> f64,
    FY: Fn(&T) -> f64,
    FC: Fn(&T) -> PlotData,
{
    if data.len() <= max_points {
        return data.iter().map(create_point).collect();
    }

    // M4 returns 4 points per bin.
    let target_bins = max_points / 4;
    let bin_size = (data.len() as f64 / target_bins.max(1) as f64).ceil() as usize;
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

        // If the first point is NaN, try to find the first non-NaN point to initialize min/max
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
            if found { first_non_nan + 1 } else { chunk.len() }
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

        // We have 4 candidates: first, min, max, last.
        // We must sort them by index to keep chronological order.
        let mut indices = vec![first_idx, min_idx, max_idx, last_idx];
        indices.sort_unstable();
        indices.dedup();

        for idx in indices {
            aggregated.push(create_point(&chunk[idx]));
        }
    }

    // Ensure we respect max_points exactly
    if aggregated.len() > max_points {
        aggregated.truncate(max_points);
    }

    aggregated
}

/// Generic LTTB aggregator.
pub fn decimate_lttb_generic<T, FX, FY, FC>(
    data: &[T],
    max_points: usize,
    get_x: FX,
    get_y: FY,
    create_point: FC,
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
    
    // Bucket size. Exclude start and end points.
    let bucket_size = (data.len() - 2) as f64 / (max_points - 2) as f64;
    
    // 1. First point
    let mut a_idx = 0;
    sampled.push(create_point(&data[a_idx]));

    for i in 0..(max_points - 2) {
        // Current bucket range
        let bucket_start = 1 + (i as f64 * bucket_size).floor() as usize;
        let bucket_end = 1 + ((i + 1) as f64 * bucket_size).floor() as usize;
        
        // Next bucket range (for average)
        let next_bucket_start = bucket_end;
        let next_bucket_end = (1 + ((i + 2) as f64 * bucket_size).floor() as usize).min(data.len() - 1);

        // Calculate average of next bucket
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
            // Fallback if next bucket is empty
             avg_x = get_x(&data[next_bucket_start.min(data.len() - 1)]);
             avg_y = get_y(&data[next_bucket_start.min(data.len() - 1)]);
        }

        // Point A
        let p_a_x = get_x(&data[a_idx]);
        let p_a_y = get_y(&data[a_idx]);

        // Find best point B in current bucket
        let mut max_area = -1.0;
        let mut next_a_idx = bucket_start; // Default

        for j in bucket_start..bucket_end {
            let p_b_x = get_x(&data[j]);
            let p_b_y = get_y(&data[j]);
            
            // Triangle Area = 0.5 * |Ax(By - Cy) + Bx(Cy - Ay) + Cx(Ay - By)|
            // We can ignore 0.5 for comparison
            let area = (p_a_x * (p_b_y - avg_y) + p_b_x * (avg_y - p_a_y) + avg_x * (p_a_y - p_b_y)).abs();
            
            if area > max_area {
                max_area = area;
                next_a_idx = j;
            }
        }
        
        a_idx = next_a_idx;
        sampled.push(create_point(&data[a_idx]));
    }

    // Last point
    sampled.push(create_point(&data[data.len() - 1]));

    sampled
}

/// Aggregates a chunk into a single point (Mean for Points, OHLCV-Merge for OHLCV).
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