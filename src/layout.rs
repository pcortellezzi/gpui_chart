use gpui::prelude::*;
use gpui::*;
use crate::ChartPane;
use crate::view_controller::ViewController;
use adabraka_ui::util::PixelsExt;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaneSize {
    Fixed(Pixels),
    Weight(f32),
}

pub struct Pane {
    pub chart: Entity<ChartPane>,
    pub size: PaneSize,
}

pub struct ChartLayout {
    panes: Vec<Pane>,
    dragging_splitter: Option<usize>,
    last_mouse_y: Option<Pixels>,
    bounds: Rc<RefCell<Bounds<Pixels>>>,
}

impl ChartLayout {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            panes: Vec::new(),
            dragging_splitter: None,
            last_mouse_y: None,
            bounds: Rc::new(RefCell::new(Bounds::default())),
        }
    }

    pub fn add_pane(&mut self, chart: Entity<ChartPane>, size: PaneSize, cx: &mut Context<Self>) {
        self.panes.push(Pane { chart, size });
        cx.notify();
    }

    pub fn remove_pane(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.panes.len() {
            self.panes.remove(index);
            cx.notify();
        }
    }

    pub fn move_pane_up(&mut self, index: usize, cx: &mut Context<Self>) {
        if index > 0 && index < self.panes.len() {
            self.panes.swap(index, index - 1);
            cx.notify();
        }
    }

    pub fn move_pane_down(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.panes.len() - 1 {
            self.panes.swap(index, index + 1);
            cx.notify();
        }
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent, _win: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.dragging_splitter {
            if let Some(last_y) = self.last_mouse_y {
                let delta = event.position.y - last_y;
                if delta.abs() > px(0.5) {
                    self.resize_panes(index, delta, cx);
                    self.last_mouse_y = Some(event.position.y);
                }
            }
        }
    }

    fn handle_mouse_up(&mut self, _: &MouseUpEvent, _win: &mut Window, cx: &mut Context<Self>) {
        self.dragging_splitter = None;
        self.last_mouse_y = None;
        cx.notify();
    }

    fn resize_panes(&mut self, index: usize, delta: Pixels, cx: &mut Context<Self>) {
        let total_height = self.bounds.borrow().size.height;
        if total_height <= px(0.0) { return; }

        if index + 1 < self.panes.len() {
            let mut weights: Vec<f32> = self.panes.iter().map(|p| match p.size {
                PaneSize::Weight(w) => w,
                PaneSize::Fixed(_) => 0.0,
            }).collect();

            // Only resize if both involved panes are weighted
            if weights[index] > 0.0 && weights[index+1] > 0.0 {
                ViewController::resize_panes(&mut weights, index, delta.as_f32(), total_height.as_f32());
                
                self.panes[index].size = PaneSize::Weight(weights[index]);
                self.panes[index + 1].size = PaneSize::Weight(weights[index + 1]);
                cx.notify();
            }
        }
    }

    fn render_pane_controls(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let is_first = index == 0;
        let is_last = index == self.panes.len() - 1;

        div()
            .absolute()
            .top_2()
            .right_2()
            .flex()
            .gap_1()
            .bg(gpui::black().alpha(0.5))
            .rounded_md()
            .p_1()
            .child(
                self.render_button("↑", !is_first, cx.listener(move |this, _, _, cx| this.move_pane_up(index, cx)))
            )
            .child(
                self.render_button("↓", !is_last, cx.listener(move |this, _, _, cx| this.move_pane_down(index, cx)))
            )
            .child(
                self.render_button("✕", true, cx.listener(move |this, _, _, cx| this.remove_pane(index, cx)))
            )
    }

    fn render_button(&self, label: &'static str, enabled: bool, on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static) -> impl IntoElement {
        div()
            .size_6()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .text_size(px(12.0))
            .when(enabled, |d| {
                d.hover(|s| s.bg(gpui::white().alpha(0.1)))
                 .cursor_pointer()
                 .on_mouse_down(MouseButton::Left, on_click)
            })
            .when(!enabled, |d| {
                d.text_color(gpui::white().alpha(0.2))
            })
            .child(label)
    }
}

impl Render for ChartLayout {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panes_count = self.panes.len();
        let mut children = Vec::new();

        let total_weight: f32 = self.panes.iter().map(|p| match p.size {
            PaneSize::Weight(w) => w,
            PaneSize::Fixed(_) => 0.0,
        }).sum();

        let bounds_rc = self.bounds.clone();

        for i in 0..panes_count {
            let pane = &self.panes[i];
            let mut pane_div = div()
                .w_full()
                .relative()
                .border_color(gpui::white().alpha(0.1));

            if i > 0 {
                pane_div = pane_div.border_t_1();
            }

            match pane.size {
                PaneSize::Fixed(p) => { pane_div = pane_div.h(p); }
                PaneSize::Weight(w) => { 
                    if total_weight > 0.0 {
                        pane_div = pane_div.h(relative(w / total_weight));
                    } else {
                        pane_div = pane_div.flex_grow();
                    }
                }
            }

            children.push(
                pane_div
                    .child(pane.chart.clone())
                    .child(self.render_pane_controls(i, cx))
                    .into_any_element()
            );

            if i < panes_count - 1 {
                children.push(
                    div()
                        .h(px(4.0))
                        .w_full()
                        .bg(gpui::white().alpha(0.05))
                        .hover(|s| s.bg(gpui::blue().alpha(0.3)))
                        .cursor(CursorStyle::ResizeUpDown)
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, _win, cx| {
                            this.dragging_splitter = Some(i);
                            this.last_mouse_y = Some(event.position.y);
                            cx.notify();
                        }))
                        .into_any_element()
                );
            }
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .child(
                canvas(|_, _, _| {}, move |bounds, (), _, _| {
                    *bounds_rc.borrow_mut() = bounds;
                })
                .absolute()
                .size_full()
            )
            .children(children)
    }
}