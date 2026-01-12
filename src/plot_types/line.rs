use super::PlotRenderer;
use crate::data_types::{LinePlotConfig, PlotData, PlotDataSource, PlotPoint, VecDataSource};
use crate::transform::PlotTransform;
use crate::utils::PixelsExt;
use crate::simd::batch_transform_points;
use gpui::*;

/// Line plot type
pub struct LinePlot {
    pub source: Box<dyn PlotDataSource>,
    pub config: LinePlotConfig,
    buffer: parking_lot::Mutex<Vec<PlotData>>,
    screen_buffer: parking_lot::Mutex<Vec<Point<Pixels>>>,
}

impl LinePlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        let plot_data = data.into_iter().map(PlotData::Point).collect();
        Self {
            source: Box::new(VecDataSource::new(plot_data)),
            config: LinePlotConfig::default(),
            buffer: parking_lot::Mutex::new(Vec::new()),
            screen_buffer: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn with_source(source: Box<dyn PlotDataSource>) -> Self {
        Self {
            source,
            config: LinePlotConfig::default(),
            buffer: parking_lot::Mutex::new(Vec::new()),
            screen_buffer: parking_lot::Mutex::new(Vec::new()),
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

        let mut buffer = self.buffer.lock();
        self.source.get_aggregated_data(x_min, x_max, max_points, &mut buffer);

        let mut screen_buffer = self.screen_buffer.lock();
        let (xm, xc, ym, yc) = transform.get_scale_coefficients();
        batch_transform_points(&buffer, xm, xc, ym, yc, &mut screen_buffer);

        let mut first = true;
        let mut builder = PathBuilder::stroke(px(self.config.line_width));

        for pt in screen_buffer.iter() {
            if first {
                builder.move_to(*pt);
                first = false;
            } else {
                builder.line_to(*pt);
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
