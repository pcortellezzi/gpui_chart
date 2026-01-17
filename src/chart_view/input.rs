use crate::chart::Chart;
use crate::data_types::{AxisRange, SharedPlotState};
use crate::view_controller::ViewController;
use crate::utils::PixelsExt;
use gpui::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use super::renderer::AxisKey;

pub struct ChartInputHandler {
    pub chart: Entity<Chart>,
    pub focus_handle: FocusHandle,

    // Shared bounds
    pub last_render_axis_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    pub bounds: Rc<RefCell<Bounds<Pixels>>>,
    pub pane_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
}

impl ChartInputHandler {
    pub fn new(
        chart: Entity<Chart>,
        focus_handle: FocusHandle,
        last_render_axis_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
        bounds: Rc<RefCell<Bounds<Pixels>>>,
        pane_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    ) -> Self {
        Self {
            chart,
            focus_handle,
            last_render_axis_bounds,
            bounds,
            pane_bounds,
        }
    }

    pub fn handle_mouse_down(
        &self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.focus(&self.focus_handle);
        let p_bounds = self.pane_bounds.borrow().clone();
        self.chart.update(cx, |c, cx| {
            for ps in c.panes.iter_mut() {
                if let Some(bounds) = p_bounds.get(&ps.id) {
                    if bounds.contains(&event.position) {
                        if event.click_count >= 2 {
                            // Auto-fit Y for this pane specifically
                            let x_range = c.shared_x_axis.read(cx);
                            let x_bounds = (x_range.min, x_range.max);
                            for (a_idx, y_axis_state) in ps.y_axes.iter().enumerate() {
                                let mut sy_min = f64::INFINITY;
                                let mut sy_max = f64::NEG_INFINITY;
                                for series in &ps.series {
                                    if series.y_axis_id.0 != a_idx
                                        || ps.hidden_series.contains(&series.id)
                                    {
                                        continue;
                                    }
                                    if let Some((s_min, s_max)) =
                                        series.plot.read().get_y_range(x_bounds.0, x_bounds.1)
                                    {
                                        sy_min = sy_min.min(s_min);
                                        sy_max = sy_max.max(s_max);
                                    }
                                }
                                if sy_min != f64::INFINITY {
                                    y_axis_state.entity.update(cx, |y, _| {
                                        ViewController::auto_fit_axis(y, sy_min, sy_max, 0.05);
                                        y.update_ticks_if_needed(10, None);
                                    });
                                }
                            }
                            cx.notify();
                            return;
                        }

                        match event.button {
                            MouseButton::Left | MouseButton::Middle => {
                                ps.drag_start = Some(event.position);
                                ps.initial_drag_start = Some(event.position);
                                ps.drag_button = Some(event.button);
                                ps.last_drag_time = Some(std::time::Instant::now());
                                ps.velocity = Point::default();
                                c.shared_state.update(cx, |s: &mut SharedPlotState, _| {
                                    s.is_dragging = true;
                                });
                                cx.notify();
                            }
                            MouseButton::Right => {
                                c.shared_state.update(cx, |s: &mut SharedPlotState, _| {
                                    s.box_zoom_start = Some(event.position);
                                    s.box_zoom_current = Some(event.position);
                                });
                                cx.notify();
                            }
                            _ => {}
                        }
                        break;
                    }
                }
            }
        });
    }

    pub fn handle_global_mouse_move(
        &self,
        event: &MouseMoveEvent,
        _win: &mut Window,
        cx: &mut App,
        view_entity_id: EntityId,
    ) {
        let axis_bounds_rc = self.last_render_axis_bounds.clone();
        let pane_bounds_ref = self.pane_bounds.borrow(); 
        let container_bounds = self.bounds.borrow().clone();
        let bh = container_bounds.size.height.as_f32();
        let estimated_height = if bh > 0.0 { bh } else { 600.0 };

        struct PendingSharedState {
            mouse_pos: Option<Option<Point<Pixels>>>,
            hover_x: Option<Option<f64>>,
            active_chart_id: Option<Option<EntityId>>,
            box_zoom_current: Option<Point<Pixels>>,
            is_dragging: Option<bool>,
        }

        impl Default for PendingSharedState {
            fn default() -> Self {
                Self {
                    mouse_pos: None,
                    hover_x: None,
                    active_chart_id: None,
                    box_zoom_current: None,
                    is_dragging: None,
                }
            }
        }

        let mut pending = PendingSharedState::default();
        let mut chart_needs_notify = false;

        self.chart.update(cx, |c, cx| {
            // 1. Safety: check if mouse is completely outside the chart view
            let inside_chart = container_bounds.contains(&event.position);
            
            // 2. Safety: if we are dragging but the button is no longer pressed (released outside), reset drag state.
            let mut any_drag_active = false;
            for ps in c.panes.iter_mut() {
                if let Some(button) = ps.drag_button {
                    if event.pressed_button != Some(button) {
                        ps.drag_start = None;
                        ps.initial_drag_start = None;
                        ps.drag_button = None;
                        ps.velocity = Point::default();
                        chart_needs_notify = true;
                    } else {
                        any_drag_active = true;
                    }
                }
            }
            if let Some(drag_info) = &c.dragging_axis {
                if event.pressed_button != Some(drag_info.button) {
                    c.dragging_axis = None;
                    c.last_mouse_pos = None;
                    chart_needs_notify = true;
                } else {
                    any_drag_active = true;
                }
            }
            if c.dragging_splitter.is_some() && event.pressed_button != Some(MouseButton::Left) {
                c.dragging_splitter = None;
                c.last_mouse_y = None;
                chart_needs_notify = true;
            }

            if !any_drag_active && c.shared_state.read(cx).is_dragging {
                pending.is_dragging = Some(false);
                chart_needs_notify = true;
            }

            if !inside_chart && !any_drag_active {
                if c.shared_state.read(cx).mouse_pos.is_some() {
                    pending.mouse_pos = Some(None);
                    pending.hover_x = Some(None);
                    chart_needs_notify = true;
                }
                return;
            }

            if let Some(index) = c.dragging_splitter {
                        if let Some(last_y) = c.last_mouse_y {
                            let delta = event.position.y - last_y;
                            if delta.abs() > px(0.5) {
                                let mut weights: Vec<f32> = c.panes.iter().map(|p| p.weight).collect();
                                ViewController::resize_panes(
                                    &mut weights,
                                    index,
                                    delta.as_f32(),
                                    estimated_height,
                                );
                                for (i, p) in c.panes.iter_mut().enumerate() {
                                    p.weight = weights[i];
                                }
                                c.last_mouse_y = Some(event.position.y);
                                chart_needs_notify = true;
                            }
                        }
                        return;
                    }
        
                    if let Some(drag_info) = c.dragging_axis.clone() {
                        if let Some(last_pos) = c.last_mouse_pos {
                            let delta = event.position - last_pos;
                            let key = if let Some(id) = &drag_info.pane_id {
                                AxisKey::Y(id.clone(), drag_info.axis_idx).key()
                            } else {
                                AxisKey::X(drag_info.axis_idx).key()
                            };
                            let axis_entity = if let Some(id) = &drag_info.pane_id {
                                c.panes
                                    .iter()
                                    .find(|p| &p.id == id)
                                    .and_then(|p| p.y_axes.get(drag_info.axis_idx))
                                    .map(|a| a.entity.clone())
                            } else {
                                c.x_axes.get(drag_info.axis_idx).map(|a| a.entity.clone())
                            };
        
                            if let (Some(axis), Some(bounds)) =
                                (axis_entity, axis_bounds_rc.borrow().get(&key).cloned())
                            {
                                let gaps = if drag_info.is_y {
                                    None
                                } else {
                                    c.shared_state.read(cx).gap_index.clone()
                                };
                                axis.update(cx, |r: &mut AxisRange, _| {
                                    match drag_info.button {
                                        MouseButton::Left => {
                                            let total_size = if drag_info.is_y {
                                                bounds.size.height
                                            } else {
                                                bounds.size.width
                                            };
                                            ViewController::pan_axis(
                                                r,
                                                (if drag_info.is_y { delta.y } else { delta.x }).as_f32(),
                                                total_size.as_f32(),
                                                drag_info.is_y,
                                                gaps.as_deref(),
                                            );
                                        }
                                        MouseButton::Middle => {
                                            let factor = ViewController::compute_zoom_factor(
                                                (if drag_info.is_y { delta.y } else { -delta.x }).as_f32(),
                                                100.0,
                                            );
                                            ViewController::zoom_axis_at(
                                                r,
                                                drag_info.pivot_pct,
                                                factor,
                                                gaps.as_deref(),
                                            );
                                        }
                                        _ => {}
                                    }
                                    r.update_ticks_if_needed(10, gaps.as_deref());
                                });
                                chart_needs_notify = true;
                            }
                            c.last_mouse_pos = Some(event.position);
                        }
                        return;
                    }
        
                    for ps in c.panes.iter_mut() {
                        if let Some(start) = ps.drag_start {
                            if let Some(bounds) = pane_bounds_ref.get(&ps.id) {
                                let delta = event.position - start;
                                let pw = bounds.size.width.as_f32();
                                let ph = bounds.size.height.as_f32();
                                let gaps = c.shared_state.read(cx).gap_index.clone();
                                match ps.drag_button {
                                    Some(MouseButton::Left) => {
                                        let gaps_x = gaps.clone();
                                        c.shared_x_axis.update(cx, move |x, _| {
                                            ViewController::pan_axis(
                                                x,
                                                delta.x.as_f32(),
                                                pw,
                                                false,
                                                gaps_x.as_deref(),
                                            );
                                        });
                                        for y_axis in &ps.y_axes {
                                            y_axis.entity.update(cx, |y, _| {
                                                ViewController::pan_axis(
                                                    y,
                                                    delta.y.as_f32(),
                                                    ph,
                                                    true,
                                                    None,
                                                );
                                            });
                                        }
                                    }
                                    Some(MouseButton::Middle) => {
                                        let factor_x =
                                            ViewController::compute_zoom_factor(delta.x.as_f32(), 100.0);
                                        let factor_y =
                                            ViewController::compute_zoom_factor(-delta.y.as_f32(), 100.0);
        
                                        let pivot_source = ps.initial_drag_start.unwrap_or(start);
                                        let pivot_x =
                                            (pivot_source.x - bounds.origin.x).as_f32() as f64 / pw as f64;
                                        let pivot_y =
                                            (pivot_source.y - bounds.origin.y).as_f32() as f64 / ph as f64;
        
                                        let gaps_x = gaps.clone();
                                        c.shared_x_axis.update(cx, move |x, _| {
                                            ViewController::zoom_axis_at(
                                                x,
                                                pivot_x,
                                                factor_x,
                                                gaps_x.as_deref(),
                                            );
                                        });
                                        for y_axis in &ps.y_axes {
                                            y_axis.entity.update(cx, |y, _| {
                                                ViewController::zoom_axis_at(
                                                    y,
                                                    1.0 - pivot_y,
                                                    factor_y,
                                                    None,
                                                );
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                                let now = std::time::Instant::now();
                                if let Some(last_time) = ps.last_drag_time {
                                    let dt = now.duration_since(last_time).as_secs_f64();
                                    if dt > 0.001 {
                                        let new_velocity = Point::new(
                                            delta.x.as_f32() as f64 / dt,
                                            delta.y.as_f32() as f64 / dt,
                                        );
                                        ps.velocity = Point::new(
                                            ps.velocity.x * 0.3 + new_velocity.x * 0.7,
                                            ps.velocity.y * 0.3 + new_velocity.y * 0.7,
                                        );
                                    }
                                }
                                ps.drag_start = Some(event.position);
                                ps.last_drag_time = Some(now);
                                chart_needs_notify = true;
                            }
                        }
                    }
        
                    if c.shared_state.read(cx).box_zoom_start.is_some() {
                        pending.box_zoom_current = Some(event.position);
                        chart_needs_notify = true;
                        return;
                    }
        
                    let mut inside_any_pane = false;
                    for ps in &c.panes {
                        if let Some(bounds) = pane_bounds_ref.get(&ps.id) {
                            if bounds.contains(&event.position) {
                                inside_any_pane = true;
                                let x_range = c.shared_x_axis.read(cx);
                                let gaps = c.shared_state.read(cx).gap_index.clone();
                                let hover_x = ViewController::map_pixels_to_value(
                                    (event.position.x - bounds.origin.x).as_f32(),
                                    bounds.size.width.as_f32(),
                                    x_range.min,
                                    x_range.max,
                                    false,
                                    gaps.as_deref(),
                                );
                                
                                let current_state = c.shared_state.read(cx);
                                // Always track position and value so toggle works immediately
                                pending.mouse_pos = Some(Some(event.position));
                                pending.hover_x = Some(Some(hover_x));
                                pending.active_chart_id = Some(Some(view_entity_id));
                                
                                if current_state.crosshair_enabled {
                                    chart_needs_notify = true;
                                }
                                break;
                            }
                        }
                    }
        
                    if !inside_any_pane {
                        let state = c.shared_state.read(cx);
                        // Hide crosshair when outside panes, unless dragging
                        if !state.is_dragging {
                             pending.mouse_pos = Some(None);
                             pending.hover_x = Some(None);
                             if state.mouse_pos.is_some() {
                                 chart_needs_notify = true;
                             }
                        }
                    }
                    
                    if chart_needs_notify {
                        cx.notify();
                    }
                });

                if pending.mouse_pos.is_some() || pending.hover_x.is_some() || pending.active_chart_id.is_some() || pending.box_zoom_current.is_some() || pending.is_dragging.is_some() {
                    let shared_state = self.chart.read(cx).shared_state.clone();
                    shared_state.update(cx, |s, _| {
                        if let Some(mp) = pending.mouse_pos { s.mouse_pos = mp; }
                        if let Some(hx) = pending.hover_x { s.hover_x = hx; }
                        if let Some(id) = pending.active_chart_id { s.active_chart_id = id; }
                        if let Some(bz) = pending.box_zoom_current { s.box_zoom_current = Some(bz); }
                        if let Some(d) = pending.is_dragging { s.is_dragging = d; }
                    });
                }
            }



                        pub fn handle_global_mouse_up(
        &self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut App,
        view_entity_id: EntityId,
    ) {
        let p_bounds = self.pane_bounds.borrow().clone();
        let mut needs_inertia = false;

        self.chart.update(cx, |c, cx| {
            if event.button == MouseButton::Right {
                let state = c.shared_state.read(cx);
                if let (Some(start), Some(end)) = (state.box_zoom_start, state.box_zoom_current) {
                    let dx = (end.x - start.x).abs();
                    let dy = (end.y - start.y).abs();
                    if dx > px(5.0) && dy > px(5.0) {
                        for ps in c.panes.iter() {
                            if let Some(bounds) = p_bounds.get(&ps.id) {
                                if bounds.contains(&start) {
                                    let x_range = c.shared_x_axis.read(cx);
                                    let x_scale = crate::scales::ChartScale::new_linear(
                                        x_range.clamped_bounds(),
                                        (0.0, bounds.size.width.as_f32()),
                                    );
                                    let px1 = x_scale.invert((start.x - bounds.origin.x).as_f32());
                                    let px2 = x_scale.invert((end.x - bounds.origin.x).as_f32());
                                    if (px1 - px2).abs() > f64::EPSILON {
                                        c.shared_x_axis.update(cx, |x, _| {
                                            ViewController::auto_fit_axis(
                                                x,
                                                px1.min(px2),
                                                px1.max(px2),
                                                0.0,
                                            );
                                        });
                                    }
                                    if let Some(y_axis) = ps.y_axes.first() {
                                        let y_range = y_axis.entity.read(cx);
                                        let y_scale = crate::scales::ChartScale::new_linear(
                                            y_range.clamped_bounds(),
                                            (bounds.size.height.as_f32(), 0.0),
                                        );
                                        let py1 =
                                            y_scale.invert((start.y - bounds.origin.y).as_f32());
                                        let py2 =
                                            y_scale.invert((end.y - bounds.origin.y).as_f32());
                                        if (py1 - py2).abs() > f64::EPSILON {
                                            y_axis.entity.update(cx, |y, _| {
                                                ViewController::auto_fit_axis(
                                                    y,
                                                    py1.min(py2),
                                                    py1.max(py2),
                                                    0.0,
                                                );
                                            });
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
                c.shared_state.update(cx, |s: &mut SharedPlotState, _| {
                    s.box_zoom_start = None;
                    s.box_zoom_current = None;
                });
                cx.notify();
            }

            let now = std::time::Instant::now();
            for ps in c.panes.iter_mut() {
                if ps.drag_start.is_some() {
                    ps.drag_start = None;
                    ps.initial_drag_start = None;
                    if let Some(last_time) = ps.last_drag_time {
                        if now.duration_since(last_time).as_millis() > 50 {
                            ps.velocity = Point::default();
                        }
                    }
                    if ps.velocity.x.abs() > 1.0 || ps.velocity.y.abs() > 1.0 {
                        needs_inertia = true;
                    }
                }
            }
            c.dragging_splitter = None;
            c.dragging_axis = None;
            c.last_mouse_pos = None;
            c.last_mouse_y = None;
            c.shared_state.update(cx, |s: &mut SharedPlotState, _| {
                s.is_dragging = false;
            });
            cx.notify();
        });

        if needs_inertia {
            self.apply_inertia(window, cx, view_entity_id);
        }
    }

    pub fn apply_inertia(&self, window: &mut Window, cx: &mut App, view_entity_id: EntityId) {
        let mut active = false;
        let dt = 1.0 / 60.0;
        let p_bounds = self.pane_bounds.borrow().clone();
        self.chart.update(cx, |c, cx| {
            for ps in c.panes.iter_mut() {
                if ps.drag_start.is_some()
                    || (ps.velocity.x.abs() < 0.01 && ps.velocity.y.abs() < 0.01)
                {
                    continue;
                }
                active = true;
                ViewController::apply_friction(&mut ps.velocity.x, 0.95, dt);
                ViewController::apply_friction(&mut ps.velocity.y, 0.95, dt);
                if let Some(bounds) = p_bounds.get(&ps.id) {
                    let pw = bounds.size.width.as_f32() as f64;
                    let ph = bounds.size.height.as_f32() as f64;
                    c.shared_x_axis.update(cx, |x, _| {
                        x.pan(-ps.velocity.x * dt * (x.span() / pw));
                        x.clamp();
                    });
                    for y_axis in &ps.y_axes {
                        y_axis.entity.update(cx, |y, _| {
                            y.pan(ps.velocity.y * dt * (y.span() / ph));
                            y.clamp();
                        });
                    }
                }
            }
            if active {
                cx.notify();
            }
        });
        if active {
            let this = self.clone();
            window.on_next_frame(move |window, cx| this.apply_inertia(window, cx, view_entity_id));
        }
    }

    pub fn handle_scroll_wheel(
        &self,
        event: &ScrollWheelEvent,
        _win: &mut Window,
        cx: &mut App,
    ) {
        let p_bounds = self.pane_bounds.borrow().clone();
        self.chart.update(cx, |c, cx| {
            for ps in &c.panes {
                if let Some(bounds) = p_bounds.get(&ps.id) {
                    if bounds.contains(&event.position) {
                        let is_zoom = event.modifiers.control || event.modifiers.platform;
                        let delta_y = match event.delta {
                            ScrollDelta::Pixels(p) => p.y.as_f32(),
                            ScrollDelta::Lines(p) => p.y as f32 * 20.0,
                        };
                        let gaps = c.shared_state.read(cx).gap_index.clone();
                        if is_zoom {
                            let factor = (1.0f64 - delta_y as f64 * 0.01).clamp(0.1, 10.0);
                            let mx_pct = (event.position.x - bounds.origin.x).as_f32() as f64
                                / bounds.size.width.as_f32() as f64;
                            let gaps_x = gaps.clone();
                            c.shared_x_axis.update(cx, move |x, _| {
                                ViewController::zoom_axis_at(x, mx_pct, factor, gaps_x.as_deref())
                            });
                            let my_pct = (event.position.y - bounds.origin.y).as_f32() as f64
                                / bounds.size.height.as_f32() as f64;
                            for y_axis in &ps.y_axes {
                                y_axis.entity.update(cx, |y, _| {
                                    ViewController::zoom_axis_at(y, 1.0 - my_pct, factor, None)
                                });
                            }
                        } else {
                            let delta_x = match event.delta {
                                ScrollDelta::Pixels(p) => p.x.as_f32(),
                                ScrollDelta::Lines(p) => p.x as f32 * 20.0,
                            };
                            let gaps_x = gaps.clone();
                            c.shared_x_axis.update(cx, move |x, _| {
                                ViewController::pan_axis(
                                    x,
                                    delta_x,
                                    bounds.size.width.as_f32(),
                                    false,
                                    gaps_x.as_deref(),
                                )
                            });
                            for y_axis in &ps.y_axes {
                                y_axis.entity.update(cx, |y, _| {
                                    ViewController::pan_axis(
                                        y,
                                        delta_y,
                                        bounds.size.height.as_f32(),
                                        true,
                                        None,
                                    )
                                });
                            }
                        }
                        break;
                    }
                }
            }
            cx.notify();
        });
    }
}

// Implement Clone manually or derive it
impl Clone for ChartInputHandler {
    fn clone(&self) -> Self {
        Self {
            chart: self.chart.clone(),
            focus_handle: self.focus_handle.clone(),
            last_render_axis_bounds: self.last_render_axis_bounds.clone(),
            bounds: self.bounds.clone(),
            pane_bounds: self.pane_bounds.clone(),
        }
    }
}
