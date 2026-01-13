#[cfg(feature = "polars")]
use gpui_chart::polars_source::PolarsDataSource;
#[cfg(feature = "polars")]
use gpui_chart::data_types::{PlotData, PlotDataSource, AggregationMode};
#[cfg(feature = "polars")]
use polars::prelude::*;

#[test]
#[cfg(feature = "polars")]
fn test_multi_source_synchronization() {
    let n = 5000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.05).sin()).collect();
    
    // 1. Polars Source
    let df = df!(
        "x" => &x,
        "y" => &y
    ).unwrap();
    let polars_source = PolarsDataSource::new(df, "x", "y")
        .with_aggregation_mode(AggregationMode::M4);
    
    // 2. We use the same decimation logic manually (simulating a VecDataSource)
    let max_points = 200;
    let polars_res: Vec<PlotData> = polars_source.iter_aggregated(0.0, (n-1) as f64, max_points).collect();
    let manual_res = gpui_chart::aggregation::decimate_m4_arrays_par(&x, &y, max_points);
    
    // They should have the same number of points and very similar X values
    assert!(polars_res.len() > 0);
    // Note: Due to slightly different binning implementations (lazy polars vs par_chunks), 
    // we allow a small margin but they must be roughly aligned.
    
    let diff = (polars_res.len() as isize - manual_res.len() as isize).abs();
    assert!(diff < 10, "Point count mismatch between sources: Polars={}, Manual={}", polars_res.len(), manual_res.len());

    // Check first and last point alignment
    if let (PlotData::Point(p1_start), PlotData::Point(p2_start)) = (&polars_res[0], &manual_res[0]) {
        assert!((p1_start.x - p2_start.x).abs() < 1.0, "Start point misalignment");
    }
    
    if let (PlotData::Point(p1_end), PlotData::Point(p2_end)) = (polars_res.last().unwrap(), manual_res.last().unwrap()) {
        assert!((p1_end.x - p2_end.x).abs() < 1.0, "End point misalignment");
    }
}
