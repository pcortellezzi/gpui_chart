use gpui_chart::data_types::{ColorOp, Ohlcv, PlotData, PlotDataSource, PlotPoint, VecDataSource};

#[test]
fn test_vec_datasource_aggregation_simple() {
    // 100 points : y = x
    let data: Vec<PlotData> = (0..100)
        .map(|i| {
            PlotData::Point(PlotPoint {
                x: i as f64,
                y: i as f64,
                color_op: ColorOp::None,
            })
        })
        .collect();

    let source = VecDataSource::new(data);

    // Ask for 10 points covering the whole range [0, 99]
    // 100 points / 10 = 10 points per bin.
    let aggregated: Vec<PlotData> = source.iter_aggregated(0.0, 99.0, 10, None).collect();

    // With simple dynamic binning (fallback when < 2000 points):
    // We expect exactly 10 points if perfectly divisible.
    assert!(
        aggregated.len() <= 10,
        "Should return at most 10 points, got {}",
        aggregated.len()
    );
    assert!(!aggregated.is_empty(), "Should return some points");

    // Check content of the first bin (0..19)
    // Current Min/Max strategy returns the actual min and max points.
    // First bin [0..19]: Min is 0, Max is 19.
    if let PlotData::Point(p) = aggregated[0] {
        assert_eq!(
            p.y, 0.0,
            "First point should be the min of the first bin, got {}",
            p.y
        );
    }
}

#[test]
fn test_vec_datasource_pyramid_trigger() {
    // 4000 points to trigger pyramid build (> 2000 threshold)
    let count = 4000;
    let data: Vec<PlotData> = (0..count)
        .map(|i| {
            PlotData::Point(PlotPoint {
                x: i as f64,
                y: i as f64,
                color_op: ColorOp::None,
            })
        })
        .collect();

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

    let aggregated: Vec<PlotData> = source
        .iter_aggregated(0.0, count as f64, 100, None)
        .collect();

    println!("Requested 100 points from 4000, got {}", aggregated.len());

    // We allow some flexibility because of level boundaries, but it should be heavily compressed.
    assert!(
        aggregated.len() < 200,
        "Should be compressed significantly, got {}",
        aggregated.len()
    );
    assert!(
        aggregated.len() > 30,
        "Should not be over-compressed, got {}",
        aggregated.len()
    );
}

#[test]
fn test_vec_datasource_ohlcv_aggregation() {
    // 100 candles.
    // Time 0..100.
    let count = 100;
    let data: Vec<PlotData> = (0..count)
        .map(|i| {
            PlotData::Ohlcv(Ohlcv {
                time: i as f64,
                span: 1.0,
                open: 10.0,
                high: 25.0,
                low: 5.0,
                close: 20.0,
                volume: 100.0,
            })
        })
        .collect();

    let source = VecDataSource::new(data);

    // Aggregate into 10 max points.
    // The stable binning logic might choose a bin size of 10 or 20.
    let aggregated: Vec<PlotData> = source.iter_aggregated(0.0, 100.0, 10, None).collect();

    assert!(!aggregated.is_empty(), "Should return aggregated data");
    assert!(aggregated.len() <= 20, "Should be reasonably compressed, got {}", aggregated.len());

    let mut total_volume = 0.0;
    for item in &aggregated {
        if let PlotData::Ohlcv(candle) = item {
            total_volume += candle.volume;
            assert!(candle.high >= 25.0);
            assert!(candle.low <= 5.0);
        } else {
            panic!("Expected Ohlcv data");
        }
    }

    // Total volume should be preserved (100 * 100 = 10000)
    assert_eq!(total_volume, 10000.0);
}
