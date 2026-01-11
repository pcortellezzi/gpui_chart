use gpui::{px, Bounds, Point, Size};
use gpui_chart::scales::ChartScale;
use gpui_chart::transform::PlotTransform;

#[test]
fn test_chart_scale_linear() {
    let scale = ChartScale::new_linear((0.0, 100.0), (0.0, 500.0));

    assert_eq!(scale.map(0.0), 0.0);
    assert_eq!(scale.map(50.0), 250.0);
    assert_eq!(scale.map(100.0), 500.0);

    assert_eq!(scale.invert(0.0), 0.0);
    assert_eq!(scale.invert(250.0), 50.0);
    assert_eq!(scale.invert(500.0), 100.0);
}

#[test]
fn test_plot_transform() {
    let x_scale = ChartScale::new_linear((0.0, 100.0), (0.0, 200.0));
    let y_scale = ChartScale::new_linear((0.0, 100.0), (200.0, 0.0));

    let bounds = Bounds::new(
        Point::new(px(0.0), px(0.0)),
        Size::new(px(200.0), px(200.0)),
    );
    let transform = PlotTransform::new(x_scale, y_scale, bounds);

    // Test Data -> Screen
    let p_data_origin = Point::new(0.0, 0.0);
    let p_screen_origin = transform.data_to_screen(p_data_origin);
    assert_eq!(p_screen_origin.x, px(0.0));
    assert_eq!(p_screen_origin.y, px(200.0));

    let p_data_center = Point::new(50.0, 50.0);
    let p_screen_center = transform.data_to_screen(p_data_center);
    assert_eq!(p_screen_center.x, px(100.0));
    assert_eq!(p_screen_center.y, px(100.0));

    // Test Screen -> Data
    let p_restored = transform.screen_to_data(p_screen_center);
    assert!((p_restored.x - 50.0).abs() < 0.001);
    assert!((p_restored.y - 50.0).abs() < 0.001);
}
