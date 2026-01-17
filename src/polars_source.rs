#![cfg(feature = "polars")]

use crate::data_types::{ColorOp, Ohlcv, PlotData, PlotDataSource, PlotPoint};
use polars::prelude::*;
use polars_ops::prelude::{search_sorted, SearchSortedSide};

pub struct PolarsDataSource {
    df: DataFrame,
    x_col: String,
    y_col: String,
    mode: crate::data_types::AggregationMode,
    // Optional columns for OHLCV
    open_col: Option<String>,
    high_col: Option<String>,
    low_col: Option<String>,
    close_col: Option<String>,
}

impl PolarsDataSource {
    pub fn new(mut df: DataFrame, x_col: &str, y_col: &str) -> Self {
        // Essential for Zero-Copy: ensure all columns are in a single memory chunk.
        df.rechunk_mut();
        Self {
            df,
            x_col: x_col.to_string(),
            y_col: y_col.to_string(),
            mode: crate::data_types::AggregationMode::M4,
            open_col: None,
            high_col: None,
            low_col: None,
            close_col: None,
        }
    }

    pub fn with_aggregation_mode(mut self, mode: crate::data_types::AggregationMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_ohlcv(mut self, open: &str, high: &str, low: &str, close: &str) -> Self {
        self.open_col = Some(open.to_string());
        self.high_col = Some(high.to_string());
        self.low_col = Some(low.to_string());
        self.close_col = Some(close.to_string());
        self
    }

    fn get_range_indices(&self, x_min: f64, x_max: f64) -> (usize, usize) {
        let x_series = match self.df.column(&self.x_col).ok().and_then(|c| c.as_series()) {
            Some(s) => s,
            None => return (0, 0),
        };

        let x_min_s = Series::new("x_min".into(), &[x_min]);
        let x_max_s = Series::new("x_max".into(), &[x_max]);

        let start_idx_ca = match search_sorted(x_series, &x_min_s, SearchSortedSide::Left, false) {
            Ok(ca) => ca,
            Err(_) => return (0, 0),
        };
        let end_idx_ca = match search_sorted(x_series, &x_max_s, SearchSortedSide::Right, false) {
            Ok(ca) => ca,
            Err(_) => return (0, 0),
        };

        let start_idx = start_idx_ca.get(0).unwrap_or(0) as usize;
        let end_idx = end_idx_ca.get(0).unwrap_or(self.df.height() as u32) as usize;

        // Ensure we include at least one point beyond x_max to avoid disappearing points at the edge
        let end_idx = (end_idx + 1).min(self.df.height());

        (start_idx, end_idx)
    }

    fn iter_aggregated_lazy_fallback(
        &self,
        start_idx: usize,
        count: usize,
        max_points: usize,
    ) -> Box<dyn Iterator<Item = PlotData> + '_> {
        let mut lf = self.df.slice(start_idx as i64, count).lazy();

        if let (Some(o_n), Some(h_n), Some(l_n), Some(c_n)) = (
            &self.open_col,
            &self.high_col,
            &self.low_col,
            &self.close_col,
        ) {
            let target_bins = max_points.max(1);
            let bin_size = (count as f64 / target_bins as f64).ceil() as i64;

            lf = lf.with_row_index("index_id", Some(0)).with_column(
                (col("index_id") / lit(bin_size))
                    .cast(DataType::Int64)
                    .alias("bin_id"),
            );

            let agg_lf = lf
                .group_by([col("bin_id")])
                .agg([
                    col(o_n).first().alias(o_n),
                    col(h_n).max().alias(h_n),
                    col(l_n).min().alias(l_n),
                    col(c_n).last().alias(c_n),
                    col(&self.x_col).first().alias(&self.x_col),
                ])
                .sort(["bin_id"], Default::default());

            let df = match agg_lf.collect() {
                Ok(df) => df,
                Err(_) => return Box::new(std::iter::empty()),
            };

            let x_c = match df
                .column(&self.x_col)
                .ok()
                .and_then(|c| c.as_series())
                .and_then(|s| s.f64().ok())
            {
                Some(c) => c,
                None => return Box::new(std::iter::empty()),
            };
            let o_c = match df
                .column(o_n)
                .ok()
                .and_then(|c| c.as_series())
                .and_then(|s| s.f64().ok())
            {
                Some(c) => c,
                None => return Box::new(std::iter::empty()),
            };
            let h_c = match df
                .column(h_n)
                .ok()
                .and_then(|c| c.as_series())
                .and_then(|s| s.f64().ok())
            {
                Some(c) => c,
                None => return Box::new(std::iter::empty()),
            };
            let l_c = match df
                .column(l_n)
                .ok()
                .and_then(|c| c.as_series())
                .and_then(|s| s.f64().ok())
            {
                Some(c) => c,
                None => return Box::new(std::iter::empty()),
            };
            let c_c = match df
                .column(c_n)
                .ok()
                .and_then(|c| c.as_series())
                .and_then(|s| s.f64().ok())
            {
                Some(c) => c,
                None => return Box::new(std::iter::empty()),
            };

            let result: Vec<_> = x_c
                .iter()
                .zip(o_c.iter())
                .zip(h_c.iter())
                .zip(l_c.iter())
                .zip(c_c.iter())
                .map(|((((x, o), h), l), c)| {
                    PlotData::Ohlcv(Ohlcv {
                        time: x.unwrap_or(0.0),
                        span: 0.0,
                        open: o.unwrap_or(0.0),
                        high: h.unwrap_or(0.0),
                        low: l.unwrap_or(0.0),
                        close: c.unwrap_or(0.0),
                        volume: 0.0,
                    })
                })
                .collect();
            return Box::new(result.into_iter());
        }

        let (target_bins, m4_mode) = match self.mode {
            crate::data_types::AggregationMode::M4 => ((max_points / 4).max(1), true),
            crate::data_types::AggregationMode::MinMax => ((max_points / 2).max(1), false),
            crate::data_types::AggregationMode::LTTB => unreachable!("LTTB is handled above"),
        };

        let bin_size = (count as f64 / target_bins as f64).ceil() as i64;
        lf = lf.with_row_index("index_id", Some(0)).with_column(
            (col("index_id") / lit(bin_size))
                .cast(DataType::Int64)
                .alias("bin_id"),
        );

        let agg_exprs = if m4_mode {
            vec![
                col(&self.x_col).first().alias("x_first"),
                col(&self.x_col).last().alias("x_last"),
                col(&self.x_col)
                    .gather(col(&self.y_col).arg_min())
                    .first()
                    .alias("x_min"),
                col(&self.x_col)
                    .gather(col(&self.y_col).arg_max())
                    .first()
                    .alias("x_max"),
                col(&self.y_col).first().alias("y_first"),
                col(&self.y_col).last().alias("y_last"),
                col(&self.y_col).min().alias("y_min"),
                col(&self.y_col).max().alias("y_max"),
            ]
        } else {
            vec![
                col(&self.x_col)
                    .gather(col(&self.y_col).arg_min())
                    .first()
                    .alias("x_min"),
                col(&self.x_col)
                    .gather(col(&self.y_col).arg_max())
                    .first()
                    .alias("x_max"),
                col(&self.y_col).min().alias("y_min"),
                col(&self.y_col).max().alias("y_max"),
            ]
        };

        let agg_lf = lf.group_by([col("bin_id")]).agg(agg_exprs);

        let select_exprs = if m4_mode {
            vec![
                col("bin_id"),
                concat_list([col("x_first"), col("x_min"), col("x_max"), col("x_last")])
                    .unwrap()
                    .alias(&self.x_col),
                concat_list([col("y_first"), col("y_min"), col("y_max"), col("y_last")])
                    .unwrap()
                    .alias(&self.y_col),
            ]
        } else {
            vec![
                col("bin_id"),
                concat_list([col("x_min"), col("x_max")])
                    .unwrap()
                    .alias(&self.x_col),
                concat_list([col("y_min"), col("y_max")])
                    .unwrap()
                    .alias(&self.y_col),
            ]
        };

        let m4_lf = agg_lf
            .select(select_exprs)
            .explode(cols([&self.x_col, &self.y_col]))
            .sort(["bin_id", &self.x_col], Default::default());

        let df = match m4_lf.collect() {
            Ok(df) => df,
            Err(_) => return Box::new(std::iter::empty()),
        };
        let x_c = match df
            .column(&self.x_col)
            .ok()
            .and_then(|c| c.as_series())
            .and_then(|s| s.f64().ok())
        {
            Some(c) => c,
            None => return Box::new(std::iter::empty()),
        };
        let y_c = match df
            .column(&self.y_col)
            .ok()
            .and_then(|c| c.as_series())
            .and_then(|s| s.f64().ok())
        {
            Some(c) => c,
            None => return Box::new(std::iter::empty()),
        };

        let result: Vec<_> = x_c
            .iter()
            .zip(y_c.iter())
            .map(|(x, y)| {
                PlotData::Point(PlotPoint {
                    x: x.unwrap_or(0.0),
                    y: y.unwrap_or(0.0),
                    color_op: ColorOp::None,
                })
            })
            .collect();

        Box::new(result.into_iter())
    }
}

impl PlotDataSource for PolarsDataSource {
    fn aggregation_mode(&self) -> crate::data_types::AggregationMode {
        self.mode
    }

    fn get_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let x = self.df.column(&self.x_col).ok()?.as_series()?;
        let y = self.df.column(&self.y_col).ok()?.as_series()?;

        let x_min = x.min::<f64>().ok()??;
        let x_max = x.max::<f64>().ok()??;

        let (y_min, y_max) = if let (Some(l), Some(h)) = (&self.low_col, &self.high_col) {
            let low = self.df.column(l).ok()?.as_series()?.min::<f64>().ok()??;
            let high = self.df.column(h).ok()?.as_series()?.max::<f64>().ok()??;
            (low, high)
        } else {
            let y_min = y.min::<f64>().ok()??;
            let y_max = y.max::<f64>().ok()??;
            (y_min, y_max)
        };

        Some((x_min, x_max, y_min, y_max))
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        let (start_idx, end_idx) = self.get_range_indices(x_min, x_max);

        if start_idx >= end_idx {
            return None;
        }

        let len = end_idx - start_idx;
        let sliced_df = self.df.slice(start_idx as i64, len);

        if let (Some(l), Some(h)) = (&self.low_col, &self.high_col) {
            let low = sliced_df.column(l).ok()?.as_series()?.min::<f64>().ok()??;
            let high = sliced_df.column(h).ok()?.as_series()?.max::<f64>().ok()??;
            Some((low, high))
        } else {
            let y_s = sliced_df.column(&self.y_col).ok()?.as_series()?;
            let y_min = y_s.min::<f64>().ok()??;
            let y_max = y_s.max::<f64>().ok()??;
            Some((y_min, y_max))
        }
    }

    fn iter_range(&self, x_min: f64, x_max: f64) -> Box<dyn Iterator<Item = PlotData> + '_> {
        let (start_idx, end_idx) = self.get_range_indices(x_min, x_max);
        let start = start_idx.saturating_sub(1);
        let end = (end_idx + 1).min(self.df.height());
        let sliced = self.df.slice(start as i64, end - start);

        let x_col = match sliced
            .column(&self.x_col)
            .ok()
            .and_then(|c| c.as_series())
            .and_then(|s| s.f64().ok())
        {
            Some(c) => c,
            None => return Box::new(std::iter::empty()),
        };
        let y_col = match sliced
            .column(&self.y_col)
            .ok()
            .and_then(|c| c.as_series())
            .and_then(|s| s.f64().ok())
        {
            Some(c) => c,
            None => return Box::new(std::iter::empty()),
        };

        let mut result = Vec::with_capacity(sliced.height());
        let suggested_span = self.suggested_x_spacing();

        if let (Some(o_n), Some(h_n), Some(l_n), Some(c_n)) = (
            &self.open_col,
            &self.high_col,
            &self.low_col,
            &self.close_col,
        ) {
            let o_col = match sliced
                .column(o_n)
                .ok()
                .and_then(|c| c.as_series())
                .and_then(|s| s.f64().ok())
            {
                Some(c) => c,
                None => return Box::new(std::iter::empty()),
            };
            let h_col = match sliced
                .column(h_n)
                .ok()
                .and_then(|c| c.as_series())
                .and_then(|s| s.f64().ok())
            {
                Some(c) => c,
                None => return Box::new(std::iter::empty()),
            };
            let l_col = match sliced
                .column(l_n)
                .ok()
                .and_then(|c| c.as_series())
                .and_then(|s| s.f64().ok())
            {
                Some(c) => c,
                None => return Box::new(std::iter::empty()),
            };
            let c_col = match sliced
                .column(c_n)
                .ok()
                .and_then(|c| c.as_series())
                .and_then(|s| s.f64().ok())
            {
                Some(c) => c,
                None => return Box::new(std::iter::empty()),
            };
            for i in 0..sliced.height() {
                result.push(PlotData::Ohlcv(Ohlcv {
                    time: x_col.get(i).unwrap_or(0.0),
                    span: suggested_span,
                    open: o_col.get(i).unwrap_or(0.0),
                    high: h_col.get(i).unwrap_or(0.0),
                    low: l_col.get(i).unwrap_or(0.0),
                    close: c_col.get(i).unwrap_or(0.0),
                    volume: 0.0,
                }));
            }
        } else {
            for i in 0..sliced.height() {
                result.push(PlotData::Point(PlotPoint {
                    x: x_col.get(i).unwrap_or(0.0),
                    y: y_col.get(i).unwrap_or(0.0),
                    color_op: ColorOp::None,
                }));
            }
        }
        Box::new(result.into_iter())
    }

    fn iter_aggregated(
        &self,
        x_min: f64,
        x_max: f64,
        max_points: usize,
        gaps: Option<&crate::gaps::GapIndex>,
    ) -> Box<dyn Iterator<Item = PlotData> + '_> {
        let mut buffer = Vec::with_capacity(max_points);
        self.get_aggregated_data(x_min, x_max, max_points, &mut buffer, gaps);
        Box::new(buffer.into_iter())
    }

    fn get_aggregated_data(
        &self,
        x_min: f64,
        x_max: f64,
        max_points: usize,
        output: &mut Vec<PlotData>,
        gaps: Option<&crate::gaps::GapIndex>,
    ) {
        output.clear();

        // Use REAL range for bin size calculation to ensure absolute stability during pan.
        let view_range = x_max - x_min;

        let target_buckets = if self.open_col.is_some() {
            max_points.max(1)
        } else {
            match self.mode {
                crate::data_types::AggregationMode::M4 => (max_points / 4).max(1),
                crate::data_types::AggregationMode::MinMax => (max_points / 2).max(1),
                crate::data_types::AggregationMode::LTTB => max_points.max(1),
            }
        };

        let stable_bin_size = crate::decimation::common::calculate_stable_bin_size(view_range, target_buckets);

        // 1. Find logical start of the first bucket on screen
        let x_min_logical = if let Some(g) = gaps {
            g.to_logical(x_min as i64) as f64
        } else {
            x_min
        };
        let aligned_start_logical = (x_min_logical / stable_bin_size).floor() * stable_bin_size;
        
        // 2. Map back to real time to find start index in dataframe
        let aligned_start_real = if let Some(g) = gaps {
            g.to_real_first(aligned_start_logical as i64) as f64
        } else {
            aligned_start_logical
        };

        // 3. Find indices with some extra safety padding
        let (start_idx_raw, end_idx_raw) = self.get_range_indices(aligned_start_real, x_max);
        
        // Important: we ensure we start BEFORE aligned_start_real to give some room to the grid alignment
        let mut start = start_idx_raw;
        let x_series = self.df.column(&self.x_col).ok().and_then(|c| c.as_series()).and_then(|s| s.f64().ok());
        if let Some(x) = x_series {
            // Take 5 extra points before the boundary for safety
            start = start.saturating_sub(5);
            while start > 0 && x.get(start).unwrap_or(f64::MAX) > aligned_start_real - (stable_bin_size * 0.1) {
                start -= 1;
            }
        }

        let end = (end_idx_raw + 10).min(self.df.height());
        let count = end - start;

        if count == 0 { return; }

        if count <= max_points {
            output.extend(self.iter_range(x_min, x_max));
            return;
        }

        // Optimized Zero-Copy Path for Points (M4, MinMax, LTTB)
        if (matches!(self.mode, crate::data_types::AggregationMode::M4)
            || matches!(self.mode, crate::data_types::AggregationMode::MinMax)
            || matches!(self.mode, crate::data_types::AggregationMode::LTTB))
            && self.open_col.is_none()
        {
            let sliced = self.df.slice(start as i64, count);

            if let (Ok(x_col), Ok(y_col)) = (sliced.column(&self.x_col), sliced.column(&self.y_col))
            {
                if let (Some(x_series), Some(y_series)) = (
                    x_col.as_series().and_then(|s| s.f64().ok()),
                    y_col.as_series().and_then(|s| s.f64().ok()),
                ) {
                    let x_vec: Vec<f64>;
                    let x_slice = if let Ok(s) = x_series.cont_slice() {
                        s
                    } else {
                        x_vec = x_series
                            .to_vec()
                            .into_iter()
                            .map(|v| v.unwrap_or(f64::NAN))
                            .collect();
                        &x_vec
                    };

                    let y_vec: Vec<f64>;
                    let y_slice = if let Ok(s) = y_series.cont_slice() {
                        s
                    } else {
                        y_vec = y_series
                            .to_vec()
                            .into_iter()
                            .map(|v| v.unwrap_or(f64::NAN))
                            .collect();
                        &y_vec
                    };

                    match self.mode {
                        crate::data_types::AggregationMode::M4 => {
                            crate::decimation::decimate_m4_arrays_par_into(
                                x_slice, y_slice, max_points, output, gaps, Some(view_range),
                            )
                        }
                        crate::data_types::AggregationMode::MinMax => {
                            crate::decimation::decimate_min_max_arrays_par_into(
                                x_slice, y_slice, max_points, output, gaps, Some(view_range),
                            )
                        }
                        crate::data_types::AggregationMode::LTTB => {
                            crate::decimation::decimate_ilttb_arrays_par_into(
                                x_slice, y_slice, max_points, output, gaps, Some(view_range),
                            )
                        }
                    };
                    return;
                }
            }
        }

        // Optimized Zero-Copy Path for OHLCV
        if let (Some(o_n), Some(h_n), Some(l_n), Some(c_n)) = (
            &self.open_col,
            &self.high_col,
            &self.low_col,
            &self.close_col,
        ) {
            let sliced = self.df.slice(start as i64, count);

            if let (Ok(x_col), Ok(o_col), Ok(h_col), Ok(l_col), Ok(c_col)) = (
                sliced.column(&self.x_col),
                sliced.column(o_n),
                sliced.column(h_n),
                sliced.column(l_n),
                sliced.column(c_n),
            ) {
                if let (Some(x_s), Some(o_s), Some(h_s), Some(l_s), Some(c_s)) = (
                    x_col.as_series().and_then(|s| s.f64().ok()),
                    o_col.as_series().and_then(|s| s.f64().ok()),
                    h_col.as_series().and_then(|s| s.f64().ok()),
                    l_col.as_series().and_then(|s| s.f64().ok()),
                    c_col.as_series().and_then(|s| s.f64().ok()),
                ) {
                    let x_vec: Vec<f64>;
                    let x_slice = if let Ok(s) = x_s.cont_slice() {
                        s
                    } else {
                        x_vec = x_s
                            .to_vec()
                            .into_iter()
                            .map(|v| v.unwrap_or(f64::NAN))
                            .collect();
                        &x_vec
                    };
                    let o_vec: Vec<f64>;
                    let o_slice = if let Ok(s) = o_s.cont_slice() {
                        s
                    } else {
                        o_vec = o_s
                            .to_vec()
                            .into_iter()
                            .map(|v| v.unwrap_or(f64::NAN))
                            .collect();
                        &o_vec
                    };
                    let h_vec: Vec<f64>;
                    let h_slice = if let Ok(s) = h_s.cont_slice() {
                        s
                    } else {
                        h_vec = h_s
                            .to_vec()
                            .into_iter()
                            .map(|v| v.unwrap_or(f64::NAN))
                            .collect();
                        &h_vec
                    };
                    let l_vec: Vec<f64>;
                    let l_slice = if let Ok(s) = l_s.cont_slice() {
                        s
                    } else {
                        l_vec = l_s
                            .to_vec()
                            .into_iter()
                            .map(|v| v.unwrap_or(f64::NAN))
                            .collect();
                        &l_vec
                    };
                    let c_vec: Vec<f64>;
                    let c_slice = if let Ok(s) = c_s.cont_slice() {
                        s
                    } else {
                        c_vec = c_s
                            .to_vec()
                            .into_iter()
                            .map(|v| v.unwrap_or(f64::NAN))
                            .collect();
                        &c_vec
                    };

                    crate::decimation::decimate_ohlcv_arrays_par_into(
                        x_slice, o_slice, h_slice, l_slice, c_slice, max_points, output, gaps, Some(view_range),
                    );
                    return;
                }
            }
        }

        let lazy_result = self.iter_aggregated_lazy_fallback(start, count, max_points);
        output.extend(lazy_result);
    }

    fn add_data(&mut self, _data: PlotData) {}

    fn set_data(&mut self, data: Vec<PlotData>) {
        if data.is_empty() {
            self.df = DataFrame::default();
            return;
        }
        let mut x_vals = Vec::with_capacity(data.len());
        let mut y_vals = Vec::with_capacity(data.len());
        for p in data {
            match p {
                PlotData::Point(pt) => {
                    x_vals.push(pt.x);
                    y_vals.push(pt.y);
                }
                PlotData::Ohlcv(o) => {
                    x_vals.push(o.time);
                    y_vals.push(o.close);
                }
            }
        }
        self.df = DataFrame::new(vec![
            Series::new(self.x_col.clone().into(), x_vals).into(),
            Series::new(self.y_col.clone().into(), y_vals).into(),
        ])
        .unwrap();
        self.df.rechunk_mut();
    }

    fn suggested_x_spacing(&self) -> f64 {
        if self.df.height() < 2 {
            return 1.0;
        }
        let x = match self
            .df
            .column(&self.x_col)
            .ok()
            .and_then(|c| c.as_series())
            .and_then(|s| s.f64().ok())
        {
            Some(x) => x,
            None => return 1.0,
        };
        if x.len() > 1 {
            (x.get(1).unwrap_or(0.0) - x.get(0).unwrap_or(0.0)).abs()
        } else {
            1.0
        }
    }

    fn len(&self) -> usize {
        self.df.height()
    }
}
