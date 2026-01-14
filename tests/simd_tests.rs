use gpui::{px, Point};
use gpui_chart::data_types::{ColorOp, PlotData, PlotPoint};
use gpui_chart::simd::batch_transform_points;

#[test]
fn test_batch_transform_points_correctness() {
    let data = vec![
        PlotData::Point(PlotPoint {
            x: 0.0,
            y: 0.0,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 10.0,
            y: 10.0,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 5.0,
            y: 20.0,
            color_op: ColorOp::None,
        }),
    ];

    let x_scale = 2.0; // 1 data unit = 2 pixels
    let x_offset = 100.0; // Start at 100px
    let y_scale = -2.0; // Inverted Y, 1 data unit = 2 pixels
    let y_offset = 500.0; // Start at 500px

    let mut output = Vec::new();
    batch_transform_points(&data, x_scale, x_offset, y_scale, y_offset, &mut output);

    assert_eq!(output.len(), 3);

    // Point 0: (0 * 2 + 100, 0 * -2 + 500) = (100, 500)
    assert_eq!(output[0].x, px(100.0));
    assert_eq!(output[0].y, px(500.0));

    // Point 1: (10 * 2 + 100, 10 * -2 + 500) = (120, 480)
    assert_eq!(output[1].x, px(120.0));
    assert_eq!(output[1].y, px(480.0));

    // Point 2: (5 * 2 + 100, 20 * -2 + 500) = (110, 460)
    assert_eq!(output[2].x, px(110.0));
    assert_eq!(output[2].y, px(460.0));
}

#[test]
fn test_batch_transform_clears_buffer() {
    let data = vec![PlotData::Point(PlotPoint {
        x: 0.0,
        y: 0.0,
        color_op: ColorOp::None,
    })];
    let mut output = vec![Point::new(px(999.0), px(999.0))];

    batch_transform_points(&data, 1.0, 0.0, 1.0, 0.0, &mut output);

    assert_eq!(output.len(), 1);
    assert_eq!(output[0].x, px(0.0));
}
