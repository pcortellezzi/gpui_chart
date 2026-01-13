use crate::data_types::{PlotData, PlotDataSource, StreamingDataSource, AggregationMode};
use std::time::Instant;

pub struct HybridDataSource {
    historical: Box<dyn PlotDataSource>,
    realtime: StreamingDataSource,
    mode: AggregationMode,
    last_commit: Instant,
    commit_threshold_points: usize,
}

impl HybridDataSource {
    pub fn new(historical: Box<dyn PlotDataSource>, realtime_capacity: usize) -> Self {
        Self {
            historical,
            realtime: StreamingDataSource::new(realtime_capacity),
            mode: AggregationMode::M4,
            last_commit: Instant::now(),
            commit_threshold_points: 5000,
        }
    }

    pub fn with_aggregation_mode(mut self, mode: AggregationMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn add_realtime(&mut self, data: PlotData) {
        self.realtime.add_data(data);
        
        // Automatic commit when threshold is reached to maintain performance
        if self.realtime.len() >= self.commit_threshold_points {
            self.commit_realtime_to_historical();
        }
    }

    /// Merges the realtime buffer into the historical data source.
    /// This ensures long-term performance by moving data to the optimized historical source.
    pub fn commit_realtime_to_historical(&mut self) {
        if self.realtime.is_empty() {
            return;
        }

        // 1. Extract all data from realtime buffer
        let rt_count = self.realtime.len();
        let mut rt_data = Vec::with_capacity(rt_count);
        rt_data.extend(self.realtime.iter_range(f64::MIN, f64::MAX));

        // 2. Add to historical source
        let mut all_data = Vec::with_capacity(self.historical.len() + rt_count);
        all_data.extend(self.historical.iter_range(f64::MIN, f64::MAX));
        all_data.extend(rt_data);

        self.historical.set_data(all_data);
        
        // 3. Reset realtime
        self.realtime.set_data(vec![]);
        self.last_commit = Instant::now();
    }
}

impl PlotDataSource for HybridDataSource {
    fn aggregation_mode(&self) -> AggregationMode {
        self.mode
    }

    fn len(&self) -> usize {
        self.historical.len() + self.realtime.len()
    }

    fn suggested_x_spacing(&self) -> f64 {
        self.historical.suggested_x_spacing()
    }

    fn get_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let b1 = self.historical.get_bounds();
        let b2 = self.realtime.get_bounds();

        match (b1, b2) {
            (Some(h), Some(r)) => Some((
                h.0.min(r.0), h.1.max(r.1),
                h.2.min(r.2), h.3.max(r.3)
            )),
            (Some(h), None) => Some(h),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        }
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        let r1 = self.historical.get_y_range(x_min, x_max);
        let r2 = self.realtime.get_y_range(x_min, x_max);

        match (r1, r2) {
            (Some(h), Some(r)) => Some((h.0.min(r.0), h.1.max(r.1))),
            (Some(h), None) => Some(h),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        }
    }

    fn iter_range(&self, x_min: f64, x_max: f64) -> Box<dyn Iterator<Item = PlotData> + '_> {
        Box::new(self.historical.iter_range(x_min, x_max)
            .chain(self.realtime.iter_range(x_min, x_max)))
    }

    fn get_aggregated_data(
        &self,
        x_min: f64,
        x_max: f64,
        max_points: usize,
        output: &mut Vec<PlotData>,
    ) {
        output.clear();
        
        // Ratio-based point budgeting
        let h_len = self.historical.len() as f64;
        let r_len = self.realtime.len() as f64;
        let total = h_len + r_len;
        
        if total == 0.0 { return; }

        let hist_budget = (max_points as f64 * (h_len / total)).ceil() as usize;
        let rt_budget = max_points.saturating_sub(hist_budget);

        if hist_budget > 0 {
            self.historical.get_aggregated_data(x_min, x_max, hist_budget, output);
        }
        
        if rt_budget > 0 {
            let mut rt_buffer = Vec::with_capacity(rt_budget);
            self.realtime.get_aggregated_data(x_min, x_max, rt_budget, &mut rt_buffer);
            output.extend(rt_buffer);
        }
    }

    fn add_data(&mut self, data: PlotData) {
        self.add_realtime(data);
    }

    fn set_data(&mut self, data: Vec<PlotData>) {
        self.historical.set_data(data);
        self.realtime.set_data(vec![]);
    }
}