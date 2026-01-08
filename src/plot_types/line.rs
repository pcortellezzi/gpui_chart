// Line plot implementation

use crate::data_types::{LinePlotConfig, PlotData, PlotPoint};
use gpui::*;

use super::PlotRenderer;

/// Line plot type
#[derive(Clone)]
pub struct LinePlot {
    pub data: Vec<PlotPoint>,
    pub config: LinePlotConfig,
}

impl LinePlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        Self {
            data,
            config: LinePlotConfig::default(),
        }
    }
}

impl PlotRenderer for LinePlot {
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

        for point in &self.data {
            let screen_point = transform.data_to_screen(Point::new(point.x, point.y));

            if first {
                builder.move_to(screen_point);
                first = false;
            } else {
                builder.line_to(screen_point);
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

    fn clear_before(&mut self, before: f64) {
        self.data.retain(|p| p.x >= before);
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
}
