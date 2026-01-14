use super::PlotRenderer;
use crate::data_types::{AreaPlotConfig, PlotData, PlotDataSource, PlotPoint, VecDataSource};
use crate::transform::PlotTransform;
use crate::utils::PixelsExt;
use gpui::*;

/// Area plot type
pub struct AreaPlot {
    pub source: Box<dyn PlotDataSource>,
    pub config: AreaPlotConfig,
    pub baseline: f64,
    buffer: parking_lot::Mutex<Vec<PlotData>>,
}

impl AreaPlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        let plot_data = data.into_iter().map(PlotData::Point).collect();
        Self {
            source: Box::new(VecDataSource::new(plot_data)),
            config: AreaPlotConfig::default(),
            baseline: 0.0,
            buffer: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn with_source(source: Box<dyn PlotDataSource>) -> Self {
        Self {
            source,
            config: AreaPlotConfig::default(),
            baseline: 0.0,
            buffer: parking_lot::Mutex::new(Vec::new()),
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
        transform: &PlotTransform,
        _series_id: &str,
        _cx: &mut App,
        state: &crate::data_types::SharedPlotState,
    ) {
        let (x_min, x_max) = transform.x_scale.domain();
        let max_points = transform.bounds.size.width.as_f32() as usize * 2;
        let baseline_y = transform.y_data_to_screen(self.baseline);

        let mut fill_builder = PathBuilder::fill();
        let mut line_builder = PathBuilder::stroke(px(self.config.line_width));

        let mut first_pt: Option<Point<Pixels>> = None;
        let mut last_pt: Option<Point<Pixels>> = None;

        let mut buffer = self.buffer.lock();
        self.source.get_aggregated_data(
            x_min,
            x_max,
            max_points,
            &mut buffer,
            state.gap_index.as_deref(),
        );

        for data in buffer.iter() {
            if let PlotData::Point(point) = data {
                let screen_point = transform.data_to_screen(Point::new(point.x, point.y));

                if first_pt.is_none() {
                    fill_builder.move_to(Point::new(screen_point.x, baseline_y));
                    fill_builder.line_to(screen_point);
                    line_builder.move_to(screen_point);
                    first_pt = Some(screen_point);
                } else {
                    fill_builder.line_to(screen_point);
                    line_builder.line_to(screen_point);
                }
                last_pt = Some(screen_point);
            }
        }

        if let (Some(first), Some(last)) = (first_pt, last_pt) {
            fill_builder.line_to(Point::new(last.x, baseline_y));
            fill_builder.line_to(Point::new(first.x, baseline_y));
            fill_builder.close();

            if let Ok(path) = fill_builder.build() {
                window.paint_path(path, self.config.fill_color);
            }
            if let Ok(path) = line_builder.build() {
                window.paint_path(path, self.config.line_color);
            }
        }
    }

    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        self.source.get_bounds()
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        self.source.get_y_range(x_min, x_max)
    }
}
