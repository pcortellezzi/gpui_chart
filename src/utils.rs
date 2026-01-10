use gpui::{Pixels, px};

pub trait PixelsExt {
    fn px(self) -> Pixels;
    fn as_f32(&self) -> f32;
    fn as_f64(&self) -> f64;
}

impl PixelsExt for f32 {
    fn px(self) -> Pixels {
        px(self)
    }
    fn as_f32(&self) -> f32 { *self }
    fn as_f64(&self) -> f64 { *self as f64 }
}

impl PixelsExt for i32 {
    fn px(self) -> Pixels {
        px(self as f32)
    }
    fn as_f32(&self) -> f32 { *self as f32 }
    fn as_f64(&self) -> f64 { *self as f64 }
}

impl PixelsExt for f64 {
    fn px(self) -> Pixels {
        px(self as f32)
    }
    fn as_f32(&self) -> f32 { *self as f32 }
    fn as_f64(&self) -> f64 { *self }
}

impl PixelsExt for Pixels {
    fn px(self) -> Pixels {
        self
    }
    fn as_f32(&self) -> f32 {
        f32::from(*self)
    }
    fn as_f64(&self) -> f64 {
        f32::from(*self) as f64
    }
}
