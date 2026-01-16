use gpui::Hsla;

#[derive(Clone, Debug, PartialEq)]
pub struct Ohlcv {
    pub time: f64,
    pub span: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ColorOp {
    Persistent(Hsla),
    OneShot(Hsla),
    Reset,
    #[default]
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PlotPoint {
    pub x: f64,
    pub y: f64,
    pub color_op: ColorOp,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlotData {
    Point(PlotPoint),
    Ohlcv(Ohlcv),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AggregationMode {
    MinMax, // 2 points par bin
    #[default]
    M4, // 4 points par bin (First, Min, Max, Last)
    LTTB,   // Largest-Triangle-Three-Buckets
}
