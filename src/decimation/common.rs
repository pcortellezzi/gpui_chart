use crate::data_types::{Ohlcv, PlotData, PlotPoint, ColorOp};
use crate::gaps::GapIndex;

/// Snaps a real timestamp to a stable grid defined by bin_size, respecting gaps.
pub fn snap_to_grid(time: f64, bin_size: f64, gaps: Option<&GapIndex>) -> f64 {
    // Add a tiny epsilon to handle floating point precision issues with large timestamps (ms)
    // 1e-7 is enough for ms precision at 1e12 magnitude.
    let epsilon = bin_size * 1e-7;
    
    if let Some(g) = gaps {
        let logical = g.to_logical(time as i64) as f64;
        let snapped = ((logical + epsilon) / bin_size).floor() * bin_size;
        g.to_real_first(snapped as i64) as f64
    } else {
        ((time + epsilon) / bin_size).floor() * bin_size
    }
}

/// Calculates a stable bin size (power of 10 or 2) that is just above the ideal resolution.
pub fn calculate_stable_bin_size(range: f64, max_points: usize) -> f64 {
    if range <= 0.0 || max_points == 0 {
        return 1.0;
    }
    let ideal = range / max_points as f64;
    let exponent = ideal.log10().floor();
    let base = 10.0f64.powf(exponent);
    let rel = ideal / base;
    // Use a larger epsilon (1e-6) safe for timestamps in the order of 1e12 ms.
    const EPS: f64 = 1e-6;

    let stable_rel = if rel <= 1.0 + EPS {
        1.0
    } else if rel <= 2.0 + EPS {
        2.0
    } else if rel <= 5.0 + EPS {
        5.0
    } else {
        10.0
    };

    base * stable_rel
}

pub fn get_data_x(p: &PlotData) -> f64 {
    match p {
        PlotData::Point(pt) => pt.x,
        PlotData::Ohlcv(o) => o.time,
    }
}

pub fn get_data_y(p: &PlotData) -> f64 {
    match p {
        PlotData::Point(pt) => pt.y,
        PlotData::Ohlcv(o) => o.close, // or handle differently? usually MinMax is for lines (y)
    }
}

/// Scans a slice to find the indices of the minimum and maximum values.
/// Handles NaN values by skipping them.
/// Returns (min_idx, max_idx).
#[inline(always)]
pub fn find_extrema_indices_generic<T, FY>(chunk: &[T], get_y: FY) -> (usize, usize)
where
    FY: Fn(&T) -> f64,
{
    let n = chunk.len();
    if n == 0 {
        return (0, 0);
    }
    
    let mut min_idx = 0;
    let mut max_idx = 0;
    let mut min_y = get_y(&chunk[0]);
    let mut max_y = min_y;

    let mut start = 1;
    if min_y.is_nan() {
        let mut found = false;
        for i in 1..n {
            let val = get_y(&chunk[i]);
            if !val.is_nan() {
                min_y = val;
                max_y = val;
                min_idx = i;
                max_idx = i;
                start = i + 1;
                found = true;
                break;
            }
        }
        if !found { return (0, 0); }
    }

    for i in start..n {
        let val = get_y(&chunk[i]);
        if val.is_nan() { continue; }
        if val < min_y {
            min_y = val;
            min_idx = i;
        } else if val > max_y {
            max_y = val;
            max_idx = i;
        }
    }

    (min_idx, max_idx)
}

/// Specialized version for raw f64 slices (avoiding closure overhead).
#[inline(always)]
pub fn find_extrema_indices_f64(chunk: &[f64]) -> (usize, usize) {
    let n = chunk.len();
    if n == 0 { return (0, 0); }
    
    let mut min_idx = 0;
    let mut max_idx = 0;
    let mut min_y = chunk[0];
    let mut max_y = min_y;

    let mut start = 1;
    if min_y.is_nan() {
        let mut found = false;
        for i in 1..n {
            let val = chunk[i];
            if !val.is_nan() {
                min_y = val;
                max_y = val;
                min_idx = i;
                max_idx = i;
                start = i + 1;
                found = true;
                break;
            }
        }
        if !found { return (0, 0); }
    }

    for i in start..n {
        let val = chunk[i];
        if val.is_nan() { continue; }
        if val < min_y {
            min_y = val;
            min_idx = i;
        } else if val > max_y {
            max_y = val;
            max_idx = i;
        }
    }

    (min_idx, max_idx)
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

        let mut found_open = false;
        for p in chunk {
            if let PlotData::Ohlcv(o) = p {
                if !o.open.is_nan() && !found_open {
                    open = o.open;
                    first_time = o.time;
                    found_open = true;
                }
                high = high.max(o.high);
                low = low.min(o.low);
                volume += o.volume;
            }
        }

        for p in chunk.iter().rev() {
            if let PlotData::Ohlcv(o) = p {
                if !o.close.is_nan() {
                    close = o.close;
                    break;
                }
            }
        }

        Some(PlotData::Ohlcv(Ohlcv {
            time: first_time,
            span: chunk.get(0).and_then(|p| if let PlotData::Ohlcv(o) = p { Some(o.span) } else { None }).unwrap_or(0.0),
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
