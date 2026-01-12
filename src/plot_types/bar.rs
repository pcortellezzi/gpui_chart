use super::PlotRenderer;
use crate::data_types::{BarPlotConfig, PlotData, PlotDataSource, PlotPoint, VecDataSource};
use crate::transform::PlotTransform;
use crate::utils::PixelsExt;
use gpui::*;

/// Bar plot type
pub struct BarPlot {
    pub source: Box<dyn PlotDataSource>,
    pub config: BarPlotConfig,
    pub baseline: f64,
    buffer: parking_lot::Mutex<Vec<PlotData>>,
}

impl BarPlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        let plot_data = data.into_iter().map(PlotData::Point).collect();
        Self {
            source: Box::new(VecDataSource::new(plot_data)),
            config: BarPlotConfig::default(),
            baseline: 0.0,
            buffer: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn with_source(source: Box<dyn PlotDataSource>) -> Self {
        Self {
            source,
            config: BarPlotConfig::default(),
            baseline: 0.0,
            buffer: parking_lot::Mutex::new(Vec::new()),
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
        _state: &crate::data_types::SharedPlotState,
    ) {
        let (x_min, x_max) = transform.x_scale.domain();

        // Limit aggregated data to ~2000 points to prevent performance issues
        let screen_width = transform.bounds.size.width.as_f32() as usize;
        let max_points = screen_width.clamp(1, 2000); // Cap at 2000 for safety

        let mut buffer = self.buffer.lock();
        self.source.get_aggregated_data(x_min, x_max, max_points, &mut buffer);
        
        if buffer.is_empty() {
            return;
        }

        // Calculate dynamic spacing if we have enough points
        let first_x = if let Some(PlotData::Point(p)) = buffer.first() { p.x } else { 0.0 };
        let last_x = if let Some(PlotData::Point(p)) = buffer.last() { p.x } else { 0.0 };
        
        let effective_spacing = if buffer.len() > 1 {
            (last_x - first_x) / (buffer.len() - 1) as f64
        } else {
            self.source.suggested_x_spacing()
        };

        // Ensure we don't make bars WIDER than the original data would allow if zoomed in
        let spacing = effective_spacing.max(self.source.suggested_x_spacing());

        // Anti-aliasing logic:
        // 1. If we are using aggregated data (LOD), we force 100% width to avoid switching artifacts.
        // 2. If gaps are too thin (< 1.2px), we remove them to avoid moire.
        let is_aggregated = spacing > self.source.suggested_x_spacing() * 1.1;
        let spacing_px =
            (transform.x_scale.map(x_min + spacing) - transform.x_scale.map(x_min)).abs();
        let gap_px = spacing_px * (1.0 - self.config.bar_width_pct);

        let effective_pct = if is_aggregated || gap_px < 1.2 {
            1.0
        } else {
            self.config.bar_width_pct as f64
        };

        for data in buffer.iter() {
            if let PlotData::Point(point) = data {
                // Edge-based snapping: calculate edges in data space, then snap both to pixels.
                // This ensures adjacent bars touch perfectly without gaps or 1px overlaps.
                let x_start_data = point.x - spacing / 2.0;
                let x_end_data = x_start_data + spacing * effective_pct;

                let px_start = transform.x_data_to_screen(x_start_data).as_f32().round();
                let px_end = transform.x_data_to_screen(x_end_data).as_f32().round();

                let rect_x = px_start;
                let rect_w = (px_end - px_start).max(1.0);

                // Optimization: Clip strictly outside
                if rect_x + rect_w < 0.0 || rect_x > transform.bounds.size.width.as_f32() {
                    continue;
                }

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
    }

    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        self.source.get_bounds()
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        self.source.get_y_range(x_min, x_max)
    }
}