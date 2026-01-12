#![recursion_limit = "512"]

/// Internal modules (implementation details)
mod axis_renderer;
mod gutter_manager;
mod rendering;
mod utils;

/// Public modules (exposed to the user)
pub mod aggregation;
pub mod chart;
pub mod chart_view;
pub mod data_types;
pub mod navigator_view;
pub mod plot_types;
#[cfg(feature = "polars")]
pub mod polars_source;
pub mod scales;
pub mod theme;
pub mod transform;
pub mod view_controller;

// Re-exports for convenience
pub use chart::{AxisState, Chart, PaneState};
pub use chart_view::ChartView;
pub use data_types::{AxisDomain, Ohlcv, PlotData, Series};
pub use navigator_view::NavigatorView;
pub use plot_types::*;
pub use scales::ChartScale;
pub use theme::ChartTheme;
pub use transform::PlotTransform;
