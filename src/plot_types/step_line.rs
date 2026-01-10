use crate::data_types::{PlotData, PlotDataSource, StepLinePlotConfig, StepMode, VecDataSource, PlotPoint};
use gpui::*;
use crate::transform::PlotTransform;
use super::PlotRenderer;

/// Step Line plot type
pub struct StepLinePlot {
    pub source: Box<dyn PlotDataSource>,
    pub config: StepLinePlotConfig,
}

impl StepLinePlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        let plot_data = data.into_iter().map(PlotData::Point).collect();
        Self {
            source: Box::new(VecDataSource::new(plot_data)),
            config: StepLinePlotConfig::default(),
        }
    }

    pub fn with_source(source: Box<dyn PlotDataSource>) -> Self {
        Self {
            source,
            config: StepLinePlotConfig::default(),
        }
    }
}

impl PlotRenderer for StepLinePlot {
    fn render(
        &self,
        window: &mut Window,
        transform: &PlotTransform,
        _series_id: &str,
    ) {
        let (x_min, x_max) = transform.x_scale.domain();
        let visible_iter = self.source.iter_range(x_min, x_max);
        let visible_data: Vec<_> = visible_iter.collect();

        if visible_data.len() < 2 {
            return;
        }

        let mut builder = PathBuilder::stroke(px(self.config.line_width));
        let mut first = true;

        for i in 0..visible_data.len() {
            if let PlotData::Point(p_curr) = &visible_data[i] {
                let s_curr = transform.data_to_screen(Point::new(p_curr.x, p_curr.y));

                if first {
                    builder.move_to(s_curr);
                    first = false;
                } else {
                    if let PlotData::Point(p_prev) = &visible_data[i - 1] {
                        let s_prev = transform.data_to_screen(Point::new(p_prev.x, p_prev.y));

                        match self.config.mode {
                            StepMode::Post => {
                                let corner = Point::new(s_curr.x, s_prev.y);
                                builder.line_to(corner);
                                builder.line_to(s_curr);
                            }
                            StepMode::Pre => {
                                let corner = Point::new(s_prev.x, s_curr.y);
                                builder.line_to(corner);
                                builder.line_to(s_curr);
                            }
                            StepMode::Mid => {
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
