use gpui_chart::ChartPane;
use gpui_chart::data_types::SharedPlotState;
use gpui::prelude::*;

#[gpui::test]
fn test_legend_visibility_toggle(cx: &mut gpui::TestAppContext) {
    let shared_state = cx.update(|cx| cx.new(|_| SharedPlotState::default()));
    
    let mut pane_entity = cx.update(|cx| {
        cx.new(|cx| ChartPane::new(shared_state, cx))
    });

    pane_entity.update(cx, |pane, cx| {
        pane.toggle_series_visibility("series1", cx);
        assert!(pane.hidden_series.contains("series1"));
        
        pane.toggle_series_visibility("series1", cx);
        assert!(!pane.hidden_series.contains("series1"));
    });
}
