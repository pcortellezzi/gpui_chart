// Bar plot implementation

use crate::data_types::{BarPlotConfig, PlotData, PlotPoint};
use gpui::*;

use super::PlotRenderer;

/// Bar plot type
#[derive(Clone)]
pub struct BarPlot {
    pub data: Vec<PlotPoint>,
    pub config: BarPlotConfig,
    pub baseline: f64,
}

impl BarPlot {
    pub fn new(data: Vec<PlotPoint>) -> Self {
        Self {
            data,
            config: BarPlotConfig::default(),
            baseline: 0.0,
        }
    }
}

impl PlotRenderer for BarPlot {
    fn render(
        &self,
        window: &mut Window,
        transform: &crate::transform::PlotTransform,
        _series_id: &str,
    ) {
        if self.data.is_empty() {
            return;
        }

        // Calculate minimum X spacing to determine max bar width
        let mut min_spacing = f64::INFINITY;
        if self.data.len() > 1 {
            for i in 0..self.data.len() - 1 {
                let spacing = (self.data[i + 1].x - self.data[i].x).abs();
                if spacing > f64::EPSILON && spacing < min_spacing {
                    min_spacing = spacing;
                }
            }
        }
        
        // If we only have one point or all points are same X, default to some width
        // Or if we can't determine, pick a reasonable default based on visible range?
        // Let's fallback to visible range / 10 if infinite.
        if min_spacing == f64::INFINITY {
             let _bounds = transform.bounds; // Assuming we could get data bounds from transform, but transform is screen <-> data
             // We can use transform X scale domain span roughly.
             // But simpler: just pick a fixed width in pixels and back-calculate?
             // No, let's stick to data coordinates.
             // If only 1 point, min_spacing is infinite. Let's arbitrarily say 1.0 unit.
             min_spacing = 1.0;
        }

        let bar_width_data = min_spacing * self.config.bar_width_pct as f64;
        let half_width = bar_width_data / 2.0;

        for point in &self.data {
            let x_left = point.x - half_width;
            let x_right = point.x + half_width;

            let p_top_left = transform.data_to_screen(Point::new(x_left, point.y));
            let p_bottom_right = transform.data_to_screen(Point::new(x_right, self.baseline));
            
            // Construct rect
            // p_top_left.y is the value Y
            // p_bottom_right.y is baseline Y
            // But screens Y grows downwards.
            // If value > baseline: top < bottom (screen coords)
            // If value < baseline: top > bottom
            
            let rect_x = p_top_left.x;
            let rect_w = p_bottom_right.x - p_top_left.x;
            
            let rect_y = p_top_left.y.min(p_bottom_right.y);
            let rect_h = (p_bottom_right.y - p_top_left.y).abs();

            let rect = Bounds::new(
                Point::new(rect_x, rect_y),
                Size::new(rect_w, rect_h),
            );

            // TODO: Optimize by checking if rect is visible
            
            window.paint_quad(fill(rect, self.config.color));
        }
    }

    fn add_data(&mut self, data: PlotData) {
        if let PlotData::Point(p) = data {
            self.data.push(p);
        }
    }

    fn set_data(&mut self, data: Vec<PlotData>) {
        self.data = data
            .into_iter()
            .filter_map(|d| if let PlotData::Point(p) = d { Some(p) } else { None })
            .collect();
    }

    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        if self.data.is_empty() {
            return None;
        }
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = self.baseline;
        let mut y_max = self.baseline;

        for p in &self.data {
            x_min = x_min.min(p.x);
            x_max = x_max.max(p.x);
            y_min = y_min.min(p.y);
            y_max = y_max.max(p.y);
        }
        Some((x_min, x_max, y_min, y_max))
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        let mut y_min = self.baseline;
        let mut y_max = self.baseline;
        let mut found = false;

        for p in &self.data {
            if p.x >= x_min && p.x <= x_max {
                y_min = y_min.min(p.y);
                y_max = y_max.max(p.y);
                found = true;
            }
        }

        if found { Some((y_min, y_max)) } else { None }
    }
}
