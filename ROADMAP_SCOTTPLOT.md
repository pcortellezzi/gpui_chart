# Roadmap: Towards ScottPlot Parity

Comparison between `gpui_chart` and **ScottPlot**, identifying key missing features to reach a comparable level of maturity for scientific and financial visualization.

## 1. Plot Types (Visualizations)
ScottPlot offers a wide variety of specialized plots.

- [ ] **Scatter Plot**: Basic XY plot with unconnected points (markers). Essential for scientific data.
- [ ] **Error Bars**: Displaying variability of data (standard deviation/error) on points.
- [ ] **Bubble Plot**: Scatter plot with a third dimension (Z) represented by marker size.
- [ ] **Box & Whisker / Violin Plots**: Statistical distribution visualization.
- [ ] **Pie / Donut Charts**: Proportional data visualization.
- [ ] **Radar / Spider Charts**: Multivariate data comparison.
- [ ] **Advanced Heatmap**: Support for interpolation (bicubic/bilinear) and custom color maps (viridis, magma, etc.).
- [ ] **Histogram**: Frequency distribution calculation and rendering.

## 2. Annotations & Decorators
Visual elements to highlight or explain data.

- [x] **Crosshair & Basic Tooltips**: Interactive crosshair displaying precise values on hover.
- [ ] **Rich Tooltips**: Advanced interactive popups with customizable layouts and metadata.
- [ ] **Markers & Arrows**: Methods to draw arrows pointing to specific data points with labels.
- [ ] **Shapes & Polygons**: Arbitrary drawing of rectangles, circles, or polygons in data coordinates.
- [ ] **Images**: Support for background images or watermarks.
- [ ] **Axis Spans (H/V Lines & Bands)**: Highlight specific ranges (e.g., "Recession", "Target Zone") across the entire plot area.

## 3. Axis System
Advanced control over scales and labeling.

- [x] **Smart Date/Time Axis**: Automatic switching of formats (Years -> Months -> Days -> Hours -> Minutes) based on zoom level.
- [x] **Multiple Axes**: Support for arbitrary numbers of Y-axes (left/right) and X-axes (top/bottom) with independent scaling.
- [ ] **Logarithmic Scale**: Robust implementation for Log10/Log2/Ln scales.
- [ ] **Tick Customization**: Rotation of labels, scientific notation (e.g., `1.2e-5`), custom formatters.
- [ ] **Inverted Axis**: Easy API to flip axes (e.g., Depth charts).

## 4. Interaction & UX
Enhancing user engagement.

- [x] **Selection / Box Zoom**: Ability to select a rectangular region to zoom in.
- [ ] **Draggable Points**: Allow users to drag specific data points (interactive editing).
- [ ] **ContextMenu**: Built-in right-click menu for common actions (Save, Reset, Toggle Grid).

## 5. Performance & Data Structures
Optimizations for specific use cases.

- [x] **High-Perf Decimation (M4/LTTB)**: Handle millions of points with peak preservation.
- [ ] **Signal Plot**: Specialized renderer for high-frequency data with fixed sample rate.
- [x] **Live/Rolling Buffer**: Optimized ring buffer implementation for real-time sensor data (StreamingDataSource).

## 6. Data Integrity & Stability (New)
Ensuring visual and numerical correctness.

- [x] **Visual Stability (Anti-Jitter)**: Stable binning anchored to fixed coordinates to prevent flickering during panning.
- [x] **Peak Preservation (Y-Integrity)**: Guaranteed rendering of local minima/maxima even at extreme compression ratios.
- [x] **Numerical Robustness**: Stability at extreme zoom levels (nanoseconds to millions of units).

## 6. Output & Export
Sharing results.

- [ ] **Image Export**: Save chart as PNG/JPG/SVG directly from the view.
- [ ] **Headless Rendering**: Ability to generate chart images on a server without opening a window (CLI/Backend usage).

---

## 🎯 Short-term Priorities (Recommendation)

1.  **Scatter Plot**: Fills the gap for non-ordered scientific data.
2.  **Logarithmic Scale**: Essential for scientific and technical charts.
3.  **Rich Tooltips**: Better layout for multi-series data inspection.
4.  **Export**: Basic PNG/SVG export feature.
