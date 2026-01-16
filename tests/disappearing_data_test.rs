use gpui_chart::decimation::decimate_m4_arrays_par_into;
use gpui_chart::data_types::PlotData;

#[test]
fn test_m4_disappearing_tail() {
    let n = 1000;
    let mut x = Vec::new();
    let mut y = Vec::new();
    for i in 0..n {
        x.push(i as f64);
        // Noisy signal: First=0, Min=-10, Max=10, Last=0 in every 4 points approx
        let phase = i % 4;
        let y_val = match phase {
            0 => 0.0,
            1 => 10.0,
            2 => -10.0,
            3 => 1.0,
            _ => 0.0,
        };
        y.push(y_val);
    }

    let max_points = 400;
    // We want ideal = range / (max_points / 4) to be around 7.0.
    // range = 7.0 * 100 = 700.
    let x_slice = &x[0..701]; // x[700] = 700.0
    let y_slice = &y[0..701];

    let mut output = Vec::new();
    decimate_m4_arrays_par_into(x_slice, y_slice, max_points, &mut output, None, 0);

    let last_x = match output.last().unwrap() {
        PlotData::Point(p) => p.x,
        _ => panic!("Expected point"),
    };

    println!(
        "Input Max X: {}, Output Max X: {}, Points: {}",
        x_slice.last().unwrap(),
        last_x,
        output.len()
    );

    assert!(
        last_x >= 700.0,
        "Tail data disappeared! Input Max X: {}, Last X in output: {}",
        x_slice.last().unwrap(),
        last_x
    );
}
