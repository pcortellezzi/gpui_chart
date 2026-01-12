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
fn test_compare_aggregations() {
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.01).sin()).collect();
    
    let df = DataFrame::new(vec![
        Series::new("x".into(), x.clone()).into(),
        Series::new("y".into(), y).into(),
    ]).unwrap();

    let source_minmax = PolarsDataSource::new(df.clone(), "x", "y")
        .with_aggregation_mode(AggregationMode::MinMax);
    
    let source_m4 = PolarsDataSource::new(df.clone(), "x", "y")
        .with_aggregation_mode(AggregationMode::M4);

    let source_lttb = PolarsDataSource::new(df, "x", "y")
        .with_aggregation_mode(AggregationMode::LTTB);
    
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

    // LTTB Benchmark
    let start = Instant::now();
    let res_lttb: Vec<_> = source_lttb.iter_aggregated(0.0, n as f64, 2000).collect();
    let dur_lttb = start.elapsed();
    println!("LTTB (Variable) took:    {:?} (points: {})", dur_lttb, res_lttb.len());

    assert!(res_minmax.len() <= 2000);
    assert!(res_m4.len() <= 2000);
    assert!(res_lttb.len() <= 2000);

    // OHLCV Benchmark
    // Construct OHLCV data
    let open: Vec<f64> = (0..n).map(|i| (i as f64).sin()).collect();
    let high: Vec<f64> = (0..n).map(|i| (i as f64).sin() + 1.0).collect();
    let low: Vec<f64> = (0..n).map(|i| (i as f64).sin() - 1.0).collect();
    let close: Vec<f64> = (0..n).map(|i| (i as f64).cos()).collect();

    let df_ohlcv = DataFrame::new(vec![
        Series::new("time".into(), x.clone()).into(),
        Series::new("open".into(), open).into(),
        Series::new("high".into(), high).into(),
        Series::new("low".into(), low).into(),
        Series::new("close".into(), close).into(),
    ]).unwrap();

    let source_ohlcv = PolarsDataSource::new(df_ohlcv, "time", "close")
        .with_ohlcv("open", "high", "low", "close");
    
    let start = Instant::now();
    let res_ohlcv: Vec<_> = source_ohlcv.iter_aggregated(0.0, n as f64, 2000).collect();
    let dur_ohlcv = start.elapsed();
    println!("OHLCV (1 pt/bin) took:   {:?} (points: {})", dur_ohlcv, res_ohlcv.len());
    
    assert!(res_ohlcv.len() <= 2000);
}
