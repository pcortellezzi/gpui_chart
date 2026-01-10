use gpui::prelude::*;
use gpui::*;
use gpui_chart::{
    ChartPane, Series, LinePlot, AreaPlot, BarPlot, StepLinePlot, AnnotationPlot, HeatmapPlot, Ohlcv, CandlestickPlot,
    chart_container::ChartContainer,
    navigator_view::NavigatorView,
    data_types::{AxisRange, SharedPlotState, AxisEdge, Annotation, HeatmapCell, StreamingDataSource, PlotData},
};
use gpui_chart::data_types::{PlotPoint, ColorOp};
use std::sync::Arc;
use parking_lot::RwLock;
use rand::{Rng, SeedableRng};

struct DemoApp {
    chart: Entity<ChartContainer>,
    navigator: Entity<NavigatorView>,
}

impl DemoApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as f64;
        let hour_ms = 3600_000.0;

        // 1. Shared State
        let shared_x = cx.new(|_cx| AxisRange::new(now - 50.0 * hour_ms, now));
        let shared_plot_state = cx.new(|_cx| SharedPlotState::default());

        // 2. Ranges
        let price_y = cx.new(|_cx| AxisRange::new(0.0, 200.0));
        let volume_y = cx.new(|_cx| AxisRange::new(0.0, 1000.0));
        let indicator_y = cx.new(|_cx| AxisRange::new(0.0, 100.0));

        // 3. Data Generation
        let mut candles = Vec::new();
        let mut ma_data = Vec::new();
        let mut volume_data = Vec::new();
        let mut step_data = Vec::new();
        let mut rng = rand::rngs::StdRng::from_os_rng();
        let mut p: f64 = 100.0;
        
        for i in 0..200 {
            let t = now - (200 - i) as f64 * hour_ms;
            let o = p;
            let c = p + rng.random_range(-5.0..5.0);
            let h = o.max(c) + rng.random_range(0.0..2.0);
            let l = o.min(c) - rng.random_range(0.0..2.0);
            let vol = rng.random_range(100.0..800.0);
            
            candles.push(Ohlcv { time: t, open: o, high: h, low: l, close: c, volume: vol, span: hour_ms * 0.8 });
            ma_data.push(PlotPoint { x: t, y: p + 2.0, color_op: ColorOp::Persistent(gpui::yellow()) });
            volume_data.push(PlotPoint { x: t, y: vol, color_op: ColorOp::Persistent(if c >= o { gpui::green() } else { gpui::red() }) });
            step_data.push(PlotPoint { x: t, y: 50.0 + (i as f64 * 0.2).sin() * 20.0, color_op: ColorOp::None });
            p = c;
        }

        // 4. Panes
        
        // --- PANE 1: PRICE & ANNOTATIONS ---
        let price_pane = cx.new(|cx| {
            let mut pane = ChartPane::new(shared_plot_state.clone(), cx);
            pane.add_x_axis(shared_x.clone(), cx);
            pane.add_y_axis(price_y.clone(), cx);
            
            pane.add_series(Series::new("Price", CandlestickPlot::new(candles.clone())));
            pane.add_series(Series::new("MA", LinePlot::new(ma_data)));

            // Annotations
            let annotations = vec![
                Annotation::VLine { x: now - 10.0 * hour_ms, color: gpui::white().alpha(0.5), width: 1.0, label: Some("Signal".into()) },
                Annotation::HLine { y: 110.0, color: gpui::blue().alpha(0.5), width: 1.0, label: Some("Resistance".into()) },
                Annotation::Rect { x_min: now - 30.0 * hour_ms, x_max: now - 20.0 * hour_ms, y_min: 90.0, y_max: 100.0, color: gpui::green().alpha(0.1), fill: true },
            ];
            pane.add_series(Series::new("Markers", AnnotationPlot::new(annotations)));
            pane
        });

        // --- PANE 2: VOLUME (BARS) & STREAMING AREA ---
        let mut momentum_plot_handle = None;
        let volume_pane = cx.new(|cx| {
            let mut pane = ChartPane::new(shared_plot_state.clone(), cx);
            pane.add_x_axis(shared_x.clone(), cx);
            pane.add_y_axis(volume_y.clone(), cx);
            
            pane.add_series(Series::new("Volume", BarPlot::new(volume_data)));

            // Real-time Streaming Area Plot
            let source = Box::new(StreamingDataSource::new(1000));
            let plot = AreaPlot::with_source(source).with_baseline(0.0);
            let handle = pane.add_plot("Momentum (Live)", plot);
            momentum_plot_handle = Some(handle);
            
            pane
        });
        let momentum_plot = momentum_plot_handle.expect("Failed to create momentum plot");

        // --- PANE 3: STEP LINE & HEATMAP ---
        let indicator_pane = cx.new(|cx| {
            let mut pane = ChartPane::new(shared_plot_state.clone(), cx);
            pane.add_x_axis(shared_x.clone(), cx);
            pane.add_y_axis(indicator_y.clone(), cx);
            
            pane.add_series(Series::new("Step", StepLinePlot::new(step_data)));

            // Small Heatmap sample
            let mut cells = vec![];
            for ix in 0..10 {
                for iy in 0..5 {
                    cells.push(HeatmapCell {
                        x: now - (ix as f64 * 5.0) * hour_ms,
                        y: 20.0 + iy as f64 * 15.0,
                        width: hour_ms * 4.0,
                        height: 10.0,
                        color: gpui::blue().alpha(0.1 * (ix + iy) as f32),
                        text: None,
                    });
                }
            }
            pane.add_series(Series::new("Heatmap", HeatmapPlot::new(cells)));
            pane
        });

        // 5. Container
        let chart = cx.new(|cx| {
            let mut container = ChartContainer::new(shared_x.clone(), shared_plot_state.clone(), cx);
            
            container.add_x_axis(shared_x.clone(), AxisEdge::Bottom, px(25.0), "Time".to_string(), cx);

            container.add_pane(price_pane, 2.0, cx);
            container.add_y_axis(0, price_y.clone(), AxisEdge::Right, px(60.0), "Price".to_string(), cx);
            
            container.add_pane(volume_pane, 1.0, cx);
            container.add_y_axis(1, volume_y.clone(), AxisEdge::Right, px(60.0), "Volume".to_string(), cx);

            container.add_pane(indicator_pane, 1.0, cx);
            container.add_y_axis(2, indicator_y.clone(), AxisEdge::Left, px(60.0), "Ind".to_string(), cx);
            
            container
        });

        let navigator = cx.new(|cx| {
            let mut nav_data = vec![];
            for c in &candles { nav_data.push(PlotPoint { x: c.time, y: c.close, color_op: ColorOp::None }); }
            let nav_series = vec![Series::new("Nav", LinePlot::new(nav_data))];
            NavigatorView::new(shared_x, price_y, shared_plot_state.clone(), nav_series, cx)
        });

        // Start streaming loop
        let x = now;
        let y = 500.0;
        let rng_stream = rand::rngs::StdRng::from_os_rng();
        let app_entity = cx.entity().clone();
        let shared_state_clone = shared_plot_state.clone();
        let momentum_plot_clone = momentum_plot.clone();
        
        window.on_next_frame(move |window, cx| {
            app_entity.update(cx, |_, cx| {
                stream_update(window, cx, shared_state_clone, momentum_plot_clone, x, y, rng_stream, hour_ms);
            });
        });

        Self { chart, navigator }
    }
}

fn stream_update(
    window: &mut Window,
    cx: &mut Context<DemoApp>,
    shared_state: Entity<SharedPlotState>,
    plot: Arc<RwLock<AreaPlot>>,
    mut x: f64,
    mut y: f64,
    mut rng: rand::rngs::StdRng,
    hour_ms: f64,
) {
    x += hour_ms * 0.05;
    y += rng.random_range(-10.0..10.0);
    y = y.clamp(100.0, 900.0);

    plot.write().source.add_data(PlotData::Point(PlotPoint {
        x, y, color_op: ColorOp::None
    }));

    shared_state.update(cx, |s, _cx| {
        s.request_render();
    });
    cx.notify(); // Force local notification to ensure frame propagation

    let app_entity = cx.entity().clone();
    let shared_state_clone = shared_state.clone();
    let plot_clone = plot.clone();
    window.on_next_frame(move |window, cx| {
        app_entity.update(cx, |_, cx| {
            stream_update(window, cx, shared_state_clone, plot_clone, x, y, rng, hour_ms);
        });
    });
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
        cx.bind_keys([
            KeyBinding::new("ctrl-d", gpui_chart::chart_pane::ToggleDebug, None),
            KeyBinding::new("cmd-d", gpui_chart::chart_pane::ToggleDebug, None),
        ]);
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| DemoApp::new(window, cx))
        }).unwrap();
    });
}
