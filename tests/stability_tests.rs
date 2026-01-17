use gpui_chart::decimation::decimate_m4_arrays_par;
use gpui_chart::data_types::PlotData;

#[test]
fn test_m4_visual_stability_panning() {
    let n = 2000;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).sin() * 100.0).collect();

    let max_points = 200;

    // Decimate original range
    let res1 = decimate_m4_arrays_par(&x, &y, max_points, None, None);

    // Pan by 1 index
    let x_shifted = &x[1..];
    let y_shifted = &y[1..];
    let res2 = decimate_m4_arrays_par(x_shifted, y_shifted, max_points, None, None);

    // In a stable binning system, the majority of points (except maybe at the edges)
    // should be identical or very close because they belong to the same "stable" bins.

    let mut matches = 0;
    for p1 in &res1 {
        if let PlotData::Point(pt1) = p1 {
            for p2 in &res2 {
                if let PlotData::Point(pt2) = p2 {
                    if (pt1.x - pt2.x).abs() < 0.0001 && (pt1.y - pt2.y).abs() < 0.0001 {
                        matches += 1;
                        break;
                    }
                }
            }
        }
    }

    let match_ratio = matches as f64 / res1.len() as f64;
    println!("Stability match ratio: {:.2}%", match_ratio * 100.0);

    // If the ratio is very low, it means every point changed just because of 1 offset,
    // which is the definition of jitter.
    // Note: With index-based chunking, this is hard to achieve perfectly,
    // but stable_bin_size helps a bit.
    assert!(
        match_ratio > 0.5,
        "Too much jitter! Only {:.2}% of points remained stable after 1-point pan.",
        match_ratio * 100.0
    );
}
