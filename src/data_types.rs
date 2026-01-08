// Data structures for the charting library

use gpui::Hsla;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use eyre::Result;

// Custom serialization module for Hsla <-> Hex String
pub mod hex_color {
    use super::*;

    pub fn serialize<S>(color: &Hsla, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:?}", color))
    }

    pub fn deserialize<'de, D>(_deserializer: D) -> Result<Hsla, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(gpui::white())
    }

    pub fn parse_hex_str(_hex: &str) -> Result<Hsla> {
        Ok(gpui::white())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CandlestickConfig {
    pub up_wick_color: Hsla,
    pub down_wick_color: Hsla,
    pub up_body_color: Hsla,
    pub down_body_color: Hsla,
    pub up_border_color: Hsla,
    pub down_border_color: Hsla,
    pub body_width_pct: f32,
    pub wick_width_pct: f32,
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
        }
    }
}

/// Représente le domaine visible avec des limites optionnelles.
#[derive(Clone, Debug, Default)]
pub struct AxisDomain {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,

    // Limites strictes (Hard Limits)
    pub x_min_limit: Option<f64>,
    pub x_max_limit: Option<f64>,
    pub y_min_limit: Option<f64>,
    pub y_max_limit: Option<f64>,
}

impl AxisDomain {
    pub fn width(&self) -> f64 {
        self.x_max - self.x_min
    }
    pub fn height(&self) -> f64 {
        self.y_max - self.y_min
    }

    /// Applique les limites configurées aux valeurs actuelles.
    pub fn clamp(&mut self) {
        if let Some(min) = self.x_min_limit {
            if self.x_min < min {
                let w = self.width();
                self.x_min = min;
                self.x_max = min + w;
            }
        }
        if let Some(max) = self.x_max_limit {
            if self.x_max > max {
                let w = self.width();
                self.x_max = max;
                self.x_min = max - w;
            }
        }
        if let Some(min) = self.y_min_limit {
            if self.y_min < min {
                let h = self.height();
                self.y_min = min;
                self.y_max = min + h;
            }
        }
        if let Some(max) = self.y_max_limit {
            if self.y_max > max {
                let h = self.height();
                self.y_max = max;
                self.y_min = max - h;
            }
        }
    }
}

#[derive(Clone)]
pub struct Ohlcv {
    pub time: f64,
    pub span: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ColorOp {
    Persistent(Hsla),
    OneShot(Hsla),
    Reset,
    None,
}

#[derive(Clone, Copy, PartialEq)]
pub struct PlotPoint {
    pub x: f64,
    pub y: f64,
    pub color_op: ColorOp,
}

#[derive(Clone)]
pub enum PlotData {
    Point(PlotPoint),
    Ohlcv(Ohlcv),
}

#[derive(Clone)]
pub struct Series {
    pub id: String,
    pub plot: std::rc::Rc<std::cell::RefCell<dyn crate::plot_types::PlotRenderer + Send + Sync>>,
}

#[derive(Clone)]
pub struct Ticks {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}