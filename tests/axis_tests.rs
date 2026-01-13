use gpui_chart::data_types::{AxisFormat, AxisRange};
use gpui_chart::scales::ChartScale;

#[test]
fn test_axis_range_pan() {
    let mut range = AxisRange::new(100.0, 200.0);
    range.pan(50.0);
    assert_eq!(range.min, 150.0);
    assert_eq!(range.max, 250.0);
    assert_eq!(range.span(), 100.0);
}

#[test]
fn test_axis_range_zoom_center() {
    let mut range = AxisRange::new(100.0, 200.0);
    // Zoom in (factor 0.5) at center (pivot_pct 0.5)
    // Center is 150.0. New span is 50.0.
    // New range should be [125.0, 175.0]
    range.zoom_at(150.0, 0.5, 0.5);
    assert_eq!(range.min, 125.0);
    assert_eq!(range.max, 175.0);
    assert_eq!(range.span(), 50.0);
}

#[test]
fn test_axis_range_zoom_edge() {
    let mut range = AxisRange::new(100.0, 200.0);
    // Zoom out (factor 2.0) at left edge (pivot_pct 0.0)
    // Pivot is 100.0. New span is 200.0.
    // New range should be [100.0, 300.0]
    range.zoom_at(100.0, 0.0, 2.0);
    assert_eq!(range.min, 100.0);
    assert_eq!(range.max, 300.0);
}

#[test]
fn test_axis_range_clamp() {
    let mut range = AxisRange::new(100.0, 200.0);
    range.min_limit = Some(50.0);
    range.max_limit = Some(250.0);

    // Pan within limits
    range.pan(-20.0); // [80, 180]
    range.clamp();
    assert_eq!(range.min, 80.0);

    // Pan outside min limit
    range.pan(-40.0); // [40, 140]
    range.clamp();
    assert_eq!(range.min, 50.0);
    assert_eq!(range.max, 150.0); // Span should be preserved

    // Pan outside max limit
    range.pan(150.0); // [200, 300]
    range.clamp();
    assert_eq!(range.max, 250.0);
    assert_eq!(range.min, 150.0); // Span preserved
}

#[test]
fn test_axis_range_zoom_pivot_with_clamping() {
    let mut range = AxisRange::new(100.0, 200.0);
    range.min_limit = Some(0.0);
    range.max_limit = Some(300.0);

    // Zoom out (factor 4.0) at pivot 150.0 (pct 0.5)
    // Virtual range: [-50, 350].
    range.zoom_at(150.0, 0.5, 4.0);

    // Avec la nouvelle logique, clamp() ne doit pas modifier les bornes
    // car elles couvrent déjà toute la zone [0, 300].
    range.clamp();

    assert_eq!(range.min, -50.0, "Virtual min should be preserved");
    assert_eq!(range.max, 350.0, "Virtual max should be preserved");

    // Cependant, pour le rendu, on verra bien [0, 300]
    let (view_min, view_max) = range.clamped_bounds();
    assert_eq!(view_min, 0.0);
    assert_eq!(view_max, 300.0);

    // Zoom back in (factor 0.25) at the same pivot 150.0.
    // 150.0 est toujours à 50% de [-50, 350].
    // New span = 400 * 0.25 = 100.
    // New min = 150 - 100 * 0.5 = 100.
    range.zoom_at(150.0, 0.5, 0.25);
    assert_eq!(range.min, 100.0);
    assert_eq!(range.max, 200.0);
}

#[test]
fn test_chart_scale_formatting() {
    let scale = ChartScale::new_linear((0.0, 1.0), (0.0, 100.0));

    // Test large numbers (timestamps)
    let ts = 1736500000000.0; // Sometime in 2025
    let formatted = scale.format_tick(ts, &AxisFormat::Numeric);
    assert!(
        formatted.contains(":"),
        "Should be formatted as time: {}",
        formatted
    );

    // Test small numbers
    assert_eq!(scale.format_tick(0.000123, &AxisFormat::Numeric), "0.0001");
    assert_eq!(scale.format_tick(123.456, &AxisFormat::Numeric), "123.46");
    assert_eq!(scale.format_tick(1234.56, &AxisFormat::Numeric), "1235");
}