use crate::utils::PixelsExt;
use crate::axis_renderer::AxisRenderer;
use crate::chart::Chart;
use crate::data_types::{AxisEdge, AxisRange, LegendConfig, LegendPosition, Orientation, SharedPlotState};
use crate::gutter_manager::GutterManager;
use crate::theme::ChartTheme;
use crate::Series;
use d3rs::scale::Scale;
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AxisKey {
    X(usize),
    Y(String, usize), // pane_id, axis_idx
}

impl AxisKey {
    pub fn key(&self) -> String {
        match self {
            Self::X(i) => format!("x_{}", i),
            Self::Y(id, a) => format!("y_{}_{}", id, a),
        }
    }
}

pub struct ChartRenderer {
    pub chart: Entity<Chart>,
    pub legend_config: LegendConfig,
    
    // Bounds shared with InputHandler
    pub last_render_axis_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    pub bounds: Rc<RefCell<Bounds<Pixels>>>,
    pub pane_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,

    // Local layout state
    gutter_left: Pixels,
    gutter_right: Pixels,
    gutter_top: Pixels,
    gutter_bottom: Pixels,
}

impl ChartRenderer {
    pub fn new(
        chart: Entity<Chart>,
        last_render_axis_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
        bounds: Rc<RefCell<Bounds<Pixels>>>,
        pane_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    ) -> Self {
        Self {
            chart,
            legend_config: LegendConfig::default(),
            last_render_axis_bounds,
            bounds,
            pane_bounds,
            gutter_left: px(0.0),
            gutter_right: px(0.0),
            gutter_top: px(0.0),
            gutter_bottom: px(0.0),
        }
    }

    fn calculate_gutters(&mut self, x_axes: &[crate::chart::AxisState], panes: &[crate::chart::PaneState]) {
        let g = GutterManager::calculate(panes, x_axes);
        self.gutter_left = g.left;
        self.gutter_right = g.right;
        self.gutter_top = g.top;
        self.gutter_bottom = g.bottom;
    }

    pub fn render_control_button(
        label: &'static str,
        enabled: bool,
        theme: &ChartTheme,
        on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        div()
            .size_7()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_size(px(14.0))
            .bg(theme.axis_label.opacity(0.05))
            .border_1()
            .border_color(theme.axis_label.opacity(0.1))
            .when(enabled, |d| {
                d.text_color(theme.axis_label)
                    .hover(|s| s.bg(theme.accent.opacity(0.4)).border_color(theme.accent))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, on_click)
            })
            .when(!enabled, |d| {
                d.text_color(theme.axis_label.opacity(0.2))
                    .bg(gpui::transparent_black())
            })
            .child(label)
    }

    fn render_legend_button(
        label: &'static str,
        enabled: bool,
        on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        div()
            .size_5()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::white().alpha(0.1))
            .rounded_sm()
            .text_size(px(10.0))
            .text_color(if enabled {
                gpui::white()
            } else {
                gpui::white().alpha(0.2)
            })
            .when(enabled, |d| {
                d.hover(|s| s.bg(gpui::blue().alpha(0.4)))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, on_click)
            })
            .child(label)
    }

    fn render_legend<V: 'static>(
        &self,
        pane_idx: usize,
        ps: &crate::chart::PaneState,
        theme: &ChartTheme,
        pane_count: usize,
        chart_handle: Entity<Chart>,
        cx: &mut Context<V>,
    ) -> Option<impl IntoElement> {
        if !self.legend_config.enabled {
            return None;
        }
        let is_vertical = self.legend_config.orientation == Orientation::Vertical;
        let mut name_col_children = vec![];
        let mut btn_col_children = vec![];
        let mut horiz_items = vec![];

        for s in &ps.series {
            let id = s.id.clone();
            let hidden = ps.hidden_series.contains(&id);

            let on_axis_0 = ps
                .series
                .iter()
                .filter(|other| other.y_axis_id.0 == 0)
                .count();
            let current_y = s.y_axis_id.0;
            let is_isolated = current_y != 0;
            let s_enabled = is_isolated || on_axis_0 > 1;

            let name_el = div()
                .h_5()
                .flex()
                .items_center()
                .gap_1()
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, {
                    let id = id.clone();
                    let chart = chart_handle.clone();
                    cx.listener(move |_, _, _, cx| {
                        chart.update(cx, |c, cx| {
                            if let Some(ps) = c.panes.get_mut(pane_idx) {
                                if ps.hidden_series.contains(&id) {
                                    ps.hidden_series.remove(&id);
                                } else {
                                    ps.hidden_series.insert(id.clone());
                                }
                                cx.notify();
                            }
                        });
                    })
                })
                .child(
                    div()
                        .w_3()
                        .h_3()
                        .bg(if hidden {
                            gpui::transparent_black()
                        } else {
                            theme.accent
                        })
                        .border_1()
                        .border_color(theme.axis_label),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(if hidden {
                            theme.axis_label.opacity(0.4)
                        } else {
                            theme.axis_label
                        })
                        .child(id.clone()),
                );

            let btn_el = div()
                .h_5()
                .flex()
                .items_center()
                .gap_1()
                .child({
                    let id = id.clone();
                    let chart = chart_handle.clone();
                    Self::render_legend_button("▲", pane_idx > 0, move |_, _, cx| {
                        cx.stop_propagation();
                        chart.update(cx, |c, cx| c.move_series(pane_idx, pane_idx - 1, &id, cx));
                    })
                })
                .child({
                    let id = id.clone();
                    let chart = chart_handle.clone();
                    Self::render_legend_button("▼", pane_idx < pane_count - 1, move |_, _, cx| {
                        cx.stop_propagation();
                        chart.update(cx, |c, cx| c.move_series(pane_idx, pane_idx + 1, &id, cx));
                    })
                })
                .child({
                    let id = id.clone();
                    let chart = chart_handle.clone();
                    Self::render_legend_button("S", s_enabled, move |_, _, cx| {
                        cx.stop_propagation();
                        chart.update(cx, |c, cx| c.toggle_series_isolation(pane_idx, &id, cx));
                    })
                })
                .child({
                    let id = id.clone();
                    let chart = chart_handle.clone();
                    Self::render_legend_button("✕", true, move |_, _, cx| {
                        cx.stop_propagation();
                        chart.update(cx, |c, cx| c.remove_series_by_id(id.clone(), cx));
                    })
                });

            if is_vertical {
                name_col_children.push(name_el.into_any_element());
                btn_col_children.push(btn_el.into_any_element());
            } else {
                horiz_items.push(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(name_el)
                        .child(btn_el)
                        .into_any_element(),
                );
            }
        }

        let mut leg = div()
            .absolute()
            .bg(theme.background.opacity(0.8))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(theme.axis_line)
            .flex()
            .gap_3();
        match self.legend_config.position {
            LegendPosition::TopLeft => leg = leg.top(px(10.0)).left(px(10.0)),
            LegendPosition::TopRight => leg = leg.top(px(10.0)).right(px(10.0)),
            LegendPosition::BottomLeft => leg = leg.bottom(px(10.0)).left(px(10.0)),
            LegendPosition::BottomRight => leg = leg.bottom(px(10.0)).right(px(10.0)),
            _ => leg = leg.top(px(10.0)).left(px(10.0)),
        }
        if is_vertical {
            Some(
                leg.child(div().flex_col().gap_1().children(name_col_children))
                    .child(div().flex_col().gap_1().children(btn_col_children)),
            )
        } else {
            Some(leg.flex_row().gap_3().children(horiz_items))
        }
    }

    pub fn render<V: 'static>(&mut self, window: &mut Window, cx: &mut Context<V>) -> Div {
        let start_time = std::time::Instant::now();

        let chart_handle = self.chart.clone();

        let (panes, x_axes, theme, shared_state_handle) = {
            let chart = chart_handle.read(cx);
            (
                chart.panes.clone(),
                chart.x_axes.clone(),
                chart.theme.clone(),
                chart.shared_state.clone(),
            )
        };

        let shared_state = shared_state_handle.read(cx).clone();

        // Debug mode frame request is handled by the View via notify/update generally, 
        // but here we can schedule next frame if needed.
        if shared_state.debug_mode {
            let shared_state_handle = shared_state_handle.clone();
            cx.on_next_frame(window, move |_, _, cx| {
                shared_state_handle.update(cx, |s, _| s.request_render());
                cx.notify();
            });
        }

        let shared_x_axis = chart_handle.read(cx).shared_x_axis.clone();

        self.calculate_gutters(&x_axes, &panes);
        let total_weight: f32 = panes.iter().map(|p| p.weight).sum();
        let container_bounds_rc = self.bounds.clone();
        let pane_bounds_rc = self.pane_bounds.clone();
        let last_render_axis_bounds = self.last_render_axis_bounds.clone();

        let hover_x = shared_state.hover_x;
        let mouse_pos = shared_state.mouse_pos;

        let mut left_y_axis_elements = Vec::new();
        let mut right_y_axis_elements = Vec::new();
        let mut current_top_pct = 0.0;

        for (pane_idx, p) in panes.iter().enumerate() {
            let h_pct = if total_weight > 0.0 {
                p.weight / total_weight
            } else {
                1.0 / panes.len() as f32
            };
            let mut left_cursor = px(0.0);
            let mut right_cursor = px(0.0);
            let pane_id = p.id.clone();
            for (axis_idx, axis) in p.y_axes.iter().enumerate() {
                let axis_entity = axis.entity.clone();
                let is_left = axis.edge == AxisEdge::Left;
                let x_pos = if is_left {
                    let pos = left_cursor;
                    left_cursor += axis.size;
                    pos
                } else {
                    let pos = right_cursor;
                    right_cursor += axis.size;
                    pos
                };
                let key = AxisKey::Y(pane_id.clone(), axis_idx).key();
                let p_id_clone = pane_id.clone();
                let chart = chart_handle.clone();

                // AxisRenderer uses MouseDownEvent listeners. 
                // We need to route these through our event system or let them be.
                // The original code attaches listeners directly to elements.
                // We can keep this logic here as it attaches handlers that modify Chart state directly.
                // Ideally, these would call into `ChartInputHandler`, but for now direct updates are fine for axes.
                
                // IMPORTANT: The `AxisRenderer::render_y_axis` attaches listeners. 
                // We need to keep providing the listeners. 
                // But the `cx` here is `Context<V>`. The original code assumed `Context<ChartView>`.
                // This means we can't easily pass `Self::handle_something`.
                // But we CAN pass closures that capture `chart_handle` and update it.
                
                let el = AxisRenderer::render_y_axis(
                    pane_idx,
                    axis_idx,
                    &axis_entity.read(cx),
                    axis.edge,
                    axis.size,
                    h_pct,
                    current_top_pct,
                    x_pos,
                    axis.label.clone(),
                    axis.format,
                    axis.min_label_spacing,
                    &theme,
                    None,
                    {
                        let key = key.clone();
                        let lab = last_render_axis_bounds.clone();
                        move |bounds| {
                            lab.borrow_mut().insert(key.clone(), bounds);
                        }
                    },
                )
                // We are keeping the event handling logic inline here for axis interactions
                // because separating it completely would require reimplementing AxisRenderer
                // or passing a complex delegate.
                .on_mouse_down(MouseButton::Right, {
                    let p_id = p_id_clone.clone();
                    let chart = chart.clone();
                    cx.listener(move |_, _, _, cx| {
                        cx.stop_propagation();
                        chart.update(cx, |c, cx| {
                            c.flip_axis_edge(Some(p_id.clone()), axis_idx, cx)
                        });
                    })
                })
                .on_mouse_down(MouseButton::Left, {
                    let key = key.clone();
                    let lab = last_render_axis_bounds.clone();
                    let p_id = p_id_clone.clone();
                    let chart = chart.clone();
                    cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        if event.click_count >= 2 {
                            chart.update(cx, |c, cx| {
                                c.dragging_axis = None;
                                if let Some(ps) = c.panes.iter().find(|p| p.id == p_id) {
                                    let x_range = c.shared_x_axis.read(cx);
                                    let x_bounds = (x_range.min, x_range.max);
                                    if let Some(y_axis_state) = ps.y_axes.get(axis_idx) {
                                        let mut sy_min = f64::INFINITY;
                                        let mut sy_max = f64::NEG_INFINITY;
                                        for series in &ps.series {
                                            if ps.hidden_series.contains(&series.id)
                                                || series.y_axis_id.0 != axis_idx
                                            {
                                                continue;
                                            }
                                            if let Some((s_min, s_max)) = series
                                                .plot
                                                .read()
                                                .get_y_range(x_bounds.0, x_bounds.1)
                                            {
                                                sy_min = sy_min.min(s_min);
                                                sy_max = sy_max.max(s_max);
                                            }
                                        }
                                        if sy_min != f64::INFINITY {
                                            y_axis_state.entity.update(cx, |y, _| {
                                                crate::view_controller::ViewController::auto_fit_axis(
                                                    y, sy_min, sy_max, 0.05,
                                                );
                                                y.update_ticks_if_needed(10, None);
                                            });
                                        }
                                    }
                                }
                                c.shared_state
                                    .update(cx, |s: &mut SharedPlotState, _| s.request_render());
                            });
                            return;
                        }
                        if let Some(bounds) = lab.borrow().get(&key) {
                            let pct = ((event.position.y - bounds.origin.y).as_f32()
                                / bounds.size.height.as_f32())
                            .clamp(0.0, 1.0) as f64;
                            chart.update(cx, |c, cx| {
                                c.dragging_axis = Some(crate::chart::AxisDragInfo {
                                    pane_id: Some(p_id.clone()),
                                    axis_idx,
                                    is_y: true,
                                    button: MouseButton::Left,
                                    pivot_pct: 1.0 - pct,
                                });
                                c.last_mouse_pos = Some(event.position);
                                c.shared_state
                                    .update(cx, |s: &mut SharedPlotState, _| s.request_render());
                            });
                        }
                    })
                })
                .on_mouse_down(MouseButton::Middle, {
                    let key = key.clone();
                    let lab = last_render_axis_bounds.clone();
                    let p_id = p_id_clone.clone();
                    let chart = chart.clone();
                    cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        if let Some(bounds) = lab.borrow().get(&key) {
                            let pct = ((event.position.y - bounds.origin.y).as_f32()
                                / bounds.size.height.as_f32())
                            .clamp(0.0, 1.0) as f64;
                            chart.update(cx, |c, cx| {
                                c.dragging_axis = Some(crate::chart::AxisDragInfo {
                                    pane_id: Some(p_id.clone()),
                                    axis_idx,
                                    is_y: true,
                                    button: MouseButton::Middle,
                                    pivot_pct: 1.0 - pct,
                                });
                                c.last_mouse_pos = Some(event.position);
                                c.shared_state
                                    .update(cx, |s: &mut SharedPlotState, _| s.request_render());
                            });
                        }
                    })
                })
                .on_scroll_wheel({
                    let axis_entity = axis_entity.clone();
                    let shared_state = shared_state_handle.clone();
                    move |event, _, cx| {
                        cx.stop_propagation();
                        let dy = match event.delta {
                            ScrollDelta::Pixels(p) => p.y.as_f32(),
                            ScrollDelta::Lines(p) => p.y as f32 * 20.0,
                        };
                        let factor = (1.0f64 - dy as f64 * 0.01).clamp(0.1, 10.0);
                        axis_entity.update(cx, |r, _| {
                            crate::view_controller::ViewController::zoom_axis_at(r, 0.5, factor, None);
                        });
                        shared_state.update(cx, |s: &mut SharedPlotState, _| s.request_render());
                    }
                })
                .into_any_element();
                if is_left {
                    left_y_axis_elements.push(el);
                } else {
                    right_y_axis_elements.push(el);
                }
            }
            current_top_pct += h_pct;
        }

        let mut x_axis_elements = Vec::new();
        let mut top_cursor = px(0.0);
        let mut bot_cursor = px(0.0);
        for (axis_idx, x_axis) in x_axes.iter().enumerate() {
            let axis_entity = x_axis.entity.clone();
            let key = AxisKey::X(axis_idx).key();
            let chart = chart_handle.clone();
            let el = AxisRenderer::render_x_axis(
                axis_idx,
                &axis_entity.read(cx),
                x_axis.edge,
                x_axis.size,
                self.gutter_left,
                self.gutter_right,
                x_axis.label.clone(),
                x_axis.format,
                x_axis.min_label_spacing,
                &theme,
                shared_state.gap_index.clone(),
                {
                    let key = key.clone();
                    let lab = last_render_axis_bounds.clone();
                    move |bounds| {
                        lab.borrow_mut().insert(key.clone(), bounds);
                    }
                },
            )
            .on_mouse_down(MouseButton::Right, {
                let chart = chart.clone();
                cx.listener(move |_, _, _, cx| {
                    cx.stop_propagation();
                    chart.update(cx, |c, cx| c.flip_axis_edge(None, axis_idx, cx));
                })
            })
            .on_mouse_down(MouseButton::Left, {
                let key = key.clone();
                let lab = last_render_axis_bounds.clone();
                let chart = chart.clone();
                cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    if event.click_count >= 2 {
                        chart.update(cx, |c, cx| {
                            c.dragging_axis = None;
                            let mut x_min = f64::INFINITY;
                            let mut x_max = f64::NEG_INFINITY;
                            for ps in &c.panes {
                                for s in &ps.series {
                                    if let Some((sx_min, sx_max, _, _)) =
                                        s.plot.read().get_min_max()
                                    {
                                        x_min = x_min.min(sx_min);
                                        x_max = x_max.max(sx_max);
                                    }
                                }
                            }
                            if x_min != f64::INFINITY {
                                let gaps = c.shared_state.read(cx).gap_index.clone();
                                c.shared_x_axis.update(cx, move |r, _| {
                                    crate::view_controller::ViewController::auto_fit_axis(r, x_min, x_max, 0.05);
                                    r.update_ticks_if_needed(10, gaps.as_deref());
                                });
                            }
                            c.shared_state
                                .update(cx, |s: &mut SharedPlotState, _| s.request_render());
                        });
                        return;
                    }
                    if let Some(bounds) = lab.borrow().get(&key) {
                        let pct = ((event.position.x - bounds.origin.x).as_f32()
                            / bounds.size.width.as_f32())
                        .clamp(0.0, 1.0) as f64;
                        chart.update(cx, |c, cx| {
                            c.dragging_axis = Some(crate::chart::AxisDragInfo {
                                pane_id: None,
                                axis_idx,
                                is_y: false,
                                button: MouseButton::Left,
                                pivot_pct: pct,
                            });
                            c.last_mouse_pos = Some(event.position);
                            c.shared_state
                                .update(cx, |s: &mut SharedPlotState, _| s.request_render());
                        });
                    }
                })
            })
            .on_mouse_down(MouseButton::Middle, {
                let key = key.clone();
                let lab = last_render_axis_bounds.clone();
                let chart = chart.clone();
                cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    if let Some(bounds) = lab.borrow().get(&key) {
                        let pct = ((event.position.x - bounds.origin.x).as_f32()
                            / bounds.size.width.as_f32())
                        .clamp(0.0, 1.0) as f64;
                        chart.update(cx, |c, cx| {
                            c.dragging_axis = Some(crate::chart::AxisDragInfo {
                                pane_id: None,
                                axis_idx,
                                is_y: false,
                                button: MouseButton::Middle,
                                pivot_pct: pct,
                            });
                            c.last_mouse_pos = Some(event.position);
                            c.shared_state
                                .update(cx, |s: &mut SharedPlotState, _| s.request_render());
                        });
                    }
                })
            })
            .on_scroll_wheel({
                let axis_entity = axis_entity.clone();
                let shared_state = shared_state_handle.clone();
                move |event, _, cx| {
                    cx.stop_propagation();
                    let dy = match event.delta {
                        ScrollDelta::Pixels(p) => p.y.as_f32(),
                        ScrollDelta::Lines(p) => p.y as f32 * 20.0,
                    };
                    let factor = (1.0f64 - dy as f64 * 0.01).clamp(0.1, 10.0);
                    let gaps = shared_state.read(cx).gap_index.clone();
                    axis_entity.update(cx, |r, _| {
                        crate::view_controller::ViewController::zoom_axis_at(r, 0.5, factor, gaps.as_deref());
                    });
                    shared_state.update(cx, |s: &mut SharedPlotState, _| s.request_render());
                }
            });
            let axis_div = match x_axis.edge {
                AxisEdge::Top => {
                    let pos = top_cursor;
                    top_cursor += x_axis.size;
                    el.top(pos)
                }
                AxisEdge::Bottom => {
                    let pos = bot_cursor;
                    bot_cursor += x_axis.size;
                    el.bottom(pos)
                }
                _ => el,
            };
            x_axis_elements.push(axis_div.into_any_element());
        }

        let mut tags = Vec::new();
        if let (Some(_pos), Some(hx)) = (mouse_pos, hover_x) {
            let container_origin = self.bounds.borrow().origin;
            for (i, x_a) in x_axes.iter().enumerate() {
                let key = AxisKey::X(i).key();
                if let Some(b) = last_render_axis_bounds.borrow().get(&key) {
                    let r = x_a.entity.read(cx);
                    let scale = crate::scales::ChartScale::new_linear(
                        r.clamped_bounds(),
                        (0.0, b.size.width.as_f32()),
                    );
                    let sx = b.origin.x - container_origin.x + px(scale.map(hx));
                    tags.push(
                        div()
                            .absolute()
                            .top(b.origin.y - container_origin.y - px(1.0))
                            .left(sx)
                            .ml(px(-40.0))
                            .w(px(80.0))
                            .h(x_a.size)
                            .child(crate::rendering::create_axis_tag(
                                scale.format_tick(hx, &x_a.format),
                                px(40.0),
                                true,
                                &theme,
                            ))
                            .into_any_element(),
                    );
                }
            }
            for ps in panes.iter() {
                for (a_idx, y_a) in ps.y_axes.iter().enumerate() {
                    let key = AxisKey::Y(ps.id.clone(), a_idx).key();
                    if let (Some(b), Some(p)) =
                        (last_render_axis_bounds.borrow().get(&key), mouse_pos)
                    {
                        if p.y >= b.origin.y && p.y <= b.origin.y + b.size.height {
                            let r = y_a.entity.read(cx);
                            let scale = crate::scales::ChartScale::new_linear(
                                r.clamped_bounds(),
                                (b.size.height.as_f32(), 0.0),
                            );
                            let val = scale.invert((p.y - b.origin.y).as_f32());
                            tags.push(
                                div()
                                    .absolute()
                                    .top(p.y - container_origin.y - px(10.0))
                                    .left(b.origin.x - container_origin.x)
                                    .w(y_a.size)
                                    .h(px(20.0))
                                    .bg(theme.tag_background)
                                    .text_color(theme.tag_text)
                                    .rounded_sm()
                                    .text_size(px(11.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(scale.format_tick(val, &y_a.format))
                                    .into_any_element(),
                            );
                        }
                    }
                }
            }
        }

        let mut pane_elements = Vec::new();
        let x_axis_entity = shared_x_axis.clone();
        for (i, ps) in panes.iter().enumerate() {
            let h_pct = if total_weight > 0.0 {
                ps.weight / total_weight
            } else {
                1.0 / panes.len() as f32
            };
            let is_last = i == panes.len() - 1;
            let is_first = i == 0;
            let pane_rc = pane_bounds_rc.clone();
            let series = ps.series.clone();
            let hidden = ps.hidden_series.clone();
            let y_axes_entities: Vec<Entity<AxisRange>> =
                ps.y_axes.iter().map(|a| a.entity.clone()).collect();
            let x_axis_entity = x_axis_entity.clone();
            let theme_for_canvas = theme.clone();
            let hx_val = shared_state.hover_x;
            let pane_id_for_canvas = ps.id.clone();
            let pane_id_for_close = ps.id.clone();
            let pane_id_for_debug = ps.id.clone();
            let chart = chart_handle.clone();
            // Pass `cx`
            let legend = self.render_legend(i, ps, &theme, panes.len(), chart.clone(), cx);
            let shared_state_for_canvas = shared_state_handle.clone();

            let mut pane_debug_overlay = None;
            if shared_state.debug_mode {
                let times = shared_state.pane_paint_times.read();
                if let Some(nanos) = times.get(&pane_id_for_debug) {
                    pane_debug_overlay = Some(
                        div()
                            .absolute()
                            .bottom_2()
                            .left_2()
                            .bg(gpui::black().opacity(0.6))
                            .p_1()
                            .rounded_sm()
                            .text_size(px(10.0))
                            .text_color(gpui::green())
                            .child(format!("{:.2?}", std::time::Duration::from_nanos(*nanos))),
                    );
                }
            }

            let shared_state_for_paint = shared_state.clone();
            pane_elements.push(
                div()
                    .h(relative(h_pct))
                    .w_full()
                    .relative()
                    .group("pane_container")
                    .cursor(CursorStyle::Crosshair)
                    .child(
                        canvas(
                            move |_, _, _| {},
                            move |bounds, (), window, cx| {
                                let paint_start = std::time::Instant::now();
                                pane_rc
                                    .borrow_mut()
                                    .insert(pane_id_for_canvas.clone(), bounds);
                                let x_range = x_axis_entity.read(cx).clone();
                                let x_bounds = x_range.clamped_bounds();
                                let mut x_scale = crate::scales::ChartScale::new_linear(
                                    x_bounds,
                                    (0.0, bounds.size.width.as_f32()),
                                );
                                if let Some(gaps) = &shared_state_for_paint.gap_index {
                                    x_scale = x_scale.with_gaps(Some(gaps.clone()));
                                }

                                let y_domains: Vec<(f64, f64)> = y_axes_entities
                                    .iter()
                                    .map(|a| a.read(cx).clamped_bounds())
                                    .collect();
                                let x_domains = vec![x_bounds];
                                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                                    if !y_axes_entities.is_empty() {
                                        let y0 = y_axes_entities[0].read(cx).clone();
                                        let y_scale = crate::scales::ChartScale::new_linear(
                                            y_domains[0],
                                            (bounds.size.height.as_f32(), 0.0),
                                        );

                                        let mut x_axis_range = x_range.clone();
                                        let x_ticks = x_axis_range
                                            .ticks(10, shared_state_for_paint.gap_index.as_deref())
                                            .to_vec();

                                        let y_render_info = crate::rendering::YAxisRenderInfo {
                                            domain: y_domains[0],
                                            scale: y_scale,
                                            ticks: d3rs::scale::LinearScale::new()
                                                .domain(y_domains[0].0, y_domains[0].1)
                                                .range(bounds.size.height.as_f32() as f64, 0.0)
                                                .ticks(10),
                                            limits: (y0.min_limit, y0.max_limit),
                                        };
                                        crate::rendering::paint_grid(
                                            window,
                                            bounds,
                                            &crate::data_types::AxisDomain {
                                                x_min: x_bounds.0,
                                                x_max: x_bounds.1,
                                                ..Default::default()
                                            },
                                            &x_scale,
                                            &x_ticks,
                                            &y_render_info,
                                            &theme_for_canvas,
                                        );
                                    }
                                    let visible_series: Vec<Series> = series
                                        .iter()
                                        .filter(|s| !hidden.contains(&s.id))
                                        .cloned()
                                        .collect();
                                    crate::rendering::paint_plot(
                                        window,
                                        bounds,
                                        &visible_series,
                                        &x_domains,
                                        &y_domains,
                                        cx,
                                        &shared_state_for_paint,
                                    );
                                    if let Some(hx) = hx_val {
                                        let sx = x_scale.map(hx);
                                        let mut builder = PathBuilder::stroke(px(1.0));
                                        builder.move_to(Point::new(
                                            bounds.origin.x + px(sx),
                                            bounds.origin.y,
                                        ));
                                        builder.line_to(Point::new(
                                            bounds.origin.x + px(sx),
                                            bounds.origin.y + bounds.size.height,
                                        ));
                                        if let Ok(path) = builder.build() {
                                            window
                                                .paint_path(path, theme_for_canvas.crosshair_line);
                                        }
                                    }

                                    if let Some(mp) = mouse_pos {
                                        if mp.y >= bounds.origin.y
                                            && mp.y <= bounds.origin.y + bounds.size.height
                                        {
                                            let mut builder = PathBuilder::stroke(px(1.0));
                                            builder.move_to(Point::new(bounds.origin.x, mp.y));
                                            builder.line_to(Point::new(
                                                bounds.origin.x + bounds.size.width,
                                                mp.y,
                                            ));
                                            if let Ok(path) = builder.build() {
                                                window.paint_path(
                                                    path,
                                                    theme_for_canvas.crosshair_line,
                                                );
                                            }
                                        }
                                    }
                                });
                                let paint_elapsed = paint_start.elapsed().as_nanos() as u64;
                                shared_state_for_canvas
                                    .read(cx)
                                    .pane_paint_times
                                    .write()
                                    .insert(pane_id_for_canvas, paint_elapsed);
                            },
                        )
                        .size_full()
                        .absolute(),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_2()
                            .right_2()
                            .flex()
                            .gap_1()
                            .bg(theme.background.opacity(0.4))
                            .rounded_lg()
                            .p_1()
                            .border_1()
                            .border_color(theme.axis_label.opacity(0.05))
                            .group_hover("pane_container", |d| {
                                d.bg(theme.background.opacity(0.8))
                                    .border_color(theme.axis_label.opacity(0.2))
                            })
                            .child(Self::render_control_button("↑", !is_first, &theme, {
                                let chart = chart.clone();
                                move |_, _, cx| {
                                    cx.stop_propagation();
                                    chart.update(cx, |c, cx| c.move_pane_up(i, cx));
                                }
                            }))
                            .child(Self::render_control_button("↓", !is_last, &theme, {
                                let chart = chart.clone();
                                move |_, _, cx| {
                                    cx.stop_propagation();
                                    chart.update(cx, |c, cx| c.move_pane_down(i, cx));
                                }
                            }))
                            .child(Self::render_control_button("+", true, &theme, {
                                let chart = chart.clone();
                                move |_, _, cx| {
                                    cx.stop_propagation();
                                    chart.update(cx, |c, cx| c.add_pane_at(i + 1, 1.0, cx));
                                }
                            }))
                            .child(Self::render_control_button("✕", true, &theme, {
                                let p_id = pane_id_for_close.clone();
                                let chart = chart.clone();
                                move |_, _, cx| {
                                    cx.stop_propagation();
                                    chart.update(cx, |c, cx| c.remove_pane_by_id(p_id.clone(), cx));
                                }
                            })),
                    )
                    .children(legend)
                    .children(pane_debug_overlay)
                    .into_any_element(),
            );
            if !is_last {
                pane_elements.push(
                    div()
                        .h(px(6.0))
                        .w_full()
                        .flex()
                        .items_center()
                        .bg(gpui::transparent_black())
                        .group("splitter")
                        .cursor(CursorStyle::ResizeUpDown)
                        .on_mouse_down(MouseButton::Left, {
                            let chart = chart_handle.clone();
                            cx.listener(move |_, event: &MouseDownEvent, _win, cx| {
                                chart.update(cx, |c, _| {
                                    c.dragging_splitter = Some(i);
                                    c.last_mouse_y = Some(event.position.y);
                                });
                                cx.notify();
                            })
                        })
                        .child(
                            div()
                                .h(px(2.0))
                                .w_full()
                                .bg(theme.axis_label.opacity(0.1))
                                .group_hover("splitter", |d| d.bg(theme.accent.opacity(0.5))),
                        )
                        .into_any_element(),
                );
            }
        }

        let mut box_zoom_element = None;
        if let (Some(start), Some(current)) =
            (shared_state.box_zoom_start, shared_state.box_zoom_current)
        {
            let container_origin = self.bounds.borrow().origin;
            let start_local = start - container_origin;
            let current_local = current - container_origin;

            let x = start_local.x.min(current_local.x);
            let y = start_local.y.min(current_local.y);
            let width = (start_local.x - current_local.x).abs();
            let height = (start_local.y - current_local.y).abs();

            box_zoom_element = Some(
                div()
                    .absolute()
                    .top(y)
                    .left(x)
                    .w(width)
                    .h(height)
                    .bg(theme.accent.opacity(0.1))
                    .border_1()
                    .border_color(theme.accent.opacity(0.5)),
            );
        }

        let mut debug_overlay = None;
        if shared_state.debug_mode {
            let elapsed = start_time.elapsed();
            debug_overlay = Some(
                div()
                    .absolute()
                    .top(px(40.0))
                    .left(px(60.0))
                    .bg(gpui::black().opacity(0.7))
                    .border_1()
                    .border_color(gpui::white().opacity(0.2))
                    .rounded_md()
                    .p_2()
                    .text_size(px(12.0))
                    .text_color(gpui::green())
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(format!("Render Time: {:.2?}", elapsed))
                    .child(format!(
                        "Total Paint: {:.2?}",
                        std::time::Duration::from_nanos(shared_state.total_paint_nanos())
                    ))
                    .child(format!("Render Version: {}", shared_state.render_version))
                    .child(format!("Panes: {}", panes.len()))
                    .child(format!("Hover X: {:?}", shared_state.hover_x)),
            );
        }

        // Return just the content elements, the events will be attached by the main view via InputHandler/ActionHandler
        div()
            .size_full()
            .relative()
            .bg(theme.background)
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, (), _, _| {
                        *container_bounds_rc.borrow_mut() = bounds;
                    },
                )
                .size_full()
                .absolute(),
            )
            .child(
                div()
                    .absolute()
                    .top(self.gutter_top)
                    .left(self.gutter_left)
                    .right(self.gutter_right)
                    .bottom(self.gutter_bottom)
                    .flex()
                    .flex_col()
                    .children(pane_elements),
            )
            .child(
                div()
                    .absolute()
                    .top(self.gutter_top)
                    .bottom(self.gutter_bottom)
                    .left_0()
                    .w(self.gutter_left)
                    .children(left_y_axis_elements),
            )
            .child(
                div()
                    .absolute()
                    .top(self.gutter_top)
                    .bottom(self.gutter_bottom)
                    .right_0()
                    .w(self.gutter_right)
                    .children(right_y_axis_elements),
            )
            .children(x_axis_elements)
            .children(tags)
            .children(box_zoom_element)
            .children(debug_overlay)
    }
}
