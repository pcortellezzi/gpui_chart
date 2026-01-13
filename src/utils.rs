use gpui::Pixels;

pub mod date_formatter;

pub trait PixelsExt {
    fn as_f32(&self) -> f32;
}

impl PixelsExt for Pixels {
    fn as_f32(&self) -> f32 {
        f32::from(*self)
    }
}
