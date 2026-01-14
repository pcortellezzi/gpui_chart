use gpui_chart::aggregation::decimate_ilttb_arrays_par_into;
#[cfg(feature = "polars")]
use gpui_chart::data_types::{AggregationMode, PlotDataSource};
use gpui_chart::gpu_renderer::{BladeComputeRenderer, GpuAggregationMode, GpuPoint};
#[cfg(feature = "polars")]
use gpui_chart::polars_source::PolarsDataSource;
#[cfg(feature = "polars")]
use polars::prelude::*;
use std::time::Instant;

#[test]
#[cfg(feature = "polars")]
fn test_lttb_ultimate_stress_100m() {
    let n = 100_000_000;
    println!("\n--- LTTB/ILTTB ULTIMATE STRESS TEST ({} Points) ---", n);

    // 1. Data Generation (f64 for CPU, f32 for GPU)
    let start_gen = Instant::now();
    let x_f64: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y_f64: Vec<f64> = (0..n).map(|i| (i as f64 * 0.001).sin()).collect();
    println!("Generation took {:?}", start_gen.elapsed());

    // --- 1. LTTB CPU SEQUENTIAL ---
    let df = DataFrame::new(vec![
        Series::new("x".into(), x_f64.clone()).into(),
        Series::new("y".into(), y_f64.clone()).into(),
    ])
    .unwrap();
    let source_lttb =
        PolarsDataSource::new(df, "x", "y").with_aggregation_mode(AggregationMode::LTTB);

    let start_seq = Instant::now();
    let _ = source_lttb
        .iter_aggregated(0.0, n as f64, 2000, None)
        .count();
    let dur_seq = start_seq.elapsed();
    println!("LTTB CPU (Sequential) took: {:?}", dur_seq);

    // --- 2. ILTTB CPU PARALLEL (RAYON) ---
    let mut out_cpu = Vec::with_capacity(2000);
    let start_par = Instant::now();
    decimate_ilttb_arrays_par_into(&x_f64, &y_f64, 2000, &mut out_cpu, None);
    let dur_par = start_par.elapsed();
    println!("ILTTB CPU (Parallel)   took: {:?}", dur_par);

    // Free some memory before GPU
    drop(x_f64);
    drop(y_f64);

    // --- 3. ILTTB GPU (BLADE) ---
    let gpu_renderer = BladeComputeRenderer::new().expect("GPU init failed");
    // Re-generate only f32 points for GPU to save RAM
    let gpu_points: Vec<GpuPoint> = (0..n)
        .map(|i| GpuPoint {
            x: i as f32,
            y: (i as f32 * 0.001).sin(),
        })
        .collect();

    let start_gpu = Instant::now();
    let res_gpu = gpu_renderer
        .aggregate(&gpu_points, 0.0, n as f32, 2000, GpuAggregationMode::ILTTB)
        .unwrap();
    let dur_gpu = start_gpu.elapsed();
    println!(
        "ILTTB GPU (Blade)      took: {:?} (Transfers incl.)",
        dur_gpu
    );

    println!("\n--- FINAL VERDICT (100M Points) ---");
    println!(
        "Parallel CPU vs Seq CPU: {:.2}x faster",
        dur_seq.as_secs_f64() / dur_par.as_secs_f64()
    );
    println!(
        "GPU vs Seq CPU:          {:.2}x faster",
        dur_seq.as_secs_f64() / dur_gpu.as_secs_f64()
    );
    println!(
        "GPU vs Parallel CPU:     {:.2}x faster",
        dur_par.as_secs_f64() / dur_gpu.as_secs_f64()
    );

    assert_eq!(res_gpu.len(), 2000);
}
