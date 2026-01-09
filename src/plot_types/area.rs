// Area plot implementation

use crate::data_types::{AreaPlotConfig, PlotData, PlotPoint};
use gpui::*;

use super::PlotRenderer;

/// Area plot type
#[derive(Clone)]
pub struct AreaPlot {
    pub data: Vec<PlotPoint>,
    pub config: AreaPlotConfig,
    pub baseline: f64, // Y value to fill down to, default 0.0
}

impl AreaPlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        Self {
            data,
            config: AreaPlotConfig::default(),
            baseline: 0.0,
        }
    }

    pub fn with_baseline(mut self, baseline: f64) -> Self {
        self.baseline = baseline;
        self
    }
}

impl PlotRenderer for AreaPlot {
    fn render(
        &self,
        window: &mut Window,
        transform: &crate::transform::PlotTransform,
        _series_id: &str,
    ) {
        if self.data.len() < 2 {
            return;
        }

        // Render Fill
        let mut fill_builder = PathBuilder::fill();
        let mut first = true;
        
        let baseline_y = transform.data_to_screen(Point::new(self.data[0].x, self.baseline)).y;

        for point in &self.data {
            let screen_point = transform.data_to_screen(Point::new(point.x, point.y));

            if first {
                fill_builder.move_to(Point::new(screen_point.x, baseline_y));
                fill_builder.line_to(screen_point);
                first = false;
            } else {
                fill_builder.line_to(screen_point);
            }
        }
        
        // Close the path for filling
        if let Some(last) = self.data.last() {
             let last_screen = transform.data_to_screen(Point::new(last.x, last.y));
             fill_builder.line_to(Point::new(last_screen.x, baseline_y));
             fill_builder.line_to(Point::new(transform.data_to_screen(Point::new(self.data[0].x, self.baseline)).x, baseline_y));
        }

        if let Ok(path) = fill_builder.build() {
            window.paint_path(path, self.config.fill_color);
        }

        // Render Line
        let mut line_builder = PathBuilder::stroke(px(self.config.line_width));
        first = true;

        for point in &self.data {
            let screen_point = transform.data_to_screen(Point::new(point.x, point.y));

            if first {
                line_builder.move_to(screen_point);
                first = false;
            } else {
                line_builder.line_to(screen_point);
            }
        }

        if let Ok(path) = line_builder.build() {
            window.paint_path(path, self.config.line_color);
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
        let mut y_min = self.baseline;
        let mut y_max = self.baseline;

        for p in &self.data {
            x_min = x_min.min(p.x);
            x_max = x_max.max(p.x);
            y_min = y_min.min(p.y);
            y_max = y_max.max(p.y);
        }
        Some((x_min, x_max, y_min, y_max))
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        let mut y_min = self.baseline;
        let mut y_max = self.baseline;
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
