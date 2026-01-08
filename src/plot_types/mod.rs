// Plot types module

pub mod candlestick;
pub mod line;

pub use candlestick::CandlestickPlot;
pub use line::LinePlot;

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

    /// Clear data before a certain time
    fn clear_before(&mut self, _before: f64) {}

    /// Get min/max bounds for auto-fitting (x_min, x_max, y_min, y_max)
    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        None
    }

    /// Update configuration from JSON value.
    /// Returns true if successful.
    fn update_config(&mut self, _config: serde_json::Value) -> bool {
        false
    }
}
