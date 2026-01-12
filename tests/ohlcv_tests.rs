use gpui_chart::aggregation::decimate_ohlcv_arrays_par;
use gpui_chart::data_types::PlotData;

#[test]
fn test_ohlcv_decimation_basic() {
    let n = 100;
    let time: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let open: Vec<f64> = (0..n).map(|_| 10.0).collect();
    let high: Vec<f64> = (0..n).map(|i| 20.0 + i as f64).collect(); // Max will be at end
    let low: Vec<f64> = (0..n).map(|i| 5.0 - i as f64).collect();   // Min will be at end
    let close: Vec<f64> = (0..n).map(|_| 15.0).collect();

    // Decimate to 10 points (10 bins of 10 items each)
    let result = decimate_ohlcv_arrays_par(&time, &open, &high, &low, &close, 10);

    assert_eq!(result.len(), 10);

    // Check first bin (0..10)
    if let PlotData::Ohlcv(c) = &result[0] {
        assert_eq!(c.time, 0.0);
        assert_eq!(c.open, 10.0);
        assert_eq!(c.close, 15.0);
        // High should be max of 0..10 -> 29.0
        assert_eq!(c.high, 29.0);
        // Low should be min of 0..10 -> 5.0 - 9.0 = -4.0
        assert_eq!(c.low, -4.0);
    } else {
        panic!("Expected Ohlcv data");
    }
}

#[test]
fn test_ohlcv_decimation_empty() {
    let result = decimate_ohlcv_arrays_par(&[], &[], &[], &[], &[], 10);
    assert!(result.is_empty());
}

#[test]
fn test_ohlcv_decimation_undersampled() {
    let time = vec![1.0, 2.0];
    let open = vec![10.0, 10.0];
    let high = vec![12.0, 12.0];
    let low = vec![8.0, 8.0];
    let close = vec![11.0, 11.0];

    // Max points 10 > len 2 -> return 1:1
    let result = decimate_ohlcv_arrays_par(&time, &open, &high, &low, &close, 10);
    assert_eq!(result.len(), 2);
    if let PlotData::Ohlcv(c) = &result[0] {
        assert_eq!(c.time, 1.0);
        assert_eq!(c.open, 10.0);
    }
}

#[test]
fn test_ohlcv_decimation_nan_handling() {
    let time = vec![1.0, 2.0, 3.0, 4.0];
    let open = vec![f64::NAN, 10.0, f64::NAN, 12.0];
    let high = vec![f64::NAN, 20.0, f64::NAN, 22.0];
    let low = vec![f64::NAN, 5.0, f64::NAN, 2.0];
    let close = vec![f64::NAN, 15.0, f64::NAN, 16.0];

    // 1 bin
    let result = decimate_ohlcv_arrays_par(&time, &open, &high, &low, &close, 1);
    assert_eq!(result.len(), 1);
    
    if let PlotData::Ohlcv(c) = &result[0] {
        assert_eq!(c.open, 10.0); // First non-nan
        assert_eq!(c.close, 16.0); // Last non-nan
        assert_eq!(c.high, 22.0); // Max
        assert_eq!(c.low, 2.0);   // Min
    }
}
