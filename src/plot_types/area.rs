use crate::data_types::{AreaPlotConfig, PlotData, PlotDataSource, VecDataSource, PlotPoint};
use gpui::*;
use crate::transform::PlotTransform;
use super::PlotRenderer;

/// Area plot type
pub struct AreaPlot {
    pub source: Box<dyn PlotDataSource>,
    pub config: AreaPlotConfig,
    pub baseline: f64, // Y value to fill down to, default 0.0
}

impl AreaPlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        let plot_data = data.into_iter().map(PlotData::Point).collect();
        Self {
            source: Box::new(VecDataSource::new(plot_data)),
            config: AreaPlotConfig::default(),
            baseline: 0.0,
        }
    }

    pub fn with_source(source: Box<dyn PlotDataSource>) -> Self {
        Self {
            source,
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
        transform: &PlotTransform,
        _series_id: &str,
    ) {
        let (x_min, x_max) = transform.x_scale.domain();
        let visible_iter = self.source.iter_range(x_min, x_max);
        let visible_data: Vec<_> = visible_iter.collect();

        if visible_data.is_empty() {
            return;
        }

        // Render Fill
        let mut fill_builder = PathBuilder::fill();
        let mut first = true;
        
        for data in &visible_data {
            if let PlotData::Point(point) = data {
                let screen_point = transform.data_to_screen(Point::new(point.x, point.y));
                let baseline_y = transform.y_data_to_screen(self.baseline);

                if first {
                    fill_builder.move_to(Point::new(screen_point.x, baseline_y));
                    fill_builder.line_to(screen_point);
                    first = false;
                } else {
                    fill_builder.line_to(screen_point);
                }
            }
        }
        
        // Close the path for filling
        if let Some(PlotData::Point(last)) = visible_data.last() {
             let last_screen_x = transform.x_data_to_screen(last.x);
             if let Some(PlotData::Point(first_pt)) = visible_data.first() {
                let first_screen_x = transform.x_data_to_screen(first_pt.x);
                let baseline_y = transform.y_data_to_screen(self.baseline);
                
                fill_builder.line_to(Point::new(last_screen_x, baseline_y));
                fill_builder.line_to(Point::new(first_screen_x, baseline_y));
                fill_builder.close();
             }
        }

        if let Ok(path) = fill_builder.build() {
            window.paint_path(path, self.config.fill_color);
        }

        // Render Line
        let mut line_builder = PathBuilder::stroke(px(self.config.line_width));
        let mut first = true;

        for data in &visible_data {
            if let PlotData::Point(point) = data {
                let screen_point = transform.data_to_screen(Point::new(point.x, point.y));

                if first {
                    line_builder.move_to(screen_point);
                    first = false;
                } else {
                    line_builder.line_to(screen_point);
                }
            }
        }

        if let Ok(path) = line_builder.build() {
            window.paint_path(path, self.config.line_color);
        }
    }

    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        self.source.get_bounds()
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        self.source.get_y_range(x_min, x_max)
    }
}
