use gpui::prelude::*;
use gpui::*;
use gpui_chart::{
    ChartView, Series, LinePlot, Ohlcv, CandlestickPlot,
    navigator_view::NavigatorView,
    data_types::{AxisRange, SharedPlotState},
    ChartLayout, PaneSize
};
use gpui_chart::data_types::{PlotPoint, ColorOp};
use std::rc::Rc;
use std::cell::RefCell;
use rand::Rng;

use gpui_chart::chart_view::{PanLeft, PanRight, PanUp, PanDown, ZoomIn, ZoomOut, ResetView};

struct DemoApp {
    layout: Entity<ChartLayout>,
    navigator: Entity<NavigatorView>,
    shared_plot_state: Entity<SharedPlotState>,
}

impl DemoApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as f64;
        let hour_ms = 3600_000.0;

        // 1. Shared State for synchronization
        let shared_x = cx.new(|_cx| AxisRange::new(now - 50.0 * hour_ms, now));
        let shared_plot_state = cx.new(|_cx| SharedPlotState::default());

        // 2. Specific Y-axis ranges
        let price_y = cx.new(|_cx| {
            let mut r = AxisRange::new(0.0, 200.0);
            r.min_limit = Some(0.0);
            r
        });
        let indicator_y = cx.new(|_cx| {
            let mut r = AxisRange::new(0.0, 100.0);
            r.min_limit = Some(0.0);
            r.max_limit = Some(100.0);
            r
        });

        // 3. Prepare Data
        let mut line_data = Vec::new();
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

        let price_series = vec![
            Series {
                id: "Price".to_string(),
                plot: Rc::new(RefCell::new(CandlestickPlot::new(candles))),
                y_axis_index: 0,
            }
        ];
        let price_series_clone = price_series.clone();

        let indicator_series = vec![
            Series {
                id: "Oscillator".to_string(),
                plot: Rc::new(RefCell::new(LinePlot::new(line_data))),
                y_axis_index: 0,
            }
        ];

        // 4. Create Views
        let margin_left = px(60.0);

        let price_chart = cx.new(|cx| {
            let mut view = ChartView::new(shared_x.clone(), price_y.clone(), shared_plot_state.clone(), cx);
            view.margin_left = margin_left;
            view.margin_right = margin_left;
            view.show_x_axis = false;
            
            // Add a second Y axis for an overlay indicator
            let overlay_y = cx.new(|_cx| {
                let mut r = AxisRange::new(0.0, 1000.0);
                r.min_limit = Some(0.0);
                r
            });
            let _idx = view.add_y_axis(overlay_y, cx);
            
            // Add a series on the second Y axis
            let mut volume_data = Vec::new();
            for i in 0..100 {
                let x = now - (100 - i) as f64 * (hour_ms / 4.0);
                let y = (i as f64 * 0.5).cos().abs() * 800.0;
                volume_data.push(PlotPoint { x, y, color_op: ColorOp::None });
            }
            view.add_series(Series {
                id: "Overlay Vol".to_string(),
                plot: Rc::new(RefCell::new(LinePlot::new(volume_data))),
                y_axis_index: 1,
            });

            for s in price_series { view.add_series(s); }
            view.auto_fit_axes(cx);
            view
        });

        let indicator_chart = cx.new(|cx| {
            let mut view = ChartView::new(shared_x.clone(), indicator_y.clone(), shared_plot_state.clone(), cx);
            view.margin_left = margin_left;
            for s in indicator_series { view.add_series(s); }
            view.auto_fit_axes(cx);
            view
        });

        let layout = cx.new(|cx| {
            let mut l = ChartLayout::new(cx);
            l.add_pane(price_chart, PaneSize::Weight(3.0), cx);
            l.add_pane(indicator_chart, PaneSize::Weight(1.0), cx);
            l
        });

        let navigator = cx.new(|cx| {
            let mut nav = NavigatorView::new(shared_x.clone(), price_y.clone(), price_series_clone, cx);
            nav.config.lock_y = false;
            nav
        });

        Self { layout, navigator, shared_plot_state }
    }
}

impl Render for DemoApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(gpui::black())
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .child(self.layout.clone())
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