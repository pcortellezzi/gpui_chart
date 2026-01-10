use gpui::*;
use gpui::prelude::FluentBuilder;
use d3rs::scale::Scale;
use crate::data_types::{AxisRange, AxisEdge};
use crate::scales::ChartScale;
use crate::theme::ChartTheme;

pub struct AxisRenderer;

impl AxisRenderer {
    pub fn render_y_axis(
        pane_idx: usize,
        axis_idx: usize,
        range: &AxisRange,
        edge: AxisEdge,
        size: Pixels,
        h_pct: f32,
        current_top_pct: f32,
        x_pos: Pixels,
        label: String,
        theme: &ChartTheme,
    ) -> Stateful<Div> {
        let (min, max) = range.clamped_bounds();
        let scale = ChartScale::new_linear((min, max), (1.0, 0.0));
        
        // Fallback si le cache est vide
        let ticks_vec;
        let ticks = if range.cached_ticks.is_empty() {
            ticks_vec = d3rs::scale::LinearScale::new()
                .domain(min, max)
                .range(1.0, 0.0)
                .ticks(10);
            &ticks_vec
        } else {
            &range.cached_ticks
        };
            
        let is_left = edge == AxisEdge::Left;
        
        div()
            .id(SharedString::from(format!("y-axis-{}-{}", pane_idx, axis_idx)))
            .absolute()
            .top(relative(current_top_pct))
            .h(relative(h_pct))
            .w(size)
            .when(is_left, |d| d.left(x_pos).border_r_1())
            .when(!is_left, |d| d.right(x_pos).border_l_1())
            .border_color(theme.axis_line)
            .cursor(CursorStyle::ResizeUpDown)
            .children(ticks.iter().map(|tick| {
                let y_pct = scale.map(*tick);
                div()
                    .absolute()
                    .top(relative(y_pct as f32))
                    .mt(px(-8.0))
                    .w_full()
                    .text_size(theme.axis_label_size)
                    .text_color(theme.axis_label)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(scale.format_tick(*tick))
            }))
            .child(
                div()
                    .absolute()
                    .bottom_1()
                    .w_full()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(9.0))
                            .text_color(theme.accent.opacity(0.5))
                            .child(label)
                    )
            )
    }

    pub fn render_x_axis(
        axis_idx: usize,
        range: &AxisRange,
        edge: AxisEdge,
        size: Pixels,
        gutter_left: Pixels,
        gutter_right: Pixels,
        label: String,
        theme: &ChartTheme,
    ) -> Stateful<Div> {
        let (min, max) = range.clamped_bounds();
        let scale = ChartScale::new_linear((min, max), (0.0, 1.0));
        
        let ticks_vec;
        let ticks = if range.cached_ticks.is_empty() {
            ticks_vec = d3rs::scale::LinearScale::new()
                .domain(min, max)
                .range(0.0, 1.0)
                .ticks(10);
            &ticks_vec
        } else {
            &range.cached_ticks
        };

        let is_top = edge == AxisEdge::Top;

        div()
            .id(SharedString::from(format!("x-axis-{}", axis_idx)))
            .absolute()
            .left(gutter_left)
            .right(gutter_right)
            .h(size)
            .bg(theme.background)
            .when(is_top, |d| d.top(px(0.0)).border_b_1())
            .when(!is_top, |d| d.bottom(px(0.0)).border_t_1())
            .border_color(theme.axis_line)
            .cursor(CursorStyle::ResizeLeftRight)
            .children(ticks.iter().map(|tick| {
                let x_pct = scale.map(*tick);
                div()
                    .absolute()
                    .left(relative(x_pct as f32))
                    .ml(px(-30.0))
                    .w(px(60.0))
                    .text_size(theme.axis_label_size)
                    .text_color(theme.axis_label)
                    .text_align(TextAlign::Center)
                    .child(scale.format_tick(*tick))
            }))
            .child(
                div()
                    .absolute()
                    .left_2()
                    .top_0()
                    .text_size(px(10.0))
                    .text_color(theme.accent)
                    .child(label)
            )
    }
}
