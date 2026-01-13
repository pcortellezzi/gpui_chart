use super::PlotRenderer;
use crate::data_types::{CandlestickConfig, Ohlcv, PlotData, PlotDataSource, VecDataSource};
use crate::transform::PlotTransform;
use crate::utils::PixelsExt;
use gpui::*;

/// Candlestick plot type
pub struct CandlestickPlot {
    pub source: Box<dyn PlotDataSource>,
    pub config: CandlestickConfig,
    buffer: parking_lot::Mutex<Vec<PlotData>>,
}

impl CandlestickPlot {
    pub fn new(data: Vec<Ohlcv>) -> Self {
        let plot_data = data.into_iter().map(PlotData::Ohlcv).collect();
        Self {
            source: Box::new(VecDataSource::new(plot_data)),
            config: CandlestickConfig::default(),
            buffer: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn with_source(source: Box<dyn PlotDataSource>) -> Self {
        Self {
            source,
            config: CandlestickConfig::default(),
            buffer: parking_lot::Mutex::new(Vec::new()),
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
        _cx: &mut App,
        state: &crate::data_types::SharedPlotState,
    ) {
        let bounds = transform.bounds;
        let width_px = bounds.size.width.as_f32();
        if width_px <= 0.0 {
            return;
        }

        let (x_min, x_max) = transform.x_scale.domain();
        let ms_per_px = (x_max - x_min) / width_px as f64;

        // Request aggregated data matching screen resolution
        let max_points = width_px as usize;

        let up_color = self.config.up_body_color;
        let down_color = self.config.down_body_color;
        
        let theme = &state.theme;
        let body_pct = theme.candle_body_width_pct;
        let wick_pct = theme.candle_wick_width_pct;

        let mut buffer = self.buffer.lock();
        self.source.get_aggregated_data(x_min, x_max, max_points, &mut buffer);

        let count = buffer.len();
        let avg_px_per_point = if count > 0 {
             width_px / count as f32
        } else {
             width_px
        };

        for data in buffer.iter() {
            if let PlotData::Ohlcv(candle) = data {
                let is_up = candle.close >= candle.open;
                let t_start_px = transform.x_data_to_screen(candle.time).as_f32();
                
                // 1. Calculate base width
                // If candle has no span (raw point), we use avg spacing.
                let span = if candle.span > 0.0 { candle.span } else { self.source.suggested_x_spacing() };
                let theoretical_span_px = (span / ms_per_px) as f32;
                
                // 2. Clamp width to prevent giant candles over gaps (market closes)
                // We ensure it's at least 3px when zoomed in to see the body clearly.
                let width_px = theoretical_span_px.min(avg_px_per_point * 1.5).max(1.0);

                let center_x = t_start_px + (theoretical_span_px / 2.0); // Center on its theoretical slot

                // 3. Body and Wick calculation
                let b_w = (width_px * body_pct).max(1.0); // Min 1px body
                let w_w = (b_w * wick_pct).max(1.0); // Min 1px wick

                let y_h = transform.y_data_to_screen(candle.high).as_f32();
                let y_l = transform.y_data_to_screen(candle.low).as_f32();
                let y_o = transform.y_data_to_screen(candle.open).as_f32();
                let y_c = transform.y_data_to_screen(candle.close).as_f32();
                let (b_top, b_bot) = if is_up { (y_c, y_o) } else { (y_o, y_c) };

                let color = if is_up { up_color } else { down_color };

                // Wick (High to Low)
                window.paint_quad(fill(
                    Bounds::new(
                        Point::new(px(center_x - w_w / 2.0), px(y_h)),
                        Size::new(px(w_w), px((y_l - y_h).max(1.0))),
                    ),
                    color,
                ));
                // Body (Open to Close)
                window.paint_quad(fill(
                    Bounds::new(
                        Point::new(px(center_x - b_w / 2.0), px(b_top)),
                        Size::new(px(b_w), px((b_bot - b_top).max(1.0))),
                    ),
                    color,
                ));
            }
        }
    }
}

