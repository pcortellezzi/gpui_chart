// ChartPane implementation

use crate::data_types::{AxisRange, Series, SharedPlotState, ChartAxis, AxisEdge, AxisId};
use crate::view_controller::ViewController;
use gpui::prelude::*;
use gpui::*;
use d3rs::scale::Scale;
use adabraka_ui::util::PixelsExt;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

actions!(
    gpui_chart,
    [PanLeft, PanRight, PanUp, PanDown, ZoomIn, ZoomOut, ResetView, ToggleDebug]
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
            friction: 0.80,
            sensitivity: 1.0,
            stop_threshold: Duration::from_millis(150),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DragMode {
    None,
    Plot,
}

/// La `Pane` (Zone de graphique) qui gère le rendu du contenu et les interactions locales.
pub struct ChartPane {
    pub x_axes: Vec<ChartAxis<AxisRange>>,
    pub y_axes: Vec<ChartAxis<AxisRange>>,
    pub shared_state: Entity<SharedPlotState>,

    pub series: Vec<Series>,
    pub hidden_series: HashSet<String>,
    pub bg_color: Hsla,
    pub label_color: Hsla,

    // UI Options
    pub show_crosshair: bool,
    pub show_tooltip: bool,
    pub legend_config: LegendConfig,
    pub inertia_config: InertiaConfig,

    pub on_move_series: Option<Box<dyn Fn(&str, bool, &mut Window, &mut Context<Self>)>>,
    pub on_isolate_series: Option<Box<dyn Fn(&str, &mut Window, &mut Context<Self>)>>,

    drag_start: Option<Point<Pixels>>,
    drag_mode: DragMode,
    last_drag_time: Option<Instant>,
    velocity: Point<f64>,

    zoom_drag_start: Option<Point<Pixels>>,
    zoom_drag_last: Option<Point<Pixels>>,
    zoom_drag_mode: DragMode,

    box_zoom_start: Option<Point<Pixels>>,
    box_zoom_current: Option<Point<Pixels>>,

    is_dragging: bool,

    pub debug_mode: bool,
    pub last_paint_stats: Rc<RefCell<crate::rendering::PaintStats>>,
    pub notify_count: u32,
    pub last_stats_reset: Instant,
    pub last_notify_rate: f64,

    pub bounds: Rc<RefCell<Bounds<Pixels>>>,
    focus_handle: FocusHandle,
}

impl ChartPane {
    pub fn new(shared_state: Entity<SharedPlotState>, cx: &mut Context<Self>) -> Self {
        Self {
            shared_state,
            series: vec![],
            hidden_series: HashSet::new(),
            x_axes: vec![],
            y_axes: vec![],
            drag_mode: DragMode::None,
            drag_start: None,
            last_drag_time: None,
            velocity: Point::default(),
            zoom_drag_start: None,
            zoom_drag_last: None,
            zoom_drag_mode: DragMode::None,
            box_zoom_start: None,
            box_zoom_current: None,
            is_dragging: false,
            debug_mode: false,
            last_paint_stats: Rc::new(RefCell::new(crate::rendering::PaintStats::default())),
            notify_count: 0,
            last_stats_reset: Instant::now(),
            last_notify_rate: 0.0,
            bg_color: gpui::black(),
            label_color: gpui::white(),
            show_crosshair: true,
            show_tooltip: true,
            legend_config: LegendConfig::default(),
            inertia_config: InertiaConfig::default(),
            on_move_series: None,
            on_isolate_series: None,
            bounds: Rc::new(RefCell::new(Bounds::default())),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn on_move_series(&mut self, f: impl Fn(&str, bool, &mut Window, &mut Context<Self>) + 'static) {
        self.on_move_series = Some(Box::new(f));
    }

    pub fn on_isolate_series(&mut self, f: impl Fn(&str, &mut Window, &mut Context<Self>) + 'static) {
        self.on_isolate_series = Some(Box::new(f));
    }

    pub fn add_x_axis(&mut self, axis: Entity<AxisRange>, _cx: &mut App) -> usize {
        self.x_axes.push(ChartAxis { axis, edge: AxisEdge::Bottom, size: px(25.0) });
        self.x_axes.len() - 1
    }

    pub fn add_y_axis(&mut self, axis: Entity<AxisRange>, _cx: &mut App) -> usize {
        self.y_axes.push(ChartAxis { axis, edge: AxisEdge::Right, size: px(60.0) });
        self.y_axes.len() - 1
    }

    pub fn add_series(&mut self, series: Series) {
        self.series.push(series);
    }

    /// Convenience method to add a plot and get back a thread-safe handle to update its data.
    pub fn add_plot<P: crate::plot_types::PlotRenderer + 'static>(
        &mut self, 
        id: impl Into<String>, 
        plot: P
    ) -> std::sync::Arc<parking_lot::RwLock<P>> {
        let id = id.into();
        let plot_arc = std::sync::Arc::new(parking_lot::RwLock::new(plot));
        self.series.push(Series {
            id,
            plot: plot_arc.clone(),
            x_axis_id: AxisId(0),
            y_axis_id: AxisId(0),
        });
        plot_arc
    }

    pub fn toggle_series_visibility(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.hidden_series.contains(id) { self.hidden_series.remove(id); }
        else { self.hidden_series.insert(id.to_string()); }
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    pub fn auto_fit_x(&mut self, cx: &mut Context<Self>) {
        let padding = 0.05;
        let mut changed = false;
        for (idx, x_axis) in self.x_axes.iter().enumerate() {
            let mut x_min = f64::INFINITY; let mut x_max = f64::NEG_INFINITY;
            for series in &self.series {
                if self.hidden_series.contains(&series.id) || series.x_axis_id.0 != idx { continue; }
                if let Some((sx_min, sx_max, _, _)) = series.plot.read().get_min_max() {
                    x_min = x_min.min(sx_min); x_max = x_max.max(sx_max);
                }
            }
            if x_min != f64::INFINITY {
                x_axis.axis.update(cx, |x, _cx| {
                    ViewController::auto_fit_axis(x, x_min, x_max, padding);
                    x.update_ticks_if_needed(10);
                });
                changed = true;
            }
        }
        if changed { self.shared_state.update(cx, |s, _| s.request_render()); }
    }

    pub fn auto_fit_y(&mut self, axis_index: Option<usize>, cx: &mut Context<Self>) {
        let padding = 0.05;
        let x_range = if let Some(xa) = self.x_axes.first() {
            let r = xa.axis.read(cx); Some((r.min, r.max))
        } else { None };

        let mut changed = false;
        for (idx, y_axis) in self.y_axes.iter().enumerate() {
            if let Some(target_idx) = axis_index {
                if idx != target_idx { continue; }
            }
            
            let mut sy_min = f64::INFINITY; let mut sy_max = f64::NEG_INFINITY;
            for series in &self.series {
                if self.hidden_series.contains(&series.id) || series.y_axis_id.0 != idx { continue; }
                let range = if let Some((xmin, xmax)) = x_range {
                    series.plot.read().get_y_range(xmin, xmax)
                } else {
                    series.plot.read().get_min_max().map(|(_, _, ymin, ymax)| (ymin, ymax))
                };
                if let Some((s_min, s_max)) = range { sy_min = sy_min.min(s_min); sy_max = sy_max.max(s_max); }
            }
            if sy_min != f64::INFINITY {
                y_axis.axis.update(cx, |y, _cx| {
                    ViewController::auto_fit_axis(y, sy_min, sy_max, padding);
                    y.update_ticks_if_needed(10);
                });
                changed = true;
            }
        }
        if changed { self.shared_state.update(cx, |s, _| s.request_render()); }
    }

    // --- Action Handlers ---
    fn handle_pan_left(&mut self, _: &PanLeft, _win: &mut Window, cx: &mut Context<Self>) { self.pan_x(-0.1, cx); }
    fn handle_pan_right(&mut self, _: &PanRight, _win: &mut Window, cx: &mut Context<Self>) { self.pan_x(0.1, cx); }
    fn handle_pan_up(&mut self, _: &PanUp, _win: &mut Window, cx: &mut Context<Self>) { self.pan_y(0.1, cx); }
    fn handle_pan_down(&mut self, _: &PanDown, _win: &mut Window, cx: &mut Context<Self>) { self.pan_y(-0.1, cx); }
    fn handle_zoom_in(&mut self, _: &ZoomIn, _win: &mut Window, cx: &mut Context<Self>) { self.keyboard_zoom(0.9, cx); }
    fn handle_zoom_out(&mut self, _: &ZoomOut, _win: &mut Window, cx: &mut Context<Self>) { self.keyboard_zoom(1.1, cx); }
    fn handle_reset_view(&mut self, _: &ResetView, _win: &mut Window, cx: &mut Context<Self>) { 
        self.auto_fit_x(cx); 
        self.auto_fit_y(None, cx); 
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    fn pan_x(&mut self, factor: f64, cx: &mut Context<Self>) {
        for x_axis in &self.x_axes {
            x_axis.axis.update(cx, |x, _cx| { 
                ViewController::pan_axis(x, (factor * 200.0) as f32, 200.0, false); 
                x.update_ticks_if_needed(10);
            });
        }
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    fn pan_y(&mut self, factor: f64, cx: &mut Context<Self>) {
        for y_axis in &self.y_axes {
            y_axis.axis.update(cx, |y, _cx| { 
                ViewController::pan_axis(y, (factor * 200.0) as f32, 200.0, true); 
                y.update_ticks_if_needed(10);
            });
        }
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    fn keyboard_zoom(&mut self, factor: f64, cx: &mut Context<Self>) {
        for x_axis in &self.x_axes {
            x_axis.axis.update(cx, |x, _cx| { 
                ViewController::zoom_axis_at(x, 0.5, factor); 
                x.update_ticks_if_needed(10);
            });
        }
        for y_axis in &self.y_axes {
            y_axis.axis.update(cx, |y, _cx| { 
                ViewController::zoom_axis_at(y, 0.5, factor); 
                y.update_ticks_if_needed(10);
            });
        }
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    fn apply_inertia(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_dragging || (self.velocity.x.abs() < 0.1 && self.velocity.y.abs() < 0.1) { return; }
        
        let dt = 1.0 / 60.0;
        ViewController::apply_friction(&mut self.velocity.x, self.inertia_config.friction, dt);
        ViewController::apply_friction(&mut self.velocity.y, self.inertia_config.friction, dt);

        let bounds = *self.bounds.borrow();
        if !bounds.is_empty() {
            let (pw, ph) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
            for x_axis in &self.x_axes {
                x_axis.axis.update(cx, |x, _cx| { 
                    x.pan(self.velocity.x * dt * (x.span() / pw)); 
                    x.clamp(); 
                    x.update_ticks_if_needed(10);
                });
            }
            for y_axis in &self.y_axes {
                y_axis.axis.update(cx, |y, _cx| { 
                    y.pan(self.velocity.y * dt * (y.span() / ph)); 
                    y.clamp(); 
                    y.update_ticks_if_needed(10);
                });
            }
            self.shared_state.update(cx, |s, _| s.request_render());
        }
        cx.on_next_frame(window, |this, window, cx| { this.apply_inertia(window, cx); });
    }

    fn handle_mouse_down(&mut self, event: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if event.button == MouseButton::Left {
            if event.click_count >= 2 {
                if event.modifiers.shift { self.auto_fit_x(cx); }
                self.auto_fit_y(None, cx);
                return;
            }
            self.drag_mode = DragMode::Plot;
            self.drag_start = Some(event.position);
            self.last_drag_time = Some(Instant::now());
            self.velocity = Point::default();
            self.is_dragging = true;
            self.shared_state.update(cx, |s, _| s.request_render());
        } else if event.button == MouseButton::Right {
            self.box_zoom_start = Some(event.position);
            self.box_zoom_current = Some(event.position);
            self.shared_state.update(cx, |s, _| s.request_render());
        } else if event.button == MouseButton::Middle {
            self.zoom_drag_mode = DragMode::Plot;
            self.zoom_drag_start = Some(event.position);
            self.zoom_drag_last = Some(event.position);
            self.is_dragging = true;
            self.shared_state.update(cx, |s, _| s.request_render());
        }
    }

    fn handle_mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        let mut changed = false;
        if event.button == MouseButton::Left {
            self.drag_mode = DragMode::None;
            self.drag_start = None;
            self.is_dragging = false;
            if let Some(lt) = self.last_drag_time {
                if Instant::now().duration_since(lt) > self.inertia_config.stop_threshold { self.velocity = Point::new(0.0, 0.0); }
            }
            if self.inertia_config.enabled && (self.velocity.x.abs() > 1.0 || self.velocity.y.abs() > 1.0) {
                self.apply_inertia(window, cx);
            }
            changed = true;
        } else if event.button == MouseButton::Middle {
            self.zoom_drag_start = None;
            self.zoom_drag_last = None;
            self.zoom_drag_mode = DragMode::None;
            self.is_dragging = false;
            changed = true;
        } else if event.button == MouseButton::Right {
            if let Some(start) = self.box_zoom_start {
                let end = event.position;
                let bounds = *self.bounds.borrow();
                if !bounds.is_empty() {
                    if let (Some(x0), Some(y0)) = (self.x_axes.first(), self.y_axes.first()) {
                        let x_range = x0.axis.read(cx);
                        let x_scale = crate::scales::ChartScale::new_linear((x_range.min, x_range.max), (0.0, bounds.size.width.as_f32()));
                        let px1 = x_scale.invert((start.x - bounds.origin.x).as_f32());
                        let px2 = x_scale.invert((end.x - bounds.origin.x).as_f32());
                        
                        x0.axis.update(cx, |x, _cx| {
                            ViewController::auto_fit_axis(x, px1.min(px2), px1.max(px2), 0.0);
                            x.update_ticks_if_needed(10);
                        });

                        let y_range = y0.axis.read(cx);
                        let y_scale = crate::scales::ChartScale::new_linear((y_range.min, y_range.max), (bounds.size.height.as_f32(), 0.0));
                        let py1 = y_scale.invert((start.y - bounds.origin.y).as_f32());
                        let py2 = y_scale.invert((end.y - bounds.origin.y).as_f32());
                        
                        y0.axis.update(cx, |y, _cx| {
                            ViewController::auto_fit_axis(y, py1.min(py2), py1.max(py2), 0.0);
                            y.update_ticks_if_needed(10);
                        });
                    }
                }
            }
            self.box_zoom_start = None;
            self.box_zoom_current = None;
            changed = true;
        }

        if changed {
            self.shared_state.update(cx, |s, _| s.request_render());
        }
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let canvas_bounds = *self.bounds.borrow();
        if canvas_bounds.is_empty() { return; }

        let local_pos = event.position - canvas_bounds.origin;
        let mut mouse_changed = false;
        let mut data_x = None;

        if canvas_bounds.contains(&event.position) {
            if let Some(x_axis) = self.x_axes.first() {
                let x_range = x_axis.axis.read(cx);
                data_x = Some(ViewController::map_pixels_to_value(
                    local_pos.x.as_f32(),
                    canvas_bounds.size.width.as_f32(),
                    x_range.min,
                    x_range.max,
                    false
                ));
            }
            mouse_changed = true;
        }

        let mut axes_changed = false;

        if let Some(start) = self.drag_start {
            let delta = event.position - start;
            let now = Instant::now();
            let pw = canvas_bounds.size.width.as_f32();
            let ph = canvas_bounds.size.height.as_f32();

            if self.drag_mode == DragMode::Plot {
                for x_axis in &self.x_axes {
                    x_axis.axis.update(cx, |x, _cx| {
                        ViewController::pan_axis(x, delta.x.as_f32(), pw, false);
                        x.update_ticks_if_needed(10);
                    });
                }
                for y_axis in &self.y_axes {
                    y_axis.axis.update(cx, |y, _cx| {
                        ViewController::pan_axis(y, delta.y.as_f32(), ph, true);
                        y.update_ticks_if_needed(10);
                    });
                }
                axes_changed = true;
            }
            if let Some(last_time) = self.last_drag_time {
                let dt = now.duration_since(last_time).as_secs_f64();
                if dt > 0.0 { self.velocity = Point::new(delta.x.as_f32() as f64 / dt, delta.y.as_f32() as f64 / dt); }
            }
            self.drag_start = Some(event.position);
            self.last_drag_time = Some(now);
        }

        if let (Some(start), Some(last)) = (self.zoom_drag_start, self.zoom_drag_last) {
            let delta = event.position - last;
            let factor_x = ViewController::compute_zoom_factor(delta.x.as_f32(), 100.0);
            let factor_y = ViewController::compute_zoom_factor(-delta.y.as_f32(), 100.0);
            let pw = canvas_bounds.size.width.as_f32() as f64;
            let ph = canvas_bounds.size.height.as_f32() as f64;
            let pivot_x_pct = (start.x.as_f32() as f64 - canvas_bounds.origin.x.as_f32() as f64) / pw;
            let pivot_y_pct = (start.y.as_f32() as f64 - canvas_bounds.origin.y.as_f32() as f64) / ph;
            
            for x_axis in &self.x_axes {
                x_axis.axis.update(cx, |x, _cx| {
                    ViewController::zoom_axis_at(x, pivot_x_pct, factor_x);
                    x.update_ticks_if_needed(10);
                });
            }
            for y_axis in &self.y_axes {
                y_axis.axis.update(cx, |y, _cx| {
                    ViewController::zoom_axis_at(y, 1.0 - pivot_y_pct, factor_y);
                    y.update_ticks_if_needed(10);
                });
            }
            axes_changed = true;
            self.zoom_drag_last = Some(event.position);
        }

        if self.box_zoom_start.is_some() { 
            self.box_zoom_current = Some(event.position); 
            mouse_changed = true;
        }

        if mouse_changed || axes_changed {
            let entity_id = cx.entity_id();
            self.shared_state.update(cx, |state, _cx| {
                if mouse_changed {
                    state.mouse_pos = Some(event.position);
                    state.active_chart_id = Some(entity_id);
                    state.hover_x = data_x;
                }
                state.request_render();
            });
        }
    }

    fn handle_scroll_wheel(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let bounds = *self.bounds.borrow();
        if bounds.is_empty() { return; }
        let is_zoom = event.modifiers.control || event.modifiers.platform;
        let delta_y = match event.delta { ScrollDelta::Pixels(p) => p.y.as_f32(), ScrollDelta::Lines(p) => p.y as f32 * 20.0 };

        if is_zoom {
            let factor = (1.0f64 - delta_y as f64 * 0.01).clamp(0.1, 10.0);
            let mx_pct = (event.position.x - bounds.origin.x).as_f32() as f64 / bounds.size.width.as_f32() as f64;
            for x_axis in &self.x_axes {
                x_axis.axis.update(cx, |x, _cx| { 
                    ViewController::zoom_axis_at(x, mx_pct, factor); 
                    x.update_ticks_if_needed(10);
                });
            }
            let my_pct = (event.position.y - bounds.origin.y).as_f32() as f64 / bounds.size.height.as_f32() as f64;
            for y_axis in &self.y_axes {
                y_axis.axis.update(cx, |y, _cx| { 
                    ViewController::zoom_axis_at(y, 1.0 - my_pct, factor); 
                    y.update_ticks_if_needed(10);
                });
            }
        } else {
            let delta_x = match event.delta { ScrollDelta::Pixels(p) => p.x.as_f32(), ScrollDelta::Lines(p) => p.x as f32 * 20.0 };
            for x_axis in &self.x_axes {
                x_axis.axis.update(cx, |x, _cx| { 
                    ViewController::pan_axis(x, delta_x, bounds.size.width.as_f32(), false); 
                    x.update_ticks_if_needed(10);
                });
            }
            for y_axis in &self.y_axes {
                y_axis.axis.update(cx, |y, _cx| { 
                    ViewController::pan_axis(y, delta_y, bounds.size.height.as_f32(), true); 
                    y.update_ticks_if_needed(10);
                });
            }
        }
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    fn render_legend_button(&self, label: &'static str, enabled: bool, cx: &mut Context<Self>, on_click: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static) -> impl IntoElement {
        div()
            .size_5()
            .flex().items_center().justify_center()
            .bg(gpui::white().alpha(0.1))
            .rounded_sm()
            .text_size(px(10.0))
            .text_color(if enabled { gpui::white() } else { gpui::white().alpha(0.2) })
            .when(enabled, |d| d.hover(|s| s.bg(gpui::blue().alpha(0.4))).cursor_pointer().on_mouse_down(MouseButton::Left, cx.listener(on_click)))
            .child(label)
    }
}

impl Focusable for ChartPane {
    fn focus_handle(&self, _cx: &App) -> FocusHandle { self.focus_handle.clone() }
}

impl Render for ChartPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Calculate notify rate
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_stats_reset).as_secs_f64() * 1000.0;
        if elapsed > 500.0 { // Refresh every 500ms
            self.last_notify_rate = self.notify_count as f64 / elapsed;
            self.notify_count = 0;
            self.last_stats_reset = now;
        }

        let vs: Vec<Series> = self.series.iter().filter(|s| !self.hidden_series.contains(&s.id)).cloned().collect();
        let bounds_rc = self.bounds.clone();
        let shared_state = self.shared_state.read(cx);
        let hx = shared_state.hover_x;
        let mouse_pos = shared_state.mouse_pos;

        let x_axes = self.x_axes.clone();
        let y_axes = self.y_axes.clone();
        let lc = self.label_color;
        let sc = self.show_crosshair;

        let cursor = if self.is_dragging { CursorStyle::Crosshair } else if self.box_zoom_start.is_some() { CursorStyle::Arrow } else { CursorStyle::Crosshair };
        let this_paint_stats = self.last_paint_stats.clone();
        let debug_mode = self.debug_mode;

        div().track_focus(&self.focus_handle).size_full().relative().bg(gpui::transparent_black()).cursor(cursor)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::handle_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::handle_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::handle_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::handle_mouse_up))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_scroll_wheel(cx.listener(Self::handle_scroll_wheel))
            .on_action(cx.listener(Self::handle_pan_left))
            .on_action(cx.listener(Self::handle_pan_right))
            .on_action(cx.listener(Self::handle_pan_up))
            .on_action(cx.listener(Self::handle_pan_down))
            .on_action(cx.listener(Self::handle_zoom_in))
            .on_action(cx.listener(Self::handle_zoom_out))
            .on_action(cx.listener(Self::handle_reset_view))
            .on_action(cx.listener(|this, _: &ToggleDebug, _win, cx| {
                this.debug_mode = !this.debug_mode;
                this.shared_state.update(cx, |s, _| s.request_render());
            }))
            .child(canvas(|_, _, _| {}, {
                let vs = vs.clone();
                let this_paint_stats = this_paint_stats.clone();
                move |bounds, (), window, cx| {
                    *bounds_rc.borrow_mut() = bounds;
                    if x_axes.is_empty() || y_axes.is_empty() { return; }
                    
                    let x_domains: Vec<(f64, f64)> = x_axes.iter().map(|a| a.axis.read(cx).clamped_bounds()).collect();
                    let y_domains: Vec<(f64, f64)> = y_axes.iter().map(|a| a.axis.read(cx).clamped_bounds()).collect();
                    
                    window.with_content_mask(Some(ContentMask { bounds }), |window| {
                        // 1. Grille (Utilisant le premier axe X et le premier axe Y)
                        let x0 = x_axes[0].axis.read(cx);
                        let y0 = y_axes[0].axis.read(cx);
                        let x_scale = crate::scales::ChartScale::new_linear(x_domains[0], (0.0, bounds.size.width.as_f32()));
                        let x_ticks = d3rs::scale::LinearScale::new().domain(x_domains[0].0, x_domains[0].1).range(0.0, bounds.size.width.as_f32() as f64).ticks(10);
                        
                        let y_render_info = crate::rendering::YAxisRenderInfo {
                            domain: y_domains[0],
                            scale: crate::scales::ChartScale::new_linear(y_domains[0], (bounds.size.height.as_f32(), 0.0)),
                            ticks: d3rs::scale::LinearScale::new().domain(y_domains[0].0, y_domains[0].1).range(bounds.size.height.as_f32() as f64, 0.0).ticks(10),
                            limits: (y0.min_limit, y0.max_limit),
                            edge: y_axes[0].edge,
                            size: y_axes[0].size,
                            offset: px(0.0),
                        };

                        let x_domain_full = crate::data_types::AxisDomain {
                            x_min: x_domains[0].0,
                            x_max: x_domains[0].1,
                            x_min_limit: x0.min_limit,
                            x_max_limit: x0.max_limit,
                            ..Default::default()
                        };

                        crate::rendering::paint_grid(window, bounds, &x_domain_full, &x_scale, &x_ticks, &y_render_info);

                        // 2. Tracé des séries
                        let stats = crate::rendering::paint_plot(window, bounds, &vs, &x_domains, &y_domains, cx);
                        if let Ok(mut s) = this_paint_stats.try_borrow_mut() {
                            *s = stats;
                        }
                        
                        if sc {
                            if let Some(hx_val) = hx {
                                let transform = crate::transform::PlotTransform::new(
                                    x_scale.clone(),
                                    y_render_info.scale.clone(),
                                    bounds
                                );
                                let sx = transform.x_data_to_screen(hx_val);
                                let mut builder = PathBuilder::stroke(px(1.0));
                                builder.move_to(Point::new(sx, bounds.origin.y));
                                builder.line_to(Point::new(sx, bounds.origin.y + bounds.size.height));
                                if let Ok(path) = builder.build() { window.paint_path(path, lc.alpha(0.3)); }
                            }
                            if let Some(pos) = mouse_pos {
                                if bounds.contains(&pos) {
                                    let mut builder = PathBuilder::stroke(px(1.0));
                                    builder.move_to(Point::new(bounds.origin.x, pos.y));
                                    builder.line_to(Point::new(bounds.origin.x + bounds.size.width, pos.y));
                                    if let Ok(path) = builder.build() { window.paint_path(path, lc.alpha(0.3)); }
                                }
                            }
                        }
                    });
                }
            }).size_full().absolute())
            .when(debug_mode, |parent| {
                if let Ok(stats) = self.last_paint_stats.try_borrow() {
                    parent.child(
                        div().absolute().bottom_2().left_2().bg(gpui::black().alpha(0.6)).p_1().rounded_sm().text_size(px(10.0)).text_color(gpui::green())
                            .child(format!("Paint: {:.2}ms | Series: {} | Notify: {:.2}/ms", 
                                stats.duration.as_secs_f64() * 1000.0, 
                                stats.series_count,
                                self.last_notify_rate))
                    )
                } else {
                    parent
                }
            })
            .children(if self.legend_config.enabled {
                let mut items = vec![];
                for s in &self.series {
                    let id = s.id.clone();
                    let hidden = self.hidden_series.contains(&id);
                    
                    let other_on_same_axis = self.series.iter()
                        .filter(|other| other.id != id && other.y_axis_id.0 == s.y_axis_id.0)
                        .count() > 0;
                    
                    let is_isolated = s.y_axis_id.0 != 0;
                    let s_enabled = is_isolated || other_on_same_axis;

                    items.push(div().flex().items_center().gap_2().group("legend_item")
                        .child(div().flex().items_center().gap_1().cursor_pointer()
                            .on_mouse_down(MouseButton::Left, { let id = id.clone(); cx.listener(move |this, _, _, cx| this.toggle_series_visibility(&id, cx)) })
                            .child(div().w_3().h_3().bg(if hidden { gpui::transparent_black() } else { gpui::blue() }).border_1().border_color(gpui::white()))
                            .child(div().text_size(px(10.0)).text_color(if hidden { self.label_color.alpha(0.4) } else { self.label_color }).child(id.clone())))
                        .child(div().flex().gap_1()
                            .child(self.render_legend_button("▲", true, cx, { let id = id.clone(); move |this, _, win, cx| if let Some(f) = &this.on_move_series { f(&id, true, win, cx); } }))
                            .child(self.render_legend_button("▼", true, cx, { let id = id.clone(); move |this, _, win, cx| if let Some(f) = &this.on_move_series { f(&id, false, win, cx); } }))
                            .child(self.render_legend_button("S", s_enabled, cx, { let id = id.clone(); move |this, _, win, cx| if s_enabled { if let Some(f) = &this.on_isolate_series { f(&id, win, cx); } } }))
                        ).into_any_element());
                }
                let mut leg = div().absolute().bg(self.bg_color.alpha(0.8)).p_2().rounded_md().border_1().border_color(self.label_color.alpha(0.2)).flex().gap_2();
                if self.legend_config.orientation == Orientation::Vertical { leg = leg.flex_col().gap_1(); } else { leg = leg.flex_row().gap_3(); }
                match self.legend_config.position {
                    LegendPosition::TopLeft => leg = leg.top(px(10.0)).left(px(10.0)),
                    LegendPosition::TopRight => leg = leg.top(px(10.0)).right(px(10.0)),
                    LegendPosition::BottomLeft => leg = leg.bottom(px(10.0)).left(px(10.0)),
                    LegendPosition::BottomRight => leg = leg.bottom(px(10.0)).right(px(10.0)),
                    _ => leg = leg.top(px(10.0)).left(px(10.0)),
                }
                Some(leg.children(items).into_any_element())
            } else { None })
    }
}