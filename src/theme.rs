use gpui::*;

#[derive(Clone, Debug)]
pub struct ChartTheme {
    pub background: Hsla,
    pub grid_line: Hsla,
    pub axis_line: Hsla,
    pub axis_label: Hsla,
    pub axis_label_size: Pixels,
    pub crosshair_line: Hsla,
    pub tooltip_background: Hsla,
    pub tooltip_text: Hsla,
    pub tag_background: Hsla,
    pub tag_text: Hsla,
}

impl Default for ChartTheme {
    fn default() -> Self {
        Self {
            background: gpui::black(),
            grid_line: gpui::white().alpha(0.1),
            axis_line: gpui::white().alpha(0.2),
            axis_label: gpui::white().alpha(0.8),
            axis_label_size: px(11.0),
            crosshair_line: gpui::white().alpha(0.3),
            tooltip_background: gpui::black().alpha(0.8),
            tooltip_text: gpui::white(),
            tag_background: gpui::white(),
            tag_text: gpui::black(),
        }
    }
}
