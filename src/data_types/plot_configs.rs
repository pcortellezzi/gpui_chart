use gpui::Hsla;

#[derive(Clone, Debug, PartialEq)]
pub struct LinePlotConfig {
    pub color: Hsla,
    pub line_width: f32,
}

impl Default for LinePlotConfig {
    fn default() -> Self {
        Self {
            color: gpui::blue(),
            line_width: 2.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandlestickConfig {
    pub up_wick_color: Hsla,
    pub down_wick_color: Hsla,
    pub up_body_color: Hsla,
    pub down_body_color: Hsla,
    pub up_border_color: Hsla,
    pub down_border_color: Hsla,
    pub body_width_pct: f32,
    pub wick_width_pct: f32,
    pub border_thickness_px: f32,
}

impl Default for CandlestickConfig {
    fn default() -> Self {
        let green = gpui::green();
        let red = gpui::red();
        Self {
            up_wick_color: green,
            down_wick_color: red,
            up_body_color: green,
            down_body_color: red,
            up_border_color: green,
            down_border_color: red,
            body_width_pct: 0.8,
            wick_width_pct: 0.1,
            border_thickness_px: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaPlotConfig {
    pub line_color: Hsla,
    pub fill_color: Hsla,
    pub line_width: f32,
}

impl Default for AreaPlotConfig {
    fn default() -> Self {
        Self {
            line_color: gpui::blue(),
            fill_color: gpui::blue().alpha(0.3),
            line_width: 2.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BarPlotConfig {
    pub color: Hsla,
    /// 0.0 to 1.0 relative to data spacing
    pub bar_width_pct: f32,
}

impl Default for BarPlotConfig {
    fn default() -> Self {
        Self {
            color: gpui::blue(),
            bar_width_pct: 0.8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepMode {
    /// Step occurs before the point
    Pre,
    /// Step occurs halfway between points
    Mid,
    /// Step occurs after the point
    Post,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepLinePlotConfig {
    pub color: Hsla,
    pub line_width: f32,
    pub mode: StepMode,
}

impl Default for StepLinePlotConfig {
    fn default() -> Self {
        Self {
            color: gpui::blue(),
            line_width: 2.0,
            mode: StepMode::Post,
        }
    }
}
