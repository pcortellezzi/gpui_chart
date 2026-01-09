// Rendering functions for the chart
#![allow(clippy::collapsible_if)]

use crate::data_types::{AxisDomain, Series, Ticks};
use crate::scales::ChartScale;
use crate::transform::PlotTransform;
use gpui::*;
use adabraka_ui::util::PixelsExt;

/// Paints the chart data on the canvas.
pub fn paint_plot(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    series: &[Series],
    domain: &AxisDomain,
    _cx: &mut App,
) {
    let width_px = bounds.size.width.as_f32();
    let height_px = bounds.size.height.as_f32();
    
    let x_scale = ChartScale::new_linear((domain.x_min, domain.x_max), (0.0, width_px));
    let y_scale = ChartScale::new_linear((domain.y_min, domain.y_max), (height_px, 0.0));

    let transform = PlotTransform::new(x_scale, y_scale, bounds);

    for series in series {
        series.plot.borrow().render(window, &transform, &series.id);
    }
}

// Creates the axis lines and label elements.
pub fn paint_axes(
    domain: &AxisDomain,
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    ticks: &Ticks,
    color: Hsla,
    show_x_labels: bool,
    show_y_labels: bool,
    margin_left: Pixels,
    margin_bottom: Pixels,
) -> Vec<gpui::AnyElement> {
    let mut elements = Vec::new();
    let effective_bottom = if show_x_labels { margin_bottom } else { px(0.0) };
    let font_size = px(12.0);

    // Vertical axis line
    elements.push(
        div()
            .absolute()
            .left(margin_left - px(1.0))
            .top(px(0.0))
            .w(px(1.0))
            .bottom(effective_bottom)
            .bg(color)
            .into_any_element(),
    );

    // Horizontal axis line
    elements.push(
        div()
            .absolute()
            .left(margin_left)
            .bottom(effective_bottom)
            .w_full()
            .h(px(1.0))
            .bg(color)
            .into_any_element(),
    );

    if show_x_labels {
        for tick_x in &ticks.x {
            // Respect limits
            if let Some(l) = domain.x_min_limit { if *tick_x < l { continue; } }
            if let Some(l) = domain.x_max_limit { if *tick_x > l { continue; } }

            let x_pct = (*tick_x - domain.x_min) / domain.width();
            let label_text = x_scale.format_tick(*tick_x);

            if (0.0..=1.0).contains(&x_pct) {
                elements.push(
                    div()
                        .absolute()
                        .left(gpui::DefiniteLength::Fraction(x_pct as f32))
                        .bottom(px(0.0))
                        .w(px(80.0))
                        .text_align(gpui::TextAlign::Center)
                        .text_color(color)
                        .text_size(font_size)
                        .ml(px(-40.0))
                        .child(label_text)
                        .into_any_element(),
                );
            }
        }
    }

    if show_y_labels {
        for tick_y in &ticks.y {
            // Respect limits
            if let Some(l) = domain.y_min_limit { if *tick_y < l { continue; } }
            if let Some(l) = domain.y_max_limit { if *tick_y > l { continue; } }

            let y_pct = (*tick_y - domain.y_min) / domain.height();
            let label_text = y_scale.format_tick(*tick_y);

            if (0.0..=1.0).contains(&y_pct) {
                elements.push(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(gpui::DefiniteLength::Fraction((1.0 - y_pct) as f32))
                        .w(margin_left - px(5.0))
                        .h(px(16.0))
                        .flex()
                        .items_center()
                        .justify_end()
                        .text_color(color)
                        .text_size(font_size)
                        .mt(px(-8.0))
                        .child(label_text)
                        .into_any_element(),
                );
            }
        }
    }

    elements
}

// Paints the grid lines on the canvas.
pub fn paint_grid(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    domain: &AxisDomain,
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    ticks: &Ticks,
) {
    let origin_x = bounds.origin.x.as_f32();
    let origin_y = bounds.origin.y.as_f32();

    let mut vertical_builder = PathBuilder::stroke(px(1.0));
    let mut has_vertical = false;

    for tick_x in &ticks.x {
        // Respect limits
        if let Some(l) = domain.x_min_limit { if *tick_x < l { continue; } }
        if let Some(l) = domain.x_max_limit { if *tick_x > l { continue; } }

        let x_pct = (*tick_x - domain.x_min) / domain.width();
        if (0.0..=1.0).contains(&x_pct) {
            let pixel_x = origin_x + x_scale.map(*tick_x);
            vertical_builder.move_to(Point::new(px(pixel_x), px(origin_y + 0.5)));
            vertical_builder.line_to(Point::new(px(pixel_x), px(origin_y + bounds.size.height.as_f32() - 0.5)));
            has_vertical = true;
        }
    }
    if has_vertical {
        if let Ok(path) = vertical_builder.build() {
            window.paint_path(path, gpui::white().opacity(0.1));
        }
    }

    let mut horizontal_builder = PathBuilder::stroke(px(1.0));
    let mut has_horizontal = false;

    for tick_y in &ticks.y {
        // Respect limits
        if let Some(l) = domain.y_min_limit { if *tick_y < l { continue; } }
        if let Some(l) = domain.y_max_limit { if *tick_y > l { continue; } }

        let y_pct = (*tick_y - domain.y_min) / domain.height();
        if (0.0..=1.0).contains(&y_pct) {
            let pixel_y = origin_y + y_scale.map(*tick_y);
            horizontal_builder.move_to(Point::new(px(origin_x + 0.5), px(pixel_y)));
            horizontal_builder.line_to(Point::new(px(origin_x + bounds.size.width.as_f32() - 0.5), px(pixel_y)));
            has_horizontal = true;
        }
    }
    if has_horizontal {
        if let Ok(path) = horizontal_builder.build() {
            window.paint_path(path, gpui::white().opacity(0.1));
        }
    }
}

/// Helper to create a tag element on an axis.
pub fn create_axis_tag(
    text: String,
    position: Pixels,
    is_x_axis: bool,
    _color: Hsla,
    _bg_color: Hsla,
    margin_left: Pixels,
) -> gpui::AnyElement {
    if is_x_axis {
        div()
            .absolute()
            .left(position)
            .bottom(px(0.0))
            .ml(px(-40.0))
            .w(px(80.0))
            .h(px(20.0))
            .bg(gpui::white())
            .text_color(gpui::black())
            .text_size(px(12.0))
            .flex()
            .items_center()
            .justify_center()
            .child(text)
            .into_any_element()
    } else {
        div()
            .absolute()
            .left(px(0.0))
            .top(position)
            .mt(px(-8.0))
            .w(margin_left - px(2.0))
            .h(px(16.0))
            .bg(gpui::white())
            .text_color(gpui::black())
            .text_size(px(12.0))
            .flex()
            .items_center()
            .justify_center()
            .child(text)
            .into_any_element()
    }
}