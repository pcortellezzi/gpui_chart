use crate::chart_pane::ChartPane;
use crate::data_types::{AxisRange, SharedPlotState, AxisEdge, AxisId};
use crate::theme::ChartTheme;
use crate::gutter_manager::GutterManager;
use crate::axis_renderer::AxisRenderer;
use crate::view_controller::ViewController;
use adabraka_ui::util::PixelsExt;
use gpui::prelude::*;
use gpui::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

/// Configuration pour un axe visuel dans le conteneur.
pub struct AxisConfig {
    pub entity: Entity<AxisRange>,
    pub edge: AxisEdge,
    pub size: Pixels,
    pub label: String,
}

/// Configuration d'une Pane au sein du conteneur.
pub struct PaneConfig {
    pub pane: Entity<ChartPane>,
    pub weight: f32,
    pub y_axes: Vec<AxisConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum AxisKey {
    X(usize),
    Y(usize, usize), // pane_idx, axis_idx
}

impl AxisKey {
    fn to_string(&self) -> String {
        match self {
            Self::X(i) => format!("x_{}", i),
            Self::Y(p, a) => format!("y_{}_{}", p, a),
        }
    }
}

#[derive(Clone, Debug)]
struct AxisDragInfo {
    pane_idx: Option<usize>,
    axis_idx: usize,
    is_y: bool,
    button: MouseButton,
    pivot_pct: f64,
}

pub struct ChartContainer {
    pub shared_x_axis: Entity<AxisRange>,
    pub shared_state: Entity<SharedPlotState>,
    pub panes: Vec<PaneConfig>,
    pub x_axes: Vec<AxisConfig>,
    pub theme: ChartTheme,

    gutter_left: Pixels,
    gutter_right: Pixels,
    gutter_top: Pixels,
    gutter_bottom: Pixels,

    dragging_splitter: Option<usize>,
    dragging_axis: Option<AxisDragInfo>,
    last_mouse_pos: Option<Point<Pixels>>,
    last_mouse_y: Option<Pixels>,

    last_render_axis_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    bounds: Rc<RefCell<Bounds<Pixels>>>,

    focus_handle: FocusHandle,
}

impl ChartContainer {
    pub fn new(shared_x_axis: Entity<AxisRange>, shared_state: Entity<SharedPlotState>, cx: &mut Context<Self>) -> Self {
        cx.observe(&shared_x_axis, |_, _, cx| cx.notify()).detach();
        cx.observe(&shared_state, |_, _, cx| cx.notify()).detach();

        Self {
            shared_x_axis,
            shared_state,
            panes: vec![],
            x_axes: vec![],
            theme: ChartTheme::default(),
            gutter_left: px(0.0),
            gutter_right: px(0.0),
            gutter_top: px(0.0),
            gutter_bottom: px(0.0),
            dragging_splitter: None,
            dragging_axis: None,
            last_mouse_pos: None,
            last_mouse_y: None,
            last_render_axis_bounds: Rc::new(RefCell::new(HashMap::new())),
            bounds: Rc::new(RefCell::new(Bounds::default())),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn with_theme(mut self, theme: ChartTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Bascule un axe Y de gauche à droite ou un axe X de haut en bas.
    pub fn flip_axis_edge(&mut self, pane_idx: Option<usize>, axis_idx: usize, cx: &mut Context<Self>) {
        if let Some(p_idx) = pane_idx {
            if let Some(pc) = self.panes.get_mut(p_idx) {
                if let Some(axis) = pc.y_axes.get_mut(axis_idx) {
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
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    pub fn move_series(&mut self, from_idx: usize, to_idx: usize, series_id: &str, cx: &mut Context<Self>) {
        if from_idx == to_idx { return; }
        let mut series_to_move = None;
        if let Some(from_pane) = self.panes.get(from_idx) {
            from_pane.pane.update(cx, |p, _cx| {
                if let Some(pos) = p.series.iter().position(|s| s.id == series_id) {
                    series_to_move = Some(p.series.remove(pos));
                }
            });
        }
        if let (Some(s), Some(to_pane)) = (series_to_move, self.panes.get(to_idx)) {
            let axis_entity = to_pane.y_axes.first().map(|a| a.entity.clone());
            to_pane.pane.update(cx, |p, cx| {
                if let Some(ae) = axis_entity {
                    if p.y_axes.is_empty() { p.add_y_axis(ae, cx); }
                }
                p.add_series(s);
            });
        }
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    pub fn remove_pane(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.panes.len() {
            self.panes.remove(idx);
            self.shared_state.update(cx, |s, _| s.request_render());
        }
    }

    pub fn move_pane_up(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx > 0 && idx < self.panes.len() {
            self.panes.swap(idx, idx - 1);
            self.shared_state.update(cx, |s, _| s.request_render());
        }
    }

    pub fn move_pane_down(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.panes.len() - 1 {
            self.panes.swap(idx, idx + 1);
            self.shared_state.update(cx, |s, _| s.request_render());
        }
    }

    pub fn add_pane_at(&mut self, idx: usize, weight: f32, cx: &mut Context<Self>) {
        let shared_state = self.shared_state.clone();
        let shared_x = self.shared_x_axis.clone();
        let new_pane = cx.new(|cx| {
            let mut p = ChartPane::new(shared_state, cx);
            p.add_x_axis(shared_x, cx);
            p
        });
        let default_y = cx.new(|_cx| AxisRange::new(0.0, 100.0));
        self.register_pane_callbacks(&new_pane, cx);
        let config = PaneConfig {
            pane: new_pane,
            weight,
            y_axes: vec![AxisConfig { entity: default_y, edge: AxisEdge::Right, size: px(60.0), label: "New Axis".to_string() }],
        };
        if idx >= self.panes.len() { self.panes.push(config); } else { self.panes.insert(idx, config); }
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    fn register_pane_callbacks(&self, pane: &Entity<ChartPane>, cx: &mut Context<Self>) {
        let container_handle = cx.entity().clone();
        let pane_id = pane.entity_id();
        pane.update(cx, |p, _cx| {
            let container_handle_move = container_handle.clone();
            p.on_move_series(move |series_id: &str, move_up: bool, _window: &mut Window, cx: &mut Context<ChartPane>| {
                let series_id = series_id.to_string();
                let container_handle = container_handle_move.clone();
                cx.defer(move |cx| {
                    cx.update_entity(&container_handle, |c: &mut ChartContainer, cx| {
                        if let Some(current_idx) = c.panes.iter().position(|pc| pc.pane.entity_id() == pane_id) {
                            let target_idx = if move_up { if current_idx > 0 { current_idx - 1 } else { current_idx } }
                                            else { if current_idx < c.panes.len() - 1 { current_idx + 1 } else { current_idx } };
                            c.move_series(current_idx, target_idx, &series_id, cx);
                        }
                    });
                });
            });

            let container_handle_isolate = container_handle.clone();
            p.on_isolate_series(move |series_id: &str, _window: &mut Window, cx: &mut Context<ChartPane>| {
                let series_id = series_id.to_string();
                let container_handle = container_handle_isolate.clone();
                cx.defer(move |cx| {
                    cx.update_entity(&container_handle, |c: &mut ChartContainer, cx| {
                        if let Some(current_idx) = c.panes.iter().position(|pc| pc.pane.entity_id() == pane_id) {
                            c.isolate_series(current_idx, &series_id, cx);
                        }
                    });
                });
            });
        });
    }

    /// Bascule une série entre l'axe principal et un axe dédié (Isolation).
    pub fn toggle_series_isolation(&mut self, pane_idx: usize, series_id: &str, cx: &mut Context<Self>) {
        if let Some(pc) = self.panes.get_mut(pane_idx) {
            let mut current_axis_idx = 0;
            pc.pane.update(cx, |p, _| {
                if let Some(s) = p.series.iter().find(|s| s.id == series_id) {
                    current_axis_idx = s.y_axis_id.0;
                }
            });

            if current_axis_idx == 0 {
                // --- ISOLER ---
                let mut s_min = 0.0; let mut s_max = 100.0;
                pc.pane.update(cx, |p, _cx| {
                    if let Some(s) = p.series.iter().find(|s| s.id == series_id) {
                        if let Some((_, _, ymin, ymax)) = s.plot.read().get_min_max() {
                            s_min = ymin; s_max = ymax;
                        }
                    }
                });

                let new_axis = cx.new(|_| AxisRange::new(s_min, s_max));
                cx.observe(&new_axis, |_, _, cx| cx.notify()).detach();

                pc.y_axes.push(AxisConfig {
                    entity: new_axis.clone(),
                    edge: AxisEdge::Right,
                    size: px(60.0),
                    label: series_id.to_string(),
                });

                let new_axis_id = AxisId(pc.y_axes.len() - 1);
                pc.pane.update(cx, |p, cx| {
                    p.add_y_axis(new_axis, cx);
                    if let Some(s) = p.series.iter_mut().find(|s| s.id == series_id) {
                        s.y_axis_id = new_axis_id;
                    }
                    cx.notify();
                });
            } else {
                // --- RÉINTÉGRER (vers l'axe 0) ---
                pc.pane.update(cx, |p, _| {
                    if let Some(s) = p.series.iter_mut().find(|s| s.id == series_id) {
                        s.y_axis_id = AxisId(0);
                    }
                });

                // Nettoyage des axes qui ne sont plus utilisés par AUCUNE série
                let mut axes_to_remove = Vec::new();
                for i in 1..pc.y_axes.len() {
                    let mut in_use = false;
                    pc.pane.read(cx).series.iter().for_each(|s| {
                        if s.y_axis_id.0 == i { in_use = true; }
                    });
                    if !in_use { axes_to_remove.push(i); }
                }

                // Supprimer de l'index le plus haut au plus bas pour ne pas fausser les décalages
                axes_to_remove.sort_by(|a, b| b.cmp(a));
                for idx in axes_to_remove {
                    pc.y_axes.remove(idx);
                    pc.pane.update(cx, |p, _| {
                        p.y_axes.remove(idx);
                        // Décalage des IDs pour les séries pointant vers des axes supérieurs
                        for s in p.series.iter_mut() {
                            if s.y_axis_id.0 > idx { s.y_axis_id.0 -= 1; }
                        }
                    });
                }
            }
        }
        cx.notify();
    }

    pub fn isolate_series(&mut self, pane_idx: usize, series_id: &str, cx: &mut Context<Self>) {
        self.toggle_series_isolation(pane_idx, series_id, cx);
    }

    pub fn add_pane(&mut self, pane: Entity<ChartPane>, weight: f32, cx: &mut Context<Self>) {
        self.register_pane_callbacks(&pane, cx);
        self.panes.push(PaneConfig { pane, weight, y_axes: vec![] });
        cx.notify();
    }

    pub fn add_y_axis(&mut self, pane_idx: usize, axis: Entity<AxisRange>, edge: AxisEdge, size: Pixels, label: String, _cx: &mut Context<Self>) {
        if let Some(pane_config) = self.panes.get_mut(pane_idx) {
            pane_config.y_axes.push(AxisConfig { entity: axis, edge, size, label });
        }
    }

    pub fn set_y_axis_edge(&mut self, pane_idx: usize, axis_idx: usize, edge: AxisEdge, cx: &mut Context<Self>) {
        if let Some(pane) = self.panes.get_mut(pane_idx) {
            if let Some(axis) = pane.y_axes.get_mut(axis_idx) { axis.edge = edge; cx.notify(); }
        }
    }

    pub fn auto_fit_axis(&mut self, pane_idx: Option<usize>, axis_idx: usize, cx: &mut Context<Self>) {
        if let Some(p_idx) = pane_idx {
            if let Some(pane_config) = self.panes.get(p_idx) { pane_config.pane.update(cx, |p, cx| p.auto_fit_y(Some(axis_idx), cx)); }
        } else {
            let mut x_min = f64::INFINITY; let mut x_max = f64::NEG_INFINITY;
            for pc in &self.panes {
                pc.pane.read(cx).series.iter().for_each(|s| {
                    if let Some((sx_min, sx_max, _, _)) = s.plot.read().get_min_max() { x_min = x_min.min(sx_min); x_max = x_max.max(sx_max); }
                });
            }
            if x_min != f64::INFINITY {
                self.shared_x_axis.update(cx, |r, cx| {
                    let span = x_max - x_min;
                    r.min = x_min - span * 0.05; r.max = x_max + span * 0.05;
                    r.clamp(); cx.notify();
                });
            }
        }
    }

    pub fn add_x_axis(&mut self, axis: Entity<AxisRange>, edge: AxisEdge, size: Pixels, label: String, _cx: &mut Context<Self>) {
        self.x_axes.push(AxisConfig { entity: axis, edge, size, label });
    }

    fn handle_pan_left(&mut self, _: &crate::chart_pane::PanLeft, _win: &mut Window, cx: &mut Context<Self>) {
        self.shared_x_axis.update(cx, |r, _cx| { ViewController::pan_axis(r, -20.0, 200.0, false); r.update_ticks_if_needed(10); });
        self.shared_state.update(cx, |s, _| s.request_render());
    }
    fn handle_pan_right(&mut self, _: &crate::chart_pane::PanRight, _win: &mut Window, cx: &mut Context<Self>) {
        self.shared_x_axis.update(cx, |r, _cx| { ViewController::pan_axis(r, 20.0, 200.0, false); r.update_ticks_if_needed(10); });
        self.shared_state.update(cx, |s, _| s.request_render());
    }
    fn handle_zoom_in(&mut self, _: &crate::chart_pane::ZoomIn, _win: &mut Window, cx: &mut Context<Self>) {
        self.shared_x_axis.update(cx, |r, _cx| { ViewController::zoom_axis_at(r, 0.5, 0.9); r.update_ticks_if_needed(10); });
        self.shared_state.update(cx, |s, _| s.request_render());
    }
    fn handle_zoom_out(&mut self, _: &crate::chart_pane::ZoomOut, _win: &mut Window, cx: &mut Context<Self>) {
        self.shared_x_axis.update(cx, |r, _cx| { ViewController::zoom_axis_at(r, 0.5, 1.1); r.update_ticks_if_needed(10); });
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    fn resize_panes(&mut self, index: usize, delta: Pixels, cx: &mut Context<Self>) {
        if index + 1 >= self.panes.len() { return; }
        let bh = self.bounds.borrow().size.height.as_f32();
        let estimated_height = if bh > 0.0 { bh } else { 600.0 };

        let mut weights: Vec<f32> = self.panes.iter().map(|p| p.weight).collect();
        ViewController::resize_panes(&mut weights, index, delta.as_f32(), estimated_height);

        for (i, p) in self.panes.iter_mut().enumerate() {
            p.weight = weights[i];
        }
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    fn handle_global_mouse_move(&mut self, event: &MouseMoveEvent, _win: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.dragging_splitter {
            if let Some(last_y) = self.last_mouse_y {
                let delta = event.position.y - last_y;
                if delta.abs() > px(0.5) { self.resize_panes(index, delta, cx); self.last_mouse_y = Some(event.position.y); }
            }
        } else if let Some(drag_info) = self.dragging_axis.clone() {
            if let Some(last_pos) = self.last_mouse_pos {
                let delta = event.position - last_pos;
                self.handle_axis_drag(&drag_info, delta, cx);
                self.last_mouse_pos = Some(event.position);
            }
        }
    }

    fn handle_axis_drag(&self, info: &AxisDragInfo, delta: Point<Pixels>, cx: &mut Context<Self>) {
        let key = if let Some(p_idx) = info.pane_idx { AxisKey::Y(p_idx, info.axis_idx).to_string() }
                  else { AxisKey::X(info.axis_idx).to_string() };
        let axis_entity = if let Some(p_idx) = info.pane_idx {
            self.panes.get(p_idx).and_then(|p| p.y_axes.get(info.axis_idx)).map(|a| a.entity.clone())
        } else { self.x_axes.get(info.axis_idx).map(|a| a.entity.clone()) };

        if let (Some(axis), Some(bounds)) = (axis_entity, self.last_render_axis_bounds.borrow().get(&key)) {
            axis.update(cx, |r: &mut AxisRange, _cx| {
                match info.button {
                    MouseButton::Left => {
                        let total_size = if info.is_y { bounds.size.height } else { bounds.size.width };
                        let delta_val = if info.is_y { delta.y } else { delta.x };
                        ViewController::pan_axis(r, delta_val.as_f32(), total_size.as_f32(), info.is_y);
                    }
                    MouseButton::Middle => {
                        let dy = if info.is_y { -delta.y.as_f32() } else { delta.x.as_f32() };
                        let factor = ViewController::compute_zoom_factor(dy, 100.0);
                        ViewController::zoom_axis_at(r, info.pivot_pct, factor);
                    }
                    _ => {}
                }
                r.update_ticks_if_needed(10);
            });
            self.shared_state.update(cx, |s, _| s.request_render());
        }
    }

    fn handle_global_mouse_up(&mut self, _: &MouseUpEvent, _win: &mut Window, cx: &mut Context<Self>) {
        self.dragging_splitter = None; self.dragging_axis = None; self.last_mouse_pos = None; self.last_mouse_y = None;
        self.shared_state.update(cx, |s, _| s.request_render());
    }

    fn calculate_gutters(&mut self) {
        let g = GutterManager::calculate(&self.panes, &self.x_axes);
        self.gutter_left = g.left;
        self.gutter_right = g.right;
        self.gutter_top = g.top;
        self.gutter_bottom = g.bottom;
    }

    fn render_control_button(&self, label: &'static str, enabled: bool, on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static) -> impl IntoElement {
        div().size_7().flex().items_center().justify_center().rounded_md().text_size(px(14.0)).bg(gpui::white().alpha(0.05)).border_1().border_color(gpui::white().alpha(0.1))
            .when(enabled, |d| { d.text_color(gpui::white()).hover(|s| s.bg(gpui::blue().alpha(0.4)).border_color(gpui::blue())).cursor_pointer().on_mouse_down(MouseButton::Left, on_click) })
            .when(!enabled, |d| { d.text_color(gpui::white().alpha(0.2)).bg(gpui::transparent_black()) })
            .child(label)
    }
}

impl Focusable for ChartContainer {
    fn focus_handle(&self, _cx: &App) -> FocusHandle { self.focus_handle.clone() }
}

impl Render for ChartContainer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.calculate_gutters();
        let total_weight: f32 = self.panes.iter().map(|p| p.weight).sum();
        let container_bounds_rc = self.bounds.clone();
        let last_render_axis_bounds = self.last_render_axis_bounds.clone();

        let mut left_y_axis_elements = Vec::new();
        let mut right_y_axis_elements = Vec::new();
        let mut current_top_pct = 0.0;
        let theme = self.theme.clone();

        for (pane_idx, p) in self.panes.iter().enumerate() {
            let h_pct = if total_weight > 0.0 { p.weight / total_weight } else { 1.0 / self.panes.len() as f32 };
            let mut left_cursor = px(0.0); let mut right_cursor = px(0.0);
            for (axis_idx, axis) in p.y_axes.iter().enumerate() {
                let axis_entity = axis.entity.clone();
                let is_left = axis.edge == AxisEdge::Left;
                let x_pos = if is_left { let pos = left_cursor; left_cursor += axis.size; pos } else { let pos = right_cursor; right_cursor += axis.size; pos };
                let key = AxisKey::Y(pane_idx, axis_idx).to_string();

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
                    &theme,
                )
                .on_mouse_down(MouseButton::Right, cx.listener(move |this, _, _, cx| {
                    this.flip_axis_edge(Some(pane_idx), axis_idx, cx);
                }))
                .on_mouse_down(MouseButton::Left, {
                    let key = key.clone(); let lab = last_render_axis_bounds.clone();
                    cx.listener(move |this, event: &MouseDownEvent, _win, cx| {
                        if event.click_count >= 2 { this.dragging_axis = None; this.auto_fit_axis(Some(pane_idx), axis_idx, cx); return; }
                        if let Some(bounds) = lab.borrow().get(&key) {
                            let pct = ((event.position.y - bounds.origin.y).as_f32() / bounds.size.height.as_f32()).clamp(0.0, 1.0) as f64;
                            this.dragging_axis = Some(AxisDragInfo { pane_idx: Some(pane_idx), axis_idx, is_y: true, button: MouseButton::Left, pivot_pct: 1.0 - pct });
                            this.last_mouse_pos = Some(event.position);
                            this.shared_state.update(cx, |s, _| s.request_render());
                        }
                    })
                })
                .on_mouse_down(MouseButton::Middle, {
                    let key = key.clone(); let lab = last_render_axis_bounds.clone();
                    cx.listener(move |this, event: &MouseDownEvent, _win, cx| {
                        if let Some(bounds) = lab.borrow().get(&key) {
                            let pct = ((event.position.y - bounds.origin.y).as_f32() / bounds.size.height.as_f32()).clamp(0.0, 1.0) as f64;
                            this.dragging_axis = Some(AxisDragInfo { pane_idx: Some(pane_idx), axis_idx, is_y: true, button: MouseButton::Middle, pivot_pct: 1.0 - pct });
                            this.last_mouse_pos = Some(event.position);
                            this.shared_state.update(cx, |s, _| s.request_render());
                        }
                    })
                })
                .on_scroll_wheel({
                    let axis_entity = axis_entity.clone();
                    let shared_state = self.shared_state.clone();
                    move |event, _, cx| {
                        let dy = match event.delta { ScrollDelta::Pixels(p) => p.y.as_f32(), ScrollDelta::Lines(p) => p.y as f32 * 20.0 };
                        let factor = (1.0f64 - dy as f64 * 0.01).clamp(0.1, 10.0);
                        axis_entity.update(cx, |r, _cx| { ViewController::zoom_axis_at(r, 0.5, factor); });
                        shared_state.update(cx, |s, _cx| s.request_render());
                    }
                })
                .child({
                    let key = key.clone(); let lab = last_render_axis_bounds.clone();
                    canvas(|_, _, _| {}, move |bounds, (), _, _| { lab.borrow_mut().insert(key, bounds); }).size_full().absolute()
                })
                .into_any_element();

                if is_left { left_y_axis_elements.push(el); } else { right_y_axis_elements.push(el); }
            }
            current_top_pct += h_pct;
        }

        let mut x_axis_elements = Vec::new();
        let mut top_cursor = px(0.0); let mut bot_cursor = px(0.0);
        for (axis_idx, x_axis) in self.x_axes.iter().enumerate() {
            let axis_entity = x_axis.entity.clone();
            let key = AxisKey::X(axis_idx).to_string();

            let el = AxisRenderer::render_x_axis(
                axis_idx,
                &axis_entity.read(cx),
                x_axis.edge,
                x_axis.size,
                self.gutter_left,
                self.gutter_right,
                x_axis.label.clone(),
                &theme,
            )
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, _, _, cx| {
                this.flip_axis_edge(None, axis_idx, cx);
            }))
            .on_mouse_down(MouseButton::Left, {
                let key = key.clone(); let lab = last_render_axis_bounds.clone();
                cx.listener(move |this, event: &MouseDownEvent, _win, cx| {
                    if event.click_count >= 2 { this.dragging_axis = None; this.auto_fit_axis(None, axis_idx, cx); return; }
                    if let Some(bounds) = lab.borrow().get(&key) {
                        let pct = ((event.position.x - bounds.origin.x).as_f32() / bounds.size.width.as_f32()).clamp(0.0, 1.0) as f64;
                        this.dragging_axis = Some(AxisDragInfo { pane_idx: None, axis_idx, is_y: false, button: MouseButton::Left, pivot_pct: pct });
                        this.last_mouse_pos = Some(event.position);
                        this.shared_state.update(cx, |s, _| s.request_render());
                    }
                })
            })
            .on_mouse_down(MouseButton::Middle, {
                let key = key.clone(); let lab = last_render_axis_bounds.clone();
                cx.listener(move |this, event: &MouseDownEvent, _win, cx| {
                    if let Some(bounds) = lab.borrow().get(&key) {
                        let pct = ((event.position.x - bounds.origin.x).as_f32() / bounds.size.width.as_f32()).clamp(0.0, 1.0) as f64;
                        this.dragging_axis = Some(AxisDragInfo { pane_idx: None, axis_idx, is_y: false, button: MouseButton::Middle, pivot_pct: pct });
                        this.last_mouse_pos = Some(event.position);
                        this.shared_state.update(cx, |s, _| s.request_render());
                    }
                })
            })
            .on_scroll_wheel({
                let axis_entity = axis_entity.clone();
                let shared_state = self.shared_state.clone();
                move |event, _, cx| {
                    let dy = match event.delta { ScrollDelta::Pixels(p) => p.y.as_f32(), ScrollDelta::Lines(p) => p.y as f32 * 20.0 };
                    let factor = (1.0f64 - dy as f64 * 0.01).clamp(0.1, 10.0);
                    axis_entity.update(cx, |r, _cx| { ViewController::zoom_axis_at(r, 0.5, factor); });
                    shared_state.update(cx, |s, _cx| s.request_render());
                }
            })
            .child({
                let key = key.clone(); let lab = last_render_axis_bounds.clone();
                canvas(|_, _, _| {}, move |bounds, (), _, _| { lab.borrow_mut().insert(key, bounds); }).size_full().absolute()
            });

            let axis_div = match x_axis.edge {
                AxisEdge::Top => { let pos = top_cursor; top_cursor += x_axis.size; el.top(pos) }
                AxisEdge::Bottom => { let pos = bot_cursor; bot_cursor += x_axis.size; el.bottom(pos) }
                _ => el,
            };

            x_axis_elements.push(axis_div.into_any_element());
        }

        let (mouse_pos, hover_x) = { let state = self.shared_state.read(cx); (state.mouse_pos, state.hover_x) };
        let mut tags = Vec::new();
        if let (Some(pos), Some(hx)) = (mouse_pos, hover_x) {
            let container_origin = container_bounds_rc.borrow().origin;
            for (i, x_a) in self.x_axes.iter().enumerate() {
                let key = AxisKey::X(i).to_string();
                if let Some(b) = last_render_axis_bounds.borrow().get(&key) {
                    let r = x_a.entity.read(cx); let scale = crate::scales::ChartScale::new_linear(r.clamped_bounds(), (0.0, b.size.width.as_f32()));
                    let sx = b.origin.x - container_origin.x + px(scale.map(hx));
                    tags.push(div().absolute().top(b.origin.y - container_origin.y).left(sx).ml(px(-40.0)).w(px(80.0)).h(x_a.size)
                        .child(crate::rendering::create_axis_tag(scale.format_tick(hx), px(40.0), true)).into_any_element());
                }
            }
            for (p_idx, pc) in self.panes.iter().enumerate() {
                for (a_idx, y_a) in pc.y_axes.iter().enumerate() {
                    let key = AxisKey::Y(p_idx, a_idx).to_string();
                    if let Some(b) = last_render_axis_bounds.borrow().get(&key) {
                        if pos.y >= b.origin.y && pos.y <= b.origin.y + b.size.height {
                            let r = y_a.entity.read(cx); let scale = crate::scales::ChartScale::new_linear(r.clamped_bounds(), (b.size.height.as_f32(), 0.0));
                            let val = scale.invert((pos.y - b.origin.y).as_f32());
                            tags.push(div().absolute().top(pos.y - container_origin.y - px(10.0)).left(b.origin.x - container_origin.x).w(y_a.size).h(px(20.0))
                                .bg(gpui::white()).text_color(gpui::black()).rounded_sm().text_size(px(11.0)).flex().items_center().justify_center().child(scale.format_tick(val)).into_any_element());
                        }
                    }
                }
            }
        }

        div().track_focus(&self.focus_handle).size_full().relative().bg(gpui::black())
            .child(canvas(|_, _, _| {}, move |bounds, (), _, _| { *container_bounds_rc.borrow_mut() = bounds; }).size_full().absolute())
            .on_mouse_down(MouseButton::Left, { let fh = self.focus_handle.clone(); move |_, window, _| { window.focus(&fh); } })
            .on_mouse_move(cx.listener(Self::handle_global_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_global_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::handle_global_mouse_up))
            .on_action(cx.listener(Self::handle_pan_left)).on_action(cx.listener(Self::handle_pan_right)).on_action(cx.listener(Self::handle_zoom_in)).on_action(cx.listener(Self::handle_zoom_out))
            .child(div().absolute().top(self.gutter_top).left(self.gutter_left).right(self.gutter_right).bottom(self.gutter_bottom).flex().flex_col().children({
                let mut children = Vec::new(); let pane_count = self.panes.len();
                for (i, p) in self.panes.iter().enumerate() {
                    let h_pct = if total_weight > 0.0 { p.weight / total_weight } else { 1.0 / pane_count as f32 };
                    let is_first = i == 0; let is_last = i == pane_count - 1;
                    children.push(div().h(relative(h_pct)).w_full().relative().group("pane_container").child(p.pane.clone())
                        .child(div().absolute().top_2().right_2().flex().gap_1().bg(gpui::black().alpha(0.4)).rounded_lg().p_1().border_1().border_color(gpui::white().alpha(0.05)).group_hover("pane_container", |d| d.bg(gpui::black().alpha(0.8)).border_color(gpui::white().alpha(0.2)))
                            .child(self.render_control_button("↑", !is_first, cx.listener(move |this, _, _, cx| this.move_pane_up(i, cx))))
                            .child(self.render_control_button("↓", !is_last, cx.listener(move |this, _, _, cx| this.move_pane_down(i, cx))))
                            .child(self.render_control_button("+", true, cx.listener(move |this, _, _, cx| this.add_pane_at(i + 1, 1.0, cx))))
                            .child(self.render_control_button("✕", true, cx.listener(move |this, _, _, cx| this.remove_pane(i, cx))))).into_any_element());
                    if !is_last {
                        children.push(div().h(px(6.0)).w_full().flex().items_center().bg(gpui::transparent_black()).group("splitter").cursor(CursorStyle::ResizeUpDown).on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, _win, cx| { this.dragging_splitter = Some(i); this.last_mouse_y = Some(event.position.y); cx.notify(); }))
                            .child(div().h(px(2.0)).w_full().bg(gpui::white().alpha(0.1)).group_hover("splitter", |d| d.bg(gpui::blue().alpha(0.5)))).into_any_element());
                    }
                }
                children
            }))
            .child(div().absolute().top(self.gutter_top).bottom(self.gutter_bottom).left_0().w(self.gutter_left).children(left_y_axis_elements))
            .child(div().absolute().top(self.gutter_top).bottom(self.gutter_bottom).right_0().w(self.gutter_right).children(right_y_axis_elements))
            .children(x_axis_elements)
            .children(tags)
    }
}

pub struct Chart;
impl Chart {
    pub fn new(x_axis: Entity<AxisRange>, state: Entity<SharedPlotState>, cx: &mut Context<ChartContainer>) -> ChartContainer { ChartContainer::new(x_axis, state, cx) }
}
