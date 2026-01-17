
#[cfg(test)]
mod tests {
    use gpui_chart::decimation::ohlcv::decimate_ohlcv_arrays_par;
    use gpui_chart::data_types::{PlotData, Ohlcv};

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
    fn test_ohlcv_sliding_window_stability() {
        // Generate enough data for a long pan
        let (t, o, h, l, c) = generate_data(3000);
        
        let max_points = 100;
        let window_size = 1000;
        let reference_range = 1000.0; // Fixed range to keep bin_size stable (10.0)

        // Capture reference result at offset 0
        let res_ref = decimate_ohlcv_arrays_par(
            &t[0..window_size],
            &o[0..window_size],
            &h[0..window_size],
            &l[0..window_size],
            &c[0..window_size],
            max_points,
            None,
            Some(reference_range),
        );
        let candles_ref: Vec<&Ohlcv> = res_ref.iter().filter_map(|p| if let PlotData::Ohlcv(o) = p { Some(o) } else { None }).collect();

        // Perform 50 small pans (1 unit each)
        for offset in 1..50 {
            let res_curr = decimate_ohlcv_arrays_par(
                &t[offset..offset+window_size],
                &o[offset..offset+window_size],
                &h[offset..offset+window_size],
                &l[offset..offset+window_size],
                &c[offset..offset+window_size],
                max_points,
                None,
                Some(reference_range),
            );
            let candles_curr: Vec<&Ohlcv> = res_curr.iter().filter_map(|p| if let PlotData::Ohlcv(o) = p { Some(o) } else { None }).collect();

            // We compare candles that are strictly inside the overlapping time range
            // Overlap starts at 'offset' and ends at 'window_size - 1'
            let overlap_start = offset as f64;
            let overlap_end = (window_size - 1) as f64;

            let mut checked_in_this_frame = 0;
            for c_ref in &candles_ref {
                // If the reference candle is still within the current visible overlap
                if c_ref.time >= overlap_start + 10.0 && c_ref.time + c_ref.span <= overlap_end - 10.0 {
                    let matching = candles_curr.iter().find(|c| (c.time - c_ref.time).abs() < 0.001)
                        .expect(&format!("Candle at time {} disappeared at offset {}!", c_ref.time, offset));
                    
                    assert_eq!(c_ref.open, matching.open, "OPEN jitter at time {} (offset {})", c_ref.time, offset);
                    assert_eq!(c_ref.close, matching.close, "CLOSE jitter at time {} (offset {})", c_ref.time, offset);
                    assert_eq!(c_ref.high, matching.high, "HIGH jitter at time {} (offset {})", c_ref.time, offset);
                    assert_eq!(c_ref.low, matching.low, "LOW jitter at time {} (offset {})", c_ref.time, offset);
                    checked_in_this_frame += 1;
                }
            }
            assert!(checked_in_this_frame > 0, "No overlapping candles to check at offset {}", offset);
        }
        println!("Sliding window stability verified over 50 steps.");
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
        // Skip first and last buckets as they may be partial due to slice boundaries
        let len = res1.len().min(res2.len());
        if len > 2 {
            for i in 1..len-1 {
                if let (PlotData::Ohlcv(c1), PlotData::Ohlcv(c2)) = (&res1[i], &res2[i]) {
                    assert_eq!(c1.time, c2.time, "Time mismatch at bucket {}", i);
                    assert_eq!(c1.open, c2.open, "Open mismatch at bucket {}", i);
                    assert_eq!(c1.close, c2.close, "Close mismatch at bucket {}", i);
                }
            }
        }
    }
}
