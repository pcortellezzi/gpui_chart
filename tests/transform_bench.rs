use gpui::{px, Point};
use gpui_chart::data_types::{ColorOp, PlotData, PlotPoint};
use gpui_chart::simd::batch_transform_points;
use std::time::Instant;

#[test]
fn bench_batch_transform() {
    let n = 1_000_000;
    let data: Vec<PlotData> = (0..n)
        .map(|i| {
            PlotData::Point(PlotPoint {
                x: i as f64,
                y: (i as f64).sin(),
                color_op: ColorOp::None,
            })
        })
        .collect();

    let x_scale = 0.5;
    let x_offset = 10.0;
    let y_scale = 50.0;
    let y_offset = 300.0;

    let mut output = Vec::with_capacity(n);

    // Warmup
    batch_transform_points(&data, x_scale, x_offset, y_scale, y_offset, &mut output);
    output.clear();

    // Benchmark SIMD/Batch
    let start_simd = Instant::now();
    batch_transform_points(&data, x_scale, x_offset, y_scale, y_offset, &mut output);
    let dur_simd = start_simd.elapsed();

    println!("Batch Transform (1M points) took: {:?}", dur_simd);

    // Benchmark Naive Loop
    output.clear();
    let start_naive = Instant::now();
    for item in &data {
        if let PlotData::Point(p) = item {
            let sx = p.x as f32 * x_scale + x_offset;
            let sy = p.y as f32 * y_scale + y_offset;
            output.push(Point::new(px(sx), px(sy)));
        }
    }
    let dur_naive = start_naive.elapsed();

    println!("Naive Loop (1M points) took:      {:?}", dur_naive);
    println!(
        "Speedup: {:.2}x",
        dur_naive.as_secs_f64() / dur_simd.as_secs_f64()
    );
}
