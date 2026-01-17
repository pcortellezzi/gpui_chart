use crate::data_types::{Ohlcv, PlotData, PlotPoint, ColorOp};

/// Calculates a stable bin size (power of 10 or 2) that is just above the ideal resolution.
pub fn calculate_stable_bin_size(range: f64, max_points: usize) -> f64 {
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

    let start_search_idx = if min_y.is_nan() {
        let mut found = false;
        let mut idx = 0;
        for (i, item) in chunk.iter().enumerate().skip(1) {
            let val = get_y(item);
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

    for (i, item) in chunk.iter().enumerate().skip(start_search_idx) {
        let val = get_y(item);
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
