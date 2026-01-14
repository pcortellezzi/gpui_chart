use gpui_chart::data_types::{Ohlcv, PlotData};
use gpui_chart::gaps::{GapIndex, GapSegment};

#[test]
fn test_ohlcv_aggregation_respects_gaps_scenario_b() {
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

    // Request 2 points across the gap.
    // Should get exactly 2 points: one for Friday, one for Monday.
    source.get_aggregated_data(900.0, 2100.0, 2, &mut output, Some(&gaps));

    println!("Output len: {}", output.len());
    for (i, p) in output.iter().enumerate() {
        if let PlotData::Ohlcv(o) = p {
            println!("Point {}: time={}", i, o.time);
        }
    }

    assert_eq!(output.len(), 2, "Scenario B should split into 2 sessions");

    if let (PlotData::Ohlcv(f), PlotData::Ohlcv(m)) = (&output[0], &output[1]) {
        assert!(f.time < friday_close as f64);
        assert!(m.time >= monday_open as f64);
    } else {
        panic!("Expected OHLCV data");
    }
}
