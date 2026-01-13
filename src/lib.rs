#![recursion_limit = "512"]

//! # GPUI Chart
//!
//! A high-performance charting library for GPUI.
//!
//! ## Performance Features
//!
//! - **Zero-Copy Aggregation**: Optimized for Polars, directly accessing memory buffers.
//! - **Parallel Decimation**: Uses Rayon for multi-threaded M4, MinMax, and OHLCV aggregation.
//! - **Zero-Alloc Rendering**: Reuses buffers across frames to minimize GC pressure.
//! - **SIMD Transforms**: Vectorized coordinate transformation for high-speed rendering.

/// Internal modules (implementation details)
mod axis_renderer;
mod gutter_manager;
mod rendering;
mod utils;

/// Public modules (exposed to the user)
pub mod aggregation;
pub mod hybrid_source;
pub mod chart;
pub mod chart_view;
pub mod data_types;
pub mod navigator_view;
pub mod plot_types;
#[cfg(feature = "polars")]
pub mod polars_source;
pub mod scales;
pub mod simd;
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