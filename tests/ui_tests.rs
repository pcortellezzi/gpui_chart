use gpui::{
    px, AppContext, MouseButton, Point,
    TestAppContext,
};
use gpui_chart::data_types::{AxisRange, SharedPlotState};
use gpui_chart::{Chart, ChartView};

#[gpui::test]
fn test_basic_chart_ui(cx: &mut TestAppContext) {
    // 1. Initialize the Chart model
    let chart_entity = cx.update(|cx| {
        let shared_x = cx.new(|_| AxisRange::new(0.0, 100.0));
        let shared_state = cx.new(|_| SharedPlotState::default());
        cx.new(|cx| Chart::new(shared_x, shared_state, cx))
    });

    // 2. Create a window and render the ChartView
    let window = cx.add_window(|_window, cx| ChartView::new(chart_entity.clone(), cx));

    // 3. Basic assertion to verify the view is alive and linked to the model
    window
        .update(cx, |view, _window, _cx| {
            assert!(
                view.chart.entity_id() == chart_entity.entity_id(),
                "ChartView should hold the correct Chart entity"
            );
        })
        .unwrap();
}

#[gpui::test]
fn test_chart_view_pane_sync(cx: &mut TestAppContext) {
    let chart_entity = cx.update(|cx| {
        let shared_x = cx.new(|_| AxisRange::new(0.0, 100.0));
        let shared_state = cx.new(|_| SharedPlotState::default());
        cx.new(|cx| Chart::new(shared_x, shared_state, cx))
    });

    let window = cx.add_window(|_window, cx| ChartView::new(chart_entity.clone(), cx));

    // Initial state: no panes
    chart_entity.read_with(cx, |c, _| {
        assert_eq!(c.panes.len(), 0);
    });

    // Add a pane to the model
    cx.update(|cx| {
        chart_entity.update(cx, |c, cx| {
            c.add_pane_at(0, 1.0, cx);
        });
    });

    // Verify the model has the pane
    chart_entity.read_with(cx, |c, _| {
        assert_eq!(c.panes.len(), 1);
        assert_eq!(c.panes[0].weight, 1.0);
    });

    window
        .update(cx, |view, _window, _cx| {
            let chart = view.chart.read(_cx);
            assert_eq!(chart.panes.len(), 1);
        })
        .unwrap();
}

#[gpui::test]
fn test_chart_view_middle_click_zoom(cx: &mut TestAppContext) {
    let chart_entity = cx.update(|cx| {
        let shared_x = cx.new(|_| AxisRange::new(0.0, 100.0));
        let shared_state = cx.new(|_| SharedPlotState::default());
        cx.new(|cx| Chart::new(shared_x, shared_state, cx))
    });

    cx.update(|cx| {
        chart_entity.update(cx, |c, cx| {
            c.add_pane_at(0, 1.0, cx);
        });
    });

    let window = cx.add_window(|_window, cx| ChartView::new(chart_entity.clone(), cx));

    // Force a render to populate pane_bounds
    cx.run_until_parked();

    // 1. Initial State
    let (initial_x_range, initial_y_range) = chart_entity.read_with(cx, |c, cx| {
        let x = c.shared_x_axis.read(cx).clone();
        let y = c.panes[0].y_axes[0].entity.read(cx).clone();
        (x, y)
    });
    assert_eq!(initial_x_range.min, 0.0);
    assert_eq!(initial_x_range.max, 100.0);

    // 2. Simulate Middle Click Down at center of the window
    let center = Point::new(px(400.0), px(300.0));

    let mut visual_cx = gpui::VisualTestContext::from_window(window.into(), cx);

    visual_cx.simulate_mouse_down(
        center,
        MouseButton::Middle,
        Default::default(),
    );

    // 3. Simulate Mouse Move (Drag) to zoom
    // Moving 100px right and 100px up (negative delta y in GPUI)
    visual_cx.simulate_mouse_move(
        center + Point::new(px(100.0), px(-100.0)),
        Some(MouseButton::Middle),
        Default::default(),
    );

    // 4. Verify ranges have changed
    let (zoomed_x_range, zoomed_y_range) = chart_entity.read_with(&visual_cx, |c, cx| {
        let x = c.shared_x_axis.read(cx).clone();
        let y = c.panes[0].y_axes[0].entity.read(cx).clone();
        (x, y)
    });

    assert!(
        zoomed_x_range.span() != initial_x_range.span(),
        "X range span should have changed after middle-click drag zoom. Got: {:?} -> {:?}",
        initial_x_range.span(),
        zoomed_x_range.span()
    );

    assert!(
        zoomed_y_range.span() != initial_y_range.span(),
        "Y range span should have changed after middle-click drag zoom. Got: {:?} -> {:?}",
        initial_y_range.span(),
        zoomed_y_range.span()
    );
}
