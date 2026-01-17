
#[cfg(test)]
mod tests {
    use gpui_chart::decimation::ohlcv::decimate_ohlcv_arrays_par;
    use gpui_chart::data_types::{PlotData, Ohlcv};
    use gpui_chart::gaps::GapIndex;

    fn generate_data(count: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut time = Vec::with_capacity(count);
        let mut open = Vec::with_capacity(count);
        let mut high = Vec::with_capacity(count);
        let mut low = Vec::with_capacity(count);
        let mut close = Vec::with_capacity(count);

        for i in 0..count {
            let t = i as f64; // 1 unit per point
            let val = (i as f64 / 100.0).sin() * 100.0;
            time.push(t);
            open.push(val);
            high.push(val + 5.0);
            low.push(val - 5.0);
            close.push(val + 1.0);
        }
        (time, open, high, low, close)
    }

    #[test]
    fn test_ohlcv_panning_stability() {
        // Generate 2000 points.
        // View 1: 0..1000
        // View 2: 10..1010
        // Overlap: 10..1000.
        // Aggregation should be stable in the overlap region.

        let (t, o, h, l, c) = generate_data(2000);
        let max_points = 100; // Force aggregation (10:1 ratio approx)

        // Run 1
        let slice1_len = 1000;
        let res1 = decimate_ohlcv_arrays_par(
            &t[0..slice1_len],
            &o[0..slice1_len],
            &h[0..slice1_len],
            &l[0..slice1_len],
            &c[0..slice1_len],
            max_points,
            None,
            None,
        );

        // Run 2 (Pan right by 10 points)
        let offset = 10;
        let slice2_len = 1000;
        let res2 = decimate_ohlcv_arrays_par(
            &t[offset..offset+slice2_len],
            &o[offset..offset+slice2_len],
            &h[offset..offset+slice2_len],
            &l[offset..offset+slice2_len],
            &c[offset..offset+slice2_len],
            max_points,
            None,
            None,
        );

        // Find common time range
        let start_time = t[offset];
        let end_time = t[slice1_len - 1];

        let candles1: Vec<&Ohlcv> = res1.iter().filter_map(|p| if let PlotData::Ohlcv(o) = p { Some(o) } else { None }).collect();
        let candles2: Vec<&Ohlcv> = res2.iter().filter_map(|p| if let PlotData::Ohlcv(o) = p { Some(o) } else { None }).collect();

        assert!(!candles1.is_empty());
        assert!(!candles2.is_empty());

        let mut matches = 0;
        for c1 in &candles1 {
            // Check if candle is fully within the overlap region
            if c1.time >= start_time && c1.time + c1.span <= end_time {
                // Should exist in c2
                if let Some(c2) = candles2.iter().find(|c| (c.time - c1.time).abs() < 0.001) {
                    assert_eq!(c1.span, c2.span, "Span mismatch at time {}", c1.time);
                    assert_eq!(c1.open, c2.open, "Open mismatch at time {}", c1.time);
                    assert_eq!(c1.high, c2.high, "High mismatch at time {}", c1.time);
                    assert_eq!(c1.low, c2.low, "Low mismatch at time {}", c1.time);
                    assert_eq!(c1.close, c2.close, "Close mismatch at time {}", c1.time);
                    matches += 1;
                }
            }
        }
        
        println!("Verified {} matching candles", matches);
        assert!(matches > 0);
    }

    #[test]
    fn test_ohlcv_range_jitter_stability() {
        let (t, o, h, l, c) = generate_data(2000);
        let max_points = 100;
        
        // Threshold case: ideal bin size is 10.0.
        // Call 1: range exactly 1000.0.
        // Call 2: range slightly more 1000.1.
        
        // Without reference range, Call 2 would jump to 20.0 bin size.
        // With reference range, both should use exactly the same grid.
        
        let stable_range = 1000.0;

        let res1 = decimate_ohlcv_arrays_par(
            &t[0..1001],
            &o[0..1001],
            &h[0..1001],
            &l[0..1001],
            &c[0..1001],
            max_points,
            None,
            Some(stable_range),
        );

        let res2 = decimate_ohlcv_arrays_par(
            &t[0..1002],
            &o[0..1002],
            &h[0..1002],
            &l[0..1002],
            &c[0..1002],
            max_points,
            None,
            Some(stable_range),
        );

        // Verify count is same
        assert_eq!(res1.len(), res2.len(), "Counts should be identical with stable reference range");
        
        // Verify values of overlapping candles are identical
        for (p1, p2) in res1.iter().zip(res2.iter()) {
            if let (PlotData::Ohlcv(c1), PlotData::Ohlcv(c2)) = (p1, p2) {
                assert_eq!(c1.time, c2.time);
                assert_eq!(c1.open, c2.open);
                assert_eq!(c1.close, c2.close);
            }
        }
    }
}
