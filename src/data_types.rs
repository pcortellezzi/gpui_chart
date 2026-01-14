// Data structures for the charting library

use d3rs::scale::{LinearScale, Scale};
use gpui::Hsla;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub struct LinePlotConfig {
    pub color: Hsla,
    pub line_width: f32,
}

impl Default for LinePlotConfig {
    fn default() -> Self {
        Self {
            color: gpui::blue(),
            line_width: 2.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandlestickConfig {
    pub up_wick_color: Hsla,
    pub down_wick_color: Hsla,
    pub up_body_color: Hsla,
    pub down_body_color: Hsla,
    pub up_border_color: Hsla,
    pub down_border_color: Hsla,
    pub body_width_pct: f32,
    pub wick_width_pct: f32,
    pub border_thickness_px: f32,
}

impl Default for CandlestickConfig {
    fn default() -> Self {
        let green = gpui::green();
        let red = gpui::red();
        Self {
            up_wick_color: green,
            down_wick_color: red,
            up_body_color: green,
            down_body_color: red,
            up_border_color: green,
            down_border_color: red,
            body_width_pct: 0.8,
            wick_width_pct: 0.1,
            border_thickness_px: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaPlotConfig {
    pub line_color: Hsla,
    pub fill_color: Hsla,
    pub line_width: f32,
}

impl Default for AreaPlotConfig {
    fn default() -> Self {
        Self {
            line_color: gpui::blue(),
            fill_color: gpui::blue().alpha(0.3),
            line_width: 2.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BarPlotConfig {
    pub color: Hsla,
    /// 0.0 to 1.0 relative to data spacing
    pub bar_width_pct: f32,
}

impl Default for BarPlotConfig {
    fn default() -> Self {
        Self {
            color: gpui::blue(),
            bar_width_pct: 0.8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepMode {
    /// Step occurs before the point
    Pre,
    /// Step occurs halfway between points
    Mid,
    /// Step occurs after the point
    Post,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepLinePlotConfig {
    pub color: Hsla,
    pub line_width: f32,
    pub mode: StepMode,
}

impl Default for StepLinePlotConfig {
    fn default() -> Self {
        Self {
            color: gpui::blue(),
            line_width: 2.0,
            mode: StepMode::Post,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Annotation {
    VLine {
        x: f64,
        color: Hsla,
        width: f32,
        label: Option<String>,
    },
    HLine {
        y: f64,
        color: Hsla,
        width: f32,
        label: Option<String>,
    },
    Rect {
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        color: Hsla,
        fill: bool,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        color: Hsla,
        font_size: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct HeatmapCell {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: Hsla,
    pub text: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum AxisFormat {
    Numeric,
    Time(TimeUnit),
}

impl Default for AxisFormat {
    fn default() -> Self {
        Self::Numeric
    }
}

/// Axis management types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct AxisId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AxisEdge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone)]
pub struct ChartAxis<T> {
    pub axis: gpui::Entity<T>,
    pub edge: AxisEdge,
    /// Size in pixels (width for vertical axes, height for horizontal)
    pub size: gpui::Pixels,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum LegendPosition {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopCenter,
    BottomCenter,
    Custom(gpui::Point<gpui::Pixels>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Orientation {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegendConfig {
    pub enabled: bool,
    pub position: LegendPosition,
    pub orientation: Orientation,
}

impl Default for LegendConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: LegendPosition::TopLeft,
            orientation: Orientation::Vertical,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InertiaConfig {
    pub enabled: bool,
    pub friction: f64,
    pub sensitivity: f64,
    pub stop_threshold: std::time::Duration,
}

impl Default for InertiaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            friction: 0.80,
            sensitivity: 1.0,
            stop_threshold: std::time::Duration::from_millis(150),
        }
    }
}

/// State for a single axis (X or Y).
#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
pub struct AxisRange {
    pub min: f64,
    pub max: f64,
    pub min_limit: Option<f64>,
    pub max_limit: Option<f64>,
    pub cached_ticks: Vec<f64>,
    pub last_tick_domain: (f64, f64),
}

impl AxisRange {
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            cached_ticks: vec![],
            last_tick_domain: (0.0, 0.0),
            ..Default::default()
        }
    }

    pub fn span(&self) -> f64 {
        self.max - self.min
    }

    pub fn ticks(&mut self, count: usize, gaps: Option<&GapIndex>) -> &[f64] {
        let (min, max) = self.clamped_bounds();
        let domain_changed = (min - self.last_tick_domain.0).abs() > (max - min) * 0.001
            || (max - self.last_tick_domain.1).abs() > (max - min) * 0.001;

        if domain_changed || self.cached_ticks.is_empty() {
            if let Some(gaps) = gaps {
                let l_min = gaps.to_logical(min as i64) as f64;
                let l_max = gaps.to_logical(max as i64) as f64;
                let logical_ticks = LinearScale::new()
                    .domain(l_min, l_max)
                    .range(0.0, 1.0)
                    .ticks(count);

                let mut cursor = gaps.cursor();
                self.cached_ticks = logical_ticks
                    .into_iter()
                    .map(|t| cursor.to_real(t as i64) as f64)
                    .filter(|&t| !gaps.is_inside(t as i64))
                    .collect();
            } else {
                self.cached_ticks = LinearScale::new()
                    .domain(min, max)
                    .range(0.0, 1.0)
                    .ticks(count);
            }
            self.last_tick_domain = (min, max);
        }
        &self.cached_ticks
    }

    pub fn update_ticks_if_needed(&mut self, count: usize, gaps: Option<&GapIndex>) {
        let _ = self.ticks(count, gaps);
    }

    /// Returns the clamped bounds for rendering.
    pub fn clamped_bounds(&self) -> (f64, f64) {
        let mut c_min = self.min;
        let mut c_max = self.max;
        if let Some(l) = self.min_limit {
            if c_min < l {
                c_min = l;
            }
            if c_max < l {
                c_max = l;
            }
        }
        if let Some(l) = self.max_limit {
            if c_max > l {
                c_max = l;
            }
            if c_min > l {
                c_min = l;
            }
        }
        (c_min, c_max)
    }

    /// Pure zoom without constraints to preserve the pivot point.
    pub fn zoom_at(&mut self, pivot_data: f64, pivot_pct: f64, factor: f64) {
        let new_span = self.span() * factor;
        self.min = pivot_data - new_span * pivot_pct;
        self.max = self.min + new_span;
        self.cached_ticks.clear();
    }

    /// Pan with optional clamping (handled manually if needed).
    pub fn pan(&mut self, delta_data: f64) {
        self.min += delta_data;
        self.max += delta_data;
        self.cached_ticks.clear();
    }

    /// Applies limits intelligently to preserve the pivot.
    pub fn clamp(&mut self) {
        let (Some(min_l), Some(max_l)) = (self.min_limit, self.max_limit) else {
            // Case where only one or no limit is present: simple clamping
            if let Some(l) = self.min_limit {
                if self.min < l {
                    let s = self.span();
                    self.min = l;
                    self.max = l + s;
                }
            }
            if let Some(l) = self.max_limit {
                if self.max > l {
                    let s = self.span();
                    self.max = l;
                    self.min = l - s;
                }
            }
            self.cached_ticks.clear();
            return;
        };

        // Case of two limits: intelligent span management
        let limit_span = max_l - min_l;
        let current_span = self.span();

        if current_span <= limit_span {
            if self.min < min_l {
                self.min = min_l;
                self.max = min_l + current_span;
            } else if self.max > max_l {
                self.max = max_l;
                self.min = max_l - current_span;
            }
        } else if self.min > min_l {
            self.min = min_l;
            self.max = min_l + current_span;
        } else if self.max < max_l {
            self.max = max_l;
            self.min = max_l - current_span;
        }
        self.cached_ticks.clear();
    }
}

/// Consolidated state for a chart view.
#[derive(Clone, Debug, Default)]
pub struct AxisDomain {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub x_min_limit: Option<f64>,
    pub x_max_limit: Option<f64>,
    pub y_min_limit: Option<f64>,
    pub y_max_limit: Option<f64>,
}

impl AxisDomain {
    pub fn width(&self) -> f64 {
        self.x_max - self.x_min
    }
    pub fn height(&self) -> f64 {
        self.y_max - self.y_min
    }
}

use crate::gaps::GapIndex;
use std::sync::Arc;

/// Shared state between multiple charts (Crosshair, etc.).
#[derive(Debug, Default)]
pub struct SharedPlotState {
    /// X coordinate in data units
    pub hover_x: Option<f64>,
    /// Global screen position
    pub mouse_pos: Option<gpui::Point<gpui::Pixels>>,
    /// ID of the chart currently hovered
    pub active_chart_id: Option<gpui::EntityId>,
    pub is_dragging: bool,
    pub debug_mode: bool,
    pub theme: crate::theme::ChartTheme,
    pub render_version: u64,

    pub box_zoom_start: Option<gpui::Point<gpui::Pixels>>,
    pub box_zoom_current: Option<gpui::Point<gpui::Pixels>>,

    /// Optional gap index for X axis compression
    pub gap_index: Option<Arc<GapIndex>>,

    /// Time taken by paint for each pane (ID -> nanoseconds)
    pub pane_paint_times:
        std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, u64>>>,
}

impl SharedPlotState {
    pub fn request_render(&mut self) {
        self.render_version = self.render_version.wrapping_add(1);
    }

    pub fn total_paint_nanos(&self) -> u64 {
        self.pane_paint_times.read().values().sum()
    }
}

impl PartialEq for SharedPlotState {
    fn eq(&self, other: &Self) -> bool {
        self.hover_x == other.hover_x
            && self.mouse_pos == other.mouse_pos
            && self.active_chart_id == other.active_chart_id
            && self.is_dragging == other.is_dragging
            && self.debug_mode == other.debug_mode
            && self.theme == other.theme
            && self.render_version == other.render_version
            && self.box_zoom_start == other.box_zoom_start
            && self.box_zoom_current == other.box_zoom_current
            && self.gap_index.as_ref().map(|g| g.segments())
                == other.gap_index.as_ref().map(|g| g.segments())
    }
}

impl Clone for SharedPlotState {
    fn clone(&self) -> Self {
        Self {
            hover_x: self.hover_x,
            mouse_pos: self.mouse_pos,
            active_chart_id: self.active_chart_id,
            is_dragging: self.is_dragging,
            debug_mode: self.debug_mode,
            theme: self.theme.clone(),
            render_version: self.render_version,
            box_zoom_start: self.box_zoom_start,
            box_zoom_current: self.box_zoom_current,
            gap_index: self.gap_index.clone(),
            pane_paint_times: self.pane_paint_times.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ohlcv {
    pub time: f64,
    pub span: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ColorOp {
    Persistent(Hsla),
    OneShot(Hsla),
    Reset,
    #[default]
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PlotPoint {
    pub x: f64,
    pub y: f64,
    pub color_op: ColorOp,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlotData {
    Point(PlotPoint),
    Ohlcv(Ohlcv),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AggregationMode {
    MinMax, // 2 points par bin
    #[default]
    M4, // 4 points par bin (First, Min, Max, Last)
    LTTB,   // Largest-Triangle-Three-Buckets
}

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
        Box::new(crate::aggregation::decimate_min_max_slice(&data, max_points, gaps).into_iter())
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

use std::collections::VecDeque;

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
                if let Some(agg) = crate::aggregation::aggregate_chunk(chunk) {
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
            crate::aggregation::decimate_min_max_slice_into(
                &self.data[start..end],
                max_points,
                output,
                None, // No gaps inside this segment
            );
        }
    }
}

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
