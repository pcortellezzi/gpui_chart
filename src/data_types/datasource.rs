use std::collections::VecDeque;
use crate::gaps::GapIndex;
use super::data::{PlotData, AggregationMode};
use super::axis::AxisDomain;

/// Trait for data sources that provide points for the chart.
pub trait PlotDataSource: Send + Sync {
    /// Get the preferred aggregation mode.
    fn aggregation_mode(&self) -> AggregationMode {
        AggregationMode::M4
    }

    /// Returns the bounds of the data as (x_min, x_max, y_min, y_max)
    fn get_bounds(&self) -> Option<(f64, f64, f64, f64)>;

    /// Y-range within a specific X-window (for auto-scaling Y)
    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)>;

    /// Iterate over data within an X-window (for rendering with culling)
    /// Includes one point before and after the range for line continuity.
    fn iter_range(&self, x_min: f64, x_max: f64) -> Box<dyn Iterator<Item = PlotData> + '_>;

    /// Iterate over aggregated data for LOD rendering.
    /// max_points: The target maximum number of points to return.
    fn iter_aggregated(
        &self,
        x_min: f64,
        x_max: f64,
        max_points: usize,
        gaps: Option<&GapIndex>,
    ) -> Box<dyn Iterator<Item = PlotData> + '_> {
        let data: Vec<PlotData> = self.iter_range(x_min, x_max).collect();
        Box::new(crate::decimation::decimate_min_max_slice(&data, max_points, gaps, None).into_iter())
    }

    /// Populate a buffer with aggregated data for LOD rendering.
    fn get_aggregated_data(
        &self,
        x_min: f64,
        x_max: f64,
        max_points: usize,
        output: &mut Vec<PlotData>,
        gaps: Option<&GapIndex>,
    ) {
        output.clear();
        output.extend(self.iter_aggregated(x_min, x_max, max_points, gaps));
    }

    /// Add a single data point
    fn add_data(&mut self, data: PlotData);

    /// Replace all data
    fn set_data(&mut self, data: Vec<PlotData>);

    /// Suggested X spacing between points (e.g. for Bar width calculation)
    fn suggested_x_spacing(&self) -> f64;

    /// Total number of points
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Data source optimized for real-time streaming with a fixed capacity.
pub struct StreamingDataSource {
    data: VecDeque<PlotData>,
    capacity: usize,
    bounds_cache: VecDeque<AxisDomain>, // Each element represents a chunk of CHUNK_SIZE points
    current_chunk_count: usize,         // Points in the first chunk
    points_in_last_chunk: usize,        // Points in the last chunk
    suggested_spacing: f64,
}

const CHUNK_SIZE: usize = 512;

impl StreamingDataSource {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
            bounds_cache: VecDeque::new(),
            current_chunk_count: 0,
            points_in_last_chunk: 0,
            suggested_spacing: 1.0,
        }
    }

    fn update_suggested_spacing(&mut self, new_x: f64) {
        if let Some(last) = self.data.back() {
            let last_x = match last {
                PlotData::Point(p) => p.x,
                PlotData::Ohlcv(o) => o.time,
            };
            let s = (new_x - last_x).abs();
            if s > f64::EPSILON {
                if self.suggested_spacing == 1.0 {
                    self.suggested_spacing = s;
                } else {
                    self.suggested_spacing = self.suggested_spacing * 0.95 + s * 0.05;
                }
            }
        }
    }

    fn rebuild_cache(&mut self) {
        let mut min_spacing = f64::INFINITY;
        let mut last_x: Option<f64> = None;
        self.bounds_cache.clear();
        self.current_chunk_count = 0;
        self.points_in_last_chunk = 0;

        let mut count = 0;
        let mut domain = AxisDomain {
            x_min: f64::INFINITY,
            x_max: f64::NEG_INFINITY,
            y_min: f64::INFINITY,
            y_max: f64::NEG_INFINITY,
            ..Default::default()
        };

        for p in self.data.iter() {
            let x = match p {
                PlotData::Point(pt) => pt.x,
                PlotData::Ohlcv(o) => o.time,
            };
            if let Some(lx) = last_x {
                let s = (x - lx).abs();
                if s > f64::EPSILON && s < min_spacing {
                    min_spacing = s;
                }
            }
            last_x = Some(x);

            match p {
                PlotData::Point(pt) => {
                    domain.x_min = domain.x_min.min(pt.x);
                    domain.x_max = domain.x_max.max(pt.x);
                    domain.y_min = domain.y_min.min(pt.y);
                    domain.y_max = domain.y_max.max(pt.y);
                }
                PlotData::Ohlcv(o) => {
                    domain.x_min = domain.x_min.min(o.time);
                    domain.x_max = domain.x_max.max(o.time + o.span);
                    domain.y_min = domain.y_min.min(o.low);
                    domain.y_max = domain.y_max.max(o.high);
                }
            }

            count += 1;
            if count == CHUNK_SIZE {
                if self.bounds_cache.is_empty() {
                    self.current_chunk_count = CHUNK_SIZE;
                }
                self.bounds_cache.push_back(domain);
                domain = AxisDomain {
                    x_min: f64::INFINITY,
                    x_max: f64::NEG_INFINITY,
                    y_min: f64::INFINITY,
                    y_max: f64::NEG_INFINITY,
                    ..Default::default()
                };
                count = 0;
            }
        }

        if count > 0 {
            if self.bounds_cache.is_empty() {
                self.current_chunk_count = count;
            }
            self.bounds_cache.push_back(domain);
            self.points_in_last_chunk = count;
        } else if !self.bounds_cache.is_empty() {
            self.points_in_last_chunk = CHUNK_SIZE;
        }

        self.suggested_spacing = if min_spacing == f64::INFINITY {
            1.0
        } else {
            min_spacing
        };
    }
}

impl PlotDataSource for StreamingDataSource {
    fn len(&self) -> usize {
        self.data.len()
    }
    fn suggested_x_spacing(&self) -> f64 {
        self.suggested_spacing
    }

    fn get_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        if self.bounds_cache.is_empty() {
            return None;
        }
        let mut b = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for d in &self.bounds_cache {
            b.0 = b.0.min(d.x_min);
            b.1 = b.1.max(d.x_max);
            b.2 = b.2.min(d.y_min);
            b.3 = b.3.max(d.y_max);
        }
        Some(b)
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        if self.data.is_empty() {
            return None;
        }
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut found = false;

        let first_chunk_size = self.current_chunk_count;
        let mut current_data_start = 0;

        for (i, chunk) in self.bounds_cache.iter().enumerate() {
            if chunk.x_max < x_min || chunk.x_min > x_max {
                current_data_start += if i == 0 { first_chunk_size } else { CHUNK_SIZE };
                continue;
            }

            if chunk.x_min >= x_min && chunk.x_max <= x_max {
                y_min = y_min.min(chunk.y_min);
                y_max = y_max.max(chunk.y_max);
                found = true;
                current_data_start += if i == 0 { first_chunk_size } else { CHUNK_SIZE };
            } else {
                let count = if i == 0 { first_chunk_size } else { CHUNK_SIZE };
                let end = current_data_start + count;

                for p in self.data.range(current_data_start..end) {
                    let x = match p {
                        PlotData::Point(pt) => pt.x,
                        PlotData::Ohlcv(o) => o.time,
                    };
                    if x >= x_min && x <= x_max {
                        match p {
                            PlotData::Point(pt) => {
                                y_min = y_min.min(pt.y);
                                y_max = y_max.max(pt.y);
                            }
                            PlotData::Ohlcv(o) => {
                                y_min = y_min.min(o.low);
                                y_max = y_max.max(o.high);
                            }
                        }
                        found = true;
                    }
                }
                current_data_start = end;
            }
        }

        if found {
            Some((y_min, y_max))
        } else {
            None
        }
    }

    fn iter_range(&self, x_min: f64, x_max: f64) -> Box<dyn Iterator<Item = PlotData> + '_> {
        let (s1, s2) = self.data.as_slices();

        let get_x = |p: &PlotData| match p {
            PlotData::Point(pt) => pt.x,
            PlotData::Ohlcv(o) => o.time,
        };

        let start1 = s1.partition_point(|p| get_x(p) < x_min);
        let end1 = s1.partition_point(|p| get_x(p) <= x_max);

        let start2 = s2.partition_point(|p| get_x(p) < x_min);
        let end2 = s2.partition_point(|p| get_x(p) <= x_max);

        Box::new(
            s1[start1..end1]
                .iter()
                .cloned()
                .chain(s2[start2..end2].iter().cloned()),
        )
    }

    fn iter_aggregated(
        &self,
        x_min: f64,
        x_max: f64,
        max_points: usize,
        gaps: Option<&GapIndex>,
    ) -> Box<dyn Iterator<Item = PlotData> + '_> {
        let _start_idx = self.data.partition_point(|p| {
            let x = match p {
                PlotData::Point(pt) => pt.x,
                PlotData::Ohlcv(o) => o.time,
            };
            x < x_min
        });
        let data: Vec<_> = self.iter_range(x_min, x_max).collect();
        Box::new(crate::decimation::decimate_min_max_slice(&data, max_points, gaps, None).into_iter())
    }

    fn add_data(&mut self, data: PlotData) {
        let x = match &data {
            PlotData::Point(p) => p.x,
            PlotData::Ohlcv(o) => o.time,
        };
        self.update_suggested_spacing(x);

        if self.data.len() >= self.capacity {
            self.data.pop_front();

            // Handle eviction from cache
            if self.current_chunk_count > 0 {
                self.current_chunk_count -= 1;
                if self.current_chunk_count == 0 {
                    // The first chunk is empty, remove it
                    self.bounds_cache.pop_front();
                    // The next chunk (if any) becomes the current first chunk
                    if !self.bounds_cache.is_empty() {
                        self.current_chunk_count = CHUNK_SIZE; // It was a full chunk
                    }
                } else {
                    // The first chunk has changed, we must recompute its bounds
                    // It corresponds to data[0..self.current_chunk_count]
                    if let Some(first_chunk_bounds) = self.bounds_cache.front_mut() {
                        *first_chunk_bounds = AxisDomain {
                            x_min: f64::INFINITY,
                            x_max: f64::NEG_INFINITY,
                            y_min: f64::INFINITY,
                            y_max: f64::NEG_INFINITY,
                            ..Default::default()
                        };
                        // Recompute bounds for the remaining points in this chunk
                        for i in 0..self.current_chunk_count {
                            let p = &self.data[i];
                            match p {
                                PlotData::Point(pt) => {
                                    first_chunk_bounds.x_min = first_chunk_bounds.x_min.min(pt.x);
                                    first_chunk_bounds.x_max = first_chunk_bounds.x_max.max(pt.x);
                                    first_chunk_bounds.y_min = first_chunk_bounds.y_min.min(pt.y);
                                    first_chunk_bounds.y_max = first_chunk_bounds.y_max.max(pt.y);
                                }
                                PlotData::Ohlcv(o) => {
                                    first_chunk_bounds.x_min = first_chunk_bounds.x_min.min(o.time);
                                    first_chunk_bounds.x_max =
                                        first_chunk_bounds.x_max.max(o.time + o.span);
                                    first_chunk_bounds.y_min = first_chunk_bounds.y_min.min(o.low);
                                    first_chunk_bounds.y_max = first_chunk_bounds.y_max.max(o.high);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.data.push_back(data.clone());

        if self.points_in_last_chunk == CHUNK_SIZE || self.bounds_cache.is_empty() {
            let mut domain = AxisDomain {
                x_min: x,
                x_max: x,
                y_min: f64::INFINITY,
                y_max: f64::NEG_INFINITY,
                ..Default::default()
            };
            match data {
                PlotData::Point(pt) => {
                    domain.y_min = pt.y;
                    domain.y_max = pt.y;
                }
                PlotData::Ohlcv(o) => {
                    domain.x_max = o.time + o.span;
                    domain.y_min = o.low;
                    domain.y_max = o.high;
                }
            }
            self.bounds_cache.push_back(domain);
            self.points_in_last_chunk = 1;
            if self.bounds_cache.len() == 1 {
                self.current_chunk_count = 1;
            }
        } else if let Some(last) = self.bounds_cache.back_mut() {
            match data {
                PlotData::Point(pt) => {
                    last.x_min = last.x_min.min(pt.x);
                    last.x_max = last.x_max.max(pt.x);
                    last.y_min = last.y_min.min(pt.y);
                    last.y_max = last.y_max.max(pt.y);
                }
                PlotData::Ohlcv(o) => {
                    last.x_min = last.x_min.min(o.time);
                    last.x_max = last.x_max.max(o.time + o.span);
                    last.y_min = last.y_min.min(o.low);
                    last.y_max = last.y_max.max(o.high);
                }
            }
            self.points_in_last_chunk += 1;
            if self.bounds_cache.len() == 1 {
                self.current_chunk_count += 1;
            }
        }
    }

    fn set_data(&mut self, data: Vec<PlotData>) {
        self.data = VecDeque::from(data);
        if self.data.len() > self.capacity {
            let to_remove = self.data.len() - self.capacity;
            for _ in 0..to_remove {
                self.data.pop_front();
            }
        }
        self.rebuild_cache();
    }
}

/// Default implementation using a simple Vec with chunked bounds cache.
pub struct VecDataSource {
    data: Vec<PlotData>,
    /// Pre-computed LOD levels: [0] = /2, [1] = /4, etc.
    lod_levels: Vec<Vec<PlotData>>,
    /// Reusing AxisDomain for chunk bounds
    bounds_cache: Vec<AxisDomain>,
    suggested_spacing: f64,
}

impl VecDataSource {
    pub fn new(data: Vec<PlotData>) -> Self {
        let mut inst = Self {
            data,
            lod_levels: Vec::new(),
            bounds_cache: Vec::new(),
            suggested_spacing: 1.0,
        };
        inst.rebuild_cache();
        inst.build_lod_pyramid();
        inst
    }

    fn build_lod_pyramid(&mut self) {
        self.lod_levels.clear();
        if self.data.len() < 2000 {
            return;
        }

        let mut next_level_capacity = self.data.len() / 2;
        // Build levels until we reach a small enough size (e.g. 100 points)
        while next_level_capacity >= 100 {
            let mut level = Vec::with_capacity(next_level_capacity);
            let source_to_read = if self.lod_levels.is_empty() {
                &self.data
            } else {
                self.lod_levels.last().unwrap()
            };

            for chunk in source_to_read.chunks(2) {
                if let Some(agg) = crate::decimation::aggregate_chunk(chunk) {
                    level.push(agg);
                }
            }

            next_level_capacity = level.len() / 2;
            self.lod_levels.push(level);
        }
    }

    fn rebuild_cache(&mut self) {
        self.bounds_cache.clear();
        let mut min_spacing = f64::INFINITY;
        let mut last_x: Option<f64> = None;

        for chunk in self.data.chunks(CHUNK_SIZE) {
            let mut domain = AxisDomain {
                x_min: f64::INFINITY,
                x_max: f64::NEG_INFINITY,
                y_min: f64::INFINITY,
                y_max: f64::NEG_INFINITY,
                ..Default::default()
            };
            for p in chunk {
                let x = self.get_x(p);
                if let Some(lx) = last_x {
                    let s = (x - lx).abs();
                    if s > f64::EPSILON && s < min_spacing {
                        min_spacing = s;
                    }
                }
                last_x = Some(x);

                match p {
                    PlotData::Point(pt) => {
                        domain.x_min = domain.x_min.min(pt.x);
                        domain.x_max = domain.x_max.max(pt.x);
                        domain.y_min = domain.y_min.min(pt.y);
                        domain.y_max = domain.y_max.max(pt.y);
                    }
                    PlotData::Ohlcv(o) => {
                        domain.x_min = domain.x_min.min(o.time);
                        domain.x_max = domain.x_max.max(o.time + o.span);
                        domain.y_min = domain.y_min.min(o.low);
                        domain.y_max = domain.y_max.max(o.high);
                    }
                }
            }
            self.bounds_cache.push(domain);
        }
        self.suggested_spacing = if min_spacing == f64::INFINITY {
            1.0
        } else {
            min_spacing
        };
    }

    fn get_x(&self, data: &PlotData) -> f64 {
        match data {
            PlotData::Point(p) => p.x,
            PlotData::Ohlcv(o) => o.time,
        }
    }
}

impl PlotDataSource for VecDataSource {
    fn len(&self) -> usize {
        self.data.len()
    }

    fn suggested_x_spacing(&self) -> f64 {
        self.suggested_spacing
    }

    fn get_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        if self.bounds_cache.is_empty() {
            return None;
        }
        let mut b = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for d in &self.bounds_cache {
            b.0 = b.0.min(d.x_min);
            b.1 = b.1.max(d.x_max);
            b.2 = b.2.min(d.y_min);
            b.3 = b.3.max(d.y_max);
        }
        Some(b)
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        if self.data.is_empty() {
            return None;
        }
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut found = false;

        for (i, chunk) in self.bounds_cache.iter().enumerate() {
            if chunk.x_max < x_min || chunk.x_min > x_max {
                continue;
            }
            if chunk.x_min >= x_min && chunk.x_max <= x_max {
                y_min = y_min.min(chunk.y_min);
                y_max = y_max.max(chunk.y_max);
                found = true;
                continue;
            }
            let start = i * CHUNK_SIZE;
            let end = (start + CHUNK_SIZE).min(self.data.len());
            for p in &self.data[start..end] {
                let x = self.get_x(p);
                if x >= x_min && x <= x_max {
                    match p {
                        PlotData::Point(pt) => {
                            y_min = y_min.min(pt.y);
                            y_max = y_max.max(pt.y);
                        }
                        PlotData::Ohlcv(o) => {
                            y_min = y_min.min(o.low);
                            y_max = y_max.max(o.high);
                        }
                    }
                    found = true;
                }
            }
        }
        if found {
            Some((y_min, y_max))
        } else {
            None
        }
    }

    fn iter_range(&self, x_min: f64, x_max: f64) -> Box<dyn Iterator<Item = PlotData> + '_> {
        // Find range via binary search (assumes sorted by X)
        let start_idx = self.data.partition_point(|p| self.get_x(p) < x_min);
        let end_idx = self.data.partition_point(|p| self.get_x(p) <= x_max);

        let mut start = start_idx;
        let mut end = end_idx;

        // Add padding points for line continuity at edges
        start = start.saturating_sub(1);
        end = (end + 1).min(self.data.len());

        Box::new(self.data[start..end].iter().cloned())
    }

    fn iter_aggregated(
        &self,
        x_min: f64,
        x_max: f64,
        max_points: usize,
        gaps: Option<&GapIndex>,
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
        gaps: Option<&GapIndex>,
    ) {
        output.clear();

        if let Some(g) = gaps {
            let intervals = g.split_range(x_min as i64, x_max as i64);
            if intervals.len() > 1 {
                let total_logical_span: f64 = intervals
                    .iter()
                    .map(|(s, e)| (g.to_logical(*e) - g.to_logical(*s)) as f64)
                    .sum();

                if total_logical_span > 0.0 {
                    for (s, e) in intervals {
                        let logical_span = (g.to_logical(e) - g.to_logical(s)) as f64;
                    let segment_max_points =
                        ((logical_span / total_logical_span) * max_points as f64).round() as usize;
                    if segment_max_points > 0 {
                            self.get_aggregated_data_lod(
                                s as f64,
                                e as f64,
                                segment_max_points,
                                output,
                            );
                        }
                    }
                    return;
                }
            }
        }

        self.get_aggregated_data_lod(x_min, x_max, max_points, output);
    }

    fn add_data(&mut self, data: PlotData) {
        self.data.push(data);
        if self.data.len() % CHUNK_SIZE == 1 {
            self.rebuild_cache();
        }
    }

    fn set_data(&mut self, data: Vec<PlotData>) {
        self.data = data;
        self.rebuild_cache();
        self.build_lod_pyramid();
    }
}

impl VecDataSource {
    /// Internal helper for LOD aggregation on a continuous range (no gaps inside).
    fn get_aggregated_data_lod(
        &self,
        x_min: f64,
        x_max: f64,
        max_points: usize,
        output: &mut Vec<PlotData>,
    ) {
        let start_idx = self.data.partition_point(|p| self.get_x(p) < x_min);
        let end_idx = self.data.partition_point(|p| self.get_x(p) <= x_max);
        let count = end_idx - start_idx;

        if count <= max_points {
            let start = start_idx.saturating_sub(1);
            let end = (end_idx + 1).min(self.data.len());
            output.extend_from_slice(&self.data[start..end]);
            return;
        }

        // Select LOD level
        let ratio = count as f64 / max_points as f64;
        let level_idx = (ratio.log2().ceil() as isize - 1).max(0) as usize;

        if level_idx < self.lod_levels.len() {
            let level_data = &self.lod_levels[level_idx];
            let l_start = level_data.partition_point(|p| self.get_x(p) < x_min);
            let l_end = level_data.partition_point(|p| self.get_x(p) <= x_max);
            let start = l_start.saturating_sub(1);
            let end = (l_end + 1).min(level_data.len());
            output.extend_from_slice(&level_data[start..end]);
        } else if !self.lod_levels.is_empty() {
            let level_data = self.lod_levels.last().unwrap();
            let l_start = level_data.partition_point(|p| self.get_x(p) < x_min);
            let l_end = level_data.partition_point(|p| self.get_x(p) <= x_max);
            let start = l_start.saturating_sub(1);
            let end = (l_end + 1).min(level_data.len());
            output.extend_from_slice(&level_data[start..end]);
        } else {
            // Fallback to dynamic binning
            let start = start_idx.saturating_sub(1);
            let end = (end_idx + 1).min(self.data.len());
            crate::decimation::decimate_min_max_slice_into(
                &self.data[start..end],
                max_points,
                output,
                None, // No gaps inside this segment
                None,
            );
        }
    }
}

use super::axis::AxisId;

#[derive(Clone)]
pub struct Series {
    pub id: String,
    pub plot:
        std::sync::Arc<parking_lot::RwLock<dyn crate::plot_types::PlotRenderer + Send + Sync>>,
    pub y_axis_id: AxisId,
    pub x_axis_id: AxisId,
}

impl Series {
    pub fn new(
        id: impl Into<String>,
        plot: impl crate::plot_types::PlotRenderer + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            plot: std::sync::Arc::new(parking_lot::RwLock::new(plot)),
            x_axis_id: AxisId(0),
            y_axis_id: AxisId(0),
        }
    }

    pub fn on_axis(mut self, y_axis_id: usize) -> Self {
        self.y_axis_id = AxisId(y_axis_id);
        self
    }
}
