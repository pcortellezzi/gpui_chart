use gpui_chart::decimation::*;
#[cfg(feature = "polars")]
use gpui_chart::data_types::AggregationMode;
use gpui_chart::data_types::{ColorOp, PlotData, PlotDataSource, PlotPoint};
#[cfg(feature = "polars")]
use gpui_chart::polars_source::PolarsDataSource;
#[cfg(feature = "polars")]
use polars::prelude::*;
use std::time::Instant;

#[test]
fn test_aggregation_edge_cases_generic() {
    // 0 points
    let data: Vec<PlotData> = vec![];
    assert_eq!(decimate_min_max_slice(&data, 100, None).len(), 0);
    assert_eq!(decimate_m4_slice(&data, 100, None).len(), 0);
    assert_eq!(decimate_lttb_slice(&data, 100, None).len(), 0);

    // 1 point
    let data = vec![PlotData::Point(PlotPoint {
        x: 0.0,
        y: 0.0,
        color_op: ColorOp::None,
    })];
    assert_eq!(decimate_min_max_slice(&data, 100, None).len(), 1);
    assert_eq!(decimate_m4_slice(&data, 100, None).len(), 1);
    assert_eq!(decimate_lttb_slice(&data, 100, None).len(), 1);

    // 2 points
    let data = vec![
        PlotData::Point(PlotPoint {
            x: 0.0,
            y: 0.0,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 1.0,
            y: 1.0,
            color_op: ColorOp::None,
        }),
    ];
    assert_eq!(decimate_min_max_slice(&data, 100, None).len(), 2);
    assert_eq!(decimate_m4_slice(&data, 100, None).len(), 2);
    assert_eq!(decimate_lttb_slice(&data, 100, None).len(), 2);

    // Max points < 1
    assert_eq!(decimate_min_max_slice(&data, 0, None).len(), 0);
    // M4 with max_points = 2 should still handle it (it uses max_points/4 which is 0, but max(1))
    let data_many: Vec<PlotData> = (0..100)
        .map(|i| {
            PlotData::Point(PlotPoint {
                x: i as f64,
                y: i as f64,
                color_op: ColorOp::None,
            })
        })
        .collect();
    assert!(decimate_m4_slice(&data_many, 2, None).len() <= 2);
}

#[test]
fn test_aggregation_nan_inf_generic() {
    let data = vec![
        PlotData::Point(PlotPoint {
            x: 0.0,
            y: f64::NAN,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 1.0,
            y: 1.0,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 2.0,
            y: f64::INFINITY,
            color_op: ColorOp::None,
        }),
        PlotData::Point(PlotPoint {
            x: 3.0,
            y: f64::NEG_INFINITY,
            color_op: ColorOp::None,
        }),
    ];

    // Min/Max
    let decimated = decimate_min_max_slice(&data, 2, None);
    assert_eq!(decimated.len(), 2);
    // Should not crash

    // M4
    let decimated = decimate_m4_slice(&data, 4, None);
    assert!(decimated.len() > 0);

    // LTTB
    let decimated = decimate_lttb_slice(&data, 3, None);
    assert!(decimated.len() > 0);
}

#[test]
#[cfg(feature = "polars")]
fn test_qa_polars_nan_inf() {
    let df = df!(
        "x" => &[0.0, 1.0, 2.0, 3.0, 4.0],
        "y" => &[f64::NAN, 1.0, f64::INFINITY, f64::NEG_INFINITY, 2.0]
    )
    .unwrap();

    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::M4);

    // Bounds check
    let bounds = source.get_bounds();
    // Polars min/max usually skip NaN if there are other values, but depends on version.
    // In Polars 0.51, min/max on series with NaN might return NaN or skip it.
    println!("Bounds with NaN/Inf: {:?}", bounds);

    // Aggregation check
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, 4.0, 4, None).collect();
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
    )
    .unwrap();

    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::MinMax);

    // Aggregate over a range including the gap
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, 110.0, 4, None).collect();
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
    ])
    .unwrap();

    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::M4);

    let start = Instant::now();
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, n as f64, 2000, None).collect();
    let duration = start.elapsed();

    println!(
        "Decimation of 5M rows to 2000 points (M4) took: {:?}",
        duration
    );
    // Target is < 20ms for 1M, so for 5M < 100ms is acceptable.
    // However, on a fast machine 5M should also be around 50-80ms.
    assert!(
        duration.as_millis() < 500,
        "Should be relatively fast even for 5M rows"
    );
    assert!(decimated.len() <= 2000);
}

#[test]
#[cfg(feature = "polars")]
fn test_qa_polars_minmax_performance() {
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64).sin()).collect();
    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ])
    .unwrap();
    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::MinMax);

    let start = Instant::now();
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, n as f64, 2000, None).collect();
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
    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ])
    .unwrap();
    let source = PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::LTTB);

    let start = Instant::now();
    let decimated: Vec<PlotData> = source.iter_aggregated(0.0, n as f64, 2000, None).collect();
    println!("LTTB 1M rows: {:?}", start.elapsed());

    // LTTB is serial, so it might be slower than M4/MinMax, but Zero-Copy should keep it fast.
    // Target < 50ms.
    assert!(start.elapsed().as_millis() < 50, "LTTB should be < 50ms");
    assert!(decimated.len() >= 100); // LTTB is stable, count is approximate
}

#[test]
#[cfg(feature = "polars")]
fn test_qa_polars_mode_switching() {
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..1000).map(|i| i as f64).collect();

    let df = DataFrame::new(vec![
        Series::new("x".into(), x).into(),
        Series::new("y".into(), y).into(),
    ])
    .unwrap();

    let mut source = PolarsDataSource::new(df, "x", "y");

    // MinMax
    source = source.with_aggregation_mode(AggregationMode::MinMax);
    let decimated_mm: Vec<PlotData> = source.iter_aggregated(0.0, 1000.0, 100, None).collect();
    assert!(decimated_mm.len() <= 100);

    // M4
    source = source.with_aggregation_mode(AggregationMode::M4);
    let decimated_m4: Vec<PlotData> = source.iter_aggregated(0.0, 1000.0, 100, None).collect();
    assert!(decimated_m4.len() <= 100);

    // LTTB
    source = source.with_aggregation_mode(AggregationMode::LTTB);
    let decimated_lttb: Vec<PlotData> = source.iter_aggregated(0.0, 1000.0, 100, None).collect();
    assert!(decimated_lttb.len() >= 10); // Stable binning produces approx max_points
}

#[test]
fn test_lttb_visual_stability() {
    // LTTB should produce approximately max_points if input is large enough
    let data: Vec<PlotData> = (0..100)
        .map(|i| {
            PlotData::Point(PlotPoint {
                x: i as f64,
                y: i as f64,
                color_op: ColorOp::None,
            })
        })
        .collect();
    let decimated = decimate_lttb_slice(&data, 10, None);
    assert!(decimated.len() >= 2);

    // First and last points should be preserved
    let final_idx = decimated.len() - 1;
    if let (PlotData::Point(p_first), PlotData::Point(p_last)) = (&decimated[0], &decimated[final_idx]) {
        assert_eq!(p_first.x, 0.0);
        assert_eq!(p_last.x, 99.0);
    } else {
        panic!("Expected Points");
    }
}

#[test]
fn test_ilttb_parallel_correctness() {
    let n = 10000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n)
        .map(|i| {
            if i == 500 {
                1000.0
            } else if i == 5000 {
                -1000.0
            } else {
                (i as f64 * 0.1).sin()
            }
        })
        .collect();

    // 1. Reference: Sequential LTTB
    let mut res_seq = Vec::new();
    decimate_lttb_arrays_into(&x, &y, 100, &mut res_seq, None);

    // 2. Target: Parallel ILTTB
    let mut res_par = Vec::new();
    decimate_ilttb_arrays_par_into(&x, &y, 100, &mut res_par, None);

    // Both must preserve the extreme peaks
    let has_top_peak_par = res_par.iter().any(|p| match p {
        PlotData::Point(pt) => pt.y == 1000.0,
        _ => false,
    });
    let has_bot_peak_par = res_par.iter().any(|p| match p {
        PlotData::Point(pt) => pt.y == -1000.0,
        _ => false,
    });

    assert!(has_top_peak_par, "Parallel ILTTB missed the top peak");
    assert!(has_bot_peak_par, "Parallel ILTTB missed the bottom peak");

    // General trend check
    let mean_y_seq = res_seq
        .iter()
        .map(|p| match p {
            PlotData::Point(pt) => pt.y,
            _ => 0.0,
        })
        .sum::<f64>()
        / res_seq.len() as f64;
    let mean_y_par = res_par
        .iter()
        .map(|p| match p {
            PlotData::Point(pt) => pt.y,
            _ => 0.0,
        })
        .sum::<f64>()
        / res_par.len() as f64;

    assert!(
        (mean_y_seq - mean_y_par).abs() < 1.0,
        "General trend mismatch between Sequential and Parallel LTTB"
    );

    // First and Last points must be identical
    assert_eq!(res_seq[0], res_par[0]);
    assert_eq!(res_seq[res_seq.len() - 1], res_par[res_par.len() - 1]);
}
