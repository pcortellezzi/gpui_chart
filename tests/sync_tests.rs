use gpui_chart::data_types::{AggregationMode, PlotData, PlotDataSource};
use gpui_chart::polars_source::PolarsDataSource;
use polars::prelude::*;

#[test]
fn test_polars_sync_with_manual() {
    let n = 10000;
    let mut x = Vec::with_capacity(n);
    let mut y = Vec::with_capacity(n);
    for i in 0..n {
        x.push(i as f64);
        y.push((i as f64 * 0.01).sin());
    }

    let df = DataFrame::new(vec![
        Series::new("x".into(), &x).into(),
        Series::new("y".into(), &y).into(),
    ])
    .unwrap();

    let polars_source =
        PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::M4);

    let max_points = 1000;
    let polars_res: Vec<PlotData> = polars_source
        .iter_aggregated(0.0, (n - 1) as f64, max_points, None)
        .collect();
    let manual_res = gpui_chart::decimation::decimate_m4_arrays_par(&x, &y, max_points, None, 0);

    assert_eq!(polars_res.len(), manual_res.len());

    for (p, m) in polars_res.iter().zip(manual_res.iter()) {
        if let (PlotData::Point(pp), PlotData::Point(mp)) = (p, m) {
            assert!((pp.x - mp.x).abs() < 1e-10);
            assert!((pp.y - mp.y).abs() < 1e-10);
        }
    }
}
