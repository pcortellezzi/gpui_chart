use gpui_chart::aggregation::decimate_lttb_slice;
use gpui_chart::data_types::{ColorOp, PlotData, PlotPoint};

#[test]
fn test_lttb_sine_wave() {
    let mut data = Vec::new();
    let count = 100;
    for i in 0..count {
        let x = i as f64;
        let y = (x * 0.1).sin();
        data.push(PlotData::Point(PlotPoint {
            x,
            y,
            color_op: ColorOp::None,
        }));
    }

    let max_points = 10;
    let decimated = decimate_lttb_slice(&data, max_points);

    assert_eq!(decimated.len(), max_points);

    // Check start and end
    if let PlotData::Point(p) = &decimated[0] {
        assert_eq!(p.x, 0.0);
    } else {
        panic!("First point is not a Point");
    }

    if let PlotData::Point(p) = &decimated[max_points - 1] {
        assert_eq!(p.x, (count - 1) as f64);
    } else {
        panic!("Last point is not a Point");
    }

    // Check monotony of X
    let mut last_x = -1.0;
    for p in &decimated {
        if let PlotData::Point(pt) = p {
            assert!(pt.x > last_x);
            last_x = pt.x;
        }
    }
}

#[test]
fn test_lttb_small_data() {
    let mut data = Vec::new();
    for i in 0..5 {
        data.push(PlotData::Point(PlotPoint {
            x: i as f64,
            y: i as f64,
            color_op: ColorOp::None,
        }));
    }

    let max_points = 10;
    let decimated = decimate_lttb_slice(&data, max_points);

    assert_eq!(decimated.len(), 5);
}

#[test]
fn test_lttb_preserves_peak() {
    // 0, 0, 100, 0, 0
    // LTTB with 3 points should keep (0,0), (2,100), (4,0)
    let data = vec![
        PlotData::Point(PlotPoint { x: 0.0, y: 0.0, color_op: ColorOp::None }),
        PlotData::Point(PlotPoint { x: 1.0, y: 0.0, color_op: ColorOp::None }),
        PlotData::Point(PlotPoint { x: 2.0, y: 100.0, color_op: ColorOp::None }),
        PlotData::Point(PlotPoint { x: 3.0, y: 0.0, color_op: ColorOp::None }),
        PlotData::Point(PlotPoint { x: 4.0, y: 0.0, color_op: ColorOp::None }),
    ];

    let max_points = 3;
    let decimated = decimate_lttb_slice(&data, max_points);

    assert_eq!(decimated.len(), 3);
    
    // First point
    if let PlotData::Point(p) = &decimated[0] {
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
    }

    // Middle point should be the peak
    if let PlotData::Point(p) = &decimated[1] {
        assert_eq!(p.x, 2.0);
        assert_eq!(p.y, 100.0);
    } else {
        panic!("Middle point is not a Point");
    }

    // Last point
    if let PlotData::Point(p) = &decimated[2] {
        assert_eq!(p.x, 4.0);
        assert_eq!(p.y, 0.0);
    }
}
