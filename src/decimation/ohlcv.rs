use crate::data_types::{PlotData, Ohlcv};
use crate::gaps::GapIndex;
use rayon::prelude::*;
use super::common::{calculate_stable_bin_size, calculate_dynamic_bin_size};
use super::bucketing::calculate_gap_aware_buckets;

pub fn decimate_ohlcv_arrays_par_into(
    time: &[f64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
    offset: usize,
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

    // Use centralized dynamic bin size logic (points_per_bucket = 1 for OHLCV usually, or treated as 1 candle)
    let bin_size = calculate_dynamic_bin_size(
        time[0],
        time[time.len() - 1],
        time.len(),
        max_points,
        1,
        gaps,
    );
    
    // We need stable_bin_size for candle width calculation
    let real_range = time[time.len() - 1] - time[0];
    let logical_range = if let Some(g) = gaps {
        (g.to_logical(time[time.len() - 1] as i64) - g.to_logical(time[0] as i64)) as f64
    } else {
        real_range
    };
    let stable_bin_size = calculate_stable_bin_size(logical_range, max_points.max(1));

    // Pre-calculate buckets that respect gaps and are aligned to global offset
    let buckets = calculate_gap_aware_buckets(time, gaps, bin_size, offset);

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

            let first_time_real = t_chunk[0];
            let candle_time = if let Some(g) = gaps {
                let logical = g.to_logical(first_time_real as i64) as f64;
                let snapped = (logical / stable_bin_size).floor() * stable_bin_size;
                g.to_real(snapped as i64) as f64
            } else {
                (first_time_real / stable_bin_size).floor() * stable_bin_size
            };

            Some(PlotData::Ohlcv(Ohlcv {
                time: candle_time,
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

pub fn decimate_ohlcv_arrays_par(
    time: &[f64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    max_points: usize,
    gaps: Option<&GapIndex>,
    offset: usize,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_ohlcv_arrays_par_into(time, open, high, low, close, max_points, &mut output, gaps, offset);
    output
}

// Re-export common dependencies needed by other modules or used here
use super::common::{aggregate_chunk, get_data_x};
use super::bucketing::calculate_gap_aware_buckets_data;

pub fn decimate_ohlcv_slice_into(
    data: &[PlotData],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
    offset: usize,
) {
    let initial_len = output.len();
    if data.is_empty() {
        return;
    }

    if data.len() <= max_points {
        output.extend_from_slice(data);
        return;
    }

    let bin_size = calculate_dynamic_bin_size(
        get_data_x(&data[0]),
        get_data_x(&data[data.len() - 1]),
        data.len(),
        max_points,
        1,
        gaps,
    );

    let buckets = calculate_gap_aware_buckets_data(data, gaps, bin_size, offset);

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
