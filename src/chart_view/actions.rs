use crate::chart::Chart;
use crate::data_types::SharedPlotState;
use crate::view_controller::ViewController;
use gpui::*;

actions!(
    gpui_chart,
    [
        PanLeft,
        PanRight,
        PanUp,
        PanDown,
        ZoomIn,
        ZoomOut,
        ResetView,
        ToggleDebug
    ]
);

#[derive(Clone)]
pub struct ChartActionHandler {
    pub chart: Entity<Chart>,
}

impl ChartActionHandler {
    pub fn new(chart: Entity<Chart>) -> Self {
        Self { chart }
    }

    pub fn handle_pan_left(&self, _: &PanLeft, _win: &mut Window, cx: &mut App) {
        self.chart.update(cx, |c, cx| {
            let gaps = c.shared_state.read(cx).gap_index.clone();
            c.shared_x_axis.update(cx, move |r, _| {
                ViewController::pan_axis(r, -20.0, 200.0, false, gaps.as_deref());
                r.update_ticks_if_needed(10, gaps.as_deref());
            });
            c.shared_state.update(cx, |s, _| s.request_render());
        });
    }
    
    pub fn handle_pan_right(&self, _: &PanRight, _win: &mut Window, cx: &mut App) {
        self.chart.update(cx, |c, cx| {
            let gaps = c.shared_state.read(cx).gap_index.clone();
            c.shared_x_axis.update(cx, move |r, _| {
                ViewController::pan_axis(r, 20.0, 200.0, false, gaps.as_deref());
                r.update_ticks_if_needed(10, gaps.as_deref());
            });
            c.shared_state
                .update(cx, |s: &mut SharedPlotState, _| s.request_render());
        });
    }
    
    pub fn handle_zoom_in(&self, _: &ZoomIn, _win: &mut Window, cx: &mut App) {
        self.chart.update(cx, |c, cx| {
            let gaps = c.shared_state.read(cx).gap_index.clone();
            c.shared_x_axis.update(cx, move |r, _| {
                ViewController::zoom_axis_at(r, 0.5, 0.9, gaps.as_deref());
                r.update_ticks_if_needed(10, gaps.as_deref());
            });
            c.shared_state.update(cx, |s, _| s.request_render());
        });
    }
    
    pub fn handle_zoom_out(&self, _: &ZoomOut, _win: &mut Window, cx: &mut App) {
        self.chart.update(cx, |c, cx| {
            let gaps = c.shared_state.read(cx).gap_index.clone();
            c.shared_x_axis.update(cx, move |r, _| {
                ViewController::zoom_axis_at(r, 0.5, 1.1, gaps.as_deref());
                r.update_ticks_if_needed(10, gaps.as_deref());
            });
            c.shared_state.update(cx, |s, _| s.request_render());
        });
    }
    
    pub fn handle_reset_view(&self, _: &ResetView, _win: &mut Window, cx: &mut App) {
        self.chart.update(cx, |c, cx| {
            let mut x_min = f64::INFINITY;
            let mut x_max = f64::NEG_INFINITY;
            for ps in &c.panes {
                for s in &ps.series {
                    if let Some((sx_min, sx_max, _, _)) = s.plot.read().get_min_max() {
                        x_min = x_min.min(sx_min);
                        x_max = x_max.max(sx_max);
                    }
                }
            }
            if x_min != f64::INFINITY {
                let gaps = c.shared_state.read(cx).gap_index.clone();
                c.shared_x_axis.update(cx, move |r, _| {
                    ViewController::auto_fit_axis(r, x_min, x_max, 0.05);
                    r.update_ticks_if_needed(10, gaps.as_deref());
                });
            }
            for ps in c.panes.iter() {
                let x_range = c.shared_x_axis.read(cx);
                let x_bounds = (x_range.min, x_range.max);
                for (a_idx, y_axis_state) in ps.y_axes.iter().enumerate() {
                    let mut sy_min = f64::INFINITY;
                    let mut sy_max = f64::NEG_INFINITY;
                    for series in &ps.series {
                        if series.y_axis_id.0 != a_idx {
                            continue;
                        }
                        if let Some((s_min, s_max)) =
                            series.plot.read().get_y_range(x_bounds.0, x_bounds.1)
                        {
                            sy_min = sy_min.min(s_min);
                            sy_max = sy_max.max(s_max);
                        }
                    }
                    if sy_min != f64::INFINITY {
                        y_axis_state.entity.update(cx, |y, _| {
                            ViewController::auto_fit_axis(y, sy_min, sy_max, 0.05);
                            y.update_ticks_if_needed(10, None);
                        });
                    }
                }
            }
            c.shared_state
                .update(cx, |s: &mut SharedPlotState, _| s.request_render());
        });
    }

    pub fn handle_toggle_debug(&self, _: &ToggleDebug, _win: &mut Window, cx: &mut App) {
        self.chart.update(cx, |c, cx| {
            c.shared_state.update(cx, |s: &mut SharedPlotState, _| {
                s.debug_mode = !s.debug_mode;
                s.request_render();
            });
        });
    }
}
