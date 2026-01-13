use gpui::prelude::*;
use gpui::*;
use gpui_chart::data_types::{ColorOp, PlotPoint};
use gpui_chart::{
    chart_view::ToggleDebug,
    data_types::{
        Annotation, AxisEdge, AxisId, AxisRange, HeatmapCell, PlotData, SharedPlotState,
        StreamingDataSource,
    },
    navigator_view::NavigatorView,
    AnnotationPlot, AreaPlot, AxisState, BarPlot, CandlestickPlot, Chart, ChartView, HeatmapPlot,
    LinePlot, Ohlcv, PaneState, Series, StepLinePlot,
};
use parking_lot::RwLock;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

struct DemoApp {
    chart_view: Entity<ChartView>,
    navigator: Entity<NavigatorView>,
}

impl DemoApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as f64;
        let hour_ms = 3600_000.0;

        // 1. Shared State & Model
        let shared_x = cx.new(|_cx| AxisRange::new(now - 50.0 * hour_ms, now));
        let shared_plot_state = cx.new(|_cx| SharedPlotState::default());

        let chart = cx.new(|cx| Chart::new(shared_x.clone(), shared_plot_state.clone(), cx));

        // 2. Data Generation
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

            candles.push(Ohlcv {
                time: t,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: vol,
                span: hour_ms * 0.8,
            });
            ma_data.push(PlotPoint {
                x: t,
                y: p + 2.0,
                color_op: ColorOp::Persistent(gpui::yellow()),
            });
            volume_data.push(PlotPoint {
                x: t,
                y: vol,
                color_op: ColorOp::Persistent(if c >= o { gpui::green() } else { gpui::red() }),
            });
            step_data.push(PlotPoint {
                x: t,
                y: 50.0 + (i as f64 * 0.2).sin() * 20.0,
                color_op: ColorOp::None,
            });
            p = c;
        }

        // Annotations
        let annotations = vec![
            Annotation::VLine {
                x: now - 10.0 * hour_ms,
                color: gpui::white().alpha(0.5),
                width: 1.0,
                label: Some("Signal".into()),
            },
            Annotation::HLine {
                y: 110.0,
                color: gpui::blue().alpha(0.5),
                width: 1.0,
                label: Some("Resistance".into()),
            },
            Annotation::Rect {
                x_min: now - 30.0 * hour_ms,
                x_max: now - 20.0 * hour_ms,
                y_min: 90.0,
                y_max: 100.0,
                color: gpui::green().alpha(0.1),
                fill: true,
            },
        ];

        // Small Heatmap sample
        let mut heatmap_cells = vec![];
        for ix in 0..10 {
            for iy in 0..5 {
                heatmap_cells.push(HeatmapCell {
                    x: now - (ix as f64 * 5.0) * hour_ms,
                    y: 20.0 + iy as f64 * 15.0,
                    width: hour_ms * 4.0,
                    height: 10.0,
                    color: gpui::blue().alpha(0.1 * (ix + iy) as f32),
                    text: None,
                });
            }
        }

        // Momentum Plot
        let source = Box::new(StreamingDataSource::new(1000));
        let momentum_plot = Arc::new(RwLock::new(
            AreaPlot::with_source(source).with_baseline(0.0),
        ));

        // 3. Setup Model Content
        chart.update(cx, |c, cx| {
            // X-Axis
            c.x_axes.push(AxisState {
                entity: shared_x.clone(),
                edge: AxisEdge::Bottom,
                size: px(25.0),
                label: "Time".into(),
                format: gpui_chart::data_types::AxisFormat::Time,
            });

            // Pane 1: Price
            let price_y = cx.new(|_| {
                let mut r = AxisRange::new(0.0, 200.0);
                r.min_limit = Some(0.0);
                r
            });
            let mut p1 = PaneState::new("price".into(), 2.0);
            p1.y_axes.push(AxisState {
                entity: price_y,
                edge: AxisEdge::Right,
                size: px(60.0),
                label: "Price".into(),
                format: gpui_chart::data_types::AxisFormat::Numeric,
            });
            p1.series
                .push(Series::new("Price", CandlestickPlot::new(candles.clone())));
            p1.series.push(Series::new("MA", LinePlot::new(ma_data)));
            p1.series
                .push(Series::new("Markers", AnnotationPlot::new(annotations)));
            c.panes.push(p1);

            // Pane 2: Volume
            let volume_y = cx.new(|_| AxisRange::new(0.0, 1000.0));
            let mut p2 = PaneState::new("volume".into(), 1.0);
            p2.y_axes.push(AxisState {
                entity: volume_y,
                edge: AxisEdge::Right,
                size: px(60.0),
                label: "Volume".into(),
                format: gpui_chart::data_types::AxisFormat::Numeric,
            });
            p2.series
                .push(Series::new("Volume", BarPlot::new(volume_data)));
            p2.series.push(Series {
                id: "Momentum".into(),
                plot: momentum_plot.clone(),
                x_axis_id: AxisId(0),
                y_axis_id: AxisId(0),
            });
            c.panes.push(p2);

            // Pane 3: Indicator
            let indicator_y = cx.new(|_| AxisRange::new(0.0, 100.0));
            let mut p3 = PaneState::new("indicator".into(), 1.0);
            p3.y_axes.push(AxisState {
                entity: indicator_y,
                edge: AxisEdge::Left,
                size: px(60.0),
                label: "Indicator".into(),
                format: gpui_chart::data_types::AxisFormat::Numeric,
            });
            p3.series
                .push(Series::new("Step", StepLinePlot::new(step_data)));
            p3.series
                .push(Series::new("Heatmap", HeatmapPlot::new(heatmap_cells)));
            c.panes.push(p3);
        });

        // 4. Create View
        let chart_view = cx.new(|cx| ChartView::new(chart.clone(), cx));

        let navigator = cx.new(|cx| {
            let mut nav_data = vec![];
            for c in &candles {
                nav_data.push(PlotPoint {
                    x: c.time,
                    y: c.close,
                    color_op: ColorOp::None,
                });
            }
            let nav_series = vec![Series::new("Nav", LinePlot::new(nav_data))];
            let price_y_nav = chart.read(cx).panes[0].y_axes[0].entity.clone();
            NavigatorView::new(
                shared_x.clone(),
                price_y_nav,
                shared_plot_state.clone(),
                nav_series,
                cx,
            )
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
                stream_update(
                    window,
                    cx,
                    shared_state_clone,
                    momentum_plot_clone,
                    x,
                    y,
                    rng_stream,
                    hour_ms,
                );
            });
        });

        Self {
            chart_view,
            navigator,
        }
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
        x,
        y,
        color_op: ColorOp::None,
    }));

    shared_state.update(cx, |s, _cx| {
        s.request_render();
    });
    cx.notify();

    let app_entity = cx.entity().clone();
    let shared_state_clone = shared_state.clone();
    let plot_clone = plot.clone();
    window.on_next_frame(move |window, cx| {
        app_entity.update(cx, |_, cx| {
            stream_update(
                window,
                cx,
                shared_state_clone,
                plot_clone,
                x,
                y,
                rng,
                hour_ms,
            );
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
            .child(div().flex_1().child(self.chart_view.clone()))
            .child(
                div()
                    .h(px(100.0))
                    .w_full()
                    .p_2()
                    .child(self.navigator.clone()),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("ctrl-d", ToggleDebug, None),
            KeyBinding::new("cmd-d", ToggleDebug, None),
        ]);
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| DemoApp::new(window, cx))
        })
        .unwrap();
    });
}
