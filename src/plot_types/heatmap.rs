// Heatmap plot implementation

use crate::data_types::{PlotData, HeatmapCell};
use gpui::*;

use super::PlotRenderer;

/// Heatmap plot type
#[derive(Clone)]
pub struct HeatmapPlot {
    pub cells: Vec<HeatmapCell>,
}

impl HeatmapPlot {
    pub fn new(cells: Vec<HeatmapCell>) -> Self {
        Self { cells }
    }
}

impl PlotRenderer for HeatmapPlot {
    fn render(
        &self,
        window: &mut Window,
        transform: &crate::transform::PlotTransform,
        _series_id: &str,
    ) {
        let bounds = transform.bounds;
        let origin = bounds.origin;
        let size = bounds.size;

        for cell in &self.cells {
            // Determine cell bounds in screen coordinates
            // Assuming x,y are center or top-left?
            // "Heatmap" usually implies a grid.
            // Let's assume x,y are the center of the cell or the starting corner.
            // Standard: x,y is center. width/height are data units.
            
            let half_w = cell.width / 2.0;
            let half_h = cell.height / 2.0;

            let x_min = cell.x - half_w;
            let x_max = cell.x + half_w;
            let y_min = cell.y - half_h;
            let y_max = cell.y + half_h;

            let p1 = transform.data_to_screen(Point::new(x_min, y_max)); // Top-Left (Y grows down)
            let p2 = transform.data_to_screen(Point::new(x_max, y_min)); // Bottom-Right

            let rect = Bounds::from_corners(p1, p2);

            // Culling
            if rect.origin.x > origin.x + size.width || rect.origin.x + rect.size.width < origin.x ||
               rect.origin.y > origin.y + size.height || rect.origin.y + rect.size.height < origin.y {
                continue;
            }

            window.paint_quad(gpui::fill(rect, cell.color));
        }
    }

    fn add_data(&mut self, _data: PlotData) {
        // Not supported via standard add_data yet
    }

    fn set_data(&mut self, _data: Vec<PlotData>) {
        // Not supported
    }

    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        if self.cells.is_empty() {
            return None;
        }

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for cell in &self.cells {
            let half_w = cell.width / 2.0;
            let half_h = cell.height / 2.0;
            
            x_min = x_min.min(cell.x - half_w);
            x_max = x_max.max(cell.x + half_w);
            y_min = y_min.min(cell.y - half_h);
            y_max = y_max.max(cell.y + half_h);
        }

        Some((x_min, x_max, y_min, y_max))
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut found = false;

        for cell in &self.cells {
            let half_w = cell.width / 2.0;
            let c_xmin = cell.x - half_w;
            let c_xmax = cell.x + half_w;

            if c_xmax >= x_min && c_xmin <= x_max {
                let half_h = cell.height / 2.0;
                y_min = y_min.min(cell.y - half_h);
                y_max = y_max.max(cell.y + half_h);
                found = true;
            }
        }

        if found { Some((y_min, y_max)) } else { None }
    }
}
