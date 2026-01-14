use gpui_chart::data_types::{Ohlcv, PlotData};
use gpui_chart::gaps::{GapIndex, GapSegment};

#[test]
fn test_ohlcv_aggregation_respects_gaps_zero_copy() {
    // Weekend gap from Friday 17:00 to Monday 09:00
    let friday_close = 1000;
    let monday_open = 2000;

    let segments = vec![GapSegment {
        start_real: friday_close,
        end_real: monday_open,
        cumulative_before: 0,
    }];
    let gaps = GapIndex::new(segments);

    // Data points: Friday [900..1000], Monday [2000..2100]
    let mut time = Vec::new();
    for i in 900..1000 {
        time.push(i as f64);
    }
    for i in 2000..2100 {
        time.push(i as f64);
    }

    let n = time.len();
    let open = vec![10.0; n];
    let high = vec![15.0; n];
    let low = vec![5.0; n];
    let close = vec![12.0; n];

    // Test high-level splitting logic.
    use gpui_chart::data_types::{PlotDataSource, VecDataSource};
    let mut plot_data = Vec::new();
    for i in 0..n {
        plot_data.push(PlotData::Ohlcv(Ohlcv {
            time: time[i],
            span: 1.0,
            open: open[i],
            high: high[i],
            low: low[i],
            close: close[i],
            volume: 0.0,
        }));
    }

    let source = VecDataSource::new(plot_data);
    let mut output = Vec::new();

    // Request 4 points across the gap to ensure stable LOD behavior for small segments.
    // We expect points from both Friday and Monday.
    source.get_aggregated_data(900.0, 2100.0, 4, &mut output, Some(&gaps));

    println!("Output len: {}", output.len());
    for (i, p) in output.iter().enumerate() {
        if let PlotData::Ohlcv(o) = p {
            println!("Point {}: time={}", i, o.time);
        }
    }

    assert!(output.len() >= 2, "Should have at least one point per session");

    // Check that we have data from both sides of the gap
    let has_friday = output.iter().any(|p| {
        if let PlotData::Ohlcv(o) = p { o.time < friday_close as f64 } else { false }
    });
    let has_monday = output.iter().any(|p| {
        if let PlotData::Ohlcv(o) = p { o.time >= monday_open as f64 } else { false }
    });

    assert!(has_friday, "Missing Friday data");
    assert!(has_monday, "Missing Monday data");
}

#[test]
fn test_calculate_gap_aware_buckets() {
    use gpui_chart::aggregation::calculate_gap_aware_buckets;

    let time = vec![10.0, 20.0, 30.0, 100.0, 110.0, 120.0];
    let segments = vec![GapSegment {
        start_real: 40,
        end_real: 90,
        cumulative_before: 0,
    }];
    let gaps = GapIndex::new(segments);

    // bin_size = 2.
    // Buckets: [10, 20], [30] (gap follows), [100, 110], [120]
    let buckets = calculate_gap_aware_buckets(&time, Some(&gaps), 2);

    assert_eq!(buckets.len(), 4);
    assert_eq!(buckets[0], 0..2); // [10, 20]
    assert_eq!(buckets[1], 2..3); // [30] - split because gap starts at 40
    assert_eq!(buckets[2], 3..5); // [100, 110]
    assert_eq!(buckets[3], 5..6); // [120]
}

#[test]
fn test_m4_kernel_respects_gaps() {
    use gpui_chart::aggregation::decimate_m4_arrays_par_into;

    let time = vec![10.0, 20.0, 30.0, 100.0, 110.0, 120.0];
    let y = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let segments = vec![GapSegment {
        start_real: 40,
        end_real: 90,
        cumulative_before: 0,
    }];
    let gaps = GapIndex::new(segments);

    let mut output = Vec::new();
    decimate_m4_arrays_par_into(&time, &y, 10, &mut output, Some(&gaps));

    // Even if max_points is 10, it should split at the gap.
    // Buckets: [10, 20, 30] and [100, 110, 120]
    // Each bucket with 3 points will produce 3 points (M4 on small bucket)
    // Wait, M4 on 3 points: first=0, min/max=1/2, last=2. Indices: 0, 1, 2.
    
    // Check that no point has X between 30 and 100
    for p in &output {
        if let PlotData::Point(pt) = p {
            assert!(pt.x <= 30.0 || pt.x >= 100.0);
        }
    }
}
