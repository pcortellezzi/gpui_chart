use super::PlotRenderer;
use crate::data_types::{
    PlotData, PlotDataSource, PlotPoint, StepLinePlotConfig, StepMode, VecDataSource,
};
use crate::transform::PlotTransform;
use crate::utils::PixelsExt;
use gpui::*;

/// StepLine plot type
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
        _cx: &mut App,
    ) {
        let (x_min, x_max) = transform.x_scale.domain();
        let max_points = transform.bounds.size.width.as_f32() as usize * 2;
        let visible_iter = self.source.iter_aggregated(x_min, x_max, max_points);

        let mut builder = PathBuilder::stroke(px(self.config.line_width));
        let mut prev_pt: Option<Point<Pixels>> = None;

        for data in visible_iter {
            if let PlotData::Point(p_curr) = data {
                let s_curr = transform.data_to_screen(Point::new(p_curr.x, p_curr.y));

                if let Some(s_prev) = prev_pt {
                    match self.config.mode {
                        StepMode::Post => {
                            builder.line_to(Point::new(s_curr.x, s_prev.y));
                            builder.line_to(s_curr);
                        }
                        StepMode::Pre => {
                            builder.line_to(Point::new(s_prev.x, s_curr.y));
                            builder.line_to(s_curr);
                        }
                        StepMode::Mid => {
                            let mid_x = (s_prev.x + s_curr.x) / 2.0;
                            builder.line_to(Point::new(mid_x, s_prev.y));
                            builder.line_to(Point::new(mid_x, s_curr.y));
                            builder.line_to(s_curr);
                        }
                    }
                } else {
                    builder.move_to(s_curr);
                }
                prev_pt = Some(s_curr);
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
