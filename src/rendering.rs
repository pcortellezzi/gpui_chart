// Rendering functions for the chart
#![allow(clippy::collapsible_if)]

use crate::data_types::{AxisDomain, Series, SharedPlotState};
use crate::scales::ChartScale;
use crate::transform::PlotTransform;
use crate::utils::PixelsExt;
use gpui::*;

/// Stats about the last paint operation.
#[derive(Debug, Clone, Default)]
pub struct PaintStats {}

/// Paints the chart data on the canvas.
pub fn paint_plot(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    series: &[Series],
    x_domains: &[(f64, f64)],
    y_domains: &[(f64, f64)],
    _cx: &mut App,
    state: &SharedPlotState,
) -> PaintStats {
    let _start = std::time::Instant::now();
    let width_px = bounds.size.width.as_f32();
    let height_px = bounds.size.height.as_f32();

    for series in series {
        let x_domain = x_domains
            .get(series.x_axis_id.0)
            .copied()
            .unwrap_or((0.0, 1.0));
        let x_scale = ChartScale::new_linear(x_domain, (0.0, width_px));

        let y_domain = y_domains
            .get(series.y_axis_id.0)
            .copied()
            .unwrap_or((0.0, 1.0));
        let y_scale = ChartScale::new_linear(y_domain, (height_px, 0.0));

        let transform = PlotTransform::new(x_scale, y_scale, bounds);
        series
            .plot
            .read()
            .render(window, &transform, &series.id, _cx, state);
    }

    PaintStats {}
}

#[derive(Clone)]
pub struct YAxisRenderInfo {
    pub domain: (f64, f64),
    pub scale: ChartScale,
    pub ticks: Vec<f64>,
    pub limits: (Option<f64>, Option<f64>),
}

// Paints the grid lines on the canvas.
pub fn paint_grid(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    x_domain: &AxisDomain,
    x_scale: &ChartScale,
    x_ticks: &[f64],
    primary_y_axis: &YAxisRenderInfo,
    theme: &crate::theme::ChartTheme,
) {
    let origin_x = bounds.origin.x.as_f32();
    let origin_y = bounds.origin.y.as_f32();

    let mut vertical_builder = PathBuilder::stroke(px(1.0));
    let mut has_vertical = false;

    let x_span = x_domain.x_max - x_domain.x_min;

    for tick_x in x_ticks {
        if let Some(l) = x_domain.x_min_limit {
            if *tick_x < l {
                continue;
            }
        }
        if let Some(l) = x_domain.x_max_limit {
            if *tick_x > l {
                continue;
            }
        }

        let x_pct = (*tick_x - x_domain.x_min) / x_span;
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
            window.paint_path(path, theme.grid_line);
        }
    }

    let mut horizontal_builder = PathBuilder::stroke(px(1.0));
    let mut has_horizontal = false;

    for tick_y in &primary_y_axis.ticks {
        if let Some(l) = primary_y_axis.limits.0 {
            if *tick_y < l {
                continue;
            }
        }
        if let Some(l) = primary_y_axis.limits.1 {
            if *tick_y > l {
                continue;
            }
        }

        let y_pct = (*tick_y - primary_y_axis.domain.1)
            / (primary_y_axis.domain.0 - primary_y_axis.domain.1);
        if (0.0..=1.0).contains(&y_pct) {
            let pixel_y = origin_y + primary_y_axis.scale.map(*tick_y);
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
            window.paint_path(path, theme.grid_line);
        }
    }
}

/// Helper to create a tag element on an axis.
pub fn create_axis_tag(
    text: String,
    position: Pixels,
    is_x_axis: bool,
    theme: &crate::theme::ChartTheme,
) -> gpui::AnyElement {
    if is_x_axis {
        div()
            .absolute()
            .left(position)
            .bottom(px(0.0))
            .ml(px(-40.0))
            .w(px(80.0))
            .h_full()
            .bg(theme.tag_background)
            .rounded_sm()
            .text_color(theme.tag_text)
            .text_size(px(12.0))
            .flex()
            .items_center()
            .justify_center()
            .child(text)
            .into_any_element()
    } else {
        div()
            .absolute()
            .top(position)
            .mt(px(-8.0))
            .h(px(16.0))
            .bg(theme.tag_background)
            .text_color(theme.tag_text)
            .text_size(px(12.0))
            .flex()
            .items_center()
            .justify_center()
            .child(text)
            .into_any_element()
    }
}
