use gpui_chart::decimation::decimate_lttb_slice;
use gpui_chart::data_types::{ColorOp, PlotData, PlotPoint};

#[test]
fn test_lttb_decimation_basic() {
    let n = 1000;
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        data.push(PlotData::Point(PlotPoint {
            x: i as f64,
            y: (i as f64 * 0.1).sin(),
            color_op: ColorOp::None,
        }));
    }

    let max_points = 100;
    // LTTB should return approximately max_points
    let decimated = decimate_lttb_slice(&data, max_points, None, None);

    assert!(decimated.len() >= 10);

    // First and last points should be preserved
    let final_idx = decimated.len() - 1;
    if let (PlotData::Point(p_start), PlotData::Point(p_end)) =
        (&decimated[0], &decimated[final_idx])
    {
        assert_eq!(p_start.x, 0.0);
        assert_eq!(p_end.x, (n - 1) as f64);
    } else {
        panic!("Expected Point data");
    }
}

#[test]
fn test_lttb_decimation_undersampled() {
    let data = vec![
        PlotData::Point(PlotPoint {
            x: 0.0,
            y: 0.0,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 1.0,
            y: 1.0,
            color_op: ColorOp::None,
        }),
    ];

    let max_points = 10;
    let decimated = decimate_lttb_slice(&data, max_points, None, None);

    assert_eq!(decimated.len(), 2);
}

#[test]
fn test_lttb_preserves_peaks() {
    // Creating a signal with a sharp peak that MinMax might miss depending on binning,
    // but LTTB should try to preserve.
    let n = 1000;
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let mut y = 0.0;
        if i == 505 {
            y = 100.0; // Sharp peak
        }
        data.push(PlotData::Point(PlotPoint {
            x: i as f64,
            y,
            color_op: ColorOp::None,
        }));
    }

    let max_points = 50;
    let decimated = decimate_lttb_slice(&data, max_points, None, None);

    let has_peak = decimated.iter().any(|p| match p {
        PlotData::Point(pt) => pt.y == 100.0,
        _ => false,
    });

    assert!(
        has_peak,
        "LTTB should have preserved the sharp peak at index 505"
    );
}

#[test]
fn test_lttb_with_nans() {
    let data = vec![
        PlotData::Point(PlotPoint {
            x: 0.0,
            y: 0.0,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 1.0,
            y: f64::NAN,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 2.0,
            y: 2.0,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 3.0,
            y: 3.0,
            color_op: ColorOp::None,
        }),
    ];

    let max_points = 3;
    let decimated = decimate_lttb_slice(&data, max_points, None, None);

    assert!(decimated.len() >= 2);
    // Should not panic and should return valid points
    for p in &decimated {
        if let PlotData::Point(pt) = p {
            assert!(!pt.x.is_nan());
        }
    }
}
