use gpui::prelude::*;
use gpui_chart::{ChartContainer, ChartPane, Series, LinePlot};
use gpui_chart::data_types::{AxisRange, SharedPlotState};

#[gpui::test]
fn test_series_migration_between_panes(cx: &mut gpui::TestAppContext) {
    let shared_state = cx.update(|cx| cx.new(|_| SharedPlotState::default()));
    let shared_x = cx.update(|cx| cx.new(|_| AxisRange::new(0.0, 100.0)));

    let container = cx.update(|cx| {
        cx.new(|cx| {
            let mut container = ChartContainer::new(shared_x.clone(), shared_state.clone(), cx);
            
            // Ajouter Pane 0
            let pane0 = cx.new(|cx| {
                let mut p = ChartPane::new(shared_state.clone(), cx);
                p.add_series(Series::new("series1", LinePlot::new(vec![])));
                p
            });
            container.add_pane(pane0, 1.0, cx);
            
            // Ajouter Pane 1 (vide)
            let pane1 = cx.new(|cx| ChartPane::new(shared_state.clone(), cx));
            container.add_pane(pane1, 1.0, cx);
            
            container
        })
    });

    // 1. Vérifier l'état initial
    container.update(cx, |c, cx| {
        let p0 = c.panes[0].pane.read(cx);
        assert_eq!(p0.series.len(), 1);
        assert_eq!(p0.series[0].id, "series1");
        
        let p1 = c.panes[1].pane.read(cx);
        assert_eq!(p1.series.len(), 0);
    });

    // 2. Déplacer la série de Pane 0 vers Pane 1
    container.update(cx, |c, cx| {
        c.move_series(0, 1, "series1", cx);
    });

    // 3. Vérifier le résultat
    container.update(cx, |c, cx| {
        let p0 = c.panes[0].pane.read(cx);
        assert_eq!(p0.series.len(), 0, "Pane 0 should be empty");
        
        let p1 = c.panes[1].pane.read(cx);
        assert_eq!(p1.series.len(), 1, "Pane 1 should contain the series");
        assert_eq!(p1.series[0].id, "series1");
    });
}

#[gpui::test]
fn test_pane_reordering(cx: &mut gpui::TestAppContext) {
    let shared_state = cx.update(|cx| cx.new(|_| SharedPlotState::default()));
    let shared_x = cx.update(|cx| cx.new(|_| AxisRange::new(0.0, 100.0)));

    let container = cx.update(|cx| {
        cx.new(|cx| {
            let mut container = ChartContainer::new(shared_x, shared_state.clone(), cx);
            container.add_pane_at(0, 1.0, cx); // Pane A
            container.add_pane_at(1, 1.0, cx); // Pane B
            container
        })
    });

    let pane_a_id = container.update(cx, |c, _| c.panes[0].pane.entity_id());
    let pane_b_id = container.update(cx, |c, _| c.panes[1].pane.entity_id());

    // Swap panes
    container.update(cx, |c, cx| {
        c.move_pane_down(0, cx);
    });

    container.update(cx, |c, _| {
        assert_eq!(c.panes[0].pane.entity_id(), pane_b_id);
        assert_eq!(c.panes[1].pane.entity_id(), pane_a_id);
    });
}
