// ChartView implementation

use crate::data_types::{AxisDomain, PlotData, Series, Ticks};
use crate::rendering::{paint_axes, paint_grid, paint_plot, paint_crosshair, create_axis_tag};
use gpui::prelude::*;
use gpui::*;
use adabraka_ui::util::PixelsExt;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use tracing::info;
use d3rs::scale::{Scale, LinearScale};

actions!(gpui_plot_element, [Init]);

pub fn init(_cx: &mut impl AppContext) {
    // Initialization code if needed
}

/// La `View` principale qui gère l'état et la logique du graphique.
pub struct ChartView {
    pub domain: AxisDomain,
    pub series: Vec<Series>,
    pub bg_color: Hsla,
    pub label_color: Hsla,
    
    // UI Options
    pub show_crosshair: bool,
    pub show_axis_tags: bool,
    pub show_tooltip: bool,

    drag_start: Option<Point<Pixels>>,
    zoom_drag_start: Option<Point<Pixels>>,
    zoom_drag_last: Option<Point<Pixels>>,
    pub mouse_pos: Option<Point<Pixels>>,
    is_dragging: bool,

    data_changed: bool,
    bounds: Rc<RefCell<Bounds<Pixels>>>,
    dirty_series: HashSet<String>,
}

impl ChartView {
    /// Crée une nouvelle ChartView.
    pub fn new(_cx: &mut Context<Self>) -> Self {
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
            bg_color: gpui::black(),
            label_color: gpui::white(),
            
            show_crosshair: true,
            show_axis_tags: true,
            show_tooltip: true,

            drag_start: None,
            zoom_drag_start: None,
            zoom_drag_last: None,
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
        }
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
            if let Some((sx_min, sx_max, sy_min, sy_max)) = series.plot.borrow().get_min_max() {
                x_min = x_min.min(sx_min);
                x_max = x_max.max(sx_max);
                y_min = y_min.min(sy_min);
                y_max = y_max.max(sy_max);
            }
        }

        if x_min == f64::INFINITY {
            // No data, keep current domain
            self.data_changed = false;
            return;
        }

        let x_data_range = x_max - x_min;
        let y_data_range = y_max - y_min;
        let padding = 0.05; // 5% margin

        // Ensure non-zero range for X
        if x_data_range <= f64::EPSILON {
            let half_range = 30.0; // Default +/- 30s
            self.domain.x_min = x_min - half_range;
            self.domain.x_max = x_max + half_range;
        } else {
            self.domain.x_min = x_min - x_data_range * padding;
            self.domain.x_max = x_max + x_data_range * padding;
        }

        // Ensure non-zero range for Y
        if y_data_range <= f64::EPSILON {
            let half_range = if y_min.abs() > f64::EPSILON {
                y_min.abs() * 0.05
            } else {
                5.0
            }; // +/- 5% or 5.0
            self.domain.y_min = y_min - half_range;
            self.domain.y_max = y_max + half_range;
        } else {
            self.domain.y_min = y_min - y_data_range * padding;
            self.domain.y_max = y_max + y_data_range * padding;
        }

        self.data_changed = false;
    }

    /// Gère l'événement de zoom (molette de la souris).
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

        let delta_y = match event.delta {
            ScrollDelta::Pixels(p) => p.y.as_f32() as f64,
            ScrollDelta::Lines(p) => p.y as f64,
        };
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
        let mouse_y_data = domain.y_min + data_h * (1.0 - mouse_y_pct); // Y inversé

        let new_data_w = data_w * zoom_factor;
        let new_data_h = data_h * zoom_factor;

        domain.x_min = mouse_x_data - new_data_w * mouse_x_pct;
        domain.x_max = domain.x_min + new_data_w;
        domain.y_min = mouse_y_data - new_data_h * (1.0 - mouse_y_pct);
        domain.y_max = domain.y_min + new_data_h;
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
            let delta = event.position - start;
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
                let dx = delta.x.as_f32() as f64 * x_ratio;
                let dy = delta.y.as_f32() as f64 * y_ratio;
                domain.x_min -= dx;
                domain.x_max -= dx;
                domain.y_min += dy;
                domain.y_max += dy;
            }
            self.drag_start = Some(event.position);
        }
        cx.notify();
    }

    /// Gère la fin du glissement.
    fn end_drag(&mut self, _event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.drag_start = None;
        self.is_dragging = false;
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
            
            // Factor: +100px = 2x zoom
            let factor_x = 1.0 + delta_x.abs() / 100.0;
            let factor_y = 1.0 + delta_y.abs() / 100.0;
            
            let domain = &mut self.domain;
            
            // Calculate pivot (mouse start position) in percentage of bounds
            let width_px = bounds.size.width.as_f32() as f64;
            let height_px = bounds.size.height.as_f32() as f64;
            
            let pivot_x_px = start.x.as_f32() as f64 - bounds.origin.x.as_f32() as f64;
            let pivot_y_px = start.y.as_f32() as f64 - bounds.origin.y.as_f32() as f64;
            
            let pivot_x_pct = (pivot_x_px / width_px).clamp(0.0, 1.0);
            let pivot_y_pct = (pivot_y_px / height_px).clamp(0.0, 1.0); // 0 is top
            
            // Calculate pivot in domain coordinates
            let pivot_x_domain = domain.x_min + domain.width() * pivot_x_pct;
            let pivot_y_domain = domain.y_min + domain.height() * (1.0 - pivot_y_pct);

            if delta_x > 0.0 { // Zoom In X
                let new_width = domain.width() / factor_x;
                domain.x_min = pivot_x_domain - new_width * pivot_x_pct;
                domain.x_max = domain.x_min + new_width;
            } else if delta_x < 0.0 { // Zoom Out X
                let new_width = domain.width() * factor_x;
                domain.x_min = pivot_x_domain - new_width * pivot_x_pct;
                domain.x_max = domain.x_min + new_width;
            }
            
            if delta_y < 0.0 { // Drag Up -> Zoom In Y (Natural scrolling)
                let new_height = domain.height() / factor_y;
                domain.y_min = pivot_y_domain - new_height * (1.0 - pivot_y_pct);
                domain.y_max = domain.y_min + new_height;
            } else if delta_y > 0.0 { // Drag Down -> Zoom Out Y
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
}

// Implémentation `Render` pour la `ChartView`
impl Render for ChartView {
    // Signature `render` de l'API v2 [1]
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.dirty_series.is_empty() {
            self.dirty_series.clear();
        }

        let series_clone = self.series.clone();
        let domain_clone = self.domain.clone();
        let bounds_rc = self.bounds.clone();
        let current_bounds = *bounds_rc.borrow();
        let mouse_pos = self.mouse_pos;

        // Calculate ticks for axis labels and grid
        let ticks =
            Self::calculate_ticks(domain_clone.width(), domain_clone.height(), &domain_clone);
        let ticks_clone = ticks.clone();

        // Create scales for label formatting and positioning
        let x_scale = crate::scales::ChartScale::new_linear(
            (domain_clone.x_min, domain_clone.x_max),
            (0.0, current_bounds.size.width.as_f32())
        );
        let y_scale = crate::scales::ChartScale::new_linear(
            (domain_clone.y_min, domain_clone.y_max),
            (current_bounds.size.height.as_f32(), 0.0)
        );

        let transform = crate::transform::PlotTransform::new(x_scale.clone(), y_scale.clone(), current_bounds);

        // Create axis backgrounds and label elements
        let mut axes_elements = paint_axes(&domain_clone, &x_scale, &y_scale, &ticks, self.label_color);

        // Add Crosshair Tags on Axes
        if self.show_axis_tags {
            if let Some(pos) = mouse_pos {
                if current_bounds.contains(&pos) {
                    let data_point = transform.screen_to_data(pos);
                    
                    // X Axis Tag
                    axes_elements.push(create_axis_tag(
                        x_scale.format_tick(data_point.x),
                        pos.x,
                        true,
                        self.bg_color,
                        self.label_color
                    ));

                    // Y Axis Tag
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

        let cursor = if self.is_dragging { CursorStyle::ClosedHand } else { CursorStyle::Crosshair };

        div()
            .size_full()
            .bg(self.bg_color)
            .relative() 
            .cursor(CursorStyle::Arrow) // Default for axis area
            .on_mouse_down(MouseButton::Left, cx.listener(Self::start_drag))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::end_drag))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::start_zoom_drag))
            .on_mouse_move(cx.listener(Self::handle_zoom_drag))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::end_zoom_drag))
            .on_scroll_wheel(cx.listener(Self::handle_zoom))
            .pl(px(50.0))
            .pb(px(20.0))
            .child(
                div()
                    .size_full()
                    .overflow_hidden()
                    .cursor(cursor) // Dynamic cursor only for the plot area
                    .child(
                        canvas(|_bounds, _window, _cx| {}, {
                            let x_scale = x_scale.clone();
                            let y_scale = y_scale.clone();
                            let label_color = self.label_color;
                            let show_crosshair = self.show_crosshair;
                            move |bounds, (), window, cx| {
                                *bounds_rc.borrow_mut() = bounds;
                                paint_grid(window, bounds, &domain_clone, &x_scale, &y_scale, &ticks_clone);
                                paint_plot(window, bounds, &series_clone, &domain_clone, cx);
                                
                                if show_crosshair {
                                    if let Some(pos) = mouse_pos {
                                        paint_crosshair(window, bounds, pos, label_color);
                                    }
                                }
                            }
                        })
                        .size_full(),
                    ),
            )
            .children(axes_elements)
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
