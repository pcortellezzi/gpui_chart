use gpui::prelude::*;
use gpui::*;
use gpui_chart::{
    ChartPane, Series, LinePlot, Ohlcv, CandlestickPlot,
    chart_container::ChartContainer,
    navigator_view::NavigatorView,
    data_types::{AxisRange, SharedPlotState, AxisId, AxisEdge},
};
use gpui_chart::data_types::{PlotPoint, ColorOp};
use std::sync::Arc;
use parking_lot::RwLock;
use rand::Rng;

struct DemoApp {
    chart: Entity<ChartContainer>,
    navigator: Entity<NavigatorView>,
}

impl DemoApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as f64;
        let hour_ms = 3600_000.0;

        // 1. Shared State
        let shared_x = cx.new(|_cx| AxisRange::new(now - 50.0 * hour_ms, now));
        let shared_plot_state = cx.new(|_cx| SharedPlotState::default());

        // 2. Ranges
        let price_y = cx.new(|_cx| AxisRange::new(0.0, 200.0));
        let indicator_y = cx.new(|_cx| AxisRange::new(0.0, 100.0));

        // 3. Data
        let mut candles = Vec::new();
        let mut ma_data = Vec::new();
        let mut rng = rand::rng();
        let mut p: f64 = 100.0;
        for i in 0..200 {
            let t = now - (200 - i) as f64 * hour_ms;
            let o = p;
            let c = p + rng.random_range(-5.0..5.0);
            let h = o.max(c) + rng.random_range(0.0..2.0);
            let l = o.min(c) - rng.random_range(0.0..2.0);
            candles.push(Ohlcv { time: t, open: o, high: h, low: l, close: c, volume: 100.0, span: hour_ms * 0.8 });
            ma_data.push(PlotPoint { x: t, y: p + 2.0, color_op: ColorOp::Persistent(gpui::yellow()) });
            p = c;
        }

        // 4. Panes
        let candles_clone = candles.clone();
        let price_pane = cx.new(|cx| {
            let mut pane = ChartPane::new(shared_plot_state.clone(), cx);
            pane.add_x_axis(shared_x.clone(), cx);
            pane.add_y_axis(price_y.clone(), cx);
            
            // Série 1 : Bougies
            pane.add_series(Series {
                id: "Price".to_string(),
                plot: Arc::new(RwLock::new(CandlestickPlot::new(candles_clone))),
                x_axis_id: AxisId(0),
                y_axis_id: AxisId(0),
            });
            
            // Série 2 : Moyenne Mobile (MA) sur le même axe
            pane.add_series(Series {
                id: "MA".to_string(),
                plot: Arc::new(RwLock::new(LinePlot::new(ma_data))),
                x_axis_id: AxisId(0),
                y_axis_id: AxisId(0),
            });
            pane
        });

        let indicator_pane = cx.new(|cx| {
            let mut pane = ChartPane::new(shared_plot_state.clone(), cx);
            pane.add_x_axis(shared_x.clone(), cx);
            pane.add_y_axis(indicator_y.clone(), cx);
            let mut data = vec![];
            for i in 0..100 { data.push(PlotPoint { x: now - i as f64 * hour_ms, y: (i as f64 * 0.1).sin() * 50.0 + 50.0, color_op: ColorOp::None }); }
            pane.add_series(Series {
                id: "RSI".to_string(),
                plot: Arc::new(RwLock::new(LinePlot::new(data))),
                x_axis_id: AxisId(0),
                y_axis_id: AxisId(0),
            });
            pane
        });

        // 5. Container
        let shared_x_clone = shared_x.clone();
        let price_y_clone = price_y.clone();
        let chart = cx.new(|cx| {
            let mut container = ChartContainer::new(shared_x_clone, shared_plot_state, cx);
            
            // Axes X multi-fuseaux
            container.add_x_axis(shared_x.clone(), AxisEdge::Bottom, px(25.0), "UTC".to_string(), cx);
            container.add_x_axis(shared_x.clone(), AxisEdge::Top, px(25.0), "New York".to_string(), cx);

            // Zones
            container.add_pane(price_pane, 3.0, cx);
            container.add_y_axis(0, price_y_clone, AxisEdge::Right, px(60.0), "Price".to_string(), cx);
            
            container.add_pane(indicator_pane, 1.0, cx);
            container.add_y_axis(1, indicator_y, AxisEdge::Left, px(60.0), "RSI".to_string(), cx);
            container
        });

        let navigator = cx.new(|cx| {
            let mut line_data = vec![];
            for c in &candles {
                line_data.push(PlotPoint { x: c.time, y: c.close, color_op: ColorOp::None });
            }
            let nav_series = vec![Series {
                id: "Nav".to_string(),
                plot: Arc::new(RwLock::new(LinePlot::new(line_data))),
                x_axis_id: AxisId(0),
                y_axis_id: AxisId(0),
            }];
            NavigatorView::new(shared_x, price_y, nav_series, cx)
        });

        Self { chart, navigator }
    }
}

impl Render for DemoApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(gpui::black())
            .flex()
            .flex_col()
            .child(div().flex_1().child(self.chart.clone()))
            .child(div().h(px(100.0)).w_full().p_2().child(self.navigator.clone()))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|cx| DemoApp::new(cx))
        }).unwrap();
    });
}