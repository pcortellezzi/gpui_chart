use gpui_chart::data_types::{PlotDataSource, VecDataSource, PlotPoint, ColorOp, PlotData, Ohlcv};

#[test]
fn test_vec_datasource_aggregation_simple() {
    // 100 points : y = x
    let data: Vec<PlotData> = (0..100).map(|i| PlotData::Point(PlotPoint {
        x: i as f64,
        y: i as f64,
        color_op: ColorOp::None,
    })).collect();

    let source = VecDataSource::new(data);

    // Ask for 10 points covering the whole range [0, 99]
    // 100 points / 10 = 10 points per bin.
    let aggregated: Vec<PlotData> = source.iter_aggregated(0.0, 99.0, 10).collect();

    // With simple dynamic binning (fallback when < 2000 points):
    // We expect exactly 10 points if perfectly divisible.
    assert!(aggregated.len() <= 10, "Should return at most 10 points, got {}", aggregated.len());
    assert!(!aggregated.is_empty(), "Should return some points");

    // Check content of the first bin (0..10)
    // Points 0,1,2,3,4,5,6,7,8,9. Sum = 45. Avg = 4.5.
    if let PlotData::Point(p) = aggregated[0] {
        assert!((p.y - 4.5).abs() < 0.1, "First point average should be around 4.5, got {}", p.y);
    }
}

#[test]
fn test_vec_datasource_pyramid_trigger() {
    // 4000 points to trigger pyramid build (> 2000 threshold)
    let count = 4000;
    let data: Vec<PlotData> = (0..count).map(|i| PlotData::Point(PlotPoint {
        x: i as f64,
        y: i as f64,
        color_op: ColorOp::None,
    })).collect();

    let source = VecDataSource::new(data);

    // Ask for 100 points.
    // Raw count = 4000. Max = 100. Ratio = 40.
    // Expected level logic:
    // log2(40) = 5.32. ceil = 6. Level index = 6 - 1 = 5.
    // L0 (/2) -> 2000 pts
    // L1 (/4) -> 1000 pts
    // L2 (/8) -> 500 pts
    // L3 (/16) -> 250 pts
    // L4 (/32) -> 125 pts
    // L5 (/64) -> 62.5 pts
    
    // So it should select L5 and return roughly 62 points.
    
    let aggregated: Vec<PlotData> = source.iter_aggregated(0.0, count as f64, 100).collect();
    
    println!("Requested 100 points from 4000, got {}", aggregated.len());
    
    // We allow some flexibility because of level boundaries, but it should be heavily compressed.
    assert!(aggregated.len() < 200, "Should be compressed significantly, got {}", aggregated.len());
    assert!(aggregated.len() > 30, "Should not be over-compressed, got {}", aggregated.len());
}

#[test]
fn test_vec_datasource_ohlcv_aggregation() {
    // 100 candles.
    // Time 0..100.
    // Each candle: Open=10, Close=20.
    let data: Vec<PlotData> = (0..100).map(|i| PlotData::Ohlcv(Ohlcv {
        time: i as f64,
        span: 1.0,
        open: 10.0,
        high: 25.0,
        low: 5.0,
        close: 20.0,
        volume: 100.0,
    })).collect();

    let source = VecDataSource::new(data);

    // Aggregate into 10 bins (10 candles per bin).
    let aggregated: Vec<PlotData> = source.iter_aggregated(0.0, 100.0, 10).collect();
    
    assert_eq!(aggregated.len(), 10);

    if let PlotData::Ohlcv(candle) = &aggregated[0] {
        // Bin 0: candles 0..9.
        // Open should be Open of candle 0 (10.0).
        // Close should be Close of candle 9 (20.0).
        // High should be max of highs (25.0).
        // Low should be min of lows (5.0).
        // Volume should be sum (10 * 100 = 1000).
        // Time should be start time (0.0).
        // Span should be end_time - start_time (10.0 - 0.0 = 10.0, assuming contiguous).
        // Note: candle 9 starts at 9.0, ends at 10.0. So total span is 10.0.
        
        assert_eq!(candle.open, 10.0);
        assert_eq!(candle.close, 20.0);
        assert_eq!(candle.high, 25.0);
        assert_eq!(candle.low, 5.0);
        assert_eq!(candle.volume, 1000.0);
        assert_eq!(candle.time, 0.0);
        assert_eq!(candle.span, 10.0);
    } else {
        panic!("Expected Ohlcv data");
    }
}
