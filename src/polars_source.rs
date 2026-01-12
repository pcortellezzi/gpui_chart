#![cfg(feature = "polars")]

use crate::data_types::{PlotData, PlotDataSource, PlotPoint, Ohlcv, ColorOp};
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
    pub fn new(df: DataFrame, x_col: &str, y_col: &str) -> Self {
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

    pub fn with_ohlcv(
        mut self,
        open: &str,
        high: &str,
        low: &str,
        close: &str,
    ) -> Self {
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

        (start_idx, end_idx)
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
        let x_series = self.df.column(&self.x_col).ok()?.as_series()?;
        
        let mask = x_series.gt_eq(x_min).ok()?
            & x_series.lt_eq(x_max).ok()?;
        
        let filtered = self.df.filter(&mask).ok()?;
        
        if filtered.height() == 0 {
            return None;
        }

        if let (Some(l), Some(h)) = (&self.low_col, &self.high_col) {
            let y_min = filtered.column(l).ok()?.as_series()?.min::<f64>().ok()??;
            let y_max = filtered.column(h).ok()?.as_series()?.max::<f64>().ok()??;
            Some((y_min, y_max))
        } else {
            let y_min = filtered.column(&self.y_col).ok()?.as_series()?.min::<f64>().ok()??;
            let y_max = filtered.column(&self.y_col).ok()?.as_series()?.max::<f64>().ok()??;
            Some((y_min, y_max))
        }
    }

    fn iter_range(&self, x_min: f64, x_max: f64) -> Box<dyn Iterator<Item = PlotData> + '_> {
        let (start_idx, end_idx) = self.get_range_indices(x_min, x_max);
        let start = start_idx.saturating_sub(1);
        let end = (end_idx + 1).min(self.df.height());
        let sliced = self.df.slice(start as i64, end - start);
        
        let x_col = sliced.column(&self.x_col).ok().unwrap().as_series().unwrap().f64().ok().unwrap();
        let y_col = sliced.column(&self.y_col).ok().unwrap().as_series().unwrap().f64().ok().unwrap();

        let mut result = Vec::with_capacity(sliced.height());
        if let (Some(o_n), Some(h_n), Some(l_n), Some(c_n)) = (&self.open_col, &self.high_col, &self.low_col, &self.close_col) {
            let o_col = sliced.column(o_n).ok().unwrap().as_series().unwrap().f64().ok().unwrap();
            let h_col = sliced.column(h_n).ok().unwrap().as_series().unwrap().f64().ok().unwrap();
            let l_col = sliced.column(l_n).ok().unwrap().as_series().unwrap().f64().ok().unwrap();
            let c_col = sliced.column(c_n).ok().unwrap().as_series().unwrap().f64().ok().unwrap();
            for i in 0..sliced.height() {
                result.push(PlotData::Ohlcv(Ohlcv {
                    time: x_col.get(i).unwrap_or(0.0),
                    span: 0.0,
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
    ) -> Box<dyn Iterator<Item = PlotData> + '_> {
        let (start_idx, end_idx) = self.get_range_indices(x_min, x_max);
        let count = end_idx - start_idx;

        if count <= max_points {
            return self.iter_range(x_min, x_max);
        }

        let mut lf = self.df.slice(start_idx as i64, count).lazy();

        if let (Some(o_n), Some(h_n), Some(l_n), Some(c_n)) = (&self.open_col, &self.high_col, &self.low_col, &self.close_col) {
            // OHLCV aggregation: 1 point per bin
            let target_bins = max_points.max(1);
            let bin_size = (count as f64 / target_bins as f64).ceil() as i64;
            
            // Use standard group_by on calculated bin index (faster than dynamic windowing)
            lf = lf.with_row_index("index_id", Some(0))
                   .with_column((col("index_id") / lit(bin_size)).cast(DataType::Int64).alias("bin_id"));

            let agg_lf = lf.group_by([col("bin_id")]).agg([
                col(o_n).first().alias(o_n),
                col(h_n).max().alias(h_n),
                col(l_n).min().alias(l_n),
                col(c_n).last().alias(c_n),
                col(&self.x_col).first().alias(&self.x_col),
            ]).sort(["bin_id"], Default::default());

            let df = agg_lf.collect().unwrap();
            let x_c = df.column(&self.x_col).unwrap().f64().unwrap();
            let o_c = df.column(o_n).unwrap().f64().unwrap();
            let h_c = df.column(h_n).unwrap().f64().unwrap();
            let l_c = df.column(l_n).unwrap().f64().unwrap();
            let c_c = df.column(c_n).unwrap().f64().unwrap();

            let result: Vec<_> = x_c.iter()
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

        // Points aggregation
        let (target_bins, m4_mode) = match self.mode {
            crate::data_types::AggregationMode::M4 => ((max_points / 4).max(1), true),
            crate::data_types::AggregationMode::MinMax => ((max_points / 2).max(1), false),
        };

        let bin_size = (count as f64 / target_bins as f64).ceil() as i64;
        
        // Use standard group_by on calculated bin index
        lf = lf.with_row_index("index_id", Some(0))
               .with_column((col("index_id") / lit(bin_size)).cast(DataType::Int64).alias("bin_id"));

        let agg_exprs = if m4_mode {
            vec![
                col(&self.x_col).first().alias("x_first"),
                col(&self.x_col).last().alias("x_last"),
                col(&self.x_col).gather(col(&self.y_col).arg_min()).first().alias("x_min"),
                col(&self.x_col).gather(col(&self.y_col).arg_max()).first().alias("x_max"),
                col(&self.y_col).first().alias("y_first"),
                col(&self.y_col).last().alias("y_last"),
                col(&self.y_col).min().alias("y_min"),
                col(&self.y_col).max().alias("y_max"),
            ]
        } else {
            vec![
                col(&self.x_col).gather(col(&self.y_col).arg_min()).first().alias("x_min"),
                col(&self.x_col).gather(col(&self.y_col).arg_max()).first().alias("x_max"),
                col(&self.y_col).min().alias("y_min"),
                col(&self.y_col).max().alias("y_max"),
            ]
        };

        let agg_lf = lf.group_by([col("bin_id")]).agg(agg_exprs);

        let select_exprs = if m4_mode {
            vec![
                col("bin_id"),
                concat_list([col("x_first"), col("x_min"), col("x_max"), col("x_last")]).unwrap().alias(&self.x_col),
                concat_list([col("y_first"), col("y_min"), col("y_max"), col("y_last")]).unwrap().alias(&self.y_col),
            ]
        } else {
            vec![
                col("bin_id"),
                concat_list([col("x_min"), col("x_max")]).unwrap().alias(&self.x_col),
                concat_list([col("y_min"), col("y_max")]).unwrap().alias(&self.y_col),
            ]
        };

        let m4_lf = agg_lf.select(select_exprs)
            .explode(cols([&self.x_col, &self.y_col]))
            .sort(["bin_id", &self.x_col], Default::default());

        let df = m4_lf.collect().unwrap();
        let x_c = df.column(&self.x_col).unwrap().f64().unwrap();
        let y_c = df.column(&self.y_col).unwrap().f64().unwrap();

        let result: Vec<_> = x_c.iter()
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

    fn add_data(&mut self, _data: PlotData) {
    }

    fn set_data(&mut self, _data: Vec<PlotData>) {
    }

    fn suggested_x_spacing(&self) -> f64 {
        if self.df.height() < 2 {
            return 1.0;
        }
        let x = self.df.column(&self.x_col).ok().unwrap().as_series().unwrap().f64().ok().unwrap();
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