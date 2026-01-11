// Transform helper for coordinate projection

use crate::scales::ChartScale;
use crate::utils::PixelsExt;
use gpui::*;

#[derive(Clone)]
pub struct PlotTransform {
    pub x_scale: ChartScale,
    pub y_scale: ChartScale,
    pub bounds: Bounds<Pixels>,
}

impl PlotTransform {
    pub fn new(x_scale: ChartScale, y_scale: ChartScale, bounds: Bounds<Pixels>) -> Self {
        Self {
            x_scale,
            y_scale,
            bounds,
        }
    }

    pub fn data_to_screen(&self, point: Point<f64>) -> Point<Pixels> {
        Point::new(
            self.bounds.origin.x + px(self.x_scale.map(point.x)),
            self.bounds.origin.y + px(self.y_scale.map(point.y)),
        )
    }

    pub fn screen_to_data(&self, point: Point<Pixels>) -> Point<f64> {
        Point::new(
            self.x_scale
                .invert((point.x - self.bounds.origin.x).as_f32()),
            self.y_scale
                .invert((point.y - self.bounds.origin.y).as_f32()),
        )
    }

    pub fn x_data_to_screen(&self, x: f64) -> Pixels {
        self.bounds.origin.x + px(self.x_scale.map(x))
    }

    pub fn y_data_to_screen(&self, y: f64) -> Pixels {
        self.bounds.origin.y + px(self.y_scale.map(y))
    }
}
