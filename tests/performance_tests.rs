use gpui_chart::data_types::{PlotPoint, ColorOp, PlotDataSource, VecDataSource, PlotData};
use std::time::Instant;

#[test]
fn test_vec_datasource_performance() {
    let mut data = Vec::new();
    let count = 100_000;
    for i in 0..count {
        data.push(PlotData::Point(PlotPoint {
            x: i as f64,
            y: (i as f64 * 0.01).sin() * 100.0,
            color_op: ColorOp::None,
        }));
    }

    let start_init = Instant::now();
    let source = VecDataSource::new(data);
    let duration_init = start_init.elapsed();
    println!("Initialization of {} points: {:?}", count, duration_init);

    // Test get_y_range performance (Auto-fit simulation)
    let start_range = Instant::now();
    for _ in 0..100 {
        let _ = source.get_y_range(40000.0, 60000.0);
    }
    let duration_range = start_range.elapsed() / 100;
    println!("Average get_y_range (20% window): {:?}", duration_range);

    // Test iter_range performance (Culling simulation)
    let start_iter = Instant::now();
    let count_iter = source.iter_range(40000.0, 41000.0).count();
    let duration_iter = start_iter.elapsed();
    println!("Iterate over {} visible points: {:?}", count_iter, duration_iter);

    assert!(duration_range.as_micros() < 500, "Auto-fit should be very fast thanks to cache");
    assert!(duration_iter.as_micros() < 500, "Culling iteration should be sub-millisecond");
}
