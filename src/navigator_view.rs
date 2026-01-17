// NavigatorView implementation

use crate::data_types::{AxisDomain, AxisRange, Series, SharedPlotState};
use crate::rendering::paint_plot;
use crate::utils::PixelsExt;
use crate::view_controller::ViewController;
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct NavigatorConfig {
    pub lock_x: bool,
    pub lock_y: bool,
    pub clamp_to_minimap: bool,
}

impl Default for NavigatorConfig {
    fn default() -> Self {
        Self {
            lock_x: false,
            lock_y: true,
            clamp_to_minimap: true,
        }
    }
}

/// A miniature view to navigate the data.
pub struct NavigatorView {
    pub x_axis: Entity<AxisRange>,
    pub y_axis: Entity<AxisRange>,
    pub shared_state: Entity<SharedPlotState>,
    pub series: Vec<Series>,
    pub config: NavigatorConfig,
    full_domain: AxisDomain,

    bounds: Rc<RefCell<Bounds<Pixels>>>,
    is_dragging: bool,
}

impl NavigatorView {
    pub fn new(
        x_axis: Entity<AxisRange>,
        y_axis: Entity<AxisRange>,
        shared_state: Entity<SharedPlotState>,
        series: Vec<Series>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&shared_state, |_, _, cx| cx.notify()).detach();

        let mut view = Self {
            x_axis,
            y_axis,
            shared_state,
            series,
            config: NavigatorConfig::default(),
            full_domain: AxisDomain::default(),
            bounds: Rc::new(RefCell::new(Bounds::default())),
            is_dragging: false,
        };
        view.update_full_domain();
        view
    }

    pub fn update_full_domain(&mut self) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for series in &self.series {
            if let Some((sx_min, sx_max, sy_min, sy_max)) = series.plot.read().get_min_max() {
                x_min = x_min.min(sx_min);
                x_max = x_max.max(sx_max);
                y_min = y_min.min(sy_min);
                y_max = y_max.max(sy_max);
            }
        }

        if x_min != f64::INFINITY {
            self.full_domain = AxisDomain {
                x_min,
                x_max,
                y_min,
                y_max,
                ..Default::default()
            };
        }
    }

    fn handle_click(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_dragging = true;
        self.move_to_pos(event.position, cx);
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_dragging {
            self.move_to_pos(event.position, cx);
        }
    }

    fn handle_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_dragging = false;
        cx.notify();
    }

    fn move_to_pos(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        let bounds = *self.bounds.borrow();
        if bounds.is_empty() {
            return;
        }

        let shared_state = self.shared_state.read(cx);
        let gaps = shared_state.gap_index.as_deref();

        let center_x = ViewController::map_pixels_to_value(
            (pos.x - bounds.origin.x).as_f32(),
            bounds.size.width.as_f32(),
            self.full_domain.x_min,
            self.full_domain.x_max,
            false,
            gaps,
        );

        let center_y = ViewController::map_pixels_to_value(
            (pos.y - bounds.origin.y).as_f32(),
            bounds.size.height.as_f32(),
            self.full_domain.y_min,
            self.full_domain.y_max,
            true,
            None,
        );

        let lock_x = self.config.lock_x;
        let lock_y = self.config.lock_y;
        let clamp_to_map = self.config.clamp_to_minimap;
        let full = self.full_domain.clone();
        if !lock_x {
            self.x_axis.update(cx, |x, _cx| {
                // Move selection rectangle to center at current mouse position
                let span = x.span();
                x.min = center_x - span / 2.0;
                x.max = center_x + span / 2.0;
                x.clamp();
            });
        }

        if !lock_y {
            self.y_axis.update(cx, |y, _cx| {
                let limit = if clamp_to_map {
                    Some((full.y_min, full.y_max))
                } else {
                    None
                };
                ViewController::move_to_center(y, center_y, limit);
            });
        }
        cx.notify();
    }
}

impl Render for NavigatorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let full_domain = self.full_domain.clone();
        let x_axis_val = self.x_axis.read(cx).clone();
        let y_axis_val = self.y_axis.read(cx).clone();
        let series = self.series.clone();
        let bounds_rc = self.bounds.clone();
        let lock_y = self.config.lock_y;
        let theme = self.shared_state.read(cx).theme.clone();
        let shared_state_handle = self.shared_state.clone();

        div()
            .size_full()
            .bg(theme.background)
            .border_1()
            .border_color(theme.axis_line)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_click))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .child(
                canvas(
                    |_bounds, _window, _cx| {},
                    move |bounds, (), window, cx| {
                        *bounds_rc.borrow_mut() = bounds;
                        let shared_state = shared_state_handle.read(cx).clone();
                        paint_plot(
                            window,
                            bounds,
                            &series,
                            &[(full_domain.x_min, full_domain.x_max)],
                            &[(full_domain.y_min, full_domain.y_max)],
                            cx,
                            &shared_state,
                        );

                        let (w, h) = (
                            bounds.size.width.as_f32() as f64,
                            bounds.size.height.as_f32() as f64,
                        );

                        let (left_pct, right_pct) = if let Some(g) = &shared_state.gap_index {
                            let l_min_full = g.to_logical(full_domain.x_min as i64) as f64;
                            let l_max_full = g.to_logical(full_domain.x_max as i64) as f64;
                            let l_span_full = l_max_full - l_min_full;

                            let l_min_view = g.to_logical(x_axis_val.min as i64) as f64;
                            let l_max_view = g.to_logical(x_axis_val.max as i64) as f64;

                            (
                                (l_min_view - l_min_full) / l_span_full,
                                (l_max_view - l_min_full) / l_span_full,
                            )
                        } else {
                            (
                                (x_axis_val.min - full_domain.x_min) / full_domain.width(),
                                (x_axis_val.max - full_domain.x_min) / full_domain.width(),
                            )
                        };

                        let rect_left = (w * left_pct).clamp(0.0, w) as f32;
                        let rect_right = (w * right_pct).clamp(0.0, w) as f32;

                        let (rect_top, rect_bot) = if lock_y {
                            (0.0, h as f32)
                        } else {
                            let top_pct =
                                (full_domain.y_max - y_axis_val.max) / full_domain.height();
                            let bot_pct =
                                (full_domain.y_max - y_axis_val.min) / full_domain.height();
                            (
                                (h * top_pct).clamp(0.0, h) as f32,
                                (h * bot_pct).clamp(0.0, h) as f32,
                            )
                        };

                        let rect = Bounds::new(
                            Point::new(
                                bounds.origin.x + px(rect_left),
                                bounds.origin.y + px(rect_top),
                            ),
                            Size {
                                width: px(rect_right - rect_left).max(px(2.0)),
                                height: px(rect_bot - rect_top).max(px(2.0)),
                            },
                        );

                        window.paint_quad(gpui::fill(rect, theme.axis_label.opacity(0.2)));
                        window.paint_quad(gpui::outline(
                            rect,
                            theme.axis_label.opacity(0.5),
                            BorderStyle::Solid,
                        ));
                    },
                )
                .size_full(),
            )
    }
}
