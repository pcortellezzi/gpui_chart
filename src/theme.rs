use gpui::*;

#[derive(Clone, Debug, PartialEq)]
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
    pub accent: Hsla,
    pub candle_body_width_pct: f32,
    pub candle_wick_width_pct: f32,
}

impl ChartTheme {
    pub fn dark() -> Self {
        Self {
            background: gpui::black(),
            grid_line: gpui::white().opacity(0.1),
            axis_line: gpui::white().opacity(0.2),
            axis_label: gpui::white().opacity(0.8),
            axis_label_size: px(11.0),
            crosshair_line: gpui::white().opacity(0.3),
            tooltip_background: gpui::black().opacity(0.8),
            tooltip_text: gpui::white(),
            tag_background: gpui::white(),
            tag_text: gpui::black(),
            accent: gpui::blue(),
            candle_body_width_pct: 0.8,
            candle_wick_width_pct: 0.1,
        }
    }

    pub fn light() -> Self {
        Self {
            background: gpui::white(),
            grid_line: gpui::black().opacity(0.1),
            axis_line: gpui::black().opacity(0.2),
            axis_label: gpui::black().opacity(0.8),
            axis_label_size: px(11.0),
            crosshair_line: gpui::black().opacity(0.3),
            tooltip_background: gpui::white().opacity(0.95),
            tooltip_text: gpui::black(),
            tag_background: gpui::black(),
            tag_text: gpui::white(),
            accent: gpui::blue(),
            candle_body_width_pct: 0.8,
            candle_wick_width_pct: 0.1,
        }
    }
}

impl Default for ChartTheme {
    fn default() -> Self {
        Self::dark()
    }
}
