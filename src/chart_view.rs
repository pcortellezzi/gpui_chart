// ChartView implementation

use crate::data_types::{AxisDomain, AxisRange, Series, SharedPlotState};
use crate::rendering::create_axis_tag;
use adabraka_ui::util::PixelsExt;
use d3rs::scale::{LinearScale, Scale};
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tracing::info;

actions!(
    gpui_chart,
    [Init, PanLeft, PanRight, PanUp, PanDown, ZoomIn, ZoomOut, ResetView]
);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LegendPosition {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopCenter,
    BottomCenter,
    Custom(Point<Pixels>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Orientation {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegendConfig {
    pub enabled: bool,
    pub position: LegendPosition,
    pub orientation: Orientation,
    pub offset: Point<Pixels>,
}

impl Default for LegendConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: LegendPosition::TopLeft,
            orientation: Orientation::Vertical,
            offset: Point::new(px(10.0), px(10.0)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InertiaConfig {
    pub enabled: bool,
    pub friction: f64,
    pub sensitivity: f64,
    pub stop_threshold: Duration,
}

impl Default for InertiaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            friction: 0.90,
            sensitivity: 1.0,
            stop_threshold: Duration::from_millis(150),
        }
    }
}

pub fn init(_cx: &mut impl AppContext) {
    // Initialization code if needed
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DragMode {
    None,
    Plot,
    AxisY(usize),
    AxisX,
}

/// La `View` principale qui gère l'état et la logique du graphique.
pub struct ChartView {
    pub x_axis: Entity<AxisRange>,
    pub y_axes: Vec<Entity<AxisRange>>,
    pub shared_state: Entity<SharedPlotState>,

    pub series: Vec<Series>,
    pub hidden_series: HashSet<String>,
    pub bg_color: Hsla,
    pub label_color: Hsla,

    // UI Options
    pub show_crosshair: bool,
    pub show_axis_tags: bool,
    pub show_tooltip: bool,
    pub show_x_axis: bool,
    pub show_y_axis: bool,
    pub legend_config: LegendConfig,
    pub inertia_config: InertiaConfig,

    // Layout Options
    pub margin_left: Pixels,
    pub margin_right: Pixels,
    pub margin_bottom: Pixels,

    drag_start: Option<Point<Pixels>>,
    drag_mode: DragMode,
    last_drag_time: Option<Instant>,
    velocity: Point<f64>,

    zoom_drag_start: Option<Point<Pixels>>,
    zoom_drag_last: Option<Point<Pixels>>,
    zoom_drag_mode: DragMode,

    box_zoom_start: Option<Point<Pixels>>,
    box_zoom_current: Option<Point<Pixels>>,

    pub mouse_pos: Option<Point<Pixels>>,
    is_dragging: bool,

    bounds: Rc<RefCell<Bounds<Pixels>>> ,
    focus_handle: FocusHandle,
}

impl ChartView {
    /// Crée une nouvelle ChartView avec des axes et un état partagé.
    pub fn new(
        x_axis: Entity<AxisRange>,
        y_axis: Entity<AxisRange>,
        shared_state: Entity<SharedPlotState>,
        cx: &mut Context<Self>,
    ) -> Self {
        info!("ChartView new called");

        // S'abonner aux changements pour rafraîchir la vue
        cx.observe(&x_axis, |_, _, cx| cx.notify()).detach();
        cx.observe(&y_axis, |_, _, cx| cx.notify()).detach();
        cx.observe(&shared_state, |_, _, cx| cx.notify()).detach();

        Self {
            x_axis,
            y_axes: vec![y_axis],
            shared_state,
            series: vec![],
            hidden_series: HashSet::new(),
            bg_color: gpui::black(),
            label_color: gpui::white(),

            show_crosshair: true,
            show_axis_tags: true,
            show_tooltip: true,
            show_x_axis: true,
            show_y_axis: true,
            legend_config: LegendConfig::default(),
            inertia_config: InertiaConfig::default(),

            margin_left: px(50.0),
            margin_right: px(50.0),
            margin_bottom: px(20.0),

            drag_start: None,
            drag_mode: DragMode::None,
            last_drag_time: None,
            velocity: Point::default(),

            zoom_drag_start: None,
            zoom_drag_last: None,
            zoom_drag_mode: DragMode::None,

            box_zoom_start: None,
            box_zoom_current: None,

            mouse_pos: None,
            is_dragging: false,

            bounds: Rc::new(RefCell::new(Bounds::new(
                Point::new(px(0.0), px(0.0)),
                Size {
                    width: px(0.0),
                    height: px(0.0),
                },
            ))),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn add_y_axis(&mut self, y_axis: Entity<AxisRange>, cx: &mut Context<Self>) -> usize {
        cx.observe(&y_axis, |_, _, cx| cx.notify()).detach();
        self.y_axes.push(y_axis);
        self.y_axes.len() - 1
    }

    // --- Action Handlers ---
    fn handle_pan_left(&mut self, _: &PanLeft, _win: &mut Window, cx: &mut Context<Self>) {
        self.pan_x(-0.1, cx);
    }
    fn handle_pan_right(&mut self, _: &PanRight, _win: &mut Window, cx: &mut Context<Self>) {
        self.pan_x(0.1, cx);
    }
    fn handle_pan_up(&mut self, _: &PanUp, _win: &mut Window, cx: &mut Context<Self>) {
        self.pan_y(0.1, cx);
    }
    fn handle_pan_down(&mut self, _: &PanDown, _win: &mut Window, cx: &mut Context<Self>) {
        self.pan_y(-0.1, cx);
    }
    fn handle_zoom_in(&mut self, _: &ZoomIn, _win: &mut Window, cx: &mut Context<Self>) {
        self.keyboard_zoom(0.9, cx);
    }
    fn handle_zoom_out(&mut self, _: &ZoomOut, _win: &mut Window, cx: &mut Context<Self>) {
        self.keyboard_zoom(1.1, cx);
    }
    fn handle_reset_view(&mut self, _: &ResetView, _win: &mut Window, cx: &mut Context<Self>) {
        self.auto_fit_axes(cx);
    }

    fn pan_x(&mut self, factor: f64, cx: &mut Context<Self>) {
        self.x_axis.update(cx, |x, cx| {
            x.pan(x.span() * factor);
            x.clamp();
            cx.notify();
        });
    }

    fn pan_y(&mut self, factor: f64, cx: &mut Context<Self>) {
        for y_axis in &self.y_axes {
            y_axis.update(cx, |y, cx| {
                y.pan(y.span() * factor);
                y.clamp();
                cx.notify();
            });
        }
    }

    fn keyboard_zoom(&mut self, factor: f64, cx: &mut Context<Self>) {
        self.x_axis.update(cx, |x, cx| {
            let pivot_data = x.min + x.span() / 2.0;
            x.zoom_at(pivot_data, 0.5, factor);
            cx.notify();
        });
        for y_axis in &self.y_axes {
            y_axis.update(cx, |y, cx| {
                let pivot_data = y.min + y.span() / 2.0;
                y.zoom_at(pivot_data, 0.5, factor);
                cx.notify();
            });
        }
    }

    pub fn auto_fit_axes(&mut self, cx: &mut Context<Self>) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;

        for series in &self.series {
            if self.hidden_series.contains(&series.id) {
                continue;
            }
            if let Some((sx_min, sx_max, _, _)) = series.plot.borrow().get_min_max() {
                x_min = x_min.min(sx_min);
                x_max = x_max.max(sx_max);
            }
        }

        if x_min == f64::INFINITY {
            return;
        }

        let padding = 0.05;
        self.x_axis.update(cx, |x: &mut AxisRange, cx| {
            let span = x_max - x_min;
            if span <= f64::EPSILON {
                x.min = x_min - 30.0;
                x.max = x_max + 30.0;
            } else {
                x.min = x_min - span * padding;
                x.max = x_max + span * padding;
            }
            x.clamp();
            cx.notify();
        });

        // Fit each Y axis independently
        for (idx, y_axis) in self.y_axes.iter().enumerate() {
            let mut sy_min = f64::INFINITY;
            let mut sy_max = f64::NEG_INFINITY;

            for series in &self.series {
                if self.hidden_series.contains(&series.id) || series.y_axis_index != idx {
                    continue;
                }
                if let Some((_, _, s_min, s_max)) = series.plot.borrow().get_min_max() {
                    sy_min = sy_min.min(s_min);
                    sy_max = sy_max.max(s_max);
                }
            }

            if sy_min != f64::INFINITY {
                y_axis.update(cx, |y: &mut AxisRange, cx| {
                    let span = sy_max - sy_min;
                    if span <= f64::EPSILON {
                        let h = if sy_min.abs() > 0.0 {
                            sy_min.abs() * 0.05
                        } else {
                            5.0
                        };
                        y.min = sy_min - h;
                        y.max = sy_max + h;
                    } else {
                        y.min = sy_min - span * padding;
                        y.max = sy_max + span * padding;
                    }
                    y.clamp();
                    cx.notify();
                });
            }
        }
    }

    fn handle_zoom(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bounds = *self.bounds.borrow();
        if bounds.is_empty() {
            return;
        }

        let is_zoom = event.modifiers.control || event.modifiers.platform;
        let delta_x = match event.delta {
            ScrollDelta::Pixels(p) => p.x.as_f32() as f64,
            ScrollDelta::Lines(p) => p.x as f64 * 20.0,
        };
        let delta_y = match event.delta {
            ScrollDelta::Pixels(p) => p.y.as_f32() as f64,
            ScrollDelta::Lines(p) => p.y as f64 * 20.0,
        };

        let local_x = event.position.x - (bounds.origin.x - self.margin_left);
        let over_left = local_x < self.margin_left;
        let over_right = local_x > bounds.size.width + self.margin_left;
        let over_bottom = event.position.y > bounds.origin.y + bounds.size.height;

        let (pw, ph) = (
            bounds.size.width.as_f32() as f64,
            bounds.size.height.as_f32() as f64,
        );

        if is_zoom {
            let factor = (1.0f64 - delta_y * 0.01).clamp(0.1, 10.0);
            
            // Zoom X if NOT over Y margins
            if !over_left && !over_right {
                let mx_pct = (event.position.x - bounds.origin.x).as_f32() as f64 / pw;
                self.x_axis.update(cx, |x, cx| {
                    let m_data = x.min + x.span() * mx_pct;
                    x.zoom_at(m_data, mx_pct, factor);
                    cx.notify();
                });
            }

            // Zoom Y axes independently
            let my_pct = (event.position.y - bounds.origin.y).as_f32() as f64 / ph;
            for (idx, y_axis) in self.y_axes.iter().enumerate() {
                let target = if over_left { idx == 0 } 
                            else if over_right { idx == 1 } 
                            else if over_bottom { false }
                            else { true };
                
                if target {
                    y_axis.update(cx, |y, cx| {
                        let m_data = y.min + y.span() * (1.0 - my_pct);
                        y.zoom_at(m_data, 1.0 - my_pct, factor);
                        cx.notify();
                    });
                }
            }
        } else {
            // Panning: apply to X if not over Y margins
            if !over_left && !over_right {
                self.x_axis.update(cx, |x, cx| {
                    x.pan(-delta_x * (x.span() / pw));
                    x.clamp();
                    cx.notify();
                });
            }

            // Panning: apply to Y if not over bottom margin
            for (idx, y_axis) in self.y_axes.iter().enumerate() {
                let target = if over_left { idx == 0 } 
                            else if over_right { idx == 1 } 
                            else if over_bottom { false }
                            else { true };
                
                if target {
                    y_axis.update(cx, |y, cx| {
                        y.pan(delta_y * (y.span() / ph));
                        y.clamp();
                        cx.notify();
                    });
                }
            }
        }
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let canvas_bounds = *self.bounds.borrow();
        if canvas_bounds.is_empty() { return; }

        // Interaction zone includes margins for Y-axis tag
        let interaction_bounds = Bounds {
            origin: Point::new(canvas_bounds.origin.x - self.margin_left, canvas_bounds.origin.y),
            size: Size::new(canvas_bounds.size.width + self.margin_left + self.margin_right, canvas_bounds.size.height + self.margin_bottom),
        };

        if interaction_bounds.contains(&event.position) {
            let x_range = self.x_axis.read(cx);
            let pct_x = (event.position.x - canvas_bounds.origin.x).as_f32() as f64
                / canvas_bounds.size.width.as_f32() as f64;
            let data_x = x_range.min + x_range.span() * pct_x;

            self.shared_state.update(cx, |state: &mut SharedPlotState, cx| {
                state.hover_x = Some(data_x);
                state.mouse_pos = Some(event.position);
                state.active_chart_id = Some(cx.entity_id());
                cx.notify();
            });
        }

        if let Some(start) = self.drag_start {
            self.is_dragging = true;
            let now = Instant::now();
            let delta = event.position - start;
            let (pw, ph) = (
                canvas_bounds.size.width.as_f32() as f64,
                canvas_bounds.size.height.as_f32() as f64,
            );

            match self.drag_mode {
                DragMode::Plot => {
                    self.x_axis.update(cx, |x, cx| {
                        x.pan(-delta.x.as_f32() as f64 * (x.span() / pw));
                        x.clamp();
                        cx.notify();
                    });
                    for y_axis in &self.y_axes {
                        y_axis.update(cx, |y, cx| {
                            y.pan(delta.y.as_f32() as f64 * (y.span() / ph));
                            y.clamp();
                            cx.notify();
                        });
                    }
                }
                DragMode::AxisY(idx) => {
                    if let Some(y_axis) = self.y_axes.get(idx) {
                        y_axis.update(cx, |y, cx| {
                            y.pan(delta.y.as_f32() as f64 * (y.span() / ph));
                            y.clamp();
                            cx.notify();
                        });
                    }
                }
                DragMode::AxisX => {
                    self.x_axis.update(cx, |x, cx| {
                        x.pan(-delta.x.as_f32() as f64 * (x.span() / pw));
                        x.clamp();
                        cx.notify();
                    });
                }
                _ => {}
            }

            if let Some(last_time) = self.last_drag_time {
                let dt = now.duration_since(last_time).as_secs_f64();
                if dt > 0.0 {
                    self.velocity =
                        Point::new(delta.x.as_f32() as f64 / dt, delta.y.as_f32() as f64 / dt);
                }
            }
            self.drag_start = Some(event.position);
            self.last_drag_time = Some(now);
        }
        if let Some(_) = self.box_zoom_start {
            self.box_zoom_current = Some(event.position);
        }
        cx.notify();
    }

    fn apply_inertia(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_dragging || (self.velocity.x.abs() < 0.1 && self.velocity.y.abs() < 0.1) {
            return;
        }
        let friction = self.inertia_config.friction;
        self.velocity.x *= friction;
        self.velocity.y *= friction;
        let bounds = *self.bounds.borrow();
        if !bounds.is_empty() {
            let (pw, ph) = (
                bounds.size.width.as_f32() as f64,
                bounds.size.height.as_f32() as f64,
            );
            let dt = 1.0 / 60.0;
            self.x_axis.update(cx, |x, cx| {
                x.pan(self.velocity.x * dt * (x.span() / pw));
                x.clamp();
                cx.notify();
            });
            for y_axis in &self.y_axes {
                y_axis.update(cx, |y, cx| {
                    y.pan(self.velocity.y * dt * (y.span() / ph));
                    y.clamp();
                    cx.notify();
                });
            }
        }
        cx.on_next_frame(window, |this, window, cx| {
            this.apply_inertia(window, cx);
        });
    }

    fn start_drag(&mut self, event: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if event.click_count >= 2 {
            self.auto_fit_axes(cx);
            return;
        }

        let bounds = *self.bounds.borrow();
        let local_x = event.position.x - (bounds.origin.x - self.margin_left);
        let local_y = event.position.y - bounds.origin.y;
        
        if local_x < self.margin_left {
            self.drag_mode = DragMode::AxisY(0);
        } else if local_x > bounds.size.width + self.margin_left {
            self.drag_mode = DragMode::AxisY(1);
        } else if local_y > bounds.size.height {
            self.drag_mode = DragMode::AxisX;
        } else {
            self.drag_mode = DragMode::Plot;
        }

        self.drag_start = Some(event.position);
        self.last_drag_time = Some(Instant::now());
        self.velocity = Point::default();
        self.is_dragging = true;
        cx.notify();
    }

    fn end_drag(&mut self, _event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.drag_start = None;
        self.drag_mode = DragMode::None;
        self.is_dragging = false;
        if let Some(lt) = self.last_drag_time {
            if Instant::now().duration_since(lt) > self.inertia_config.stop_threshold {
                self.velocity = Point::default();
            }
        }
        if self.inertia_config.enabled
            && (self.velocity.x.abs() > 1.0 || self.velocity.y.abs() > 1.0)
        {
            self.apply_inertia(window, cx);
        }
        cx.notify();
    }

    fn start_box_zoom(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.box_zoom_start = Some(event.position);
        self.box_zoom_current = Some(event.position);
        cx.notify();
    }

    fn end_box_zoom(&mut self, event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(start) = self.box_zoom_start {
            let end = event.position;
            let bounds = *self.bounds.borrow();
            if !bounds.is_empty() {
                let x_range = self.x_axis.read(cx);
                let x_scale = crate::scales::ChartScale::new_linear(
                    (x_range.min, x_range.max),
                    (0.0, bounds.size.width.as_f32()),
                );
                
                // Box zoom on X is unified
                let px1 = x_scale.invert((start.x - bounds.origin.x).as_f32());
                let px2 = x_scale.invert((end.x - bounds.origin.x).as_f32());
                
                self.x_axis.update(cx, |x, cx| {
                    x.min = px1.min(px2);
                    x.max = px1.max(px2);
                    x.clamp();
                    cx.notify();
                });

                // Box zoom on Y affects ALL axes for now
                for y_axis_entity in &self.y_axes {
                    y_axis_entity.update(cx, |y, cx| {
                        let y_scale = crate::scales::ChartScale::new_linear(
                            (y.min, y.max),
                            (bounds.size.height.as_f32(), 0.0),
                        );
                        let py1 = y_scale.invert((start.y - bounds.origin.y).as_f32());
                        let py2 = y_scale.invert((end.y - bounds.origin.y).as_f32());
                        y.min = py1.min(py2);
                        y.max = py1.max(py2);
                        y.clamp();
                        cx.notify();
                    });
                }
            }
        }
        self.box_zoom_start = None;
        self.box_zoom_current = None;
        cx.notify();
    }

    fn start_zoom_drag(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bounds = *self.bounds.borrow();
        let local_x = event.position.x - (bounds.origin.x - self.margin_left);
        let local_y = event.position.y - bounds.origin.y;
        
        if local_x < self.margin_left {
            self.zoom_drag_mode = DragMode::AxisY(0);
        } else if local_x > bounds.size.width + self.margin_left {
            self.zoom_drag_mode = DragMode::AxisY(1);
        } else if local_y > bounds.size.height {
            self.zoom_drag_mode = DragMode::AxisX;
        } else {
            self.zoom_drag_mode = DragMode::Plot;
        }

        self.zoom_drag_start = Some(event.position);
        self.zoom_drag_last = Some(event.position);
        self.is_dragging = true;
        cx.notify();
    }

    fn handle_zoom_drag(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let (Some(start), Some(last)) = (self.zoom_drag_start, self.zoom_drag_last) {
            self.is_dragging = true;
            let bounds = *self.bounds.borrow();
            if bounds.is_empty() {
                return;
            }
            let delta = event.position - last;
            let factor_x = 1.0 + delta.x.as_f32().abs() as f64 / 100.0;
            let factor_y = 1.0 + delta.y.as_f32().abs() as f64 / 100.0;
            let (pw, ph) = (
                bounds.size.width.as_f32() as f64,
                bounds.size.height.as_f32() as f64,
            );
            
            let pivot_x_pct = (start.x.as_f32() as f64 - bounds.origin.x.as_f32() as f64) / pw;
            let pivot_y_pct = (start.y.as_f32() as f64 - bounds.origin.y.as_f32() as f64) / ph;
            
            let mode = self.zoom_drag_mode;

            if mode == DragMode::Plot || mode == DragMode::AxisX {
                self.x_axis.update(cx, |x, cx| {
                    let pivot_x_data = x.min + x.span() * pivot_x_pct;
                    let factor = if delta.x.as_f32() > 0.0 { 1.0 / factor_x } else { factor_x };
                    x.zoom_at(pivot_x_data, pivot_x_pct, factor);
                    cx.notify();
                });
            }
            
            for (idx, y_axis) in self.y_axes.iter().enumerate() {
                let target = match mode {
                    DragMode::Plot => true,
                    DragMode::AxisY(i) => idx == i,
                    _ => false,
                };
                
                if target {
                    y_axis.update(cx, |y, cx| {
                        let pivot_y_data = y.min + y.span() * (1.0 - pivot_y_pct);
                        let factor = if delta.y.as_f32() < 0.0 { 1.0 / factor_y } else { factor_y };
                        y.zoom_at(pivot_y_data, 1.0 - pivot_y_pct, factor);
                        cx.notify();
                    });
                }
            }
            
            self.zoom_drag_last = Some(event.position);
            cx.notify();
        }
    }

    fn end_zoom_drag(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.zoom_drag_start = None;
        self.zoom_drag_last = None;
        self.zoom_drag_mode = DragMode::None;
        self.is_dragging = false;
        cx.notify();
    }

    pub fn add_series(&mut self, series: Series) {
        self.series.push(series);
    }
    pub fn toggle_series_visibility(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.hidden_series.contains(id) {
            self.hidden_series.remove(id);
        } else {
            self.hidden_series.insert(id.to_string());
        }
        cx.notify();
    }
}

impl Focusable for ChartView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ChartView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible_series: Vec<Series> = self
            .series
            .iter()
            .filter(|s| !self.hidden_series.contains(&s.id))
            .cloned()
            .collect();
        let bounds_rc = self.bounds.clone();
        let current_bounds = *bounds_rc.borrow();
        let shared_state = self.shared_state.read(cx);

        let x_axis_model = self.x_axis.read(cx);
        let (x_min, x_max) = x_axis_model.clamped_bounds();

        let x_domain_full = AxisDomain {
            x_min,
            x_max,
            y_min: 0.0,
            y_max: 0.0,
            x_min_limit: x_axis_model.min_limit,
            x_max_limit: x_axis_model.max_limit,
            y_min_limit: None,
            y_max_limit: None,
        };

        let x_scale = crate::scales::ChartScale::new_linear(
            (x_min, x_max),
            (0.0, current_bounds.size.width.as_f32()),
        );

        let x_ticks = LinearScale::new()
            .domain(x_min, x_max)
            .range(0.0, current_bounds.size.width.as_f32() as f64)
            .ticks(10);

        let mut y_renders = Vec::new();
        let mut y_domains_raw = Vec::new();

        for y_axis_entity in &self.y_axes {
            let y_axis_model = y_axis_entity.read(cx);
            let (y_min, y_max) = y_axis_model.clamped_bounds();
            y_domains_raw.push((y_min, y_max));

            let y_scale = crate::scales::ChartScale::new_linear(
                (y_min, y_max),
                (current_bounds.size.height.as_f32(), 0.0),
            );

            let y_ticks = LinearScale::new()
                .domain(y_min, y_max)
                .range(current_bounds.size.height.as_f32() as f64, 0.0)
                .ticks(10);

            y_renders.push(crate::rendering::YAxisRenderInfo {
                domain: (y_min, y_max),
                scale: y_scale,
                ticks: y_ticks,
                limits: (y_axis_model.min_limit, y_axis_model.max_limit),
            });
        }

        let axes_elements = crate::rendering::paint_axes(
            &x_domain_full,
            &x_scale,
            &x_ticks,
            &y_renders,
            self.label_color,
            self.show_x_axis,
            self.show_y_axis,
            self.margin_left,
            self.margin_bottom,
            self.margin_right,
        );

        let root_origin = Point::new(current_bounds.origin.x - self.margin_left, current_bounds.origin.y);
        let mouse_pos_global = shared_state.mouse_pos;
        let is_mouse_here = mouse_pos_global.map_or(false, |pos| {
            pos.x >= root_origin.x && pos.x <= root_origin.x + current_bounds.size.width + self.margin_left + self.margin_right &&
            pos.y >= root_origin.y && pos.y <= root_origin.y + current_bounds.size.height + self.margin_bottom
        });

        let mut overlay_elements = Vec::new();

        if self.show_axis_tags {
            if let Some(hx) = shared_state.hover_x {
                let transform = crate::transform::PlotTransform::new(x_scale.clone(), y_renders[0].scale.clone(), current_bounds);
                let sx = transform.x_data_to_screen(hx);
                if self.show_x_axis {
                    let in_limits = x_axis_model.min_limit.map_or(true, |l| hx >= l) &&
                                   x_axis_model.max_limit.map_or(true, |l| hx <= l);
                    if in_limits {
                        overlay_elements.push(create_axis_tag(
                            x_scale.format_tick(hx),
                            sx - root_origin.x,
                            true,
                            self.label_color,
                            self.bg_color,
                            self.margin_left,
                            self.margin_right,
                            true,
                        ));
                    }
                }
            }

            if is_mouse_here {
                if let Some(pos) = mouse_pos_global {
                    if self.show_y_axis {
                        for (idx, y_info) in y_renders.iter().enumerate() {
                            let transform = crate::transform::PlotTransform::new(x_scale.clone(), y_info.scale.clone(), current_bounds);
                            let data_point = transform.screen_to_data(pos);
                            let in_limits = y_info.limits.0.map_or(true, |l| data_point.y >= l) &&
                                           y_info.limits.1.map_or(true, |l| data_point.y <= l);
                            
                            if in_limits {
                                // For now, we only show primary tag on left, secondary on right
                                if idx == 0 || idx == 1 {
                                    overlay_elements.push(create_axis_tag(
                                        y_info.scale.format_tick(data_point.y),
                                        pos.y - root_origin.y,
                                        false,
                                        self.label_color,
                                        self.bg_color,
                                        self.margin_left,
                                        self.margin_right,
                                        idx == 0,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        let cursor = if self.is_dragging {
            match (self.drag_mode, self.zoom_drag_mode) {
                (DragMode::AxisY(_), _) | (_, DragMode::AxisY(_)) => CursorStyle::ResizeUpDown,
                (DragMode::AxisX, _) | (_, DragMode::AxisX) => CursorStyle::ResizeLeftRight,
                (_, DragMode::Plot) | (DragMode::Plot, _) => CursorStyle::Crosshair,
                _ => CursorStyle::ClosedHand,
            }
        } else if self.box_zoom_start.is_some() {
            CursorStyle::Arrow
        } else if let Some(pos) = shared_state.mouse_pos {
            let local_x = pos.x - (current_bounds.origin.x - self.margin_left);
            let over_left = local_x < self.margin_left;
            let over_right = local_x > current_bounds.size.width + self.margin_left;
            let over_bottom = pos.y > current_bounds.origin.y + current_bounds.size.height;

            if over_left || over_right {
                CursorStyle::ResizeUpDown
            } else if over_bottom {
                CursorStyle::ResizeLeftRight
            } else {
                CursorStyle::Crosshair
            }
        } else {
            CursorStyle::Crosshair
        };

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(self.bg_color)
            .relative()
            .cursor(cursor)
            .on_mouse_down(MouseButton::Left, {
                let fh = self.focus_handle.clone();
                move |_, window, _| {
                    window.focus(&fh);
                }
            })
            .on_action(cx.listener(Self::handle_pan_left))
            .on_action(cx.listener(Self::handle_pan_right))
            .on_action(cx.listener(Self::handle_pan_up))
            .on_action(cx.listener(Self::handle_pan_down))
            .on_action(cx.listener(Self::handle_zoom_in))
            .on_action(cx.listener(Self::handle_zoom_out))
            .on_action(cx.listener(Self::handle_reset_view))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::start_drag))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::end_drag))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::start_zoom_drag))
            .on_mouse_move(cx.listener(Self::handle_zoom_drag))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::end_zoom_drag))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::start_box_zoom))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::end_box_zoom))
            .on_scroll_wheel(cx.listener(Self::handle_zoom))
            .pl(self.margin_left)
            .pr(self.margin_right)
            .pb(if self.show_x_axis {
                self.margin_bottom
            } else {
                px(0.0)
            })
            .child(
                div()
                    .size_full()
                    .overflow_hidden()
                    .cursor(cursor)
                    .child(
                        canvas(|_, _, _| {}, {
                            let xs = x_scale.clone();
                            let lc = self.label_color;
                            let sc = self.show_crosshair;
                            let hx = shared_state.hover_x;
                            let vs = visible_series.clone();
                            let x_df = x_domain_full.clone();
                            let y_ds = y_domains_raw.clone();
                            let y_renders = y_renders.clone();
                            let mouse_pos = shared_state.mouse_pos;
                            
                            move |bounds, (), window, cx| {
                                *bounds_rc.borrow_mut() = bounds;
                                // Primary grid uses index 0
                                crate::rendering::paint_grid(window, bounds, &x_df, &xs, &x_ticks, &y_renders[0]);
                                crate::rendering::paint_plot(window, bounds, &vs, (x_df.x_min, x_df.x_max), &y_ds, cx);
                                
                                if sc {
                                    if let Some(hx_val) = hx {
                                        let in_limits = x_df.x_min_limit.map_or(true, |l| hx_val >= l) &&
                                                       x_df.x_max_limit.map_or(true, |l| hx_val <= l);
                                        if in_limits {
                                            let transform = crate::transform::PlotTransform::new(xs.clone(), y_renders[0].scale.clone(), bounds);
                                            let sx = transform.x_data_to_screen(hx_val);
                                            let p1 = Point::new(sx, bounds.origin.y);
                                            let p2 = Point::new(sx, bounds.origin.y + bounds.size.height);
                                            let mut builder = PathBuilder::stroke(px(1.0));
                                            builder.move_to(p1);
                                            builder.line_to(p2);
                                            if let Ok(path) = builder.build() {
                                                window.paint_path(path, lc.alpha(0.3));
                                            }
                                        }
                                    }

                                    if let Some(pos) = mouse_pos {
                                        if bounds.contains(&pos) {
                                            // Show horizontal line based on primary Y axis
                                            let transform = crate::transform::PlotTransform::new(xs.clone(), y_renders[0].scale.clone(), bounds);
                                            let data_pt = transform.screen_to_data(pos);
                                            let y0 = &y_renders[0];
                                            let in_limits = y0.limits.0.map_or(true, |l| data_pt.y >= l) &&
                                                           y0.limits.1.map_or(true, |l| data_pt.y <= l);
                                            if in_limits {
                                                let mut builder = PathBuilder::stroke(px(1.0));
                                                builder.move_to(Point::new(bounds.origin.x, pos.y));
                                                builder.line_to(Point::new(bounds.origin.x + bounds.size.width, pos.y));
                                                if let Ok(path) = builder.build() {
                                                    window.paint_path(path, lc.alpha(0.3));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        })
                        .size_full(),
                    )
                    .children(
                        if let (Some(start), Some(current)) = (self.box_zoom_start, self.box_zoom_current) {
                            let origin = current_bounds.origin;
                            let min_x = (start.x.min(current.x)) - origin.x;
                            let max_x = (start.x.max(current.x)) - origin.x;
                            let min_y = (start.y.min(current.y)) - origin.y;
                            let max_y = (start.y.max(current.y)) - origin.y;
                            Some(
                                div()
                                    .absolute()
                                    .left(min_x)
                                    .top(min_y)
                                    .w(max_x - min_x)
                                    .h(max_y - min_y)
                                    .bg(gpui::blue().alpha(0.2))
                                    .border_1()
                                    .border_color(gpui::blue()),
                            )
                        } else {
                            None
                        },
                    ),
            )
            .children(axes_elements)
            .children(overlay_elements)
            .children(if self.legend_config.enabled {
                let mut items = vec![];
                for s in &self.series {
                    let id = s.id.clone();
                    let hidden = self.hidden_series.contains(&id);
                    items.push(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, {
                                let id = id.clone();
                                cx.listener(move |this, _, _, cx| {
                                    this.toggle_series_visibility(&id, cx)
                                })
                            })
                            .child(
                                div()
                                    .w_3()
                                    .h_3()
                                    .bg(if hidden {
                                        gpui::transparent_black()
                                    } else {
                                        gpui::blue()
                                    })
                                    .border_1()
                                    .border_color(gpui::white()),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(if hidden {
                                        self.label_color.alpha(0.4)
                                    } else {
                                        self.label_color
                                    })
                                    .child(id),
                            ),
                    );
                }
                let mut leg = div()
                    .absolute()
                    .bg(self.bg_color.alpha(0.8))
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(self.label_color.alpha(0.2))
                    .flex()
                    .gap_2();
                if self.legend_config.orientation == Orientation::Vertical {
                    leg = leg.flex_col().gap_1();
                } else {
                    leg = leg.flex_row().gap_3();
                }
                match self.legend_config.position {
                    LegendPosition::TopLeft => {
                        leg = leg
                            .top(px(10.0) + self.legend_config.offset.y)
                            .left(self.margin_left + px(10.0) + self.legend_config.offset.x);
                    }
                    LegendPosition::TopRight => {
                        leg = leg
                            .top(px(10.0) + self.legend_config.offset.y)
                            .right(px(10.0) + self.legend_config.offset.x);
                    }
                    LegendPosition::BottomLeft => {
                        leg = leg
                            .bottom(self.margin_bottom + px(10.0) + self.legend_config.offset.y)
                            .left(self.margin_left + px(10.0) + self.legend_config.offset.x);
                    }
                    LegendPosition::BottomRight => {
                        leg = leg
                            .bottom(self.margin_bottom + px(10.0) + self.legend_config.offset.y)
                            .right(px(10.0) + self.legend_config.offset.x);
                    }
                    LegendPosition::TopCenter => {
                        leg = leg
                            .top(px(10.0) + self.legend_config.offset.y)
                            .left_1_2()
                            .ml(px(-50.0));
                    }
                    LegendPosition::BottomCenter => {
                        leg = leg
                            .bottom(self.margin_bottom + px(10.0) + self.legend_config.offset.y)
                            .left_1_2()
                            .ml(px(-50.0));
                    }
                    LegendPosition::Custom(p) => {
                        leg = leg.top(p.y).left(p.x);
                    }
                }
                Some(leg.children(items).into_any_element())
            } else {
                None
            })
            .children(if self.show_tooltip && is_mouse_here {
                if let Some(pos) = mouse_pos_global {
                    let transform = crate::transform::PlotTransform::new(x_scale.clone(), y_renders[0].scale.clone(), current_bounds);
                    let data_point = transform.screen_to_data(pos);
                    let x_in = x_axis_model.min_limit.map_or(true, |l| data_point.x >= l) &&
                              x_axis_model.max_limit.map_or(true, |l| data_point.x <= l);
                    
                    let y0 = &y_renders[0];
                    let y_in = y0.limits.0.map_or(true, |l| data_point.y >= l) &&
                              y0.limits.1.map_or(true, |l| data_point.y <= l);

                    if x_in && y_in {
                        let local_x = pos.x - root_origin.x;
                        let local_y = pos.y - root_origin.y;

                        Some(
                            div()
                                .absolute()
                                .left(local_x + px(15.0))
                                .top(local_y + px(15.0))
                                .bg(gpui::white())
                                .text_color(gpui::black())
                                .p_1()
                                .rounded_sm()
                                .text_size(px(10.0))
                                .shadow_md()
                                .child(format!(
                                    "X: {}\nY: {}",
                                    x_scale.format_tick(data_point.x),
                                    y0.scale.format_tick(data_point.y)
                                ))
                                .into_any_element(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            })
    }
}
