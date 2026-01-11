use gpui::AppContext;
use gpui_chart::data_types::{AxisRange, SharedPlotState};
use gpui_chart::{Chart, LinePlot, PaneState, Series};

#[gpui::test]
fn test_chart_model_manipulation(cx: &mut gpui::TestAppContext) {
    let chart = cx.update(|cx| {
        let shared_x = cx.new(|_| AxisRange::new(0.0, 100.0));
        let shared_state = cx.new(|_| SharedPlotState::default());
        cx.new(|cx| Chart::new(shared_x, shared_state, cx))
    });

    // 1. Initial State
    cx.update(|cx| {
        chart.update(cx, |c, _| {
            let mut p1 = PaneState::new("p1".into(), 2.0);
            p1.series.push(Series::new("s1", LinePlot::new(vec![])));
            c.panes.push(p1);
            c.panes.push(PaneState::new("p2".into(), 1.0));
        });
    });

    chart.read_with(cx, |c, _| {
        assert_eq!(c.panes.len(), 2);
        assert_eq!(c.panes[0].id, "p1");
        assert_eq!(c.panes[0].weight, 2.0);
        assert_eq!(c.panes[0].series.len(), 1);
    });

    // 2. Modification manuelle
    cx.update(|cx| {
        chart.update(cx, |c, _| {
            c.panes[0].weight = 3.0;
        });
    });

    chart.read_with(cx, |c, _| {
        assert_eq!(c.panes[0].weight, 3.0);
    });

    // 3. Réordonnement
    cx.update(|cx| {
        chart.update(cx, |c: &mut Chart, cx| {
            c.move_pane_down(0, cx);
        });
    });

    chart.read_with(cx, |c, _| {
        assert_eq!(c.panes[0].id, "p2");
        assert_eq!(c.panes[1].id, "p1");
    });
}
