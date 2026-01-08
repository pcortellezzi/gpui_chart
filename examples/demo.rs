use gpui::prelude::*;
use gpui::*;
use gpui_chart::{
    ChartView, Series, LinePlot, CandlestickPlot, Ohlcv, AxisDomain
};
use gpui_chart::data_types::{PlotPoint, ColorOp};
use std::rc::Rc;
use std::cell::RefCell;
use rand::Rng;

struct DemoApp {
    chart: Entity<ChartView>,
}

impl DemoApp {
    // In GPUI 0.2.2, views are usually created via cx.new()
    pub fn new(cx: &mut Context<Self>) -> Self {
        let chart = cx.new(|cx| {
            let mut view = ChartView::new(cx);
            
            let now = chrono::Utc::now().timestamp_millis() as f64;
            let hour_ms = 3600_000.0;

            // 1. Generate Line Data (Sine Wave) over the last 24 hours
            let mut line_data = Vec::new();
            for i in 0..100 {
                let x = now - (100 - i) as f64 * (hour_ms / 4.0);
                let y = (i as f64 * 0.2).sin() * 50.0 + 100.0;
                line_data.push(PlotPoint { 
                    x, 
                    y, 
                    color_op: ColorOp::None 
                });
            }
            
            let line_series = Series {
                id: "sine_wave".to_string(),
                plot: Rc::new(RefCell::new(LinePlot::new(line_data))),
            };
            view.add_series(line_series);

            // 2. Generate Candlestick Data for the last 50 intervals
            let mut candles = Vec::new();
            let mut price: f64 = 100.0;
            let mut rng = rand::rng();
            
            for i in 0..50 {
                let time = now - (50 - i) as f64 * hour_ms;
                let open = price;
                let close = price + rng.random_range(-10.0..10.0);
                let high = open.max(close) + rng.random_range(0.0..5.0);
                let low = open.min(close) - rng.random_range(0.0..5.0);
                
                candles.push(Ohlcv {
                    time,
                    open,
                    high,
                    low,
                    close,
                    volume: 1000.0,
                    span: hour_ms * 0.8,
                });
                price = close;
            }

            let candle_series = Series {
                id: "candles".to_string(),
                plot: Rc::new(RefCell::new(CandlestickPlot::new(candles))),
            };
            view.add_series(candle_series);

            view.domain = AxisDomain {
                x_min: 0.0,
                x_max: 800.0,
                y_min: 50.0,
                y_max: 200.0,
            };
            
            view.auto_fit_axes();
            view
        });

        Self { chart }
    }
}

impl Render for DemoApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(gpui::white())
            .child(self.chart.clone())
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|cx| DemoApp::new(cx))
        }).expect("failed to open window");
    });
}