use crate::data_types::{PlotData, Ohlcv};
use crate::gaps::GapIndex;
use rayon::prelude::*;

pub fn decimate_ohlcv_arrays_par_into(
    time: &[f64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) {
    if time.is_empty() {
        return;
    }

    // We calculate stable_bin_size based on the visible range.
    let (stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets(time, gaps, max_points, 1, reference_logical_range);

    let chunks: Vec<PlotData> = buckets
        .into_par_iter()
        .filter_map(|range| {
            let start_idx = range.start;
            let end_idx = range.end;
            let t_chunk = &time[start_idx..end_idx];

            if t_chunk.is_empty() {
                return None;
            }

            // Group memory accesses
            let o_chunk = &open[start_idx..end_idx.min(open.len())];
            let h_chunk = &high[start_idx..end_idx.min(high.len())];
            let l_chunk = &low[start_idx..end_idx.min(low.len())];
            let c_chunk = &close[start_idx..end_idx.min(close.len())];

            if o_chunk.is_empty() {
                return None;
            }

            // SIMD find extremas first
            let agg_high = crate::simd::max_f64(h_chunk);
            let agg_low = crate::simd::min_f64(l_chunk);

            if agg_high.is_nan() {
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
            for i in (0..c_chunk.len()).rev() {
                if !c_chunk[i].is_nan() {
                    agg_close = c_chunk[i];
                    break;
                }
            }

            let first_time_real = t_chunk[0];
            // Snap to grid using centralized logic
            let candle_time = super::common::snap_to_grid(first_time_real, stable_bin_size, gaps);

            Some(PlotData::Ohlcv(Ohlcv {
                time: candle_time,
                span: stable_bin_size,
                open: agg_open,
                high: agg_high,
                low: agg_low,
                close: agg_close,
                volume: 0.0,
            }))
        })
        .collect();

    output.extend(chunks);
}

pub fn decimate_ohlcv_arrays_par(
    time: &[f64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    max_points: usize,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_ohlcv_arrays_par_into(time, open, high, low, close, max_points, &mut output, gaps, reference_logical_range);
    output
}

// Re-export common dependencies needed by other modules or used here
use super::common::{aggregate_chunk, get_data_x};

pub fn decimate_ohlcv_slice_into(
    data: &[PlotData],
    max_points: usize,
    output: &mut Vec<PlotData>,
    gaps: Option<&GapIndex>,
    reference_logical_range: Option<f64>,
) {
    if data.is_empty() {
        return;
    }

    if data.len() <= max_points {
        output.extend_from_slice(data);
        return;
    }

    let (stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets_data(data, gaps, max_points, 1, reference_logical_range);

    let chunks: Vec<PlotData> = buckets
        .into_par_iter()
        .filter_map(|range| {
            let chunk = &data[range.start..range.end];
            // aggregate_chunk usually returns Point or OHLCV.
            // But we might need to snap the time of the result to the grid?
            // aggregate_chunk preserves the time of the first point.
            // If we want grid alignment, we should modify the time.
            
            let mut res = aggregate_chunk(chunk)?;
            
            // Snap time using centralized logic
            let time = get_data_x(&res);
            let snapped_time = super::common::snap_to_grid(time, stable_bin_size, gaps);
            
            match &mut res {
                PlotData::Ohlcv(o) => {
                    o.time = snapped_time;
                    o.span = stable_bin_size;
                }
                PlotData::Point(p) => {
                    // Points usually don't have span, but we snap x
                    p.x = snapped_time;
                }
            }
            Some(res)
        })
        .collect();
    
    output.extend(chunks);
}
