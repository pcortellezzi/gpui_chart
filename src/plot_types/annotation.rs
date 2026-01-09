// Annotation layer implementation

use crate::data_types::{Annotation, PlotData};
use gpui::*;

use super::PlotRenderer;

/// Annotation plot type (Layer of annotations)
#[derive(Clone)]
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
        transform: &crate::transform::PlotTransform,
        _series_id: &str,
    ) {
        let bounds = transform.bounds;
        let origin = bounds.origin;
        let size = bounds.size;

        for annotation in &self.annotations {
            match annotation {
                Annotation::VLine { x, color, width, label } => {
                    let screen_x = transform.x_data_to_screen(*x);
                    
                    // Only draw if within (or close to) X bounds
                    if screen_x >= origin.x - px(*width) && screen_x <= origin.x + size.width + px(*width) {
                        let p1 = Point::new(screen_x, origin.y);
                        let p2 = Point::new(screen_x, origin.y + size.height);
                        
                        let mut builder = PathBuilder::stroke(px(*width));
                        builder.move_to(p1);
                        builder.line_to(p2);
                        
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, *color);
                        }

                        if let Some(_text) = label {
                             // Simple label at top
                             // window.paint_string(...) // GPUI doesn't expose simple paint_string easily in 0.2?
                             // We might need to use `TextSystem`. 
                             // For now, let's skip label rendering or use a placeholder if possible.
                             // GPUI 0.2.2 usually requires TextLayout.
                             // Let's assume for now we just draw the line. Labels are complex in raw paint.
                        }
                    }
                }
                Annotation::HLine { y, color, width, label: _ } => {
                     let screen_y = transform.y_data_to_screen(*y);

                     if screen_y >= origin.y - px(*width) && screen_y <= origin.y + size.height + px(*width) {
                        let p1 = Point::new(origin.x, screen_y);
                        let p2 = Point::new(origin.x + size.width, screen_y);

                        let mut builder = PathBuilder::stroke(px(*width));
                        builder.move_to(p1);
                        builder.line_to(p2);

                        if let Ok(path) = builder.build() {
                            window.paint_path(path, *color);
                        }
                     }
                }
                Annotation::Rect { x_min, x_max, y_min, y_max, color, fill } => {
                    let p1 = transform.data_to_screen(Point::new(*x_min, *y_max)); // Top-Left (since Y grows down, max Y is top)
                    let p2 = transform.data_to_screen(Point::new(*x_max, *y_min)); // Bottom-Right

                    let rect = Bounds::from_corners(p1, p2);
                    
                    if *fill {
                        window.paint_quad(gpui::fill(rect, *color));
                    } else {
                        window.paint_quad(gpui::outline(rect, *color, gpui::BorderStyle::Solid));
                    }
                }
                Annotation::Text { x, y, text: _, color: _, font_size: _ } => {
                    let _pos = transform.data_to_screen(Point::new(*x, *y));
                    // Text rendering in raw paint context is tricky without access to ViewContext/TextSystem.
                    // PlotRenderer only gets &mut Window.
                    // Window has `text_system()`.
                    // But we usually render text via Elements.
                    // However, we are inside `paint_plot` which uses `canvas`.
                    // We can't easily spawn elements here.
                    // We need to use `window.text_system()` to shape and draw.
                    // This is advanced usage. For now, let's skip Text implementation or leave a TODO.
                    // TODO: Implement text rendering
                }
            }
        }
    }

    fn add_data(&mut self, _data: PlotData) {
        // Annotations are typically added via set_data or constructor
    }

    fn set_data(&mut self, _data: Vec<PlotData>) {
        // Not used for annotations usually, unless we wrap Annotation in PlotData?
        // PlotData currently supports Point and Ohlcv.
        // We'll leave this empty for now.
    }

    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        if self.annotations.is_empty() {
            return None;
        }

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for ann in &self.annotations {
            match ann {
                Annotation::VLine { x, .. } => {
                    x_min = x_min.min(*x);
                    x_max = x_max.max(*x);
                }
                Annotation::HLine { y, .. } => {
                    y_min = y_min.min(*y);
                    y_max = y_max.max(*y);
                }
                Annotation::Rect { x_min: x1, x_max: x2, y_min: y1, y_max: y2, .. } => {
                    x_min = x_min.min(*x1).min(*x2);
                    x_max = x_max.max(*x1).max(*x2);
                    y_min = y_min.min(*y1).min(*y2);
                    y_max = y_max.max(*y1).max(*y2);
                }
                Annotation::Text { x, y, .. } => {
                    x_min = x_min.min(*x);
                    x_max = x_max.max(*x);
                    y_min = y_min.min(*y);
                    y_max = y_max.max(*y);
                }
            }
        }

        Some((x_min, x_max, y_min, y_max))
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut found = false;

        for ann in &self.annotations {
            match ann {
                Annotation::VLine { .. } => {}
                Annotation::HLine { y, .. } => {
                    y_min = y_min.min(*y);
                    y_max = y_max.max(*y);
                    found = true;
                }
                Annotation::Rect { x_min: x1, x_max: x2, y_min: y1, y_max: y2, .. } => {
                    if (*x1 >= x_min && *x1 <= x_max) || (*x2 >= x_min && *x2 <= x_max) || (*x1 <= x_min && *x2 >= x_max) {
                        y_min = y_min.min(*y1).min(*y2);
                        y_max = y_max.max(*y1).max(*y2);
                        found = true;
                    }
                }
                Annotation::Text { x, y, .. } => {
                    if *x >= x_min && *x <= x_max {
                        y_min = y_min.min(*y);
                        y_max = y_max.max(*y);
                        found = true;
                    }
                }
            }
        }

        if found { Some((y_min, y_max)) } else { None }
    }
}
