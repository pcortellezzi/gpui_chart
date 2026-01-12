use gpui_chart::aggregation::*;
use gpui_chart::data_types::{PlotData, PlotPoint, ColorOp, AggregationMode, PlotDataSource};
#[cfg(feature = "polars")]
use gpui_chart::polars_source::PolarsDataSource;
#[cfg(feature = "polars")]
use polars::prelude::*;
use std::time::Instant;

#[test]
fn test_aggregation_edge_cases_generic() {
    // 0 points
    let data: Vec<PlotData> = vec![];
    assert_eq!(decimate_min_max_slice(&data, 100).len(), 0);
    assert_eq!(decimate_m4_slice(&data, 100).len(), 0);
    assert_eq!(decimate_lttb_slice(&data, 100).len(), 0);

    // 1 point
    let data = vec![PlotData::Point(PlotPoint { x: 0.0, y: 0.0, color_op: ColorOp::None })];
    assert_eq!(decimate_min_max_slice(&data, 100).len(), 1);
    assert_eq!(decimate_m4_slice(&data, 100).len(), 1);
    assert_eq!(decimate_lttb_slice(&data, 100).len(), 1);

    // 2 points
    let data = vec![
        PlotData::Point(PlotPoint { x: 0.0, y: 0.0, color_op: ColorOp::None }),
        PlotData::Point(PlotPoint { x: 1.0, y: 1.0, color_op: ColorOp::None }),
    ];
    assert_eq!(decimate_min_max_slice(&data, 100).len(), 2);
    assert_eq!(decimate_m4_slice(&data, 100).len(), 2);
    assert_eq!(decimate_lttb_slice(&data, 100).len(), 2);

    // Max points < 1
    assert_eq!(decimate_min_max_slice(&data, 0).len(), 0);
    // M4 with max_points = 2 should still handle it (it uses max_points/4 which is 0, but max(1))
    let data_many: Vec<PlotData> = (0..100).map(|i| PlotData::Point(PlotPoint { x: i as f64, y: i as f64, color_op: ColorOp::None })).collect();
    assert!(decimate_m4_slice(&data_many, 2).len() <= 2);
}

#[test]
fn test_aggregation_nan_inf_generic() {
    let data = vec![
        PlotData::Point(PlotPoint { x: 0.0, y: f64::NAN, color_op: ColorOp::None }),
        PlotData::Point(PlotPoint { x: 1.0, y: 1.0, color_op: ColorOp::None }),
        PlotData::Point(PlotPoint { x: 2.0, y: f64::INFINITY, color_op: ColorOp::None }),
        PlotData::Point(PlotPoint { x: 3.0, y: f64::NEG_INFINITY, color_op: ColorOp::None }),
    ];

    // Min/Max
    let decimated = decimate_min_max_slice(&data, 2);
    assert_eq!(decimated.len(), 2);
    // Should not crash

    // M4
    let decimated = decimate_m4_slice(&data, 4);
    assert!(decimated.len() > 0);

    // LTTB
    let decimated = decimate_lttb_slice(&data, 3);
    assert!(decimated.len() > 0);
}

#[test]
#[cfg(feature = "polars")]
fn test_qa_polars_nan_inf() {
    let df = df!(
        "x" => &[0.0, 1.0, 2.0, 3.0, 4.0],
        "y" => &[f64::NAN, 1.0, f64::INFINITY, f64::NEG_INFINITY, 2.0]
    ).unwrap();

    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::M4);
    
    // Bounds check
    let bounds = source.get_bounds();
    // Polars min/max usually skip NaN if there are other values, but depends on version.
    // In Polars 0.51, min/max on series with NaN might return NaN or skip it.
    println!("Bounds with NaN/Inf: {:?}", bounds);

    // Aggregation check
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, 4.0, 4).collect();
    assert!(decimated.len() > 0);
    for p in &decimated {
        if let PlotData::Point(pt) = p {
            println!("Point: x={}, y={}", pt.x, pt.y);
        }
    }
}

#[test]
#[cfg(feature = "polars")]
fn test_qa_polars_gaps() {
    // Gaps in X (non-uniform)
    let df = df!(
        "x" => &[0.0, 1.0, 10.0, 11.0, 100.0, 101.0],
        "y" => &[0.0, 1.0, 0.0, 1.0, 0.0, 1.0]
    ).unwrap();

    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::MinMax);
    
    // Aggregate over a range including the gap
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, 110.0, 4).collect();
    // We expect points from the start and end of the range, and potentially middle if bins allow.
    assert!(decimated.len() > 0);
}

#[test]
#[cfg(feature = "polars")]
fn test_qa_polars_massive_dataset() {
    let n = 5_000_000;
    println!("Generating {} rows...", n);
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.001).sin()).collect();
    
    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ]).unwrap();

    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::M4);
    
    let start = Instant::now();
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, n as f64, 2000).collect();
    let duration = start.elapsed();
    
    println!("Decimation of 5M rows to 2000 points (M4) took: {:?}", duration);
    // Target is < 20ms for 1M, so for 5M < 100ms is acceptable.
    // However, on a fast machine 5M should also be around 50-80ms.
    assert!(duration.as_millis() < 500, "Should be relatively fast even for 5M rows");
    assert!(decimated.len() <= 2000);
}

#[test]
#[cfg(feature = "polars")]
fn test_qa_polars_minmax_performance() {
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64).sin()).collect();
    let df = DataFrame::new(vec![Series::new("x".into(), x).into(), Series::new("y".into(), y).into()]).unwrap();
    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::MinMax);
    
    let start = Instant::now();
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, n as f64, 2000).collect();
    println!("MinMax 1M rows: {:?}", start.elapsed());
    assert!(start.elapsed().as_millis() < 20, "MinMax should be < 20ms");
    assert!(decimated.len() <= 2000);
}

#[test]
#[cfg(feature = "polars")]
fn test_qa_polars_lttb_performance() {
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64).sin()).collect();
    let df = DataFrame::new(vec![Series::new("x".into(), x).into(), Series::new("y".into(), y).into()]).unwrap();
    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::LTTB);
    
    let start = Instant::now();
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, n as f64, 2000).collect();
    println!("LTTB 1M rows: {:?}", start.elapsed());
    
    // LTTB is serial, so it might be slower than M4/MinMax, but Zero-Copy should keep it fast.
    // Target < 50ms.
    assert!(start.elapsed().as_millis() < 50, "LTTB should be < 50ms");
    assert_eq!(decimated.len(), 2000); // LTTB is exact
}

#[test]
#[cfg(feature = "polars")]
fn test_qa_polars_mode_switching() {
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    
    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ]).unwrap();

    let mut source = PolarsDataSource::new(df, "x", "y");
    
    // MinMax
    source = source.with_aggregation_mode(AggregationMode::MinMax);
    let decimated_mm: Vec<PlotData> = source.iter_aggregated(0.0, 1000.0, 100).collect();
    assert!(decimated_mm.len() <= 100);

    // M4
    source = source.with_aggregation_mode(AggregationMode::M4);
    let decimated_m4: Vec<PlotData> = source.iter_aggregated(0.0, 1000.0, 100).collect();
    assert!(decimated_m4.len() <= 100);

    // LTTB
    source = source.with_aggregation_mode(AggregationMode::LTTB);
    let decimated_lttb: Vec<PlotData> = source.iter_aggregated(0.0, 1000.0, 100).collect();
    assert_eq!(decimated_lttb.len(), 100); // LTTB is usually exact if N > max_points
}

#[test]
fn test_lttb_visual_stability() {
    // LTTB should produce exactly max_points if input is large enough
    let data: Vec<PlotData> = (0..100).map(|i| PlotData::Point(PlotPoint { x: i as f64, y: i as f64, color_op: ColorOp::None })).collect();
    let decimated = decimate_lttb_slice(&data, 10);
    assert_eq!(decimated.len(), 10);
    
    // First and last points should be preserved
    if let (PlotData::Point(p_first), PlotData::Point(p_last)) = (&decimated[0], &decimated[9]) {
        assert_eq!(p_first.x, 0.0);
        assert_eq!(p_last.x, 99.0);
    } else {
        panic!("Expected Points");
    }
}
