use crate::data_types::{StepLinePlotConfig, PlotData, PlotDataSource, VecDataSource, StepMode, PlotPoint};
use gpui::*;
use adabraka_ui::util::PixelsExt;
use crate::transform::PlotTransform;
use super::PlotRenderer;

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
    ) {
        let (x_min, x_max) = transform.x_scale.domain();
        let visible_iter = self.source.iter_range(x_min, x_max);

        let mut builder = PathBuilder::stroke(px(self.config.line_width));
        let mut prev_pt: Option<Point<Pixels>> = None;
        let mut last_px_x = f64::MIN;
        let mut first = true;

        for data in visible_iter {
            if let PlotData::Point(p_curr) = data {
                let s_curr = transform.data_to_screen(Point::new(p_curr.x, p_curr.y));
                let px_x = s_curr.x.as_f64();

                // Decimation: only process if we moved at least half a pixel
                if first || (px_x - last_px_x).abs() >= 0.5 {
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
                    last_px_x = px_x;
                    first = false;
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
