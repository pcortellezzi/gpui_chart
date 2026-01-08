use gpui::prelude::*;
use gpui::*;
use gpui_chart::{
    ChartView, Series, LinePlot, Ohlcv, AxisDomain, CandlestickPlot
};
use gpui_chart::data_types::{PlotPoint, ColorOp};
use std::rc::Rc;
use std::cell::RefCell;
use rand::Rng;

// Import actions for key binding
use gpui_chart::chart_view::{PanLeft, PanRight, PanUp, PanDown, ZoomIn, ZoomOut, ResetView};

struct DemoApp {
    chart: Entity<ChartView>,
}

impl DemoApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let chart = cx.new(|cx| {
            let mut view = ChartView::new(cx);
            
            let now = chrono::Utc::now().timestamp_millis() as f64;
            let hour_ms = 3600_000.0;

            let mut line_data = Vec::new();
            for i in 0..100 {
                let x = now - (100 - i) as f64 * (hour_ms / 4.0);
                let y = (i as f64 * 0.2).sin() * 50.0 + 100.0;
                line_data.push(PlotPoint { x, y, color_op: ColorOp::None });
            }
            
            view.add_series(Series {
                id: "sine_wave".to_string(),
                plot: Rc::new(RefCell::new(LinePlot::new(line_data))),
            });

            let mut candles = Vec::new();
            let mut price: f64 = 100.0;
            let mut rng = rand::rng();
            for i in 0..50 {
                let time = now - (50 - i) as f64 * hour_ms;
                let open = price;
                let close = price + rng.random_range(-10.0..10.0);
                let high = open.max(close) + rng.random_range(0.0..5.0);
                let low = open.min(close) - rng.random_range(0.0..5.0);
                candles.push(Ohlcv { time, open, high, low, close, volume: 1000.0, span: hour_ms * 0.8 });
                price = close;
            }

            view.add_series(Series {
                id: "candles".to_string(),
                plot: Rc::new(RefCell::new(CandlestickPlot::new(candles))),
            });

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
        // 1. Register Key Bindings
        cx.bind_keys([
            KeyBinding::new("left", PanLeft, None),
            KeyBinding::new("right", PanRight, None),
            KeyBinding::new("up", PanUp, None),
            KeyBinding::new("down", PanDown, None),
            KeyBinding::new("+", ZoomIn, None),
            KeyBinding::new("=", ZoomIn, None), // also supports '=' for '+' key
            KeyBinding::new("-", ZoomOut, None),
            KeyBinding::new("0", ResetView, None),
            KeyBinding::new("cmd-0", ResetView, None),
            KeyBinding::new("ctrl-0", ResetView, None),
        ]);

        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|cx| DemoApp::new(cx))
        }).expect("failed to open window");
    });
}
