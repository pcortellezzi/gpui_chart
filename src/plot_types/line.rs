use crate::data_types::{LinePlotConfig, PlotData, PlotDataSource, VecDataSource, PlotPoint};
use gpui::*;
use crate::transform::PlotTransform;
use super::PlotRenderer;

/// Line plot type
pub struct LinePlot {
    pub source: Box<dyn PlotDataSource>,
    pub config: LinePlotConfig,
}

impl LinePlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        let plot_data = data.into_iter().map(PlotData::Point).collect();
        Self {
            source: Box::new(VecDataSource::new(plot_data)),
            config: LinePlotConfig::default(),
        }
    }

    pub fn with_source(source: Box<dyn PlotDataSource>) -> Self {
        Self {
            source,
            config: LinePlotConfig::default(),
        }
    }
}

impl PlotRenderer for LinePlot {
    fn render(
        &self,
        window: &mut Window,
        transform: &PlotTransform,
        _series_id: &str,
    ) {
        let (x_min, x_max) = transform.x_scale.domain();
        let visible_iter = self.source.iter_range(x_min, x_max);

        let mut builder = PathBuilder::stroke(px(self.config.line_width));
        let mut first = true;

        for data in visible_iter {
            if let PlotData::Point(point) = data {
                let screen_point = transform.data_to_screen(Point::new(point.x, point.y));
                if first {
                    builder.move_to(screen_point);
                    first = false;
                } else {
                    builder.line_to(screen_point);
                }
            }
        }

        if let Ok(path) = builder.build() {
            window.paint_path(path, self.config.color);
        }
    }

    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        self.source.get_bounds()
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        self.source.get_y_range(x_min, x_max)
    }
}
