//! Chart Model
//!
//! This module defines the `Chart` entity, which serves as the Single Source of Truth
//! for the chart state. It holds the data (panes, series, axes) and the business logic
//! to manipulate this data (add/remove panes, move series, etc.).
//!
//! As a GPUI Entity, it can be updated from any context and notifies its observers
//! (like `ChartView`) of any changes.

use crate::data_types::{AxisEdge, AxisFormat, AxisId, AxisRange, SharedPlotState};
use crate::theme::ChartTheme;
use crate::Series;
use gpui::*;
use std::collections::HashSet;

#[derive(Clone)]
pub struct AxisState {
    pub entity: Entity<AxisRange>,
    pub edge: AxisEdge,
    pub size: Pixels,
    pub label: String,
    pub format: AxisFormat,
    pub min_label_spacing: Pixels,
}

impl AxisState {
    pub fn new(entity: Entity<AxisRange>, edge: AxisEdge, size: Pixels, label: String) -> Self {
        Self {
            entity,
            edge,
            size,
            label,
            format: AxisFormat::Numeric,
            min_label_spacing: px(20.0),
        }
    }
}

#[derive(Clone)]
pub struct PaneState {
    pub id: String,
    pub weight: f32,
    pub y_axes: Vec<AxisState>,
    pub series: Vec<Series>,
    pub hidden_series: HashSet<String>,

    /// Local interaction states
    pub drag_start: Option<Point<Pixels>>,
    pub initial_drag_start: Option<Point<Pixels>>,
    pub drag_button: Option<MouseButton>,
    pub velocity: Point<f64>,
    pub last_drag_time: Option<std::time::Instant>,
}

impl PaneState {
    pub fn new(id: String, weight: f32) -> Self {
        Self {
            id,
            weight,
            y_axes: vec![],
            series: vec![],
            hidden_series: HashSet::new(),
            drag_start: None,
            initial_drag_start: None,
            drag_button: None,
            velocity: Point::default(),
            last_drag_time: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AxisDragInfo {
    pub pane_id: Option<String>,
    pub axis_idx: usize,
    pub is_y: bool,
    pub button: MouseButton,
    pub pivot_pct: f64,
}

pub struct Chart {
    pub shared_x_axis: Entity<AxisRange>,
    pub shared_state: Entity<SharedPlotState>,
    pub panes: Vec<PaneState>,
    pub x_axes: Vec<AxisState>,
    pub theme: ChartTheme,

    pub dragging_splitter: Option<usize>,
    pub dragging_axis: Option<AxisDragInfo>,
    pub last_mouse_pos: Option<Point<Pixels>>,
    pub last_mouse_y: Option<Pixels>,
}

impl Chart {
    pub fn new(
        shared_x_axis: Entity<AxisRange>,
        shared_state: Entity<SharedPlotState>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&shared_x_axis, |_, _, cx| cx.notify()).detach();
        cx.observe(&shared_state, |_, _, cx| cx.notify()).detach();

        let theme = ChartTheme::default();
        shared_state.update(cx, |s, _| s.theme = theme.clone());

        Self {
            shared_x_axis,
            shared_state,
            panes: vec![],
            x_axes: vec![],
            theme,
            dragging_splitter: None,
            dragging_axis: None,
            last_mouse_pos: None,
            last_mouse_y: None,
        }
    }

    pub fn add_pane_at(&mut self, idx: usize, weight: f32, cx: &mut Context<Self>) {
        let id = format!(
            "new_pane_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        let mut ps = PaneState::new(id.clone(), weight);
        let default_y = cx.new(|_| AxisRange::new(0.0, 100.0));
        ps.y_axes.push(AxisState::new(
            default_y,
            AxisEdge::Right,
            px(60.0),
            "New".to_string(),
        ));

        if idx >= self.panes.len() {
            self.panes.push(ps);
        } else {
            self.panes.insert(idx, ps);
        }
        self.notify_render(cx);
    }

    pub fn remove_pane_by_id(&mut self, id: String, cx: &mut Context<Self>) {
        self.panes.retain(|ps| ps.id != id);
        self.notify_render(cx);
    }

    pub fn remove_series_by_id(&mut self, id: String, cx: &mut Context<Self>) {
        for pane in &mut self.panes {
            pane.series.retain(|s| s.id != id);
        }
        self.notify_render(cx);
    }

    pub fn move_pane_up(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx > 0 && idx < self.panes.len() {
            self.panes.swap(idx, idx - 1);
            self.notify_render(cx);
        }
    }

    pub fn move_pane_down(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.panes.len() - 1 {
            self.panes.swap(idx, idx + 1);
            self.notify_render(cx);
        }
    }

    pub fn move_series(
        &mut self,
        from_idx: usize,
        to_idx: usize,
        series_id: &str,
        cx: &mut Context<Self>,
    ) {
        if from_idx == to_idx || from_idx >= self.panes.len() || to_idx >= self.panes.len() {
            return;
        }

        // 1. Find and extract the series (clone needed as we modify self.panes)
        let mut series_to_move = None;
        if let Some(src_pane) = self.panes.get_mut(from_idx) {
            if let Some(pos) = src_pane.series.iter().position(|s| s.id == series_id) {
                series_to_move = Some(src_pane.series.remove(pos));
            }
        }

        // 2. Insert into destination
        if let Some(series) = series_to_move {
            if let Some(dst_pane) = self.panes.get_mut(to_idx) {
                dst_pane.series.push(series);
            }
        }

        self.notify_render(cx);
    }

    pub fn toggle_series_isolation(
        &mut self,
        pane_idx: usize,
        series_id: &str,
        cx: &mut Context<Self>,
    ) {
        if let Some(ps) = self.panes.get_mut(pane_idx) {
            let mut current_y = 0;
            // Find current axis
            if let Some(s) = ps.series.iter().find(|s| s.id == series_id) {
                current_y = s.y_axis_id.0;
            }

            if current_y == 0 {
                // Isolate
                let mut s_min = 0.0;
                let mut s_max = 100.0;
                if let Some(s) = ps.series.iter().find(|s| s.id == series_id) {
                    if let Some((_, _, ymin, ymax)) = s.plot.read().get_min_max() {
                        s_min = ymin;
                        s_max = ymax;
                    }
                }
                let new_axis = cx.new(|_| AxisRange::new(s_min, s_max));
                cx.observe(&new_axis, |_, _, cx| cx.notify()).detach();

                ps.y_axes.push(AxisState::new(
                    new_axis.clone(),
                    AxisEdge::Right,
                    px(60.0),
                    series_id.to_string(),
                ));
                let new_axis_id = AxisId(ps.y_axes.len() - 1);

                if let Some(s) = ps.series.iter_mut().find(|s| s.id == series_id) {
                    s.y_axis_id = new_axis_id;
                }
            } else {
                // Reintegrate to Axis 0
                if let Some(s) = ps.series.iter_mut().find(|s| s.id == series_id) {
                    s.y_axis_id = AxisId(0);
                }

                // Clean up orphaned axes
                let mut axes_to_remove = Vec::new();
                for i in 1..ps.y_axes.len() {
                    let in_use = ps.series.iter().any(|s| s.y_axis_id.0 == i);
                    if !in_use {
                        axes_to_remove.push(i);
                    }
                }
                axes_to_remove.sort_by(|a, b| b.cmp(a));
                for idx in axes_to_remove {
                    ps.y_axes.remove(idx);
                    for s in ps.series.iter_mut() {
                        if s.y_axis_id.0 > idx {
                            s.y_axis_id.0 -= 1;
                        }
                    }
                }
            }
            self.notify_render(cx);
        }
    }

    pub fn flip_axis_edge(
        &mut self,
        pane_id: Option<String>,
        axis_idx: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = pane_id {
            if let Some(ps) = self.panes.iter_mut().find(|ps| ps.id == id) {
                if let Some(axis) = ps.y_axes.get_mut(axis_idx) {
                    axis.edge = match axis.edge {
                        AxisEdge::Left => AxisEdge::Right,
                        AxisEdge::Right => AxisEdge::Left,
                        _ => axis.edge,
                    };
                }
            }
        } else {
            if let Some(axis) = self.x_axes.get_mut(axis_idx) {
                axis.edge = match axis.edge {
                    AxisEdge::Top => AxisEdge::Bottom,
                    AxisEdge::Bottom => AxisEdge::Top,
                    _ => axis.edge,
                };
            }
        }
        self.notify_render(cx);
    }

    pub fn set_theme(&mut self, theme: ChartTheme, cx: &mut Context<Self>) {
        self.theme = theme.clone();
        self.shared_state.update(cx, |s, _| s.theme = theme);
        self.notify_render(cx);
    }

    pub fn set_x_axis_format(
        &mut self,
        axis_idx: usize,
        format: crate::data_types::AxisFormat,
        cx: &mut Context<Self>,
    ) {
        if let Some(axis) = self.x_axes.get_mut(axis_idx) {
            axis.format = format;
            self.notify_render(cx);
        }
    }

    pub fn set_y_axis_format(
        &mut self,
        pane_idx: usize,
        axis_idx: usize,
        format: crate::data_types::AxisFormat,
        cx: &mut Context<Self>,
    ) {
        if let Some(pane) = self.panes.get_mut(pane_idx) {
            if let Some(axis) = pane.y_axes.get_mut(axis_idx) {
                axis.format = format;
                self.notify_render(cx);
            }
        }
    }

    pub fn set_x_axis_min_spacing(
        &mut self,
        axis_idx: usize,
        spacing: Pixels,
        cx: &mut Context<Self>,
    ) {
        if let Some(axis) = self.x_axes.get_mut(axis_idx) {
            axis.min_label_spacing = spacing;
            self.notify_render(cx);
        }
    }

    pub fn set_y_axis_min_spacing(
        &mut self,
        pane_idx: usize,
        axis_idx: usize,
        spacing: Pixels,
        cx: &mut Context<Self>,
    ) {
        if let Some(pane) = self.panes.get_mut(pane_idx) {
            if let Some(axis) = pane.y_axes.get_mut(axis_idx) {
                axis.min_label_spacing = spacing;
                self.notify_render(cx);
            }
        }
    }

    pub fn notify_render(&self, cx: &mut Context<Self>) {
        self.shared_state.update(cx, |s, _| s.request_render());
        cx.notify();
    }
}
