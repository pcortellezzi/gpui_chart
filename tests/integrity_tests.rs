use gpui_chart::decimation::decimate_m4_arrays_par;
use gpui_chart::data_types::PlotData;

#[test]
fn test_y_bound_integrity() {
    let n = 10000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let mut y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.01).cos()).collect();

    // Inject extreme peaks at random positions
    let min_val = -5000.0;
    let max_val = 5000.0;
    y[456] = max_val;
    y[7890] = min_val;

    let max_points = 100; // High compression ratio (100:1)
    let decimated = decimate_m4_arrays_par(&x, &y, max_points, None);

    let mut dec_min = f64::INFINITY;
    let mut dec_max = f64::NEG_INFINITY;

    for p in &decimated {
        if let PlotData::Point(pt) = p {
            dec_min = dec_min.min(pt.y);
            dec_max = dec_max.max(pt.y);
        }
    }

    assert_eq!(dec_min, min_val, "Min value lost during decimation!");
    assert_eq!(dec_max, max_val, "Max value lost during decimation!");
}
