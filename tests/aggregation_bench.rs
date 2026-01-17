#[cfg(feature = "polars")]
use gpui_chart::decimation::decimate_ilttb_arrays_par_into;
#[cfg(feature = "polars")]
use gpui_chart::data_types::{AggregationMode, PlotDataSource};
#[cfg(feature = "polars")]
use gpui_chart::gaps::{GapIndex, GapSegment};
#[cfg(feature = "polars")]
use gpui_chart::polars_source::PolarsDataSource;
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
    ])
    .unwrap();

    let source_minmax =
        PolarsDataSource::new(df.clone(), "x", "y").with_aggregation_mode(AggregationMode::MinMax);

    let source_m4 =
        PolarsDataSource::new(df.clone(), "x", "y").with_aggregation_mode(AggregationMode::M4);

    let source_lttb =
        PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::LTTB);

    // Create a GapIndex with 100 gaps
    let mut segments = Vec::new();
    for i in 0..100 {
        segments.push(GapSegment {
            start_real: (i * 10000 + 5000) as i64,
            end_real: (i * 10000 + 6000) as i64,
            cumulative_before: 0,
        });
    }
    let gaps = GapIndex::new(segments);

    println!("\n--- Aggregation Benchmark (1M rows) ---");

    // MinMax Benchmark
    let start = Instant::now();
    let res_minmax: Vec<_> = source_minmax
        .iter_aggregated(0.0, n as f64, 2000, None)
        .collect();
    let dur_minmax = start.elapsed();
    println!(
        "MinMax (2 pts/bin) took:      {:?} (points: {})",
        dur_minmax,
        res_minmax.len()
    );

    let start = Instant::now();
    let res_minmax_gaps: Vec<_> = source_minmax
        .iter_aggregated(0.0, n as f64, 2000, Some(&gaps))
        .collect();
    let dur_minmax_gaps = start.elapsed();
    println!(
        "MinMax WITH GAPS took:        {:?} (points: {})",
        dur_minmax_gaps,
        res_minmax_gaps.len()
    );

    // M4 Benchmark
    let start = Instant::now();
    let res_m4: Vec<_> = source_m4
        .iter_aggregated(0.0, n as f64, 2000, None)
        .collect();
    let dur_m4 = start.elapsed();
    println!(
        "M4 (4 pts/bin) took:          {:?} (points: {})",
        dur_m4,
        res_m4.len()
    );

    let start = Instant::now();
    let res_m4_gaps: Vec<_> = source_m4
        .iter_aggregated(0.0, n as f64, 2000, Some(&gaps))
        .collect();
    let dur_m4_gaps = start.elapsed();
    println!(
        "M4 WITH GAPS took:            {:?} (points: {})",
        dur_m4_gaps,
        res_m4_gaps.len()
    );

    // LTTB Benchmark (Sequential)
    let start = Instant::now();
    let res_lttb: Vec<_> = source_lttb
        .iter_aggregated(0.0, n as f64, 2000, None)
        .collect();
    let dur_lttb = start.elapsed();
    println!(
        "LTTB (Sequential) took:       {:?} (points: {})",
        dur_lttb,
        res_lttb.len()
    );

    // ILTTB Benchmark (Parallel)
    let mut out_buffer = Vec::with_capacity(2000);
    let x_arr: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y_arr: Vec<f64> = (0..n).map(|i| (i as f64 * 0.01).sin()).collect();
    let start_ilttb = Instant::now();
    decimate_ilttb_arrays_par_into(&x_arr, &y_arr, 2000, &mut out_buffer, None, None);
    let dur_ilttb = start_ilttb.elapsed();
    println!(
        "ILTTB (Parallel) took:        {:?} (points: {})",
        dur_ilttb,
        out_buffer.len()
    );

    let mut out_buffer_gaps = Vec::with_capacity(2000);
    let start_ilttb_gaps = Instant::now();
    decimate_ilttb_arrays_par_into(&x_arr, &y_arr, 2000, &mut out_buffer_gaps, Some(&gaps), None);
    let dur_ilttb_gaps = start_ilttb_gaps.elapsed();
    println!(
        "ILTTB WITH GAPS took:         {:?} (points: {})",
        dur_ilttb_gaps,
        out_buffer_gaps.len()
    );

    // OHLCV Benchmark
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
    ])
    .unwrap();

    let source_ohlcv =
        PolarsDataSource::new(df_ohlcv, "time", "close").with_ohlcv("open", "high", "low", "close");

    let start = Instant::now();
    let res_ohlcv: Vec<_> = source_ohlcv
        .iter_aggregated(0.0, n as f64, 2000, None)
        .collect();
    let dur_ohlcv = start.elapsed();
    println!(
        "OHLCV (1 pt/bin) took:        {:?} (points: {})",
        dur_ohlcv,
        res_ohlcv.len()
    );

    let start = Instant::now();
    let res_ohlcv_gaps: Vec<_> = source_ohlcv
        .iter_aggregated(0.0, n as f64, 2000, Some(&gaps))
        .collect();
    let dur_ohlcv_gaps = start.elapsed();
    println!(
        "OHLCV WITH GAPS took:         {:?} (points: {})",
        dur_ohlcv_gaps,
        res_ohlcv_gaps.len()
    );

    assert!(res_minmax.len() <= 2000);
    assert!(res_m4.len() <= 2000);
    assert!(res_lttb.len() <= 2000);
    assert!(out_buffer.len() <= 2000);
    assert!(res_ohlcv.len() <= 2000);
}
