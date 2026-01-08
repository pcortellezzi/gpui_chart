use gpui::prelude::*;
use gpui::*;
use gpui_chart::{
    ChartView, Series, LinePlot, Ohlcv, AxisDomain, CandlestickPlot,
    navigator_view::NavigatorView
};
use gpui_chart::data_types::{PlotPoint, ColorOp};
use std::rc::Rc;
use std::cell::RefCell;
use rand::Rng;

use gpui_chart::chart_view::{PanLeft, PanRight, PanUp, PanDown, ZoomIn, ZoomOut, ResetView};

struct DemoApp {
    chart: Entity<ChartView>,
    navigator: Entity<NavigatorView>,
}

impl DemoApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // 1. Create the shared Domain Entity with Y limit
        let domain = cx.new(|_cx| AxisDomain {
            x_min: 0.0,
            x_max: 10.0,
            y_min: 0.0,
            y_max: 10.0,
            y_min_limit: Some(0.0), // Prevent going into negative prices
            ..Default::default()
        });

        // 2. Prepare Data
        let mut line_data = Vec::new();
        let now = chrono::Utc::now().timestamp_millis() as f64;
        let hour_ms = 3600_000.0;
        for i in 0..100 {
            let x = now - (100 - i) as f64 * (hour_ms / 4.0);
            let y = (i as f64 * 0.2).sin() * 50.0 + 100.0;
            line_data.push(PlotPoint { x, y, color_op: ColorOp::None });
        }
        
        let mut candles = Vec::new();
        let mut price: f64 = 100.0;
        let mut rng = rand::rng();
        for i in 0..500 {
            let time = now - (500 - i) as f64 * hour_ms;
            let open = price;
            let close = price + rng.random_range(-10.0..10.0);
            let high = open.max(close) + rng.random_range(0.0..5.0);
            let low = open.min(close) - rng.random_range(0.0..5.0);
            candles.push(Ohlcv { time, open, high, low, close, volume: 1000.0, span: hour_ms * 0.8 });
            price = close;
        }

        let series = vec![
            Series {
                id: "sine_wave".to_string(),
                plot: Rc::new(RefCell::new(LinePlot::new(line_data))),
            },
            Series {
                id: "candles".to_string(),
                plot: Rc::new(RefCell::new(CandlestickPlot::new(candles))),
            }
        ];

        // 3. Create Views
        let series_clone = series.clone();
        let chart = cx.new(|cx| {
            let mut view = ChartView::new(domain.clone(), cx);
            for s in series { view.add_series(s); }
            view.auto_fit_axes(cx);
            view
        });

        let navigator = cx.new(|cx| {
            let mut nav = NavigatorView::new(domain.clone(), series_clone, cx);
            // Configure Navigator for 2D navigation (unlock Y)
            nav.config.lock_y = false;
            nav
        });

        Self { chart, navigator }
    }
}

impl Render for DemoApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(gpui::white())
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .child(self.chart.clone())
            )
            .child(
                div()
                    .h(px(100.0))
                    .w_full()
                    .p_2()
                    .bg(gpui::black())
                    .child(self.navigator.clone())
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("left", PanLeft, None),
            KeyBinding::new("right", PanRight, None),
            KeyBinding::new("up", PanUp, None),
            KeyBinding::new("down", PanDown, None),
            KeyBinding::new("+", ZoomIn, None),
            KeyBinding::new("=", ZoomIn, None),
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
