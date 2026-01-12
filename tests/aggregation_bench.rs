#[cfg(feature = "polars")]
use gpui_chart::polars_source::PolarsDataSource;
#[cfg(feature = "polars")]
use gpui_chart::data_types::{PlotDataSource, AggregationMode};
#[cfg(feature = "polars")]
use polars::prelude::*;
#[cfg(feature = "polars")]
use std::time::Instant;

#[test]
#[cfg(feature = "polars")]
fn test_compare_minmax_vs_m4() {
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.01).sin()).collect();
    
    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ]).unwrap();

    let source_minmax = PolarsDataSource::new(df.clone(), "x", "y")
        .with_aggregation_mode(AggregationMode::MinMax);
    
    let source_m4 = PolarsDataSource::new(df, "x", "y")
        .with_aggregation_mode(AggregationMode::M4);
    
    println!("\n--- Aggregation Benchmark (1M rows) ---");

    // MinMax Benchmark
    let start = Instant::now();
    let res_minmax: Vec<_> = source_minmax.iter_aggregated(0.0, n as f64, 2000).collect();
    let dur_minmax = start.elapsed();
    println!("MinMax (2 pts/bin) took: {:?} (points: {})", dur_minmax, res_minmax.len());

    // M4 Benchmark
    let start = Instant::now();
    let res_m4: Vec<_> = source_m4.iter_aggregated(0.0, n as f64, 2000).collect();
    let dur_m4 = start.elapsed();
    println!("M4 (4 pts/bin) took:     {:?} (points: {})", dur_m4, res_m4.len());

    assert!(res_minmax.len() <= 2000);
    assert!(res_m4.len() <= 2000);
}
