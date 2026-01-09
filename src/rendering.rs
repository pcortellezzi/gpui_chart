// Rendering functions for the chart
#![allow(clippy::collapsible_if)]

use crate::data_types::{AxisDomain, Series};
use crate::scales::ChartScale;
use crate::transform::PlotTransform;
use gpui::*;
use adabraka_ui::util::PixelsExt;

/// Paints the chart data on the canvas.
pub fn paint_plot(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    series: &[Series],
    x_domain: (f64, f64),
    y_domains: &[(f64, f64)],
    _cx: &mut App,
) {
    let width_px = bounds.size.width.as_f32();
    let height_px = bounds.size.height.as_f32();
    
    let x_scale = ChartScale::new_linear(x_domain, (0.0, width_px));
    
    for series in series {
        let y_domain = y_domains.get(series.y_axis_index).copied().unwrap_or((0.0, 1.0));
        let y_scale = ChartScale::new_linear(y_domain, (height_px, 0.0));
        let transform = PlotTransform::new(x_scale.clone(), y_scale, bounds);
        series.plot.borrow().render(window, &transform, &series.id);
    }
}

#[derive(Clone)]
pub struct YAxisRenderInfo {
    pub domain: (f64, f64),
    pub scale: ChartScale,
    pub ticks: Vec<f64>,
    pub limits: (Option<f64>, Option<f64>),
}

// Creates the axis lines and label elements.
pub fn paint_axes(
    x_domain: &AxisDomain,
    x_scale: &ChartScale,
    x_ticks: &[f64],
    y_axes: &[YAxisRenderInfo],
    color: Hsla,
    show_x_labels: bool,
    show_y_labels: bool,
    margin_left: Pixels,
    margin_bottom: Pixels,
    margin_right: Pixels,
) -> Vec<gpui::AnyElement> {
    let mut elements = Vec::new();
    let effective_bottom = if show_x_labels { margin_bottom } else { px(0.0) };
    let font_size = px(12.0);

    // Primary Vertical axis line (Left)
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

    // Secondary Vertical axis line (Right) - if we have more than one axis
    if y_axes.len() > 1 {
        elements.push(
            div()
                .absolute()
                .right(margin_right - px(1.0))
                .top(px(0.0))
                .w(px(1.0))
                .bottom(effective_bottom)
                .bg(color)
                .into_any_element(),
        );
    }

    // Horizontal axis line
    elements.push(
        div()
            .absolute()
            .left(margin_left)
            .right(margin_right)
            .bottom(effective_bottom)
            .h(px(1.0))
            .bg(color)
            .into_any_element(),
    );

    if show_x_labels {
        let x_span = x_domain.x_max - x_domain.x_min;
        for tick_x in x_ticks {
            // Respect limits
            if let Some(l) = x_domain.x_min_limit { if *tick_x < l { continue; } }
            if let Some(l) = x_domain.x_max_limit { if *tick_x > l { continue; } }

            let x_pct = (*tick_x - x_domain.x_min) / x_span;
            let label_text = x_scale.format_tick(*tick_x);

            if (0.0..=1.0).contains(&x_pct) {
                elements.push(
                    div()
                        .absolute()
                        .left(margin_left + px(x_pct as f32 * (x_scale.range().1 - x_scale.range().0)))
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
        for (idx, y_info) in y_axes.iter().enumerate() {
            let y_span = y_info.domain.1 - y_info.domain.0;
            let is_primary = idx == 0;
            
            for tick_y in &y_info.ticks {
                if let Some(l) = y_info.limits.0 { if *tick_y < l { continue; } }
                if let Some(l) = y_info.limits.1 { if *tick_y > l { continue; } }

                let y_pct = (*tick_y - y_info.domain.0) / y_span;
                let label_text = y_info.scale.format_tick(*tick_y);

                if (0.0..=1.0).contains(&y_pct) {
                    let mut label_div = div()
                        .absolute()
                        .top(gpui::DefiniteLength::Fraction((1.0 - y_pct) as f32))
                        .h(px(16.0))
                        .flex()
                        .items_center()
                        .text_color(color)
                        .text_size(font_size)
                        .mt(px(-8.0));

                    if is_primary {
                        label_div = label_div
                            .left(px(0.0))
                            .w(margin_left - px(5.0))
                            .justify_end();
                    } else {
                        // Position on the right
                        label_div = label_div
                            .right(px(0.0))
                            .w(margin_right - px(5.0))
                            .justify_start();
                    }

                    elements.push(label_div.child(label_text).into_any_element());
                }
            }
        }
    }

    elements
}

// Paints the grid lines on the canvas.
pub fn paint_grid(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    x_domain: &AxisDomain,
    x_scale: &ChartScale,
    x_ticks: &[f64],
    primary_y_axis: &YAxisRenderInfo,
) {
    let origin_x = bounds.origin.x.as_f32();
    let origin_y = bounds.origin.y.as_f32();

    let mut vertical_builder = PathBuilder::stroke(px(1.0));
    let mut has_vertical = false;

    let x_span = x_domain.x_max - x_domain.x_min;

    for tick_x in x_ticks {
        if let Some(l) = x_domain.x_min_limit { if *tick_x < l { continue; } }
        if let Some(l) = x_domain.x_max_limit { if *tick_x > l { continue; } }

        let x_pct = (*tick_x - x_domain.x_min) / x_span;
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

    let y_span = primary_y_axis.domain.1 - primary_y_axis.domain.0;

    for tick_y in &primary_y_axis.ticks {
        if let Some(l) = primary_y_axis.limits.0 { if *tick_y < l { continue; } }
        if let Some(l) = primary_y_axis.limits.1 { if *tick_y > l { continue; } }

        let y_pct = (*tick_y - primary_y_axis.domain.0) / y_span;
        if (0.0..=1.0).contains(&y_pct) {
            let pixel_y = origin_y + primary_y_axis.scale.map(*tick_y);
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
    margin_right: Pixels,
    is_primary_y: bool,
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
        let mut tag_div = div()
            .absolute()
            .top(position)
            .mt(px(-8.0))
            .h(px(16.0))
            .bg(gpui::white())
            .text_color(gpui::black())
            .text_size(px(12.0))
            .flex()
            .items_center()
            .justify_center();

        if is_primary_y {
            tag_div = tag_div
                .left(px(0.0))
                .w(margin_left - px(2.0));
        } else {
            tag_div = tag_div
                .right(px(0.0))
                .w(margin_right - px(2.0));
        }

        tag_div.child(text).into_any_element()
    }
}