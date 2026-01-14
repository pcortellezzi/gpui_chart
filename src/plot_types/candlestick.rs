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

        let mut buffer = self.buffer.lock();
        self.source.get_aggregated_data(x_min, x_max, max_points, &mut buffer);

        let count = buffer.len();
        let avg_px_per_point = if count > 0 {
             width_px / count as f32
        } else {
             width_px
        };

        let theme = &state.theme;
        let body_pct = theme.candle_body_width_pct;
        let wick_pct = theme.candle_wick_width_pct;
        let contour_thickness = theme.candle_contour_thickness_px;

        for data in buffer.iter() {
            if let PlotData::Ohlcv(candle) = data {
                let is_up = candle.close >= candle.open;
                let t_start_px = transform.x_data_to_screen(candle.time).as_f32();
                
                // 1. Calculate base width
                let span = if candle.span > 0.0 { candle.span } else { self.source.suggested_x_spacing() };
                let theoretical_span_px = (span / ms_per_px) as f32;
                
                let width_px = theoretical_span_px.min(avg_px_per_point * 1.5).max(1.0);
                let center_x = t_start_px + (theoretical_span_px / 2.0);

                // 2. High density mode: just a vertical line if we have less than 2px to draw
                if width_px < 2.0 {
                    let y_h = transform.y_data_to_screen(candle.high).as_f32();
                    let y_l = transform.y_data_to_screen(candle.low).as_f32();
                    let color = if is_up { theme.up_candle_contour_color } else { theme.down_candle_contour_color };
                    window.paint_quad(fill(
                        Bounds::new(
                            Point::new(px(center_x - 0.5), px(y_h)),
                            Size::new(px(1.0), px((y_l - y_h).max(1.0))),
                        ),
                        color,
                    ));
                    continue;
                }

                // 3. Body and Wick calculation
                let b_w = (width_px * body_pct).max(1.0); 
                let w_w = (b_w * wick_pct).max(1.0); 

                let y_h = transform.y_data_to_screen(candle.high).as_f32();
                let y_l = transform.y_data_to_screen(candle.low).as_f32();
                let y_o = transform.y_data_to_screen(candle.open).as_f32();
                let y_c = transform.y_data_to_screen(candle.close).as_f32();
                let (b_top, b_bot) = if is_up { (y_c, y_o) } else { (y_o, y_c) };

                let body_color = if is_up { theme.up_candle_body_color } else { theme.down_candle_body_color };
                let contour_color = if is_up { theme.up_candle_contour_color } else { theme.down_candle_contour_color };

                // Top Wick (High to Body Top)
                if y_h < b_top {
                    window.paint_quad(fill(
                        Bounds::new(
                            Point::new(px(center_x - w_w / 2.0), px(y_h)),
                            Size::new(px(w_w), px(b_top - y_h)),
                        ),
                        contour_color,
                    ));
                }

                // Bottom Wick (Body Bottom to Low)
                if y_l > b_bot {
                    window.paint_quad(fill(
                        Bounds::new(
                            Point::new(px(center_x - w_w / 2.0), px(b_bot)),
                            Size::new(px(w_w), px(y_l - b_bot)),
                        ),
                        contour_color,
                    ));
                }

                // Body Contour
                // We draw it using 4 quads to represent the border
                let b_left = center_x - b_w / 2.0;
                let b_right = center_x + b_w / 2.0;
                let b_height = (b_bot - b_top).max(1.0);

                // Top border
                window.paint_quad(fill(
                    Bounds::new(Point::new(px(b_left), px(b_top)), Size::new(px(b_w), px(contour_thickness))),
                    contour_color,
                ));
                // Bottom border
                window.paint_quad(fill(
                    Bounds::new(Point::new(px(b_left), px(b_bot - contour_thickness)), Size::new(px(b_w), px(contour_thickness))),
                    contour_color,
                ));
                // Left border
                window.paint_quad(fill(
                    Bounds::new(Point::new(px(b_left), px(b_top)), Size::new(px(contour_thickness), px(b_height))),
                    contour_color,
                ));
                // Right border
                window.paint_quad(fill(
                    Bounds::new(Point::new(px(b_right - contour_thickness), px(b_top)), Size::new(px(contour_thickness), px(b_height))),
                    contour_color,
                ));

                // Body Fill
                let fill_top = b_top + contour_thickness;
                let fill_bot = b_bot - contour_thickness;
                if fill_bot > fill_top {
                    window.paint_quad(fill(
                        Bounds::new(
                            Point::new(px(b_left + contour_thickness), px(fill_top)),
                            Size::new(px(b_w - 2.0 * contour_thickness), px(fill_bot - fill_top)),
                        ),
                        body_color,
                    ));
                }
            }
        }
    }
}

