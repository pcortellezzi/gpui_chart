// ChartView implementation

use crate::data_types::{AxisDomain, PlotData, Series, Ticks};
use crate::rendering::{paint_axes, paint_grid, paint_plot, paint_crosshair, create_axis_tag};
use gpui::prelude::*;
use gpui::*;
use adabraka_ui::util::PixelsExt;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Instant, Duration};
use tracing::info;
use d3rs::scale::{Scale, LinearScale};

actions!(gpui_chart, [
    Init, 
    PanLeft, 
    PanRight, 
    PanUp, 
    PanDown, 
    ZoomIn, 
    ZoomOut, 
    ResetView
]);

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
    pub friction: f64,    // Factor per frame (e.g., 0.92)
    pub sensitivity: f64, // Multiplier for captured velocity
    pub stop_threshold: Duration, // Threshold to cancel inertia if no move detected before release
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

/// La `View` principale qui gère l'état et la logique du graphique.
pub struct ChartView {
    pub domain: AxisDomain,
    pub series: Vec<Series>,
    pub hidden_series: HashSet<String>,
    pub bg_color: Hsla,
    pub label_color: Hsla,
    
    // UI Options
    pub show_crosshair: bool,
    pub show_axis_tags: bool,
    pub show_tooltip: bool,
    pub legend_config: LegendConfig,
    pub inertia_config: InertiaConfig,

    drag_start: Option<Point<Pixels>>,
    last_drag_time: Option<Instant>,
    velocity: Point<f64>, // Data units per second
    
    zoom_drag_start: Option<Point<Pixels>>,
    zoom_drag_last: Option<Point<Pixels>>,
    
    box_zoom_start: Option<Point<Pixels>>,
    box_zoom_current: Option<Point<Pixels>>,

    pub mouse_pos: Option<Point<Pixels>>,
    is_dragging: bool,

    data_changed: bool,
    bounds: Rc<RefCell<Bounds<Pixels>>>,
    dirty_series: HashSet<String>,
    focus_handle: FocusHandle,
}

impl ChartView {
    /// Crée une nouvelle ChartView.
    pub fn new(cx: &mut Context<Self>) -> Self {
        info!("ChartPanel new called");
        let domain = AxisDomain {
            x_min: 0.0,
            x_max: 10.0,
            y_min: 0.0,
            y_max: 10.0,
        };

        Self {
            domain,
            series: vec![],
            hidden_series: HashSet::new(),
            bg_color: gpui::black(),
            label_color: gpui::white(),
            
            show_crosshair: true,
            show_axis_tags: true,
            show_tooltip: true,
            legend_config: LegendConfig::default(),
            inertia_config: InertiaConfig::default(),

            drag_start: None,
            last_drag_time: None,
            velocity: Point::default(),

            zoom_drag_start: None,
            zoom_drag_last: None,
            
            box_zoom_start: None,
            box_zoom_current: None,

            mouse_pos: None,
            is_dragging: false,

            data_changed: true,
            bounds: Rc::new(RefCell::new(Bounds::new(
                Point::new(px(0.0), px(0.0)),
                Size {
                    width: px(0.0),
                    height: px(0.0),
                },
            ))),
            dirty_series: HashSet::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    fn handle_pan_left(&mut self, _: &PanLeft, _win: &mut Window, cx: &mut Context<Self>) { self.pan_x(-0.1, cx); }
    fn handle_pan_right(&mut self, _: &PanRight, _win: &mut Window, cx: &mut Context<Self>) { self.pan_x(0.1, cx); }
    fn handle_pan_up(&mut self, _: &PanUp, _win: &mut Window, cx: &mut Context<Self>) { self.pan_y(0.1, cx); }
    fn handle_pan_down(&mut self, _: &PanDown, _win: &mut Window, cx: &mut Context<Self>) { self.pan_y(-0.1, cx); }
    fn handle_zoom_in(&mut self, _: &ZoomIn, _win: &mut Window, cx: &mut Context<Self>) { self.keyboard_zoom(0.9, cx); }
    fn handle_zoom_out(&mut self, _: &ZoomOut, _win: &mut Window, cx: &mut Context<Self>) { self.keyboard_zoom(1.1, cx); }
    fn handle_reset_view(&mut self, _: &ResetView, _win: &mut Window, cx: &mut Context<Self>) {
        self.auto_fit_axes();
        cx.notify();
    }

    fn pan_x(&mut self, factor: f64, cx: &mut Context<Self>) {
        let width = self.domain.width();
        let dx = width * factor;
        self.domain.x_min += dx;
        self.domain.x_max += dx;
        cx.notify();
    }

    fn pan_y(&mut self, factor: f64, cx: &mut Context<Self>) {
        let height = self.domain.height();
        let dy = height * factor;
        self.domain.y_min += dy;
        self.domain.y_max += dy;
        cx.notify();
    }

    fn keyboard_zoom(&mut self, factor: f64, cx: &mut Context<Self>) {
        let width = self.domain.width();
        let height = self.domain.height();
        let mid_x = self.domain.x_min + width / 2.0;
        let mid_y = self.domain.y_min + height / 2.0;

        let new_width = width * factor;
        let new_height = height * factor;

        self.domain.x_min = mid_x - new_width / 2.0;
        self.domain.x_max = mid_x + new_width / 2.0;
        self.domain.y_min = mid_y - new_height / 2.0;
        self.domain.y_max = mid_y + new_height / 2.0;
        cx.notify();
    }

    pub fn add_data(&mut self, series_id: &str, data: PlotData, cx: &mut Context<Self>) {
        if let Some(series) = self.series.iter_mut().find(|s| s.id == series_id) {
            series.plot.borrow_mut().add_data(data);
            self.data_changed = true;
            self.dirty_series.insert(series_id.to_string());
            cx.notify();
        }
    }

    pub fn add_series(&mut self, series: Series) {
        self.series.push(series);
    }

    pub fn set_data(&mut self, series_id: &str, data: Vec<PlotData>, cx: &mut Context<Self>) {
        if let Some(series) = self.series.iter_mut().find(|s| s.id == series_id) {
            series.plot.borrow_mut().set_data(data);
            self.data_changed = true;
            self.dirty_series.insert(series_id.to_string());
            cx.notify();
        }
    }

    pub fn clear_old_data(&mut self, series_id: &str, before_time: f64, cx: &mut Context<Self>) {
        if let Some(series) = self.series.iter_mut().find(|s| s.id == series_id) {
            series.plot.borrow_mut().clear_before(before_time);
            self.data_changed = true;
            self.dirty_series.insert(series_id.to_string());
            cx.notify();
        }
    }

    pub fn add_batch(&mut self, series_id: &str, data: Vec<PlotData>, cx: &mut Context<Self>) {
        if let Some(series) = self.series.iter_mut().find(|s| s.id == series_id) {
            for d in data {
                series.plot.borrow_mut().add_data(d);
            }
            self.data_changed = true;
            self.dirty_series.insert(series_id.to_string());
            cx.notify();
        }
    }

    pub fn set_x_domain(&mut self, range: std::ops::RangeInclusive<f64>) {
        self.domain.x_min = *range.start();
        self.domain.x_max = *range.end();
    }

    pub fn get_pixel_width(&self) -> f32 {
        self.bounds.borrow().size.width.as_f32()
    }

    /// Automatically adjusts the axes to fit all series data points with padding.
    pub fn auto_fit_axes(&mut self) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for series in &self.series {
            if self.hidden_series.contains(&series.id) { continue; }
            if let Some((sx_min, sx_max, sy_min, sy_max)) = series.plot.borrow().get_min_max() {
                x_min = x_min.min(sx_min);
                x_max = x_max.max(sx_max);
                y_min = y_min.min(sy_min);
                y_max = y_max.max(sy_max);
            }
        }

        if x_min == f64::INFINITY {
            self.data_changed = false;
            return;
        }

        let x_data_range = x_max - x_min;
        let y_data_range = y_max - y_min;
        let padding = 0.05;

        if x_data_range <= f64::EPSILON {
            let half_range = 30.0;
            self.domain.x_min = x_min - half_range;
            self.domain.x_max = x_max + half_range;
        } else {
            self.domain.x_min = x_min - x_data_range * padding;
            self.domain.x_max = x_max + x_data_range * padding;
        }

        if y_data_range <= f64::EPSILON {
            let half_range = if y_min.abs() > f64::EPSILON {
                y_min.abs() * 0.05
            } else {
                5.0
            };
            self.domain.y_min = y_min - half_range;
            self.domain.y_max = y_max + half_range;
        } else {
            self.domain.y_min = y_min - y_data_range * padding;
            self.domain.y_max = y_max + y_data_range * padding;
        }

        self.data_changed = false;
    }

    /// Gère l'événement de zoom (molette de la souris) et le pan à deux doigts (Trackpad).
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

        if is_zoom {
            let zoom_factor = (1.0f64 - delta_y * 0.01).clamp(0.1, 10.0);
            let mouse_pos = event.position;

            let domain = &mut self.domain;
            let (pixels_w, pixels_h) = (
                bounds.size.width.as_f32() as f64,
                bounds.size.height.as_f32() as f64,
            );
            let (data_w, data_h) = (domain.width(), domain.height());
            let mouse_x_pct =
                (mouse_pos.x.as_f32() as f64 - bounds.origin.x.as_f32() as f64) / pixels_w;
            let mouse_y_pct =
                (mouse_pos.y.as_f32() as f64 - bounds.origin.y.as_f32() as f64) / pixels_h;

            let mouse_x_data = domain.x_min + data_w * mouse_x_pct;
            let mouse_y_data = domain.y_min + data_h * (1.0 - mouse_y_pct);

            let new_data_w = data_w * zoom_factor;
            let new_data_h = data_h * zoom_factor;

            domain.x_min = mouse_x_data - new_data_w * mouse_x_pct;
            domain.x_max = domain.x_min + new_data_w;
            domain.y_min = mouse_y_data - new_data_h * (1.0 - mouse_y_pct);
            domain.y_max = domain.y_min + new_data_h;
        } else {
            let domain = &mut self.domain;
            let (pixels_w, pixels_h) = (
                bounds.size.width.as_f32() as f64,
                bounds.size.height.as_f32() as f64,
            );
            let (data_w, data_h) = (domain.width(), domain.height());
            let dx = delta_x * (data_w / pixels_w);
            let dy = delta_y * (data_h / pixels_h);
            
            domain.x_min -= dx;
            domain.x_max -= dx;
            domain.y_min += dy;
            domain.y_max += dy;
        }
        cx.notify();
    }

    /// Gère le début du glissement.
    fn start_drag(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.click_count >= 2 {
            self.auto_fit_axes();
            cx.notify();
            return;
        }
        self.drag_start = Some(event.position);
        self.last_drag_time = Some(Instant::now());
        self.velocity = Point::default();
        self.is_dragging = true;
        cx.notify();
    }

    /// Gère le glissement et le survol.
    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mouse_pos = Some(event.position);

        if let Some(start) = self.drag_start {
            self.is_dragging = true;
            let now = Instant::now();
            let delta_pixels = event.position - start;
            let bounds = *self.bounds.borrow();
            
            if !bounds.is_empty() {
                let domain = &mut self.domain;
                let (pixels_w, pixels_h) = (
                    bounds.size.width.as_f32() as f64,
                    bounds.size.height.as_f32() as f64,
                );
                let (data_w, data_h) = (domain.width(), domain.height());
                let x_ratio = data_w / pixels_w;
                let y_ratio = data_h / pixels_h;
                
                let dx = delta_pixels.x.as_f32() as f64 * x_ratio;
                let dy = delta_pixels.y.as_f32() as f64 * y_ratio;
                
                if let Some(last_time) = self.last_drag_time {
                    let dt = now.duration_since(last_time).as_secs_f64();
                    if dt > 0.0 {
                        self.velocity = Point::new(
                            (dx / dt) * self.inertia_config.sensitivity, 
                            (dy / dt) * self.inertia_config.sensitivity
                        );
                    }
                }

                domain.x_min -= dx;
                domain.x_max -= dx;
                domain.y_min += dy;
                domain.y_max += dy;
            }
            self.drag_start = Some(event.position);
            self.last_drag_time = Some(now);
        }

        if let Some(_) = self.box_zoom_start {
            self.box_zoom_current = Some(event.position);
        }

        cx.notify();
    }

    /// Gère la fin du glissement.
    fn end_drag(&mut self, _event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.drag_start = None;
        self.is_dragging = false;
        
        if let Some(last_time) = self.last_drag_time {
            if Instant::now().duration_since(last_time) > self.inertia_config.stop_threshold {
                self.velocity = Point::default();
            }
        }

        if self.inertia_config.enabled && (self.velocity.x.abs() > 1.0 || self.velocity.y.abs() > 1.0) {
            self.apply_inertia(window, cx);
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

        let dt = 1.0 / 60.0;
        let dx = self.velocity.x * dt;
        let dy = self.velocity.y * dt;

        self.domain.x_min -= dx;
        self.domain.x_max -= dx;
        self.domain.y_min += dy;
        self.domain.y_max += dy;

        cx.notify();

        cx.on_next_frame(window, |this, window, cx| {
            this.apply_inertia(window, cx);
        });
    }

    /// Gère le début du Box Zoom.
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

    /// Gère la fin du Box Zoom.
    fn end_box_zoom(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(start) = self.box_zoom_start {
            let end = event.position;
            let bounds = *self.bounds.borrow();
            
            if !bounds.is_empty() {
                let x_scale = crate::scales::ChartScale::new_linear(
                    (self.domain.x_min, self.domain.x_max),
                    (0.0, bounds.size.width.as_f32())
                );
                let y_scale = crate::scales::ChartScale::new_linear(
                    (self.domain.y_min, self.domain.y_max),
                    (bounds.size.height.as_f32(), 0.0)
                );
                let transform = crate::transform::PlotTransform::new(x_scale, y_scale, bounds);

                let p1 = transform.screen_to_data(start);
                let p2 = transform.screen_to_data(end);

                if (end.x - start.x).abs() > px(5.0) || (end.y - start.y).abs() > px(5.0) {
                    self.domain.x_min = p1.x.min(p2.x);
                    self.domain.x_max = p1.x.max(p2.x);
                    self.domain.y_min = p1.y.min(p2.y);
                    self.domain.y_max = p1.y.max(p2.y);
                }
            }
        }
        self.box_zoom_start = None;
        self.box_zoom_current = None;
        cx.notify();
    }

    /// Gère le début du zoom par glissement.
    fn start_zoom_drag(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.zoom_drag_start = Some(event.position);
        self.zoom_drag_last = Some(event.position);
        self.is_dragging = true;
        cx.notify();
    }

    /// Gère le zoom par glissement.
    fn handle_zoom_drag(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let (Some(start), Some(last)) = (self.zoom_drag_start, self.zoom_drag_last) {
            self.is_dragging = true;
            let bounds = *self.bounds.borrow();
            if bounds.is_empty() { return; }

            let delta = event.position - last;
            let delta_x = delta.x.as_f32() as f64;
            let delta_y = delta.y.as_f32() as f64;
            
            let factor_x = 1.0 + delta_x.abs() / 100.0;
            let factor_y = 1.0 + delta_y.abs() / 100.0;
            
            let domain = &mut self.domain;
            
            let width_px = bounds.size.width.as_f32() as f64;
            let height_px = bounds.size.height.as_f32() as f64;
            
            let pivot_x_px = start.x.as_f32() as f64 - bounds.origin.x.as_f32() as f64;
            let pivot_y_px = start.y.as_f32() as f64 - bounds.origin.y.as_f32() as f64;
            
            let pivot_x_pct = (pivot_x_px / width_px).clamp(0.0, 1.0);
            let pivot_y_pct = (pivot_y_px / height_px).clamp(0.0, 1.0);
            
            let pivot_x_domain = domain.x_min + domain.width() * pivot_x_pct;
            let pivot_y_domain = domain.y_min + domain.height() * (1.0 - pivot_y_pct);

            if delta_x > 0.0 { 
                let new_width = domain.width() / factor_x;
                domain.x_min = pivot_x_domain - new_width * pivot_x_pct;
                domain.x_max = domain.x_min + new_width;
            } else if delta_x < 0.0 { 
                let new_width = domain.width() * factor_x;
                domain.x_min = pivot_x_domain - new_width * pivot_x_pct;
                domain.x_max = domain.x_min + new_width;
            }
            
            if delta_y < 0.0 { 
                let new_height = domain.height() / factor_y;
                domain.y_min = pivot_y_domain - new_height * (1.0 - pivot_y_pct);
                domain.y_max = domain.y_min + new_height;
            } else if delta_y > 0.0 { 
                let new_height = domain.height() * factor_y;
                domain.y_min = pivot_y_domain - new_height * (1.0 - pivot_y_pct);
                domain.y_max = domain.y_min + new_height;
            }
            
            self.zoom_drag_last = Some(event.position);
            cx.notify();
        }
    }

    /// Gère la fin du zoom par glissement.
    fn end_zoom_drag(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.zoom_drag_start = None;
        self.zoom_drag_last = None;
        self.is_dragging = false;
        cx.notify();
    }

    /// Logique de génération de graduations utilisant d3rs
    fn calculate_ticks(domain_w: f64, domain_h: f64, domain: &AxisDomain) -> Ticks {
        let x_scale = LinearScale::new()
            .domain(domain.x_min, domain.x_max)
            .range(0.0, domain_w);
        
        let y_scale = LinearScale::new()
            .domain(domain.y_min, domain.y_max)
            .range(domain_h, 0.0);

        Ticks {
            x: x_scale.ticks(10),
            y: y_scale.ticks(10),
        }
    }

    pub fn toggle_series_visibility(&mut self, series_id: &str, cx: &mut Context<Self>) {
        if self.hidden_series.contains(series_id) {
            self.hidden_series.remove(series_id);
        } else {
            self.hidden_series.insert(series_id.to_string());
        }
        cx.notify();
    }
}

impl Focusable for ChartView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// Implémentation `Render` pour la `ChartView`
impl Render for ChartView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.dirty_series.is_empty() {
            self.dirty_series.clear();
        }

        let visible_series: Vec<Series> = self.series.iter()
            .filter(|s| !self.hidden_series.contains(&s.id))
            .cloned()
            .collect();

        let domain_clone = self.domain.clone();
        let bounds_rc = self.bounds.clone();
        let current_bounds = *bounds_rc.borrow();
        let mouse_pos = self.mouse_pos;

        let ticks =
            Self::calculate_ticks(domain_clone.width(), domain_clone.height(), &domain_clone);
        let ticks_clone = ticks.clone();

        let x_scale = crate::scales::ChartScale::new_linear(
            (domain_clone.x_min, domain_clone.x_max),
            (0.0, current_bounds.size.width.as_f32())
        );
        let y_scale = crate::scales::ChartScale::new_linear(
            (domain_clone.y_min, domain_clone.y_max),
            (current_bounds.size.height.as_f32(), 0.0)
        );

        let transform = crate::transform::PlotTransform::new(x_scale.clone(), y_scale.clone(), current_bounds);

        let mut axes_elements = paint_axes(&domain_clone, &x_scale, &y_scale, &ticks, self.label_color);

        if self.show_axis_tags {
            if let Some(pos) = mouse_pos {
                if current_bounds.contains(&pos) {
                    let data_point = transform.screen_to_data(pos);
                    
                    axes_elements.push(create_axis_tag(
                        x_scale.format_tick(data_point.x),
                        pos.x,
                        true,
                        self.bg_color,
                        self.label_color
                    ));

                    axes_elements.push(create_axis_tag(
                        y_scale.format_tick(data_point.y),
                        pos.y,
                        false,
                        self.bg_color,
                        self.label_color
                    ));
                }
            }
        }

        let cursor = if self.is_dragging { 
            CursorStyle::ClosedHand 
        } else if self.box_zoom_start.is_some() {
            CursorStyle::Arrow
        } else { 
            CursorStyle::Crosshair 
        };

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(self.bg_color)
            .relative() 
            .cursor(CursorStyle::Arrow)
            .on_mouse_down(MouseButton::Left, {
                let focus_handle = self.focus_handle.clone();
                move |_event, window, _cx| {
                    window.focus(&focus_handle);
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
            .pl(px(50.0))
            .pb(px(20.0))
            .child(
                div()
                    .size_full()
                    .overflow_hidden()
                    .cursor(cursor)
                    .child(
                        canvas(|_bounds, _window, _cx| {}, {
                            let x_scale = x_scale.clone();
                            let y_scale = y_scale.clone();
                            let label_color = self.label_color;
                            let show_crosshair = self.show_crosshair;
                            move |bounds, (), window, cx| {
                                *bounds_rc.borrow_mut() = bounds;
                                paint_grid(window, bounds, &domain_clone, &x_scale, &y_scale, &ticks_clone);
                                paint_plot(window, bounds, &visible_series, &domain_clone, cx);
                                
                                if show_crosshair {
                                    if let Some(pos) = mouse_pos {
                                        paint_crosshair(window, bounds, pos, label_color);
                                    }
                                }
                            }
                        })
                        .size_full(),
                    )
                    .children(if let (Some(start), Some(current)) = (self.box_zoom_start, self.box_zoom_current) {
                        let origin = current_bounds.origin;
                        let min_x = (start.x.min(current.x)) - origin.x;
                        let max_x = (start.x.max(current.x)) - origin.x;
                        let min_y = (start.y.min(current.y)) - origin.y;
                        let max_y = (start.y.max(current.y)) - origin.y;
                        
                        Some(div()
                            .absolute()
                            .left(min_x)
                            .top(min_y)
                            .w(max_x - min_x)
                            .h(max_y - min_y)
                            .bg(gpui::blue().alpha(0.2))
                            .border_1()
                            .border_color(gpui::blue())
                        )
                    } else {
                        None
                    }),
            )
            .children(axes_elements)
            .children(if self.legend_config.enabled {
                let mut legend_items = vec![];
                for s in &self.series {
                    let id = s.id.clone();
                    let is_hidden = self.hidden_series.contains(&id);
                    legend_items.push(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, {
                                let id = id.clone();
                                cx.listener(move |this, _event, _win, cx| {
                                    this.toggle_series_visibility(&id, cx);
                                })
                            })
                            .child(
                                div()
                                    .w_3()
                                    .h_3()
                                    .bg(if is_hidden { gpui::transparent_black() } else { gpui::blue() })
                                    .border_1()
                                    .border_color(gpui::white())
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(if is_hidden { self.label_color.alpha(0.4) } else { self.label_color })
                                    .child(id)
                            )
                    );
                }

                let mut legend_div = div()
                    .absolute()
                    .bg(self.bg_color.alpha(0.8))
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(self.label_color.alpha(0.2))
                    .flex()
                    .gap_2();

                if self.legend_config.orientation == Orientation::Vertical {
                    legend_div = legend_div.flex_col().gap_1();
                } else {
                    legend_div = legend_div.flex_row().gap_3();
                }

                match self.legend_config.position {
                    LegendPosition::TopLeft => {
                        legend_div = legend_div.top(px(10.0) + self.legend_config.offset.y).left(px(60.0) + self.legend_config.offset.x);
                    }
                    LegendPosition::TopRight => {
                        legend_div = legend_div.top(px(10.0) + self.legend_config.offset.y).right(px(10.0) + self.legend_config.offset.x);
                    }
                    LegendPosition::BottomLeft => {
                        legend_div = legend_div.bottom(px(30.0) + self.legend_config.offset.y).left(px(60.0) + self.legend_config.offset.x);
                    }
                    LegendPosition::BottomRight => {
                        legend_div = legend_div.bottom(px(30.0) + self.legend_config.offset.y).right(px(10.0) + self.legend_config.offset.x);
                    }
                    LegendPosition::TopCenter => {
                        legend_div = legend_div.top(px(10.0) + self.legend_config.offset.y).left_1_2().ml(px(-50.0));
                    }
                    LegendPosition::BottomCenter => {
                        legend_div = legend_div.bottom(px(30.0) + self.legend_config.offset.y).left_1_2().ml(px(-50.0));
                    }
                    LegendPosition::Custom(p) => {
                        legend_div = legend_div.top(p.y).left(p.x);
                    }
                }

                Some(legend_div.children(legend_items).into_any_element())
            } else {
                None
            })
            .children(if self.show_tooltip {
                mouse_pos.and_then(|pos| {
                    if current_bounds.contains(&pos) {
                        let data_point = transform.screen_to_data(pos);
                        Some(
                            div()
                                .absolute()
                                .left(pos.x + px(10.0))
                                .top(pos.y + px(10.0))
                                .bg(self.label_color)
                                .text_color(self.bg_color)
                                .p_1()
                                .rounded_sm()
                                .text_size(px(10.0))
                                .child(format!("X: {}\nY: {}", x_scale.format_tick(data_point.x), y_scale.format_tick(data_point.y)))
                                .into_any_element()
                        )
                    } else {
                        None
                    }
                })
            } else {
                None
            })
    }
}
