use gpui::AppContext;
use gpui_chart::data_types::{AxisRange, SharedPlotState};
use gpui_chart::{Chart, LinePlot, PaneState, Series};

#[gpui::test]
fn test_legend_visibility_toggle(cx: &mut gpui::TestAppContext) {
    let chart = cx.update(|cx| {
        let shared_x = cx.new(|_| AxisRange::new(0.0, 100.0));
        let shared_state = cx.new(|_| SharedPlotState::default());
        cx.new(|cx| Chart::new(shared_x, shared_state, cx))
    });

    cx.update(|cx| {
        chart.update(cx, |c, _| {
            let mut p = PaneState::new("p1".into(), 1.0);
            p.series.push(Series::new("series1", LinePlot::new(vec![])));
            c.panes.push(p);
        });
    });

    chart.read_with(cx, |c, _| {
        assert!(!c.panes[0].hidden_series.contains("series1"));
    });

    // Toggle
    cx.update(|cx| {
        chart.update(cx, |c, _| {
            c.panes[0].hidden_series.insert("series1".to_string());
        });
    });

    chart.read_with(cx, |c, _| {
        assert!(c.panes[0].hidden_series.contains("series1"));
    });
}
