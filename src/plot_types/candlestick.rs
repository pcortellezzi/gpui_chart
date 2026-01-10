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

    fn draw_quad_bucket(
        &self,
        window: &mut Window,
        transform: &PlotTransform,
        origin_x: f32,
        px_idx: i32, 
        o: f64, h: f64, l: f64, c: f64
    ) {
        let is_up = c >= o;
        let x_center = origin_x + px_idx as f32;
        let y_h = transform.y_data_to_screen(h).as_f32();
        let y_l = transform.y_data_to_screen(l).as_f32();
        let y_o = transform.y_data_to_screen(o).as_f32();
        let y_c = transform.y_data_to_screen(c).as_f32();
        let (body_top, body_bottom) = if is_up { (y_c, y_o) } else { (y_o, y_c) };
        
        let color = if is_up { self.config.up_body_color } else { self.config.down_body_color };
        let half = 0.5;

        // One-pixel wide candle for high density
        window.paint_quad(fill(Bounds::new(Point::new(px(x_center - half), px(y_h)), Size::new(px(1.0), px(y_l - y_h))), color));
        let body_h = (body_bottom - body_top).max(1.0); 
        window.paint_quad(fill(Bounds::new(Point::new(px(x_center - half), px(body_top)), Size::new(px(1.0), px(body_h))), color));
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
        let origin_x = bounds.origin.x.as_f32();
        let domain_x_min = x_min;
        let ms_per_px = (x_max - x_min) / width_px as f64;
        
        let mut current_pixel_idx: i32 = -1;
        let mut agg_open = 0.0;
        let mut agg_high = f64::NEG_INFINITY;
        let mut agg_low = f64::INFINITY;
        let mut agg_close = 0.0;

        let up_color = self.config.up_body_color;
        let down_color = self.config.down_body_color;

        for data in self.source.iter_range(x_min, x_max) {
            if let PlotData::Ohlcv(candle) = data {
                let candle_width_px = candle.span / ms_per_px;

                if candle_width_px < 1.0 {
                    let mid_t = candle.time + candle.span / 2.0;
                    let px_idx = ((mid_t - domain_x_min) / ms_per_px) as i32;
                    
                    if px_idx != current_pixel_idx {
                        if current_pixel_idx != -1 {
                            self.draw_quad_bucket(window, transform, origin_x, current_pixel_idx, agg_open, agg_high, agg_low, agg_close);
                        }
                        current_pixel_idx = px_idx;
                        agg_open = candle.open;
                        agg_high = candle.high;
                        agg_low = candle.low;
                        agg_close = candle.close;
                    } else {
                        agg_high = agg_high.max(candle.high);
                        agg_low = agg_low.min(candle.low);
                        agg_close = candle.close;
                    }
                } else {
                    let is_up = candle.close >= candle.open;
                    let t_start_px = transform.x_data_to_screen(candle.time).as_f32();
                    let width_px = (candle.span / ms_per_px) as f32;
                    let center_x = t_start_px + width_px / 2.0;
                    let b_w = width_px * self.config.body_width_pct;
                    let w_w = (b_w * self.config.wick_width_pct).max(1.0);
                    let y_h = transform.y_data_to_screen(candle.high).as_f32();
                    let y_l = transform.y_data_to_screen(candle.low).as_f32();
                    let y_o = transform.y_data_to_screen(candle.open).as_f32();
                    let y_c = transform.y_data_to_screen(candle.close).as_f32();
                    let (b_top, b_bot) = if is_up { (y_c, y_o) } else { (y_o, y_c) };
                    
                    let color = if is_up { up_color } else { down_color };
                    
                    // Wick
                    window.paint_quad(fill(Bounds::new(Point::new(px(center_x - w_w/2.0), px(y_h)), Size::new(px(w_w), px(y_l - y_h))), color));
                    // Body
                    window.paint_quad(fill(Bounds::new(Point::new(px(center_x - b_w/2.0), px(b_top)), Size::new(px(b_w), px((b_bot - b_top).max(1.0)))), color));
                }
            }
        }

        if current_pixel_idx != -1 {
            self.draw_quad_bucket(window, transform, origin_x, current_pixel_idx, agg_open, agg_high, agg_low, agg_close);
        }
    }
}