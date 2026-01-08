// Candlestick plot implementation

use crate::data_types::{CandlestickConfig, Ohlcv};
use gpui::*;
use adabraka_ui::util::PixelsExt;

use super::PlotRenderer;

/// Candlestick plot type
#[derive(Clone)]
pub struct CandlestickPlot {
    pub data: Vec<Ohlcv>,
    pub config: CandlestickConfig,
}

impl CandlestickPlot {
    pub fn new(data: Vec<Ohlcv>) -> Self {
        Self {
            data,
            config: CandlestickConfig::default(),
        }
    }
}

impl PlotRenderer for CandlestickPlot {
    fn get_min_max(&self) -> Option<(f64, f64, f64, f64)> {
        if self.data.is_empty() { return None; }
        
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for candle in &self.data {
            x_min = x_min.min(candle.time);
            x_max = x_max.max(candle.time + candle.span);
            y_min = y_min.min(candle.low);
            y_max = y_max.max(candle.high);
        }

        Some((x_min, x_max, y_min, y_max))
    }

    fn render(
        &self,
        window: &mut Window,
        transform: &crate::transform::PlotTransform,
        _series_id: &str,
    ) {
        if self.data.is_empty() { return; }

        let bounds = transform.bounds;
        let width_px = bounds.size.width.as_f32();
        if width_px <= 0.0 { return; }

        let domain_x_min = transform.x_scale.invert(0.0);
        let domain_x_max = transform.x_scale.invert(width_px);
        let margin_x = (domain_x_max - domain_x_min) * 0.01;
        let start_time = domain_x_min - margin_x;
        let end_time = domain_x_max + margin_x;

        let start_idx = self.data.partition_point(|c| c.time + c.span < start_time);
        let end_idx = self.data.partition_point(|c| c.time < end_time);
        let visible_candles = &self.data[start_idx..end_idx];

        if visible_candles.is_empty() { return; }

        let mut up_body_fill = PathBuilder::fill();
        let mut up_wick_fill = PathBuilder::fill(); 
        let mut down_body_fill = PathBuilder::fill();
        let mut down_wick_fill = PathBuilder::fill();

        let mut has_up = false;
        let mut has_down = false;

        let origin_x = bounds.origin.x.as_f32();
        
        let mut current_pixel_idx: i32 = -1;
        let mut agg_open = 0.0;
        let mut agg_high = f64::NEG_INFINITY;
        let mut agg_low = f64::INFINITY;
        let mut agg_close = 0.0;

        let ms_per_px = (domain_x_max - domain_x_min) / width_px as f64;
        
        let mut draw_bucket = |
            px_idx: i32, 
            o: f64, h: f64, l: f64, c: f64
        | {
            let is_up = c >= o;
            let x_center = origin_x + px_idx as f32;
            let y_h = transform.y_data_to_screen(h).as_f32();
            let y_l = transform.y_data_to_screen(l).as_f32();
            let y_o = transform.y_data_to_screen(o).as_f32();
            let y_c = transform.y_data_to_screen(c).as_f32();
            let (body_top, body_bottom) = if is_up { (y_c, y_o) } else { (y_o, y_c) };
            
            let line_width = 1.0;
            let half = 0.5;

            let wick_rect = Bounds::new(Point::new(px(x_center - half), px(y_h)), Size::new(px(line_width), px(y_l - y_h)));
            let body_h = (body_bottom - body_top).max(1.0); 
            let body_rect = Bounds::new(Point::new(px(x_center - half), px(body_top)), Size::new(px(line_width), px(body_h)));

            if is_up {
                has_up = true;
                add_rect_to_path(&mut up_wick_fill, wick_rect);
                add_rect_to_path(&mut up_body_fill, body_rect);
            } else {
                has_down = true;
                add_rect_to_path(&mut down_wick_fill, wick_rect);
                add_rect_to_path(&mut down_body_fill, body_rect);
            }
        };

        let first_candle_span = visible_candles[0].span;
        let candle_width_px = first_candle_span / ms_per_px;
        
        if candle_width_px < 1.0 {
            for candle in visible_candles {
                let mid_t = candle.time + candle.span / 2.0;
                let px_idx = ((mid_t - domain_x_min) / ms_per_px) as i32;
                
                if px_idx != current_pixel_idx {
                    if current_pixel_idx != -1 {
                        draw_bucket(current_pixel_idx, agg_open, agg_high, agg_low, agg_close);
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
            }
            if current_pixel_idx != -1 {
                draw_bucket(current_pixel_idx, agg_open, agg_high, agg_low, agg_close);
            }
        } else {
            for candle in visible_candles {
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
                
                let wick_rect = Bounds::new(Point::new(px(center_x - w_w/2.0), px(y_h)), Size::new(px(w_w), px(y_l - y_h)));
                let body_rect = Bounds::new(Point::new(px(center_x - b_w/2.0), px(b_top)), Size::new(px(b_w), px((b_bot - b_top).max(1.0))));
                
                if is_up {
                    has_up = true;
                    add_rect_to_path(&mut up_wick_fill, wick_rect);
                    add_rect_to_path(&mut up_body_fill, body_rect);
                } else {
                    has_down = true;
                    add_rect_to_path(&mut down_wick_fill, wick_rect);
                    add_rect_to_path(&mut down_body_fill, body_rect);
                }
            }
        }

        if has_up {
            if let Ok(p) = up_body_fill.build() { window.paint_path(p, self.config.up_body_color); }
            if let Ok(p) = up_wick_fill.build() { window.paint_path(p, self.config.up_wick_color); }
        }
        if has_down {
            if let Ok(p) = down_body_fill.build() { window.paint_path(p, self.config.down_body_color); }
            if let Ok(p) = down_wick_fill.build() { window.paint_path(p, self.config.down_wick_color); }
        }
    }
}

fn add_rect_to_path(builder: &mut PathBuilder, bounds: Bounds<Pixels>) {
    let min = bounds.origin;
    let max = Point::new(min.x + bounds.size.width, min.y + bounds.size.height);
    builder.move_to(min);
    builder.line_to(Point::new(max.x, min.y));
    builder.line_to(max);
    builder.line_to(Point::new(min.x, max.y));
    builder.close();
}