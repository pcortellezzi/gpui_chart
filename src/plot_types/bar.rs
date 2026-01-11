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
        _cx: &mut App,
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
        
        // Anti-aliasing logic: 
        // 1. If we are using aggregated data (LOD), we force 100% width to avoid switching artifacts.
        // 2. If gaps are too thin (< 1.2px), we remove them to avoid moire.
        let is_aggregated = spacing > self.source.suggested_x_spacing() * 1.1;
        let spacing_px = (transform.x_scale.map(x_min + spacing) - transform.x_scale.map(x_min)).abs();
        let gap_px = spacing_px * (1.0 - self.config.bar_width_pct);
        
        let effective_pct = if is_aggregated || gap_px < 1.2 { 1.0 } else { self.config.bar_width_pct as f64 };

        for point in points {
            // Edge-based snapping: calculate edges in data space, then snap both to pixels.
            // This ensures adjacent bars touch perfectly without gaps or 1px overlaps.
            let x_start_data = point.x - spacing / 2.0;
            let x_end_data = x_start_data + spacing * effective_pct;
            
            let px_start = transform.x_data_to_screen(x_start_data).as_f32().round();
            let px_end = transform.x_data_to_screen(x_end_data).as_f32().round();
            
            let rect_x = px_start;
            let rect_w = (px_end - px_start).max(1.0);
            
            // Optimization: Clip strictly outside
            if rect_x + rect_w < 0.0 || rect_x > transform.bounds.size.width.as_f32() { continue; }

            // Calculate Y
            let p_top_left = transform.data_to_screen(Point::new(point.x, point.y));
            let p_bottom_right = transform.data_to_screen(Point::new(point.x, self.baseline));
            
            let rect_y = p_top_left.y.min(p_bottom_right.y);
            let rect_h = (p_bottom_right.y - p_top_left.y).abs().max(px(1.0));

            let rect = Bounds::new(
                Point::new(px(rect_x), rect_y),
                Size::new(px(rect_w), rect_h),
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
