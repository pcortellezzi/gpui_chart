// Step Line plot implementation

use crate::data_types::{PlotData, PlotPoint, StepLinePlotConfig, StepMode};
use gpui::*;

use super::PlotRenderer;

/// Step Line plot type
#[derive(Clone)]
pub struct StepLinePlot {
    pub data: Vec<PlotPoint>,
    pub config: StepLinePlotConfig,
}

impl StepLinePlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        Self {
            data,
            config: StepLinePlotConfig::default(),
        }
    }
}

impl PlotRenderer for StepLinePlot {
    fn render(
        &self,
        window: &mut Window,
        transform: &crate::transform::PlotTransform,
        _series_id: &str,
    ) {
        if self.data.len() < 2 {
            return;
        }

        let mut builder = PathBuilder::stroke(px(self.config.line_width));
        let mut first = true;

        for i in 0..self.data.len() {
            let p_curr = &self.data[i];
            let s_curr = transform.data_to_screen(Point::new(p_curr.x, p_curr.y));

            if first {
                builder.move_to(s_curr);
                first = false;
            } else {
                let p_prev = &self.data[i - 1];
                let s_prev = transform.data_to_screen(Point::new(p_prev.x, p_prev.y));

                match self.config.mode {
                    StepMode::Post => {
                        // Horizontal then Vertical
                        // (prev.x, prev.y) -> (curr.x, prev.y) -> (curr.x, curr.y)
                        // s_prev is already where we are.
                        let corner = Point::new(s_curr.x, s_prev.y);
                        builder.line_to(corner);
                        builder.line_to(s_curr);
                    }
                    StepMode::Pre => {
                        // Vertical then Horizontal
                        // (prev.x, prev.y) -> (prev.x, curr.y) -> (curr.x, curr.y)
                        let corner = Point::new(s_prev.x, s_curr.y);
                        builder.line_to(corner);
                        builder.line_to(s_curr);
                    }
                    StepMode::Mid => {
                        // Midpoint step
                        let mid_x_data = (p_prev.x + p_curr.x) / 2.0;
                        let mid_x_screen = transform.data_to_screen(Point::new(mid_x_data, 0.0)).x;
                        
                        let corner1 = Point::new(mid_x_screen, s_prev.y);
                        let corner2 = Point::new(mid_x_screen, s_curr.y);
                        
                        builder.line_to(corner1);
                        builder.line_to(corner2);
                        builder.line_to(s_curr);
                    }
                }
            }
        }

        if let Ok(path) = builder.build() {
            window.paint_path(path, self.config.color);
        }
    }

    fn add_data(&mut self, data: PlotData) {
        if let PlotData::Point(p) = data {
            self.data.push(p);
        }
    }

    fn set_data(&mut self, data: Vec<PlotData>) {
        self.data = data
            .into_iter()
            .filter_map(|d| if let PlotData::Point(p) = d { Some(p) } else { None })
            .collect();
    }

    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        if self.data.is_empty() {
            return None;
        }
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for p in &self.data {
            x_min = x_min.min(p.x);
            x_max = x_max.max(p.x);
            y_min = y_min.min(p.y);
            y_max = y_max.max(p.y);
        }
        Some((x_min, x_max, y_min, y_max))
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut found = false;

        for p in &self.data {
            if p.x >= x_min && p.x <= x_max {
                y_min = y_min.min(p.y);
                y_max = y_max.max(p.y);
                found = true;
            }
        }

        if found { Some((y_min, y_max)) } else { None }
    }
}
