use crate::data_types::{AxisEdge, AxisFormat, AxisRange};
use crate::gaps::GapIndex;
use crate::scales::ChartScale;
use crate::theme::ChartTheme;
use crate::utils::PixelsExt;
use gpui::prelude::FluentBuilder;
use gpui::*;
use std::sync::Arc;

pub struct AxisRenderer;

impl AxisRenderer {
    fn paint_axis(
        range: &mut AxisRange,
        is_vertical: bool,
        edge: AxisEdge,
        theme: &ChartTheme,
        label: &str,
        format: &AxisFormat,
        min_label_spacing: Pixels,
        gaps: Option<&GapIndex>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
        on_draw: &impl Fn(Bounds<Pixels>),
    ) {
        on_draw(bounds);

        let (min, max) = range.clamped_bounds();
        let scale_range = if is_vertical {
            (bounds.size.height.as_f32(), 0.0)
        } else {
            (0.0, bounds.size.width.as_f32())
        };

        let mut scale = ChartScale::new_linear((min, max), scale_range);
        if let Some(g) = gaps {
            scale = scale.with_gaps(Some(Arc::new(g.clone())));
        }

        let max_px = if is_vertical {
            bounds.size.height.as_f32()
        } else {
            bounds.size.width.as_f32()
        };

        // Dynamic density calculation
        let margin = min_label_spacing.as_f32();
        let label_size_est = match format {
            AxisFormat::Time(_) => 80.0 + margin,
            AxisFormat::Numeric => 50.0 + margin,
        };
        let count = (max_px / label_size_est).floor() as usize;
        let count = count.clamp(2, 20);

        let ticks = range.ticks(count, gaps);

        // 1. Axis Border Line
        // ... (existing code for border line)
        let mut line_builder = PathBuilder::stroke(px(1.0));
        if is_vertical {
            let x = if edge == AxisEdge::Left {
                bounds.origin.x + bounds.size.width
            } else {
                bounds.origin.x
            };
            line_builder.move_to(point(x, bounds.origin.y));
            line_builder.line_to(point(x, bounds.origin.y + bounds.size.height));
        } else {
            let y = if edge == AxisEdge::Top {
                bounds.origin.y + bounds.size.height
            } else {
                bounds.origin.y
            };
            line_builder.move_to(point(bounds.origin.x, y));
            line_builder.line_to(point(bounds.origin.x + bounds.size.width, y));
        }
        if let Ok(path) = line_builder.build() {
            window.paint_path(path, theme.axis_line);
        }

        // 2. Labels
        let font_size = theme.axis_label_size;
        let font = TextStyle::default().font();

        for tick in ticks {
            let tick_px = scale.map(*tick) as f32;
            let tick_text = scale.format_tick(*tick, format);

            let run = TextRun {
                len: tick_text.len(),
                font: font.clone(),
                color: theme.axis_label,
                background_color: None,
                underline: None,
                strikethrough: None,
            };

            if let Ok(lines) =
                window
                    .text_system()
                    .shape_text(tick_text.into(), font_size, &[run], None, None)
            {
                for line in lines {
                    let origin = if is_vertical {
                        let y_centered = px(tick_px) - font_size / 2.0;
                        let line_width = line.width();
                        // Center horizontally
                        let x_text = (bounds.size.width - line_width) / 2.0;
                        bounds.origin + point(x_text, y_centered)
                    } else {
                        // Horizontal: Center on tick horizontally
                        let line_width = line.width();
                        let x_centered = px(tick_px) - line_width / 2.0;
                        // Center vertically
                        let y_text = (bounds.size.height - font_size) / 2.0;
                        bounds.origin + point(x_centered, y_text)
                    };

                    let _ =
                        line.paint(origin, font_size, TextAlign::Left, Some(bounds), window, cx);
                }
            }
        }

        // 3. Axis Title
        if !label.is_empty() {
            let title_font_size = px(9.0); // Y axis small
            let title_font_size_x = px(10.0); // X axis slightly larger
            let effective_size = if is_vertical {
                title_font_size
            } else {
                title_font_size_x
            };

            let title_run = TextRun {
                len: label.len(),
                font: font.clone(),
                color: theme.accent.opacity(if is_vertical { 0.5 } else { 1.0 }),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            if let Ok(lines) = window.text_system().shape_text(
                label.to_string().into(),
                effective_size,
                &[title_run],
                None,
                None,
            ) {
                let origin = if is_vertical {
                    bounds.origin + point(px(0.0), bounds.size.height - px(12.0))
                } else {
                    bounds.origin + point(px(8.0), px(0.0))
                };

                // Y: TextAlign::Center. X: TextAlign::Left.
                let align = if is_vertical {
                    TextAlign::Center
                } else {
                    TextAlign::Left
                };

                for line in lines {
                    let _ = line.paint(origin, effective_size, align, Some(bounds), window, cx);
                }
            }
        }
    }

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

        format: AxisFormat,

        min_label_spacing: Pixels,

        theme: &ChartTheme,

        gaps: Option<Arc<GapIndex>>,

        on_draw: impl Fn(Bounds<Pixels>) + 'static,
    ) -> Stateful<Div> {
        let is_left = edge == AxisEdge::Left;

        let mut range = range.clone();

        let theme = theme.clone();

        div()
            .id(SharedString::from(format!(
                "y-axis-{}-{}",
                pane_idx, axis_idx
            )))
            .absolute()
            .top(relative(current_top_pct))
            .h(relative(h_pct))
            .w(size)
            .when(is_left, |d| d.left(x_pos))
            .when(!is_left, |d| d.right(x_pos))
            .cursor(CursorStyle::ResizeUpDown)
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, (), window: &mut Window, cx| {
                        Self::paint_axis(
                            &mut range,
                            true,
                            edge,
                            &theme,
                            &label,
                            &format,
                            min_label_spacing,
                            gaps.as_deref(),
                            bounds,
                            window,
                            cx,
                            &on_draw,
                        );
                    },
                )
                .size_full(),
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

        format: AxisFormat,

        min_label_spacing: Pixels,

        theme: &ChartTheme,

        gaps: Option<Arc<GapIndex>>,

        on_draw: impl Fn(Bounds<Pixels>) + 'static,
    ) -> Stateful<Div> {
        let is_top = edge == AxisEdge::Top;

        let mut range = range.clone();

        let theme = theme.clone();

        div()
            .id(SharedString::from(format!("x-axis-{}", axis_idx)))
            .absolute()
            .left_0()
            .right_0()
            .h(size)
            .bg(theme.background)
            .when(is_top, |d| d.top(px(0.0)).border_b_1())
            .when(!is_top, |d| d.bottom(px(0.0)).border_t_1())
            .border_color(theme.axis_line)
            .cursor(CursorStyle::ResizeLeftRight)
            .flex()
            .flex_row()
            .child(div().w(gutter_left))
            .child(
                div().flex_1().h_full().child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, (), window: &mut Window, cx| {
                            Self::paint_axis(
                                &mut range,
                                false,
                                edge,
                                &theme,
                                &label,
                                &format,
                                min_label_spacing,
                                gaps.as_deref(),
                                bounds,
                                window,
                                cx,
                                &on_draw,
                            );
                        },
                    )
                    .size_full(),
                ),
            )
            .child(div().w(gutter_right))
    }
}
