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
    
    // Create scales (Linear for now, but ready for Time/Log)
    // Note: Y axis in GPUI is top-down (0 at top), so range is (height, 0) for Cartesian up
    let x_scale = ChartScale::new_linear(
        (domain.x_min, domain.x_max),
        (0.0, width_px)
    );
    let y_scale = ChartScale::new_linear(
        (domain.y_min, domain.y_max),
        (height_px, 0.0) // Invert Y for standard plot coordinates (0 at bottom)
    );

    let transform = PlotTransform::new(x_scale, y_scale, bounds);

    for series in series {
        series
            .plot
            .borrow()
            .render(window, &transform, &series.id);
    }
}

// Creates the axis lines and label elements.
pub fn paint_axes(
    domain: &AxisDomain,
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    ticks: &Ticks,
    color: Hsla,
) -> Vec<gpui::AnyElement> {
    let mut elements = Vec::new();

    // Vertical axis line at y-axis
    elements.push(
        div()
            .absolute()
            .left(px(49.0))
            .top(px(0.0))
            .w(px(1.0))
            .bottom(px(20.0))
            .bg(color)
            .into_any_element(),
    );

    // Horizontal axis line at x-axis
    elements.push(
        div()
            .absolute()
            .left(px(50.0))
            .bottom(px(20.0))
            .w_full()
            .h(px(1.0))
            .bg(color)
            .into_any_element(),
    );

    for tick_x in &ticks.x {
        let x_pct = (*tick_x - domain.x_min) / domain.width();
        let label_text = x_scale.format_tick(*tick_x);

        if (0.0..=1.0).contains(&x_pct) {
            elements.push(
                div()
                    .absolute()
                    .left(gpui::DefiniteLength::Fraction(x_pct as f32))
                    .bottom(px(0.0))
                    .w(px(60.0))
                    .text_align(gpui::TextAlign::Center)
                    .text_color(color)
                    .ml(px(-30.0))
                    .child(label_text)
                    .into_any_element(),
            );
        }
    }

    for tick_y in &ticks.y {
        let y_pct = (*tick_y - domain.y_min) / domain.height();
        let label_text = y_scale.format_tick(*tick_y);

        if (0.0..=1.0).contains(&y_pct) {
            elements.push(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(gpui::DefiniteLength::Fraction((1.0 - y_pct) as f32))
                    .w(px(50.0))
                    .h(px(16.0))
                    .flex()
                    .items_center()
                    .text_color(color)
                    .mt(px(-8.0))
                    .child(label_text)
                    .into_any_element(),
            );
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

    // Vertical grid lines
    let mut vertical_builder = PathBuilder::stroke(px(1.0));
    let mut has_vertical = false;

    for tick_x in &ticks.x {
        let x_pct = (*tick_x - domain.x_min) / domain.width();
        if (0.0..=1.0).contains(&x_pct) {
            let pixel_x = origin_x + x_scale.map(*tick_x);
            vertical_builder.move_to(Point::new(px(pixel_x), px(origin_y + 0.5)));
            vertical_builder.line_to(Point::new(
                px(pixel_x),
                px(origin_y + bounds.size.height.as_f32() - 0.5),
            ));
            has_vertical = true;
        }
    }
    if has_vertical {
        if let Ok(path) = vertical_builder.build() {
            window.paint_path(path, gpui::white().opacity(0.1));
        }
    }

    // Horizontal grid lines
    let mut horizontal_builder = PathBuilder::stroke(px(1.0));
    let mut has_horizontal = false;

    for tick_y in &ticks.y {
        let y_pct = (*tick_y - domain.y_min) / domain.height();
        if (0.0..=1.0).contains(&y_pct) {
            let pixel_y = origin_y + y_scale.map(*tick_y);
            horizontal_builder.move_to(Point::new(px(origin_x + 0.5), px(pixel_y)));
            horizontal_builder.line_to(Point::new(
                px(origin_x + bounds.size.width.as_f32() - 0.5),
                px(pixel_y),
            ));
            has_horizontal = true;
        }
    }
    if has_horizontal {
        if let Ok(path) = horizontal_builder.build() {
            window.paint_path(path, gpui::white().opacity(0.1));
        }
    }
}

/// Paints the crosshair lines.
pub fn paint_crosshair(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    mouse_pos: Point<Pixels>,
    color: Hsla,
) {
    if !bounds.contains(&mouse_pos) { return; }

    let mut builder = PathBuilder::stroke(px(1.0));
    // Vertical line
    builder.move_to(Point::new(mouse_pos.x, bounds.origin.y));
    builder.line_to(Point::new(mouse_pos.x, bounds.origin.y + bounds.size.height));
    
    // Horizontal line
    builder.move_to(Point::new(bounds.origin.x, mouse_pos.y));
    builder.line_to(Point::new(bounds.origin.x + bounds.size.width, mouse_pos.y));

    if let Ok(path) = builder.build() {
        window.paint_path(path, color.opacity(0.5));
    }
}

/// Helper to create a tag element on an axis.
pub fn create_axis_tag(
    text: String,
    position: Pixels,
    is_x_axis: bool,
    color: Hsla,
    bg_color: Hsla,
) -> gpui::AnyElement {
    if is_x_axis {
        // Label at bottom
        div()
            .absolute()
            .left(position)
            .bottom(px(0.0))
            .ml(px(-30.0))
            .w(px(60.0))
            .h(px(20.0))
            .bg(bg_color)
            .text_color(color)
            .text_size(px(10.0))
            .flex()
            .items_center()
            .justify_center()
            .child(text)
            .into_any_element()
    } else {
        // Label at left
        div()
            .absolute()
            .left(px(0.0))
            .top(position)
            .mt(px(-8.0))
            .w(px(50.0))
            .h(px(16.0))
            .bg(bg_color)
            .text_color(color)
            .text_size(px(10.0))
            .flex()
            .items_center()
            .justify_center()
            .child(text)
            .into_any_element()
    }
}
