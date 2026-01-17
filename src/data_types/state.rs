use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::gaps::GapIndex;

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
    pub crosshair_enabled: bool,
    pub theme: crate::theme::ChartTheme,

    pub box_zoom_start: Option<gpui::Point<gpui::Pixels>>,
    pub box_zoom_current: Option<gpui::Point<gpui::Pixels>>,

    /// Optional gap index for X axis compression
    pub gap_index: Option<Arc<GapIndex>>,

    /// Time taken by paint for each pane (ID -> nanoseconds)
    pub pane_paint_times:
        std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, u64>>>,
}

impl SharedPlotState {
    pub fn total_paint_nanos(&self) -> u64 {
        self.pane_paint_times.read().values().sum()
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
            crosshair_enabled: self.crosshair_enabled,
            theme: self.theme.clone(),
            box_zoom_start: self.box_zoom_start,
            box_zoom_current: self.box_zoom_current,
            gap_index: self.gap_index.clone(),
            pane_paint_times: self.pane_paint_times.clone(),
        }
    }
}
