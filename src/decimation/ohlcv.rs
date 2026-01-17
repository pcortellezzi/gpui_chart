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
) {
    let initial_len = output.len();
    if time.is_empty() {
        return;
    }

    // Direct pass if enough space (this logic might need revisit if we want strict grid alignment even for few points,
    // but usually exact display is preferred when zoomed in).
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

    // We calculate stable_bin_size based on the visible range.
    let (stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets(time, gaps, max_points, 1);

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
            // Snap to grid
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
) -> Vec<PlotData> {
    let mut output = Vec::with_capacity(max_points);
    decimate_ohlcv_arrays_par_into(time, open, high, low, close, max_points, &mut output, gaps);
    output
}

// Re-export common dependencies needed by other modules or used here
use super::common::{aggregate_chunk, get_data_x};

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

    let (stable_bin_size, buckets) = super::bucketing::calculate_stable_buckets_data(data, gaps, max_points, 1);

    let chunks: Vec<PlotData> = buckets
        .into_par_iter()
        .filter_map(|range| {
            let chunk = &data[range.start..range.end];
            // aggregate_chunk usually returns Point or OHLCV.
            // But we might need to snap the time of the result to the grid?
            // aggregate_chunk preserves the time of the first point.
            // If we want grid alignment, we should modify the time.
            
            let mut res = aggregate_chunk(chunk)?;
            
            // Snap time
            let time = get_data_x(&res);
            let snapped_time = if let Some(g) = gaps {
                let logical = g.to_logical(time as i64) as f64;
                let s = (logical / stable_bin_size).floor() * stable_bin_size;
                g.to_real(s as i64) as f64
            } else {
                (time / stable_bin_size).floor() * stable_bin_size
            };
            
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

    if output.len() > initial_len + max_points {
        output.truncate(initial_len + max_points);
    }
}
