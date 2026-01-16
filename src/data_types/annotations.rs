use gpui::Hsla;

#[derive(Clone, Debug, PartialEq)]
pub enum Annotation {
    VLine {
        x: f64,
        color: Hsla,
        width: f32,
        label: Option<String>,
    },
    HLine {
        y: f64,
        color: Hsla,
        width: f32,
        label: Option<String>,
    },
    Rect {
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        color: Hsla,
        fill: bool,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        color: Hsla,
        font_size: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct HeatmapCell {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: Hsla,
    pub text: Option<String>,
}
