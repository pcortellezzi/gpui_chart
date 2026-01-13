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

- [ ] **Rich Tooltips**: Interactive popups on hover displaying precise values or metadata (customizable layout).
- [ ] **Markers & Arrows**: Methods to draw arrows pointing to specific data points with labels.
- [ ] **Shapes & Polygons**: Arbitrary drawing of rectangles, circles, or polygons in data coordinates.
- [ ] **Images**: Support for background images or watermarks.
- [ ] **Axis Spans (H/V Lines & Bands)**: Highlight specific ranges (e.g., "Recession", "Target Zone") across the entire plot area.

## 3. Axis System
Advanced control over scales and labeling.

- [ ] **Smart Date/Time Axis**: Automatic switching of formats (Years -> Months -> Days -> Hours -> Minutes) based on zoom level.
- [ ] **Multiple Axes**: Support for arbitrary numbers of Y-axes (left/right) and X-axes (top/bottom) with independent scaling.
- [ ] **Logarithmic Scale**: Robust implementation for Log10/Log2/Ln scales.
- [ ] **Tick Customization**: Rotation of labels, scientific notation (e.g., `1.2e-5`), custom formatters.
- [ ] **Inverted Axis**: Easy API to flip axes (e.g., Depth charts).

## 4. Interaction & UX
Enhancing user engagement.

- [ ] **Selection / Region of Interest**: Ability to select a range or rectangular region and get a callback with the data inside.
- [ ] **Draggable Points**: Allow users to drag specific data points (interactive editing).
- [ ] **ContextMenu**: Built-in right-click menu for common actions (Save, Reset, Toggle Grid).

## 5. Performance & Data Structures
Optimizations for specific use cases.

- [ ] **Signal Plot**: Specialized renderer for high-frequency data with fixed sample rate (only `Y` array + `rate`), faster than generic XY.
- [ ] **Live/Rolling Buffer**: Optimized ring buffer implementation for real-time sensor data (oscilloscope style).

## 6. Output & Export
Sharing results.

- [ ] **Image Export**: Save chart as PNG/JPG/SVG directly from the view.
- [ ] **Headless Rendering**: Ability to generate chart images on a server without opening a window (CLI/Backend usage).

---

## 🎯 Short-term Priorities (Recommendation)

1.  **Smart Date Axis**: Critical for financial/time-series usage.
2.  **Tooltips**: Essential for data inspection.
3.  **Scatter Plot**: Fills the gap for non-ordered scientific data.
4.  **Export**: Basic PNG export feature.
