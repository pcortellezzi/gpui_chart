use gpui_chart::decimation::decimate_m4_arrays_par;

#[test]
fn test_extreme_zoom_precision() {
    let n = 1000;

    // Tiny range (nanoseconds or smaller)
    let x_tiny: Vec<f64> = (0..n).map(|i| 1_000_000.0 + i as f64 * 1e-12).collect();
    let y: Vec<f64> = (0..n).map(|i| i as f64).collect();

    let res_tiny = decimate_m4_arrays_par(&x_tiny, &y, 100, None, 0);
    assert!(res_tiny.len() > 0);
    assert!(!res_tiny.is_empty());

    // Huge range
    let x_huge: Vec<f64> = (0..n).map(|i| i as f64 * 1e15).collect();
    let res_huge = decimate_m4_arrays_par(&x_huge, &y, 100, None, 0);
    assert!(res_huge.len() > 0);

    // Verify no NaNs in output
    for p in res_huge {
        match p {
            gpui_chart::data_types::PlotData::Point(pt) => {
                assert!(pt.x.is_finite());
                assert!(pt.y.is_finite());
            }
            _ => {}
        }
    }
}

#[test]
fn test_zero_range_stability() {
    // All points have same X
    let n = 100;
    let x = vec![1.0; n];
    let y: Vec<f64> = (0..n).map(|i| i as f64).collect();

    let res = decimate_m4_arrays_par(&x, &y, 10, None, 0);
    // Should not panic or return NaN
    assert!(res.len() <= 10);
}
