use crate::data_types::{CandlestickConfig, Ohlcv, PlotData, PlotDataSource, VecDataSource};
use gpui::*;
use adabraka_ui::util::PixelsExt;
use crate::transform::PlotTransform;
use super::PlotRenderer;

/// Candlestick plot type
pub struct CandlestickPlot {
    pub source: Box<dyn PlotDataSource>,
    pub config: CandlestickConfig,
}

impl CandlestickPlot {
    pub fn new(data: Vec<Ohlcv>) -> Self {
        let plot_data = data.into_iter().map(PlotData::Ohlcv).collect();
        Self {
            source: Box::new(VecDataSource::new(plot_data)),
            config: CandlestickConfig::default(),
        }
    }
}

impl PlotRenderer for CandlestickPlot {
    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        self.source.get_bounds()
    }

    fn get_y_range(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        self.source.get_y_range(x_min, x_max)
    }

    fn render(
        &self,
        window: &mut Window,
        transform: &PlotTransform,
        _series_id: &str,
    ) {
        let bounds = transform.bounds;
        let width_px = bounds.size.width.as_f32();
        if width_px <= 0.0 { return; }

        let (x_min, x_max) = transform.x_scale.domain();
        let ms_per_px = (x_max - x_min) / width_px as f64;
        
        // Request aggregated data matching screen resolution
        let max_points = width_px as usize; 
        
        let up_color = self.config.up_body_color;
        let down_color = self.config.down_body_color;

        for data in self.source.iter_aggregated(x_min, x_max, max_points) {
            if let PlotData::Ohlcv(candle) = data {
                let is_up = candle.close >= candle.open;
                let t_start_px = transform.x_data_to_screen(candle.time).as_f32();
                // Use actual span from data, or fallback to ms_per_px if span is tiny (point data masquerading as candle)
                let span_px = (candle.span / ms_per_px).max(1.0);
                let width_px = span_px as f32;
                
                let center_x = t_start_px + width_px / 2.0;
                
                // Adaptive width: if candles are dense, fill the space. If sparse, use fixed pct.
                // Aggregated data is usually dense (packed bins).
                let b_w = width_px * self.config.body_width_pct;
                let w_w = (b_w * self.config.wick_width_pct).max(1.0); // Min 1px wick
                
                let y_h = transform.y_data_to_screen(candle.high).as_f32();
                let y_l = transform.y_data_to_screen(candle.low).as_f32();
                let y_o = transform.y_data_to_screen(candle.open).as_f32();
                let y_c = transform.y_data_to_screen(candle.close).as_f32();
                let (b_top, b_bot) = if is_up { (y_c, y_o) } else { (y_o, y_c) };
                
                let color = if is_up { up_color } else { down_color };
                
                // Wick
                window.paint_quad(fill(Bounds::new(Point::new(px(center_x - w_w/2.0), px(y_h)), Size::new(px(w_w), px(y_l - y_h).max(px(1.0)))), color));
                // Body
                window.paint_quad(fill(Bounds::new(Point::new(px(center_x - b_w/2.0), px(b_top)), Size::new(px(b_w), px((b_bot - b_top).max(1.0)))), color));
            }
        }
    }
}