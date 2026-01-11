use super::PlotRenderer;
use crate::data_types::{LinePlotConfig, PlotData, PlotDataSource, PlotPoint, VecDataSource};
use crate::transform::PlotTransform;
use crate::utils::PixelsExt;
use gpui::*;

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
        _cx: &mut App,
        _state: &crate::data_types::SharedPlotState,
    ) {
        let (x_min, x_max) = transform.x_scale.domain();
        let max_points = transform.bounds.size.width.as_f32() as usize * 2;

        let mut first = true;
        let mut builder = PathBuilder::stroke(px(self.config.line_width));

        for data in self.source.iter_aggregated(x_min, x_max, max_points) {
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
