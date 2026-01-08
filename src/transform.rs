use crate::scales::ChartScale;
use gpui::{Bounds, Pixels, Point};
use adabraka_ui::util::PixelsExt;

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

    /// Converts data coordinates to screen pixels.
    pub fn data_to_screen(&self, data_point: Point<f64>) -> Point<Pixels> {
        let x = self.x_scale.map(data_point.x);
        let y = self.y_scale.map(data_point.y);
        
        Point::new(
            self.bounds.origin.x + Pixels::from(x),
            self.bounds.origin.y + Pixels::from(y),
        )
    }

    /// Converts screen pixels back to data coordinates.
    pub fn screen_to_data(&self, screen_point: Point<Pixels>) -> Point<f64> {
        let local_x = (screen_point.x - self.bounds.origin.x).as_f32();
        let local_y = (screen_point.y - self.bounds.origin.y).as_f32();

        Point::new(
            self.x_scale.invert(local_x),
            self.y_scale.invert(local_y),
        )
    }

    pub fn x_data_to_screen(&self, x: f64) -> Pixels {
        self.bounds.origin.x + Pixels::from(self.x_scale.map(x))
    }

    pub fn y_data_to_screen(&self, y: f64) -> Pixels {
        self.bounds.origin.y + Pixels::from(self.y_scale.map(y))
    }
}
