use gpui_chart::data_types::{ColorOp, PlotData, PlotDataSource, PlotPoint, VecDataSource};
use gpui_chart::gaps::{GapIndex, GapSegment};
use std::time::Instant;

#[test]
fn test_gap_performance_impact() {
    let n = 1_000_000;
    println!("Generating {} points...", n);
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        data.push(PlotData::Point(PlotPoint {
            x: i as f64,
            y: (i as f64 * 0.01).sin(),
            color_op: ColorOp::None,
        }));
    }
    let source = VecDataSource::new(data);

    // 1. Create a GapIndex with 1000 segments
    // Simulating 1000 nights/weekends over a long period
    let mut segments = Vec::new();
    for i in 0..1000 {
        segments.push(GapSegment {
            start_real: (i * 2000 + 1500) as i64,
            end_real: (i * 2000 + 2000) as i64,
            cumulative_before: 0,
        });
    }
    let gaps = GapIndex::new(segments);

    let max_points = 2000;
    let mut output = Vec::with_capacity(max_points);

    // Benchmark aggregation WITHOUT gaps
    let start_no_gaps = Instant::now();
    source.get_aggregated_data(0.0, (n * 2) as f64, max_points, &mut output, None);
    let dur_no_gaps = start_no_gaps.elapsed();
    println!("Aggregation WITHOUT gaps: {:?}", dur_no_gaps);

    // Benchmark aggregation WITH gaps (Scenario B - Splitting)
    // Note: get_aggregated_data will split the range into many intervals
    let start_with_gaps = Instant::now();
    source.get_aggregated_data(0.0, (n * 2) as f64, max_points, &mut output, Some(&gaps));
    let dur_with_gaps = start_with_gaps.elapsed();
    println!(
        "Aggregation WITH 1000 gaps (Scenario B): {:?}",
        dur_with_gaps
    );

    // Benchmark raw mapping performance (to_logical)
    // This is called many times during rendering and interactions
    let iterations = 100_000;
    let start_mapping = Instant::now();
    let mut dummy_sum = 0;
    for i in 0..iterations {
        dummy_sum += gaps.to_logical(i as i64);
    }
    let dur_mapping = start_mapping.elapsed();
    println!(
        "Mapping (to_logical) 100,000 iterations: {:?} (avg per call: {:?})",
        dur_mapping,
        dur_mapping / iterations as u32
    );

    // Assertions to ensure performance stays within acceptable bounds for 60FPS
    // Even with 1000 gaps, aggregation of 1M rows should be fast.
    assert!(
        dur_with_gaps.as_millis() < 50,
        "Aggregation with gaps should be < 50ms"
    );
    assert!(
        dur_mapping.as_millis() < 10,
        "100k mappings should be < 10ms"
    );

    // Just use dummy_sum to prevent compiler optimization
    assert!(dummy_sum != 0);
}
