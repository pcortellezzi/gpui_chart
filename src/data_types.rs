// Data structures for the charting library

use gpui::Hsla;
use serde::{Deserialize, Serialize};

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
}

impl AxisRange {
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max, ..Default::default() }
    }

    pub fn span(&self) -> f64 {
        self.max - self.min
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

    /// Applique les limites de manière stricte (utile pour le panoramique).
    pub fn clamp(&mut self) {
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
#[derive(Clone, Debug, Default)]
pub struct SharedPlotState {
    pub hover_x: Option<f64>, // Coordonnée X en unités de données
    pub mouse_pos: Option<gpui::Point<gpui::Pixels>>, // Position écran globale
    pub active_chart_id: Option<gpui::EntityId>, // ID du graphique actuellement survolé
    pub is_dragging: bool,
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

#[derive(Clone)]
pub struct Series {
    pub id: String,
    pub plot: std::rc::Rc<std::cell::RefCell<dyn crate::plot_types::PlotRenderer + Send + Sync>>,
    pub y_axis_id: AxisId,
    pub x_axis_id: AxisId,
}