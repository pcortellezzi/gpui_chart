use gpui::*;
use crate::data_types::{AxisEdge};
use crate::chart_container::PaneConfig;
use crate::chart_container::AxisConfig;

#[derive(Default)]
pub struct Gutters {
    pub left: Pixels,
    pub right: Pixels,
    pub top: Pixels,
    pub bottom: Pixels,
}

pub struct GutterManager;

impl GutterManager {
    pub fn calculate(panes: &[PaneConfig], x_axes: &[AxisConfig]) -> Gutters {
        let mut left = px(0.0);
        let mut right = px(0.0);
        let mut top = px(0.0);
        let mut bottom = px(0.0);

        for pane in panes {
            let mut p_left = px(0.0);
            let mut p_right = px(0.0);
            for axis in &pane.y_axes {
                match axis.edge {
                    AxisEdge::Left => p_left += axis.size,
                    AxisEdge::Right => p_right += axis.size,
                    _ => {}
                }
            }
            left = left.max(p_left);
            right = right.max(p_right);
        }

        for x_axis in x_axes {
            match x_axis.edge {
                AxisEdge::Top => top += x_axis.size,
                AxisEdge::Bottom => bottom += x_axis.size,
                _ => {}
            }
        }

        // Default bottom gutter if empty to ensure some space
        if bottom == px(0.0) {
            bottom = px(25.0);
        }

        Gutters { left, right, top, bottom }
    }
}
