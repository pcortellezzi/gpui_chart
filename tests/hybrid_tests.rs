use gpui_chart::data_types::{ColorOp, PlotData, PlotDataSource, PlotPoint};
use gpui_chart::hybrid_source::HybridDataSource;
#[cfg(feature = "polars")]
use gpui_chart::polars_source::PolarsDataSource;
#[cfg(feature = "polars")]
use polars::prelude::*;

#[test]
#[cfg(feature = "polars")]
fn test_hybrid_data_integrity() {
    let n_hist = 1000;
    let x: Vec<f64> = (0..n_hist).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n_hist).map(|i| i as f64).collect();

    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ])
    .unwrap();

    let historical = Box::new(PolarsDataSource::new(df, "x", "y"));
    let mut hybrid = HybridDataSource::new(historical, 5000);

    // 1. Add 500 realtime points
    for i in 0..500 {
        hybrid.add_realtime(PlotData::Point(PlotPoint {
            x: (n_hist + i) as f64,
            y: (n_hist + i) as f64,
            color_op: ColorOp::None,
        }));
    }

    assert_eq!(hybrid.len(), 1500, "Should contain hist + realtime");

    // 2. Commit
    hybrid.commit_realtime_to_historical();

    assert_eq!(hybrid.len(), 1500, "Len should be preserved after commit");

    // 3. Verify order and values
    let all_points: Vec<_> = hybrid.iter_range(0.0, 2000.0).collect();
    assert_eq!(all_points.len(), 1500);

    if let PlotData::Point(p) = &all_points[0] {
        assert_eq!(p.x, 0.0);
    }
    if let PlotData::Point(p) = &all_points[1499] {
        assert_eq!(p.x, 1499.0);
    }
}

#[test]
#[cfg(feature = "polars")]
fn test_hybrid_empty_historical() {
    let df = DataFrame::new(vec![
        Series::new("x".into(), Vec::<f64>::new()).into(),
        Series::new("y".into(), Vec::<f64>::new()).into(),
    ])
    .unwrap();

    let historical = Box::new(PolarsDataSource::new(df, "x", "y"));
    let mut hybrid = HybridDataSource::new(historical, 1000);

    hybrid.add_realtime(PlotData::Point(PlotPoint {
        x: 10.0,
        y: 20.0,
        color_op: ColorOp::None,
    }));

    assert_eq!(hybrid.len(), 1);
    let bounds = hybrid.get_bounds().unwrap();
    assert_eq!(bounds.0, 10.0);

    hybrid.commit_realtime_to_historical();
    assert_eq!(hybrid.len(), 1);
}

#[test]
#[cfg(feature = "polars")]
fn test_hybrid_aggregation_consistency() {
    let n = 10000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ])
    .unwrap();

    let historical = Box::new(PolarsDataSource::new(df, "x", "y"));
    let hybrid = HybridDataSource::new(historical, 1000);

    // Request aggregation on an empty range
    let mut output = Vec::new();
    hybrid.get_aggregated_data(20000.0, 30000.0, 100, &mut output, None);
    assert_eq!(output.len(), 0);

    // Request aggregation on the existing range
    hybrid.get_aggregated_data(0.0, 10000.0, 100, &mut output, None);
    assert!(output.len() > 0 && output.len() <= 100);
}
