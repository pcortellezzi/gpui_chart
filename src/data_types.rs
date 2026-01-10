// Data structures for the charting library

use gpui::Hsla;
use serde::{Deserialize, Serialize};
use d3rs::scale::{LinearScale, Scale};

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CandlestickConfig {
    pub up_wick_color: Hsla,
    pub down_wick_color: Hsla,
    pub up_body_color: Hsla,
    pub down_body_color: Hsla,
    pub up_border_color: Hsla,
    pub down_border_color: Hsla,
    pub body_width_pct: f32,
    pub wick_width_pct: f32,
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
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BarPlotConfig {
    pub color: Hsla,
    pub bar_width_pct: f32, // 0.0 to 1.0 relative to data spacing
}

impl Default for BarPlotConfig {
    fn default() -> Self {
        Self {
            color: gpui::blue(),
            bar_width_pct: 0.8,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum StepMode {
    Pre,  // Step occurs before the point
    Mid,  // Step occurs halfway between points
    Post, // Step occurs after the point
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Annotation {
    VLine { x: f64, color: Hsla, width: f32, label: Option<String> },
    HLine { y: f64, color: Hsla, width: f32, label: Option<String> },
    Rect { x_min: f64, x_max: f64, y_min: f64, y_max: f64, color: Hsla, fill: bool },
    Text { x: f64, y: f64, text: String, color: Hsla, font_size: f32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeatmapCell {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: Hsla,
    pub text: Option<String>, 
}

// Axis management types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    // Size in pixels (width for vertical axes, height for horizontal)
    pub size: gpui::Pixels,
}

/// État pour un axe unique (X ou Y).
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
        Self { min, max, cached_ticks: vec![], last_tick_domain: (0.0, 0.0), ..Default::default() }
    }

    pub fn span(&self) -> f64 {
        self.max - self.min
    }

    pub fn ticks(&mut self, count: usize) -> &[f64] {
        let (min, max) = self.clamped_bounds();
        let domain_changed = (min - self.last_tick_domain.0).abs() > (max - min) * 0.001 
                          || (max - self.last_tick_domain.1).abs() > (max - min) * 0.001;

        if domain_changed || self.cached_ticks.is_empty() {
            self.cached_ticks = LinearScale::new()
                .domain(min, max)
                .range(0.0, 1.0)
                .ticks(count);
            self.last_tick_domain = (min, max);
        }
        &self.cached_ticks
    }

    pub fn update_ticks_if_needed(&mut self, count: usize) {
        let _ = self.ticks(count);
    }

    /// Retourne les bornes clampées pour le rendu.
    pub fn clamped_bounds(&self) -> (f64, f64) {
        let mut c_min = self.min;
        let mut c_max = self.max;
        if let Some(l) = self.min_limit {
            if c_min < l { c_min = l; }
            if c_max < l { c_max = l; }
        }
        if let Some(l) = self.max_limit {
            if c_max > l { c_max = l; }
            if c_min > l { c_min = l; }
        }
        (c_min, c_max)
    }

    /// Zoom pur sans contraintes pour préserver le point pivot.
    pub fn zoom_at(&mut self, pivot_data: f64, pivot_pct: f64, factor: f64) {
        let new_span = self.span() * factor;
        self.min = pivot_data - new_span * pivot_pct;
        self.max = self.min + new_span;
    }

    /// Panoramique avec clamping optionnel (géré manuellement si besoin).
    pub fn pan(&mut self, delta_data: f64) {
        self.min += delta_data;
        self.max += delta_data;
    }

    /// Applique les limites de manière intelligente pour préserver le pivot.
    pub fn clamp(&mut self) {
        let (Some(min_l), Some(max_l)) = (self.min_limit, self.max_limit) else {
            // Cas où une seule ou aucune limite est présente : clamping simple
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
            return;
        };

        // Cas des deux limites : gestion intelligente du span
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
        } else {
            if self.min > min_l {
                self.min = min_l;
                self.max = min_l + current_span;
            } else if self.max < max_l {
                self.max = max_l;
                self.min = max_l - current_span;
            }
        }
    }
}

/// État consolidé pour une vue de graphique.
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
    pub fn width(&self) -> f64 { self.x_max - self.x_min }
    pub fn height(&self) -> f64 { self.y_max - self.y_min }
}

/// État partagé entre plusieurs graphiques (Crosshair, etc.).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SharedPlotState {
    pub hover_x: Option<f64>, // Coordonnée X en unités de données
    pub mouse_pos: Option<gpui::Point<gpui::Pixels>>, // Position écran globale
    pub active_chart_id: Option<gpui::EntityId>, // ID du graphique actuellement survolé
    pub is_dragging: bool,
    pub render_version: u64,
}

impl SharedPlotState {
    pub fn request_render(&mut self) {
        self.render_version = self.render_version.wrapping_add(1);
    }
}

#[derive(Clone)]
pub struct Ohlcv {
    pub time: f64,
    pub span: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ColorOp {
    Persistent(Hsla),
    OneShot(Hsla),
    Reset,
    None,
}

#[derive(Clone, Copy, PartialEq)]
pub struct PlotPoint {
    pub x: f64,
    pub y: f64,
    pub color_op: ColorOp,
}

#[derive(Clone)]
pub enum PlotData {
    Point(PlotPoint),
    Ohlcv(Ohlcv),
}

/// Trait abstraction for data providers.
/// This allows using Polars, RingBuffers, or simple Vecs as backends.
pub trait PlotDataSource: Send + Sync {
    /// Total bounds of the data (x_min, x_max, y_min, y_max)
    fn get_bounds(&self) -> Option<(f64, f64, f64, f64)>;

    /// Y-range within a specific X-window (for auto-scaling Y)
    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)>;

    /// Iterate over data within an X-window (for rendering with culling)
    /// Includes one point before and after the range for line continuity.
    fn iter_range(&self, x_min: f64, x_max: f64) -> Box<dyn Iterator<Item = PlotData> + '_>;

    /// Add a single data point
    fn add_data(&mut self, data: PlotData);

    /// Replace all data
    fn set_data(&mut self, data: Vec<PlotData>);

    /// Suggested X spacing between points (e.g. for Bar width calculation)
    fn suggested_x_spacing(&self) -> f64;

    /// Total number of points
    fn len(&self) -> usize;
    
    fn is_empty(&self) -> bool { self.len() == 0 }
}

use std::collections::VecDeque;

/// Data source optimized for real-time streaming with a fixed capacity.
pub struct StreamingDataSource {
    data: VecDeque<PlotData>,
    capacity: usize,
    bounds_cache: VecDeque<AxisDomain>, // Each element represents a chunk of CHUNK_SIZE points
    current_chunk_count: usize, // Points in the first chunk
    points_in_last_chunk: usize, // Points in the last chunk
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
            let last_x = match last { PlotData::Point(p) => p.x, PlotData::Ohlcv(o) => o.time };
            let s = (new_x - last_x).abs();
            if s > f64::EPSILON {
                if self.suggested_spacing == 1.0 { self.suggested_spacing = s; }
                else { self.suggested_spacing = self.suggested_spacing * 0.95 + s * 0.05; }
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
            x_min: f64::INFINITY, x_max: f64::NEG_INFINITY,
            y_min: f64::INFINITY, y_max: f64::NEG_INFINITY,
            ..Default::default()
        };

        for p in self.data.iter() {
            let x = match p { PlotData::Point(pt) => pt.x, PlotData::Ohlcv(o) => o.time };
            if let Some(lx) = last_x {
                let s = (x - lx).abs();
                if s > f64::EPSILON && s < min_spacing { min_spacing = s; }
            }
            last_x = Some(x);

            match p {
                PlotData::Point(pt) => {
                    domain.x_min = domain.x_min.min(pt.x); domain.x_max = domain.x_max.max(pt.x);
                    domain.y_min = domain.y_min.min(pt.y); domain.y_max = domain.y_max.max(pt.y);
                }
                PlotData::Ohlcv(o) => {
                    domain.x_min = domain.x_min.min(o.time); domain.x_max = domain.x_max.max(o.time + o.span);
                    domain.y_min = domain.y_min.min(o.low); domain.y_max = domain.y_max.max(o.high);
                }
            }
            
            count += 1;
            if count == CHUNK_SIZE {
                if self.bounds_cache.is_empty() { self.current_chunk_count = CHUNK_SIZE; }
                self.bounds_cache.push_back(domain);
                domain = AxisDomain {
                    x_min: f64::INFINITY, x_max: f64::NEG_INFINITY,
                    y_min: f64::INFINITY, y_max: f64::NEG_INFINITY,
                    ..Default::default()
                };
                count = 0;
            }
        }

        if count > 0 {
            if self.bounds_cache.is_empty() { self.current_chunk_count = count; }
            self.bounds_cache.push_back(domain);
            self.points_in_last_chunk = count;
        } else if !self.bounds_cache.is_empty() {
            self.points_in_last_chunk = CHUNK_SIZE;
        }

        self.suggested_spacing = if min_spacing == f64::INFINITY { 1.0 } else { min_spacing };
    }
}

impl PlotDataSource for StreamingDataSource {
    fn len(&self) -> usize { self.data.len() }
    fn suggested_x_spacing(&self) -> f64 { self.suggested_spacing }

    fn get_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        if self.bounds_cache.is_empty() { return None; }
        let mut b = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for d in &self.bounds_cache {
            b.0 = b.0.min(d.x_min); b.1 = b.1.max(d.x_max);
            b.2 = b.2.min(d.y_min); b.3 = b.3.max(d.y_max);
        }
        Some(b)
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        if self.data.is_empty() { return None; }
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
                    let x = match p { PlotData::Point(pt) => pt.x, PlotData::Ohlcv(o) => o.time };
                    if x >= x_min && x <= x_max {
                        match p {
                            PlotData::Point(pt) => { y_min = y_min.min(pt.y); y_max = y_max.max(pt.y); }
                            PlotData::Ohlcv(o) => { y_min = y_min.min(o.low); y_max = y_max.max(o.high); }
                        }
                        found = true;
                    }
                }
                current_data_start = end;
            }
        }

        if found { Some((y_min, y_max)) } else { None }
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
            s1[start1..end1].iter().cloned()
                .chain(s2[start2..end2].iter().cloned())
        )
    }

    fn add_data(&mut self, data: PlotData) {
        let x = match &data { PlotData::Point(p) => p.x, PlotData::Ohlcv(o) => o.time };
        self.update_suggested_spacing(x);

        if self.data.len() >= self.capacity {
            self.data.pop_front();
            self.current_chunk_count = self.current_chunk_count.saturating_sub(1);
            if self.current_chunk_count == 0 && !self.bounds_cache.is_empty() {
                self.bounds_cache.pop_front();
                self.current_chunk_count = CHUNK_SIZE;
            }
        }

        self.data.push_back(data.clone());
        
        if self.points_in_last_chunk == CHUNK_SIZE || self.bounds_cache.is_empty() {
            let mut domain = AxisDomain {
                x_min: x, x_max: x, y_min: f64::INFINITY, y_max: f64::NEG_INFINITY, ..Default::default()
            };
            match data {
                PlotData::Point(pt) => { domain.y_min = pt.y; domain.y_max = pt.y; }
                PlotData::Ohlcv(o) => { domain.x_max = o.time + o.span; domain.y_min = o.low; domain.y_max = o.high; }
            }
            self.bounds_cache.push_back(domain);
            self.points_in_last_chunk = 1;
            if self.bounds_cache.len() == 1 { self.current_chunk_count = 1; }
        } else if let Some(last) = self.bounds_cache.back_mut() {
            match data {
                PlotData::Point(pt) => { 
                    last.x_min = last.x_min.min(pt.x); last.x_max = last.x_max.max(pt.x);
                    last.y_min = last.y_min.min(pt.y); last.y_max = last.y_max.max(pt.y);
                }
                PlotData::Ohlcv(o) => {
                    last.x_min = last.x_min.min(o.time); last.x_max = last.x_max.max(o.time + o.span);
                    last.y_min = last.y_min.min(o.low); last.y_max = last.y_max.max(o.high);
                }
            }
            self.points_in_last_chunk += 1;
        }
    }

    fn set_data(&mut self, data: Vec<PlotData>) {
        self.data = VecDeque::from(data);
        if self.data.len() > self.capacity {
            let to_remove = self.data.len() - self.capacity;
            for _ in 0..to_remove { self.data.pop_front(); }
        }
        self.rebuild_cache();
    }
}

/// Default implementation using a simple Vec with chunked bounds cache.
pub struct VecDataSource {
    data: Vec<PlotData>,
    bounds_cache: Vec<AxisDomain>, // Reusing AxisDomain for chunk bounds
    suggested_spacing: f64,
}

impl VecDataSource {
    pub fn new(data: Vec<PlotData>) -> Self {
        let mut inst = Self { data, bounds_cache: Vec::new(), suggested_spacing: 1.0 };
        inst.rebuild_cache();
        inst
    }

    fn rebuild_cache(&mut self) {
        self.bounds_cache.clear();
        let mut min_spacing = f64::INFINITY;
        let mut last_x: Option<f64> = None;

        for chunk in self.data.chunks(CHUNK_SIZE) {
            let mut domain = AxisDomain {
                x_min: f64::INFINITY, x_max: f64::NEG_INFINITY,
                y_min: f64::INFINITY, y_max: f64::NEG_INFINITY,
                ..Default::default()
            };
            for p in chunk {
                let x = self.get_x(p);
                if let Some(lx) = last_x {
                    let s = (x - lx).abs();
                    if s > f64::EPSILON && s < min_spacing { min_spacing = s; }
                }
                last_x = Some(x);

                match p {
                    PlotData::Point(pt) => {
                        domain.x_min = domain.x_min.min(pt.x); domain.x_max = domain.x_max.max(pt.x);
                        domain.y_min = domain.y_min.min(pt.y); domain.y_max = domain.y_max.max(pt.y);
                    }
                    PlotData::Ohlcv(o) => {
                        domain.x_min = domain.x_min.min(o.time); domain.x_max = domain.x_max.max(o.time + o.span);
                        domain.y_min = domain.y_min.min(o.low); domain.y_max = domain.y_max.max(o.high);
                    }
                }
            }
            self.bounds_cache.push(domain);
        }
        self.suggested_spacing = if min_spacing == f64::INFINITY { 1.0 } else { min_spacing };
    }

    fn get_x(&self, data: &PlotData) -> f64 {
        match data {
            PlotData::Point(p) => p.x,
            PlotData::Ohlcv(o) => o.time,
        }
    }
}

impl PlotDataSource for VecDataSource {
    fn len(&self) -> usize { self.data.len() }

    fn suggested_x_spacing(&self) -> f64 {
        self.suggested_spacing
    }

    fn get_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        if self.bounds_cache.is_empty() { return None; }
        let mut b = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for d in &self.bounds_cache {
            b.0 = b.0.min(d.x_min); b.1 = b.1.max(d.x_max);
            b.2 = b.2.min(d.y_min); b.3 = b.3.max(d.y_max);
        }
        Some(b)
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        if self.data.is_empty() { return None; }
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut found = false;

        for (i, chunk) in self.bounds_cache.iter().enumerate() {
            if chunk.x_max < x_min || chunk.x_min > x_max { continue; }
            if chunk.x_min >= x_min && chunk.x_max <= x_max {
                y_min = y_min.min(chunk.y_min); y_max = y_max.max(chunk.y_max);
                found = true; continue;
            }
            let start = i * CHUNK_SIZE;
            let end = (start + CHUNK_SIZE).min(self.data.len());
            for p in &self.data[start..end] {
                let x = self.get_x(p);
                if x >= x_min && x <= x_max {
                    match p {
                        PlotData::Point(pt) => { y_min = y_min.min(pt.y); y_max = y_max.max(pt.y); }
                        PlotData::Ohlcv(o) => { y_min = y_min.min(o.low); y_max = y_max.max(o.high); }
                    }
                    found = true;
                }
            }
        }
        if found { Some((y_min, y_max)) } else { None }
    }

    fn iter_range(&self, x_min: f64, x_max: f64) -> Box<dyn Iterator<Item = PlotData> + '_> {
        // Find range via binary search (assumes sorted by X)
        let start_idx = self.data.partition_point(|p| self.get_x(p) < x_min);
        let end_idx = self.data.partition_point(|p| self.get_x(p) <= x_max);
        
        let start = start_idx.saturating_sub(1);
        let end = (end_idx + 1).min(self.data.len());
        
        Box::new(self.data[start..end].iter().cloned())
    }

    fn add_data(&mut self, data: PlotData) {
        self.data.push(data);
        if self.data.len() % CHUNK_SIZE == 1 { self.rebuild_cache(); } // Simple re-trigger for now
    }

    fn set_data(&mut self, data: Vec<PlotData>) {
        self.data = data;
        self.rebuild_cache();
    }
}

#[derive(Clone)]
pub struct Series {
    pub id: String,
    pub plot: std::sync::Arc<parking_lot::RwLock<dyn crate::plot_types::PlotRenderer + Send + Sync>>,
    pub y_axis_id: AxisId,
    pub x_axis_id: AxisId,
}

impl Series {
    pub fn new(id: impl Into<String>, plot: impl crate::plot_types::PlotRenderer + 'static) -> Self {
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
