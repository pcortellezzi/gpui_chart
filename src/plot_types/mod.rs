// Plot types module

pub mod area;
pub mod bar;
pub mod candlestick;
pub mod line;
pub mod step_line;
pub mod annotation;
pub mod heatmap;

pub use area::AreaPlot;
pub use bar::BarPlot;
pub use candlestick::CandlestickPlot;
pub use line::LinePlot;
pub use step_line::StepLinePlot;
pub use annotation::AnnotationPlot;
pub use heatmap::HeatmapPlot;

use crate::data_types::PlotData;
use crate::transform::PlotTransform;
use gpui::*;

/// Trait for rendering plot types
pub trait PlotRenderer: Send + Sync {
    fn render(
        &self,
        window: &mut Window,
        transform: &PlotTransform,
        series_id: &str,
    );

    /// Add data (default no-op)
    fn add_data(&mut self, _data: PlotData) {}

    /// Set all data
    fn set_data(&mut self, _data: Vec<PlotData>) {}

    /// Get min/max bounds for auto-fitting (x_min, x_max, y_min, y_max)
    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        None
    }

    /// Get Y min/max range within a specific X range.
    fn get_y_range(&self, _x_min: f64, _x_max: f64) -> Option<(f64, f64)> {
        None
    }
}
