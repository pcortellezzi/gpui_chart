use crate::data_types::{BarPlotConfig, PlotData, PlotDataSource, VecDataSource, PlotPoint};
use gpui::*;
use crate::utils::PixelsExt;
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
        let (x_min, x_max) = transform.x_scale.domain();
        
        // Calculate max points based on screen width
        // A bar needs at least 1px (plus spacing), so screen_width is a hard upper bound for useful bars.
        let screen_width = transform.bounds.size.width.as_f32() as usize;
        let max_points = screen_width.max(1).min(2000); // Cap at 2000 for safety

        let visible_iter = self.source.iter_aggregated(x_min, x_max, max_points);

        // We need to estimate the width of the bars based on the density of data returned.
        // If we are aggregated, the spacing is larger than source.suggested_x_spacing().
        // Let's rely on the effective spacing of the returned data? 
        // Hard to do with a single pass iterator.
        // Heuristic: If we requested aggregation, assume we got roughly max_points or less.
        // Ideally the data source would tell us the "bin width".
        // Fallback: use the original spacing, but this might result in thin bars.
        // Better: Calculate local spacing.

        let mut points: Vec<PlotPoint> = Vec::with_capacity(max_points);
        for data in visible_iter {
            if let PlotData::Point(point) = data {
                points.push(point);
            }
        }

        if points.is_empty() { return; }

        // Calculate dynamic spacing if we have enough points
        let effective_spacing = if points.len() > 1 {
            (points.last().unwrap().x - points.first().unwrap().x) / (points.len() - 1) as f64
        } else {
            self.source.suggested_x_spacing()
        };
        
        // Ensure we don't make bars WIDER than the original data would allow if zoomed in
        let spacing = effective_spacing.max(self.source.suggested_x_spacing());
        let bar_width_data = spacing * self.config.bar_width_pct as f64;
        let half_width = bar_width_data / 2.0;

        for point in points {
            let x_left = point.x - half_width;
            let x_right = point.x + half_width;
            
            // Optimization: Clip strictly outside
            if x_right < x_min || x_left > x_max { continue; }

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
        }
    }


    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        self.source.get_bounds()
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        self.source.get_y_range(x_min, x_max)
    }
}
