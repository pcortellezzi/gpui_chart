#[cfg(feature = "polars")]
use gpui_chart::data_types::{PlotData, PlotDataSource};
#[cfg(feature = "polars")]
use gpui_chart::polars_source::PolarsDataSource;
#[cfg(feature = "polars")]
use polars::prelude::*;
#[cfg(feature = "polars")]
use std::time::Instant;

#[test]
#[cfg(feature = "polars")]
fn test_polars_datasource_basic() {
    let df = df!(
        "x" => &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        "y" => &[10.0, 20.0, 15.0, 25.0, 20.0, 30.0]
    )
    .unwrap();

    let source = PolarsDataSource::new(df, "x", "y");
    assert_eq!(source.len(), 6);

    let bounds = source.get_bounds().unwrap();
    assert_eq!(bounds.0, 0.0);
    assert_eq!(bounds.1, 5.0);
    assert_eq!(bounds.2, 10.0);
    assert_eq!(bounds.3, 30.0);

    let y_range = source.get_y_range(1.5, 3.5).unwrap();
    assert_eq!(y_range.0, 15.0);
    assert_eq!(y_range.1, 25.0);
}

#[test]
#[cfg(feature = "polars")]
fn test_polars_datasource_aggregation() {
    // Create 1000 points (Sine wave to avoid deduplication of collinear points)
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.1).sin()).collect();

    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ])
    .unwrap();

    let source = PolarsDataSource::new(df, "x", "y");

    // Aggregate to 100 points
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, 1000.0, 100, None).collect();

    // M4 decimation returns up to 4 points per bin.
    // 1000 points / 25 bins = 40 points per bin.
    // Sine wave should trigger min/max distinct from first/last most of the time.
    assert!(decimated.len() <= 100);
    assert!(decimated.len() >= 40);
}

#[test]
#[cfg(feature = "polars")]
fn test_polars_performance_1m_rows() {
    println!("Generating 1M rows...");
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64).sin()).collect();

    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ])
    .unwrap();

    let source = PolarsDataSource::new(df, "x", "y");

    let start = Instant::now();
    // Simulate rendering 2000 points (typical screen width)
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, n as f64, 2000, None).collect();
    let duration = start.elapsed();

    println!("Decimation of 1M rows to 2000 points took: {:?}", duration);
    // Allow slight variance in exact count due to parallel chunking and stable binning
    assert!(decimated.len() <= 2000);
    assert!(decimated.len() >= 1500);

    // Target is < 10ms for aggregation logic (Native Rust + Rayon)
    assert!(
        duration.as_millis() < 20,
        "Decimation should be extremely fast (<20ms)"
    );
}
