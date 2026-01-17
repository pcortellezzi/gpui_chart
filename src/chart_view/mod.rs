pub mod renderer;
pub mod input;
pub mod actions;

use crate::chart::Chart;
use crate::data_types::{InertiaConfig, LegendConfig};
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub use renderer::AxisKey;
pub use actions::{PanLeft, PanRight, PanUp, PanDown, ZoomIn, ZoomOut, ResetView, ToggleDebug, ToggleCrosshair};

use self::renderer::ChartRenderer;
use self::input::ChartInputHandler;
use self::actions::ChartActionHandler;

pub struct ChartView {
    pub chart: Entity<Chart>,
    
    // Components (Delegates)
    renderer: ChartRenderer,
    input: ChartInputHandler,
    actions: ChartActionHandler,

    // Configuration exposed for modification (needs to sync with renderer)
    pub inertia_config: InertiaConfig,
    
    focus_handle: FocusHandle,
}

impl Focusable for ChartView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ChartView {
    pub fn new(chart: Entity<Chart>, cx: &mut Context<Self>) -> Self {
        cx.observe(&chart, |_, _, cx| cx.notify()).detach();
        let shared_state = chart.read(cx).shared_state.clone();
        cx.observe(&shared_state, |_, _, cx| {
            eprintln!("DEBUG: ChartView observed shared_state update");
            cx.notify()
        }).detach();

        let focus_handle = cx.focus_handle();
        
        // Shared state containers
        let last_render_axis_bounds = Rc::new(RefCell::new(HashMap::new()));
        let bounds = Rc::new(RefCell::new(Bounds::default()));
        let pane_bounds = Rc::new(RefCell::new(HashMap::new()));

        let renderer = ChartRenderer::new(
            chart.clone(),
            last_render_axis_bounds.clone(),
            bounds.clone(),
            pane_bounds.clone(),
        );

        let input = ChartInputHandler::new(
            chart.clone(),
            focus_handle.clone(),
            last_render_axis_bounds.clone(),
            bounds.clone(),
            pane_bounds.clone(),
        );

        let actions = ChartActionHandler::new(chart.clone());

        Self {
            chart,
            renderer,
            input,
            actions,
            inertia_config: InertiaConfig::default(),
            focus_handle,
        }
    }
    
    // Forward configuration to renderer
    pub fn set_legend_config(&mut self, config: LegendConfig) {
        self.renderer.legend_config = config;
    }
}

impl Render for ChartView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Delegate rendering to renderer
        let element = self.renderer.render(window, cx);
        
        // Attach event listeners here using the input handler and action handler
        // Since input and actions are stored in self, we need to clone them or pass references.
        // Event listeners take ownership of the closure, so we need to clone the handlers 
        // (which are cheap to clone as they mostly hold Entity/Rc handles).
        
        let input = self.input.clone();
        let actions = self.actions.clone();
        let entity_id = cx.entity_id();
        
        element
            .id(("chart-view", entity_id))
            .track_focus(&self.focus_handle)
            .on_mouse_down(MouseButton::Left, {
                let input = input.clone();
                move |e, w, c| input.handle_mouse_down(e, w, c)
            })
            .on_mouse_down(MouseButton::Right, {
                let input = input.clone();
                move |e, w, c| input.handle_mouse_down(e, w, c)
            })
            .on_mouse_down(MouseButton::Middle, {
                let input = input.clone();
                move |e, w, c| input.handle_mouse_down(e, w, c)
            })
            .on_mouse_move({
                let input = input.clone();
                move |e, w, c| input.handle_global_mouse_move(e, w, c, entity_id)
            })
            .on_mouse_up(MouseButton::Left, {
                let input = input.clone();
                move |e, w, c| input.handle_global_mouse_up(e, w, c, entity_id)
            })
            .on_mouse_up(MouseButton::Right, {
                let input = input.clone();
                move |e, w, c| input.handle_global_mouse_up(e, w, c, entity_id)
            })
            .on_mouse_up(MouseButton::Middle, {
                let input = input.clone();
                move |e, w, c| input.handle_global_mouse_up(e, w, c, entity_id)
            })
            .on_scroll_wheel({
                let input = input.clone();
                move |e, w, c| input.handle_scroll_wheel(e, w, c)
            })
            .on_action({
                let actions = actions.clone();
                move |a, w, c| actions.handle_pan_left(a, w, c)
            })
            .on_action({
                let actions = actions.clone();
                move |a, w, c| actions.handle_pan_right(a, w, c)
            })
            .on_action({
                let actions = actions.clone();
                move |a, w, c| actions.handle_zoom_in(a, w, c)
            })
            .on_action({
                let actions = actions.clone();
                move |a, w, c| actions.handle_zoom_out(a, w, c)
            })
            .on_action({
                let actions = actions.clone();
                move |a, w, c| actions.handle_reset_view(a, w, c)
            })
            .on_action({
                let actions = actions.clone();
                move |a, w, c| actions.handle_toggle_debug(a, w, c)
            })
            .on_action({
                let actions = actions.clone();
                move |a, w, c| actions.handle_toggle_crosshair(a, w, c)
            })
    }
}
