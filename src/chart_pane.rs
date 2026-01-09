use crate::data_types::{AxisDomain, AxisRange, Series, SharedPlotState, ChartAxis, AxisEdge};
use adabraka_ui::util::PixelsExt;
use d3rs::scale::Scale;
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

actions!(
    gpui_chart,
    [PanLeft, PanRight, PanUp, PanDown, ZoomIn, ZoomOut, ResetView]
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

    // UI Options (Contenu uniquement)
    pub show_crosshair: bool,
    pub show_tooltip: bool,
    pub legend_config: LegendConfig,
    pub inertia_config: InertiaConfig,

    pub move_callback: Option<Box<dyn Fn(&str, bool, &mut Window, &mut App)>>,

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

    bounds: Rc<RefCell<Bounds<Pixels>>> ,
    focus_handle: FocusHandle,
}

impl ChartPane {
    /// Crée une nouvelle ChartPane avec un état partagé.
    pub fn new(
        shared_state: Entity<SharedPlotState>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&shared_state, |_, _, cx| cx.notify()).detach();

        Self {
            x_axes: vec![],
            y_axes: vec![],
            shared_state,
            series: vec![],
            hidden_series: HashSet::new(),
            bg_color: gpui::black(),
            label_color: gpui::white(),

            show_crosshair: true,
            show_tooltip: true,
            legend_config: LegendConfig::default(),
            inertia_config: InertiaConfig::default(),
            move_callback: None,

            drag_start: None,
            drag_mode: DragMode::None,
            last_drag_time: None,
            velocity: Point::default(),

            zoom_drag_start: None,
            zoom_drag_last: None,
            zoom_drag_mode: DragMode::None,

            box_zoom_start: None,
            box_zoom_current: None,

            is_dragging: false,

            bounds: Rc::new(RefCell::new(Bounds::default())),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn add_x_axis(&mut self, x_axis: Entity<AxisRange>, cx: &mut Context<Self>) -> usize {
        cx.observe(&x_axis, |_, _, cx| cx.notify()).detach();
        self.x_axes.push(ChartAxis { axis: x_axis, edge: AxisEdge::Bottom, size: px(0.0) });
        self.x_axes.len() - 1
    }

    pub fn add_y_axis(&mut self, y_axis: Entity<AxisRange>, cx: &mut Context<Self>) -> usize {
        cx.observe(&y_axis, |_, _, cx| cx.notify()).detach();
        self.y_axes.push(ChartAxis { axis: y_axis, edge: AxisEdge::Right, size: px(0.0) });
        self.y_axes.len() - 1
    }

    pub fn auto_fit_x(&mut self, cx: &mut Context<Self>) {
        let padding = 0.05;
        for (idx, x_axis) in self.x_axes.iter().enumerate() {
            let mut x_min = f64::INFINITY;
            let mut x_max = f64::NEG_INFINITY;
            for series in &self.series {
                if self.hidden_series.contains(&series.id) || series.x_axis_id.0 != idx { continue; }
                if let Some((sx_min, sx_max, _, _)) = series.plot.borrow().get_min_max() {
                    x_min = x_min.min(sx_min);
                    x_max = x_max.max(sx_max);
                }
            }
            if x_min != f64::INFINITY {
                x_axis.axis.update(cx, |x: &mut AxisRange, cx| {
                    let span = x_max - x_min;
                    if span <= f64::EPSILON { x.min = x_min - 30.0; x.max = x_max + 30.0; }
                    else { x.min = x_min - span * padding; x.max = x_max + span * padding; }
                    x.clamp(); cx.notify();
                });
            }
        }
    }

    pub fn auto_fit_y(&mut self, cx: &mut Context<Self>) {
        let padding = 0.05;
        
        // On récupère d'abord la plage X visible actuelle (depuis le premier axe X)
        let x_range = if let Some(xa) = self.x_axes.first() {
            let r = xa.axis.read(cx);
            Some((r.min, r.max))
        } else {
            None
        };

        for (idx, y_axis) in self.y_axes.iter().enumerate() {
            let mut sy_min = f64::INFINITY;
            let mut sy_max = f64::NEG_INFINITY;
            for series in &self.series {
                if self.hidden_series.contains(&series.id) || series.y_axis_id.0 != idx { continue; }
                
                let range = if let Some((xmin, xmax)) = x_range {
                    series.plot.borrow().get_y_range(xmin, xmax)
                } else {
                    series.plot.borrow().get_min_max().map(|(_, _, ymin, ymax)| (ymin, ymax))
                };

                if let Some((s_min, s_max)) = range {
                    sy_min = sy_min.min(s_min);
                    sy_max = sy_max.max(s_max);
                }
            }
            if sy_min != f64::INFINITY {
                y_axis.axis.update(cx, |y: &mut AxisRange, cx| {
                    let span = sy_max - sy_min;
                    if span <= f64::EPSILON {
                        let h = if sy_min.abs() > 0.0 { sy_min.abs() * 0.05 } else { 5.0 };
                        y.min = sy_min - h; y.max = sy_max + h;
                    } else {
                        y.min = sy_min - span * padding; y.max = sy_max + span * padding;
                    }
                    y.clamp(); cx.notify();
                });
            }
        }
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
        self.auto_fit_x(cx);
        self.auto_fit_y(cx);
    }

    fn pan_x(&mut self, factor: f64, cx: &mut Context<Self>) {
        for x_axis in &self.x_axes {
            x_axis.axis.update(cx, |x: &mut AxisRange, cx| {
                x.pan(x.span() * factor);
                x.clamp();
                cx.notify();
            });
        }
    }

    fn pan_y(&mut self, factor: f64, cx: &mut Context<Self>) {
        for y_axis in &self.y_axes {
            y_axis.axis.update(cx, |y: &mut AxisRange, cx| {
                y.pan(y.span() * factor);
                y.clamp();
                cx.notify();
            });
        }
    }

    fn keyboard_zoom(&mut self, factor: f64, cx: &mut Context<Self>) {
        for x_axis in &self.x_axes {
            x_axis.axis.update(cx, |x: &mut AxisRange, cx| {
                let pivot_data = x.min + x.span() / 2.0;
                x.zoom_at(pivot_data, 0.5, factor);
                cx.notify();
            });
        }
        for y_axis in &self.y_axes {
            y_axis.axis.update(cx, |y: &mut AxisRange, cx| {
                let pivot_data = y.min + y.span() / 2.0;
                y.zoom_at(pivot_data, 0.5, factor);
                cx.notify();
            });
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

        let local_pos = event.position - bounds.origin;
        let (pw, ph) = (
            bounds.size.width.as_f32() as f64,
            bounds.size.height.as_f32() as f64,
        );

        if is_zoom {
            let factor = (1.0f64 - delta_y * 0.01).clamp(0.1, 10.0);
            let mx_pct = local_pos.x.as_f32() as f64 / pw;
            for x_axis in &self.x_axes {
                x_axis.axis.update(cx, |x: &mut AxisRange, cx| {
                    let m_data = x.min + x.span() * mx_pct;
                    x.zoom_at(m_data, mx_pct, factor);
                    cx.notify();
                });
            }

            let my_pct = local_pos.y.as_f32() as f64 / ph;
            for y_axis in &self.y_axes {
                y_axis.axis.update(cx, |y: &mut AxisRange, cx| {
                    let m_data = y.min + y.span() * (1.0 - my_pct);
                    y.zoom_at(m_data, 1.0 - my_pct, factor);
                    cx.notify();
                });
            }
        } else {
            for x_axis in &self.x_axes {
                x_axis.axis.update(cx, |x: &mut AxisRange, cx| {
                    x.pan(-delta_x * (x.span() / pw));
                    x.clamp();
                    cx.notify();
                });
            }

            for y_axis in &self.y_axes {
                y_axis.axis.update(cx, |y: &mut AxisRange, cx| {
                    y.pan(delta_y * (y.span() / ph));
                    y.clamp();
                    cx.notify();
                });
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

        let local_pos = event.position - canvas_bounds.origin;

        if canvas_bounds.contains(&event.position) {
            // Toujours mettre à jour la position et l'ID actif pour le Container
            self.shared_state.update(cx, |state: &mut SharedPlotState, cx| {
                state.mouse_pos = Some(event.position);
                state.active_chart_id = Some(cx.entity_id());
                cx.notify();
            });

            if let Some(x_axis) = self.x_axes.first() {
                let x_range = x_axis.axis.read(cx);
                let pct_x = local_pos.x.as_f32() as f64 / canvas_bounds.size.width.as_f32() as f64;
                let data_x = x_range.min + x_range.span() * pct_x;

                self.shared_state.update(cx, |state: &mut SharedPlotState, _cx| {
                    state.hover_x = Some(data_x);
                });
            }
        }

        if let Some(start) = self.drag_start {
            self.is_dragging = true;
            let now = Instant::now();
            let delta = event.position - start;
            
            let pw = canvas_bounds.size.width.as_f32() as f64;
            let ph = canvas_bounds.size.height.as_f32() as f64;

            if self.drag_mode == DragMode::Plot {
                for x_axis in &self.x_axes {
                    x_axis.axis.update(cx, |x: &mut AxisRange, cx| {
                        x.pan(-delta.x.as_f32() as f64 * (x.span() / pw));
                        x.clamp();
                        cx.notify();
                    });
                }
                for y_axis in &self.y_axes {
                    y_axis.axis.update(cx, |y: &mut AxisRange, cx| {
                        y.pan(delta.y.as_f32() as f64 * (y.span() / ph));
                        cx.notify();
                    });
                }
            }

            if let Some(last_time) = self.last_drag_time {
                let dt = now.duration_since(last_time).as_secs_f64();
                if dt > 0.0 {
                    self.velocity = Point::new(delta.x.as_f32() as f64 / dt, delta.y.as_f32() as f64 / dt);
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
            
            for x_axis in &self.x_axes {
                x_axis.axis.update(cx, |x: &mut AxisRange, cx| {
                    x.pan(self.velocity.x * dt * (x.span() / pw));
                    x.clamp();
                    cx.notify();
                });
            }
            for y_axis in &self.y_axes {
                y_axis.axis.update(cx, |y: &mut AxisRange, cx| {
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
            if event.modifiers.shift {
                self.auto_fit_x(cx);
            }
            self.auto_fit_y(cx);
            return;
        }

        self.drag_mode = DragMode::Plot;
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
                let start_pct = (start.x - bounds.origin.x).as_f32() as f64 / bounds.size.width.as_f32() as f64;
                let end_pct = (end.x - bounds.origin.x).as_f32() as f64 / bounds.size.width.as_f32() as f64;
                let min_x_pct = start_pct.min(end_pct);
                let max_x_pct = start_pct.max(end_pct);

                for x_axis in &self.x_axes {
                    x_axis.axis.update(cx, |x: &mut AxisRange, cx| {
                        let current_span = x.span();
                        x.min = x.min + current_span * min_x_pct;
                        x.max = x.min + current_span * (max_x_pct - min_x_pct);
                        x.clamp();
                        cx.notify();
                    });
                }

                let start_y_pct = (start.y - bounds.origin.y).as_f32() as f64 / bounds.size.height.as_f32() as f64;
                let end_y_pct = (end.y - bounds.origin.y).as_f32() as f64 / bounds.size.height.as_f32() as f64;
                let min_y_pct = start_y_pct.min(end_y_pct);
                let max_y_pct = start_y_pct.max(end_y_pct);

                for y_axis in &self.y_axes {
                    y_axis.axis.update(cx, |y: &mut AxisRange, cx| {
                        let current_span = y.span();
                        y.min = y.min + current_span * (1.0 - max_y_pct);
                        y.max = y.min + current_span * (max_y_pct - min_y_pct);
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
        self.zoom_drag_mode = DragMode::Plot;
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
            if bounds.is_empty() { return; }
            let delta = event.position - last;
            let factor_x = 1.0 + delta.x.as_f32().abs() as f64 / 100.0;
            let factor_y = 1.0 + delta.y.as_f32().abs() as f64 / 100.0;
            
            let pw = bounds.size.width.as_f32() as f64;
            let ph = bounds.size.height.as_f32() as f64;
            
            let pivot_x_pct = (start.x.as_f32() as f64 - bounds.origin.x.as_f32() as f64) / pw;
            let pivot_y_pct = (start.y.as_f32() as f64 - bounds.origin.y.as_f32() as f64) / ph;
            
            for x_axis in &self.x_axes {
                x_axis.axis.update(cx, |x: &mut AxisRange, cx| {
                    let pivot_x_data = x.min + x.span() * pivot_x_pct;
                    let factor = if delta.x.as_f32() > 0.0 { 1.0 / factor_x } else { factor_x };
                    x.zoom_at(pivot_x_data, pivot_x_pct, factor);
                    cx.notify();
                });
            }
            
            for y_axis in &self.y_axes {
                y_axis.axis.update(cx, |y: &mut AxisRange, cx| {
                    let pivot_y_data = y.min + y.span() * (1.0 - pivot_y_pct);
                    let factor = if delta.y.as_f32() < 0.0 { 1.0 / factor_y } else { factor_y };
                    y.zoom_at(pivot_y_data, 1.0 - pivot_y_pct, factor);
                    cx.notify();
                });
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

    pub fn on_move_series(&mut self, callback: impl Fn(&str, bool, &mut Window, &mut App) + 'static) {
        self.move_callback = Some(Box::new(callback));
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

impl Focusable for ChartPane {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ChartPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible_series: Vec<Series> = self
            .series
            .iter()
            .filter(|s| !self.hidden_series.contains(&s.id))
            .cloned()
            .collect();
        let bounds_rc = self.bounds.clone();
        
        let (mouse_pos_global, hover_x) = {
            let state = self.shared_state.read(cx);
            (state.mouse_pos, state.hover_x)
        };

        // No margins here. The Pane fills its parent.
        let cursor = if self.is_dragging {
            CursorStyle::Crosshair
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
            .child(
                canvas(|_, _, _| {}, {
                    let lc = self.label_color;
                    let sc = self.show_crosshair;
                    let hx = hover_x;
                    let vs = visible_series.clone();
                    let mouse_pos = mouse_pos_global;
                    let x_axes = self.x_axes.clone();
                    let y_axes = self.y_axes.clone();

                    move |bounds, (), window, cx| {
                        *bounds_rc.borrow_mut() = bounds;
                        
                        let mut x_domains = vec![];
                        for x in &x_axes { x_domains.push(x.axis.read(cx).clamped_bounds()); }
                        if x_domains.is_empty() { x_domains.push((0.0, 1.0)); }

                        let mut y_domains = vec![];
                        for y in &y_axes { y_domains.push(y.axis.read(cx).clamped_bounds()); }
                        if y_domains.is_empty() { y_domains.push((0.0, 1.0)); }

                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            // Primary Grid (Using first axis)
                            if let Some(y0) = y_axes.first() {
                                let y0_model = y0.axis.read(cx);
                                let x0_model = x_axes.first().map(|x| x.axis.read(cx).clone()).unwrap_or_default();
                                
                                let x_scale = crate::scales::ChartScale::new_linear(x0_model.clamped_bounds(), (0.0, bounds.size.width.as_f32()));
                                let x_ticks = d3rs::scale::LinearScale::new().domain(x0_model.clamped_bounds().0, x0_model.clamped_bounds().1).range(0.0, bounds.size.width.as_f32() as f64).ticks(10);
                                
                                let y_render_info = crate::rendering::YAxisRenderInfo {
                                    domain: y0_model.clamped_bounds(),
                                    scale: crate::scales::ChartScale::new_linear(y0_model.clamped_bounds(), (bounds.size.height.as_f32(), 0.0)),
                                    ticks: d3rs::scale::LinearScale::new().domain(y0_model.clamped_bounds().0, y0_model.clamped_bounds().1).range(bounds.size.height.as_f32() as f64, 0.0).ticks(10),
                                    limits: (y0_model.min_limit, y0_model.max_limit),
                                    edge: y0.edge,
                                    size: y0.size,
                                    offset: px(0.0),
                                };

                                let x_domain_full = AxisDomain {
                                    x_min: x0_model.clamped_bounds().0,
                                    x_max: x0_model.clamped_bounds().1,
                                    x_min_limit: x0_model.min_limit,
                                    x_max_limit: x0_model.max_limit,
                                    ..Default::default()
                                };

                                crate::rendering::paint_grid(window, bounds, &x_domain_full, &x_scale, &x_ticks, &y_render_info);
                            }

                            crate::rendering::paint_plot(window, bounds, &vs, &x_domains, &y_domains, cx);
                            
                            if sc {
                                // Vertical Crosshair
                                if let Some(hx_val) = hx {
                                    if let (Some(x0), Some(y0)) = (x_axes.first(), y_axes.first()) {
                                        let transform = crate::transform::PlotTransform::new(
                                            crate::scales::ChartScale::new_linear(x0.axis.read(cx).clamped_bounds(), (0.0, bounds.size.width.as_f32())),
                                            crate::scales::ChartScale::new_linear(y0.axis.read(cx).clamped_bounds(), (bounds.size.height.as_f32(), 0.0)),
                                            bounds
                                        );
                                        let sx = transform.x_data_to_screen(hx_val);
                                        let mut builder = PathBuilder::stroke(px(1.0));
                                        builder.move_to(Point::new(sx, bounds.origin.y));
                                        builder.line_to(Point::new(sx, bounds.origin.y + bounds.size.height));
                                        if let Ok(path) = builder.build() { window.paint_path(path, lc.alpha(0.3)); }
                                    }
                                }
                                // Horizontal Crosshair (if mouse in bounds)
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
                })
                .size_full()
            )
            .children(if self.legend_config.enabled {
                // Legend items logic remains but uses simple top/left since no margins
                let mut items = vec![];
                for s in &self.series {
                    let id = s.id.clone();
                    let hidden = self.hidden_series.contains(&id);
                    items.push(
                        div()
                            .flex().items_center().gap_2().group("legend_item")
                            .child(
                                div()
                                    .flex().items_center().gap_1().cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, {
                                        let id = id.clone();
                                        cx.listener(move |this, _, _, cx| this.toggle_series_visibility(&id, cx))
                                    })
                                    .child(div().w_3().h_3().bg(if hidden { gpui::transparent_black() } else { gpui::blue() }).border_1().border_color(gpui::white()))
                                    .child(div().text_size(px(10.0)).text_color(if hidden { self.label_color.alpha(0.4) } else { self.label_color }).child(id.clone()))
                            )
                            .child(
                                div()
                                    .flex().gap_1()
                                    .child(
                                        div()
                                            .size_5()
                                            .flex().items_center().justify_center()
                                            .bg(gpui::white().alpha(0.1))
                                            .rounded_sm()
                                            .text_size(px(10.0))
                                            .text_color(gpui::white())
                                            .hover(|s| s.bg(gpui::blue().alpha(0.4)))
                                            .cursor_pointer()
                                            .on_mouse_down(MouseButton::Left, {
                                                let id = id.clone();
                                                cx.listener(move |this, _, window, cx| {
                                                    if let Some(ref cb) = this.move_callback { cb(&id, true, window, cx); }
                                                })
                                            }).child("▲")
                                    )
                                    .child(
                                        div()
                                            .size_5()
                                            .flex().items_center().justify_center()
                                            .bg(gpui::white().alpha(0.1))
                                            .rounded_sm()
                                            .text_size(px(10.0))
                                            .text_color(gpui::white())
                                            .hover(|s| s.bg(gpui::blue().alpha(0.4)))
                                            .cursor_pointer()
                                            .on_mouse_down(MouseButton::Left, {
                                                let id = id.clone();
                                                cx.listener(move |this, _, window, cx| {
                                                    if let Some(ref cb) = this.move_callback { cb(&id, false, window, cx); }
                                                })
                                            }).child("▼")
                                    )
                            ),
                    );
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