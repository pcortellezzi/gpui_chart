# GPUI Chart

A high-performance, interactive, and composable charting library for [GPUI](https://github.com/zed-industries/zed). Designed for financial and scientific data visualization with hundreds of thousands of points.

## ✨ Features

### 🚀 Core Performance
- **High Performance**: Optimized for rendering massive datasets (LOD, Occlusion Culling, Zero-copy painting).
- **Smooth Navigation**: Inertial scrolling, 60fps zooming and panning.

### 🖱️ Advanced Interactivity
The chart is fully interactive out-of-the-box:

**Navigation:**
- **Pan**: Left Click Drag or Two-finger scroll.
- **Zoom**: Ctrl + Scroll or Pinch gesture.
- **Box Zoom**: Right Click Drag on the chart area to zoom into a specific region.
- **Reset**: Double Click on the chart or an axis to reset the view.

**Axis Management:**
- **Axis Zoom/Pan**: Drag an axis to pan it, Ctrl+Scroll on an axis to zoom it independently.
- **Auto-Fit**: Double Click on an axis to auto-fit the visible data range.
- **Flip Edge**: Right Click on an axis to move it to the opposite side (Left/Right or Top/Bottom).

**Pane Management:**
- **Resizing**: Drag the separators (splitters) between panes to adjust their height.
- **Reordering**: Use the pane control buttons (top-right overlay) to move panes Up/Down.
- **Management**: Add new panes (`+`) or close existing ones (`✕`) dynamically.

**Legend & Series:**
- **Visibility**: Click a series name in the legend to toggle its visibility.
- **Isolation**: Click the `S` button in the legend to "Isolate" a series on its own Y-axis (useful for overlaying indicators with different scales).
- **Moving**: Use `▲` / `▼` in the legend to move a series to the pane above or below.

### 🛠️ Developer Experience
- **Declarative API**: Build charts using a fluid builder pattern.
- **Themable**: Full support for Light/Dark modes and custom themes via `ChartTheme`.
- **Debug Mode**: Built-in performance overlay (toggle via action) showing render times and paint stats.

> **Note on Performance:** 
> Currently, geometry generation happens on the CPU using GPUI's drawing primitives. While optimized for large datasets via **Level-of-Detail (LOD)** and **Occlusion Culling**, we are awaiting **Custom Shaders** support in GPUI to transition to a full-GPU pipeline (WGPU), which will unlock even higher scale performance.

---

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
gpui = "0.2"
gpui_chart = { path = "path/to/gpui_chart" } # Or git repo
```

## 🚀 Quick Start

```rust
use gpui::*;
use gpui_chart::{Chart, ChartView, AxisRange, Series, LinePlot, PlotPoint, SharedPlotState};

struct AppState {
    chart_view: View<ChartView>,
}

impl Render for AppState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.chart_view.clone().into_any_element()
    }
}

fn main() {
    App::new().run(|cx: &mut AppContext| {
        cx.open_window(WindowOptions::default(), |cx| {
            // 1. Prepare Data
            let points: Vec<PlotPoint> = (0..10000)
                .map(|i| PlotPoint {
                    x: i as f64,
                    y: (i as f64 / 50.0).sin() + (i as f64 / 100.0).cos(),
                    color_op: gpui_chart::data_types::ColorOp::None,
                })
                .collect();

            // 2. Create Model
            let x_axis = cx.new(|_| AxisRange::new(0.0, 1000.0));
            let shared_state = cx.new(|_| SharedPlotState::default());
            
            let chart = cx.new(|cx| {
                let mut c = Chart::new(x_axis.clone(), shared_state.clone(), cx);
                
                // Add a Pane
                c.add_pane_at(0, 1.0, cx);
                if let Some(pane) = c.panes.get_mut(0) {
                    pane.series.push(Series::new(
                        "Signal",
                        LinePlot::new(points),
                    ));
                }
                c
            });

            // 3. Create View
            let chart_view = cx.new(|cx| ChartView::new(chart, cx));

            AppState { chart_view }
        });
    });
}
```

## 🧩 Architecture

The library strictly separates **Model** from **View** to allow for complex application states.

- **`Chart` (Model)**: A GPUI `Entity` that holds the entire state:
    - List of `PaneState` (series, layout weights).
    - List of `AxisState` (configuration, edges).
    - `ChartTheme`.
    - It is the "Single Source of Truth". Modifying the model automatically triggers a repaint.

- **`ChartView` (View)**: A purely visual component that renders the `Chart`.
    - Handles input events and dispatches actions to the Model.
    - Can be embedded anywhere in your GPUI application.

- **`SharedPlotState`**: A shared synchronization primitive.
    - Synchronizes crosshairs, hover states, and tooltips across multiple charts (e.g. main chart + minimap).

## ⌨️ Controls Reference

| Context | Action | Interaction |
|---|---|---|
| **Chart** | Pan | Left Drag / Scroll |
| **Chart** | Zoom X/Y | Ctrl + Scroll / Pinch / Middle Drag |
| **Chart** | Zoom Box | Right Drag |
| **Chart** | Inspect | Hover (Crosshair) |
| **Chart** | Auto-Fit Y | Double Click |
| **Axis** | Pan Axis | Left Drag on axis |
| **Axis** | Zoom Axis | Middle Click Drag / Ctrl + Scroll on axis |
| **Axis** | Auto-Fit | Double Click on axis |
| **Axis** | Flip Side | Right Click on axis |
| **Pane** | Resize | Drag splitter between panes |
| **Pane** | Move | Buttons `↑` `↓` in top-right overlay |
| **Pane** | Add/Remove | Buttons `+` `✕` in top-right overlay |
| **Legend** | Toggle Series | Click on series name |
| **Legend** | Isolate Series | Click `S` button (creates new Y-axis) |
| **Legend** | Move Series | Click `▲` / `▼` buttons |

## 🎨 Theming

You can switch themes dynamically:

```rust
// Switch to Light Theme
chart.update(cx, |c, cx| {
    c.set_theme(ChartTheme::light(), cx);
});
```

Or customize every aspect:

```rust
let my_theme = ChartTheme {
    background: gpui::black(),
    axis_line: gpui::red(),
    // ...
    ..ChartTheme::dark()
};
```

## 🛠️ Debugging

To debug rendering performance or layout issues:
1. Register the `ToggleDebug` action in your keymap (optional).
2. Or trigger it programmatically:
   ```rust
   cx.dispatch_action(gpui_chart::chart_view::ToggleDebug);
   ```
3. An overlay will appear showing:
   - Frame render time.
   - Paint duration per pane.
   - Coordinate under cursor.

---

## 🤖 Acknowledgment

This library was architected and implemented through a symbiotic collaboration between human intent and AI execution. It stands as a testament to what modern AI-assisted engineering can achieve: rapidly iterating from concept to a robust, production-ready graphics engine.