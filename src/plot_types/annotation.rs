use super::PlotRenderer;
use crate::data_types::Annotation;
use crate::transform::PlotTransform;
use gpui::*;

/// Annotation plot type (Layer of annotations)
pub struct AnnotationPlot {
    pub annotations: Vec<Annotation>,
}

impl AnnotationPlot {
    pub fn new(annotations: Vec<Annotation>) -> Self {
        Self { annotations }
    }
}

impl PlotRenderer for AnnotationPlot {
    fn render(
        &self,
        window: &mut Window,
        transform: &PlotTransform,
        _series_id: &str,
        cx: &mut App,
        _state: &crate::data_types::SharedPlotState,
    ) {
        let bounds = transform.bounds;
        let origin = bounds.origin;
        let size = bounds.size;

        for annotation in &self.annotations {
            match annotation {
                Annotation::VLine {
                    x,
                    color,
                    width,
                    label,
                } => {
                    let is_inside = transform
                        .x_scale
                        .gap_index()
                        .map(|g| g.is_inside(*x as i64))
                        .unwrap_or(false);

                    if is_inside {
                        continue;
                    }

                    let screen_x = transform.x_data_to_screen(*x);

                    // Only draw if within (or close to) X bounds
                    if screen_x >= origin.x - px(*width)
                        && screen_x <= origin.x + size.width + px(*width)
                    {
                        let p1 = Point::new(screen_x, origin.y);
                        let p2 = Point::new(screen_x, origin.y + size.height);

                        let mut builder = PathBuilder::stroke(px(*width));
                        builder.move_to(p1);
                        builder.line_to(p2);

                        if let Ok(path) = builder.build() {
                            window.paint_path(path, *color);
                        }

                        if let Some(text) = label {
                            // Render label near the top
                            let font_size = px(10.0);
                            let run = TextRun {
                                len: text.len(),
                                font: TextStyle::default().font(),
                                color: *color,
                                background_color: None,
                                underline: None,
                                strikethrough: None,
                            };
                            if let Ok(lines) = window.text_system().shape_text(
                                text.clone().into(),
                                font_size,
                                &[run],
                                None,
                                None,
                            ) {
                                for line in lines {
                                    let _ = line.paint(
                                        p1 + point(px(2.0), px(2.0)),
                                        font_size,
                                        TextAlign::Left,
                                        None,
                                        window,
                                        cx,
                                    );
                                }
                            }
                        }
                    }
                }
                Annotation::HLine {
                    y,
                    color,
                    width,
                    label,
                } => {
                    let screen_y = transform.y_data_to_screen(*y);

                    if screen_y >= origin.y - px(*width)
                        && screen_y <= origin.y + size.height + px(*width)
                    {
                        let p1 = Point::new(origin.x, screen_y);
                        let p2 = Point::new(origin.x + size.width, screen_y);

                        let mut builder = PathBuilder::stroke(px(*width));
                        builder.move_to(p1);
                        builder.line_to(p2);

                        if let Ok(path) = builder.build() {
                            window.paint_path(path, *color);
                        }

                        if let Some(text) = label {
                            let font_size = px(10.0);
                            let run = TextRun {
                                len: text.len(),
                                font: TextStyle::default().font(),
                                color: *color,
                                background_color: None,
                                underline: None,
                                strikethrough: None,
                            };
                            if let Ok(lines) = window.text_system().shape_text(
                                text.clone().into(),
                                font_size,
                                &[run],
                                None,
                                None,
                            ) {
                                for line in lines {
                                    let _ = line.paint(
                                        p1 + point(px(2.0), px(-12.0)),
                                        font_size,
                                        TextAlign::Left,
                                        None,
                                        window,
                                        cx,
                                    );
                                }
                            }
                        }
                    }
                }
                Annotation::Rect {
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    color,
                    fill,
                } => {
                    let ranges = if let Some(gaps) = transform.x_scale.gap_index() {
                        gaps.split_range(*x_min as i64, *x_max as i64)
                    } else {
                        vec![(*x_min as i64, *x_max as i64)]
                    };

                    for (r_start, r_end) in ranges {
                        let p1 = transform.data_to_screen(Point::new(r_start as f64, *y_max));
                        let p2 = transform.data_to_screen(Point::new(r_end as f64, *y_min));

                        let rect = Bounds::from_corners(p1, p2);

                        if rect.size.width > px(0.0) {
                            if *fill {
                                window.paint_quad(gpui::fill(rect, *color));
                            } else {
                                window
                                    .paint_quad(gpui::outline(rect, *color, gpui::BorderStyle::Solid));
                            }
                        }
                    }
                }
                Annotation::Text {
                    x,
                    y,
                    text,
                    color,
                    font_size,
                } => {
                    let pos = transform.data_to_screen(Point::new(*x, *y));
                    let font_size_px = px(*font_size);
                    let run = TextRun {
                        len: text.len(),
                        font: TextStyle::default().font(),
                        color: *color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    if let Ok(lines) = window.text_system().shape_text(
                        text.clone().into(),
                        font_size_px,
                        &[run],
                        None,
                        None,
                    ) {
                        let mut origin = pos;
                        for line in lines {
                            let _ =
                                line.paint(origin, font_size_px, TextAlign::Left, None, window, cx);
                            origin.y += font_size_px;
                        }
                    }
                }
            }
        }
    }

    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        None
    }

    fn get_y_range(&self, _x_min: f64, _x_max: f64) -> Option<(f64, f64)> {
        None
    }
}
