use crate::data_types::{BarPlotConfig, PlotData, PlotDataSource, VecDataSource, PlotPoint};
use gpui::*;
use adabraka_ui::util::PixelsExt;
use crate::transform::PlotTransform;
use super::PlotRenderer;

/// Bar plot type
pub struct BarPlot {
    pub source: Box<dyn PlotDataSource>,
    pub config: BarPlotConfig,
    pub baseline: f64,
}

impl BarPlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        let plot_data = data.into_iter().map(PlotData::Point).collect();
        Self {
            source: Box::new(VecDataSource::new(plot_data)),
            config: BarPlotConfig::default(),
            baseline: 0.0,
        }
    }

    pub fn with_source(source: Box<dyn PlotDataSource>) -> Self {
        Self {
            source,
            config: BarPlotConfig::default(),
            baseline: 0.0,
        }
    }
}

impl PlotRenderer for BarPlot {
    fn render(
        &self,
        window: &mut Window,
        transform: &PlotTransform,
        _series_id: &str,
    ) {
        let min_spacing = self.source.suggested_x_spacing();
        let bar_width_data = min_spacing * self.config.bar_width_pct as f64;
        let half_width = bar_width_data / 2.0;

        let (x_min, x_max) = transform.x_scale.domain();
        let visible_iter = self.source.iter_range(x_min - half_width, x_max + half_width);

        let mut last_px_x = f64::MIN;

        for data in visible_iter {
            if let PlotData::Point(point) = data {
                let p_center = transform.data_to_screen(Point::new(point.x, point.y));
                let px_x = p_center.x.as_f64();

                // Only render if we moved at least 1 pixel
                if (px_x - last_px_x).abs() < 1.0 {
                    continue;
                }

                let x_left = point.x - half_width;
                let x_right = point.x + half_width;

                let p_top_left = transform.data_to_screen(Point::new(x_left, point.y));
                let p_bottom_right = transform.data_to_screen(Point::new(x_right, self.baseline));
                
                let rect_x = p_top_left.x;
                let rect_w = (p_bottom_right.x - p_top_left.x).max(px(1.0));
                
                let rect_y = p_top_left.y.min(p_bottom_right.y);
                let rect_h = (p_bottom_right.y - p_top_left.y).abs().max(px(1.0));

                let rect = Bounds::new(
                    Point::new(rect_x, rect_y),
                    Size::new(rect_w, rect_h),
                );

                window.paint_quad(fill(rect, self.config.color));
                last_px_x = px_x;
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
