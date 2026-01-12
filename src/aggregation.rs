use crate::data_types::{ColorOp, Ohlcv, PlotData, PlotPoint};

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

        for (i, item) in chunk.iter().enumerate().skip(1) {
            let y = get_y(item);
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

        for (i, item) in chunk.iter().enumerate().skip(1) {
            let y = get_y(item);
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
