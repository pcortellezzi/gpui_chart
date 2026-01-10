use crate::data_types::{Annotation};
use gpui::*;
use crate::transform::PlotTransform;
use super::PlotRenderer;

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
