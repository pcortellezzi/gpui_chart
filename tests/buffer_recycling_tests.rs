use gpui_chart::aggregation::decimate_m4_arrays_par_into;
use gpui_chart::data_types::{ColorOp, PlotData, PlotPoint};

#[test]
fn test_decimate_into_clears_buffer() {
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![10.0, 20.0, 30.0, 40.0];
    let max_points = 2; // Should result in 2 points (First/Last or Min/Max)

    // Pre-fill buffer with garbage
    let mut buffer = vec![
        PlotData::Point(PlotPoint {
            x: 999.0,
            y: 999.0,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 888.0,
            y: 888.0,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 777.0,
            y: 777.0,
            color_op: ColorOp::None,
        }),
    ];

    decimate_m4_arrays_par_into(&x, &y, max_points, &mut buffer, None);

    assert_eq!(buffer.len(), 2);
    if let PlotData::Point(p) = buffer[0] {
        assert_eq!(p.x, 1.0);
    }
}

#[test]
fn test_decimate_into_reserves_capacity() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let max_points = 10; // Request more than input

    let mut buffer = Vec::new();
    // Capacity should grow
    decimate_m4_arrays_par_into(&x, &y, max_points, &mut buffer, None);

    assert_eq!(buffer.len(), 5);
}
