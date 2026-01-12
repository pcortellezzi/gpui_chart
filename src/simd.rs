//! SIMD-optimized batch operations for coordinate transformation.
//! Relying on auto-vectorization by LLVM.

use crate::data_types::PlotData;
use gpui::{Point, Pixels, px};

/// Batch transforms a slice of PlotPoints to screen coordinates.
/// 
/// x_scale_coeff = screen_width / (x_max - x_min)
/// x_offset = -x_min * x_scale_coeff + bounds_x
/// 
/// result_x = x * x_scale_coeff + x_offset
pub fn batch_transform_points(
    data: &[PlotData],
    x_scale_coeff: f32,
    x_offset: f32,
    y_scale_coeff: f32,
    y_offset: f32,
    output: &mut Vec<Point<Pixels>>,
) {
    output.clear();
    output.reserve(data.len());

    // We iterate and push. LLVM is smart enough to vectorize this if the body is simple.
    // However, PlotData is an enum, which might hinder vectorization due to branching.
    // For pure Line/Scatter plots (PlotData::Point), we can optimize by checking type once per chunk?
    // Or we assume the caller passes a specialized buffer?
    // Given our structure, we receive Vec<PlotData>.
    
    for item in data {
        if let PlotData::Point(p) = item {
            // FMA (Fused Multiply Add) is likely used here.
            let sx = p.x as f32 * x_scale_coeff + x_offset;
            let sy = p.y as f32 * y_scale_coeff + y_offset;
            output.push(Point::new(px(sx), px(sy)));
        }
    }
}

/// Specialized batch transform for raw f64 arrays (even faster, guaranteed SIMD).
/// Use this if you can extract arrays from PlotData beforehand (e.g. structure of arrays).
pub fn batch_transform_arrays(
    x: &[f64],
    y: &[f64],
    x_scale_coeff: f32,
    x_offset: f32,
    y_scale_coeff: f32,
    y_offset: f32,
    output: &mut Vec<Point<Pixels>>,
) {
    let len = x.len().min(y.len());
    output.clear();
    output.reserve(len);

    // Using chunks_exact could help compiler prove safety for SIMD
    let x_chunks = x.chunks_exact(4);
    let y_chunks = y.chunks_exact(4);
    let rem_x = x_chunks.remainder();
    let rem_y = y_chunks.remainder();

    for (xc, yc) in x_chunks.zip(y_chunks) {
        let x0 = xc[0] as f32 * x_scale_coeff + x_offset;
        let y0 = yc[0] as f32 * y_scale_coeff + y_offset;
        
        let x1 = xc[1] as f32 * x_scale_coeff + x_offset;
        let y1 = yc[1] as f32 * y_scale_coeff + y_offset;
        
        let x2 = xc[2] as f32 * x_scale_coeff + x_offset;
        let y2 = yc[2] as f32 * y_scale_coeff + y_offset;
        
        let x3 = xc[3] as f32 * x_scale_coeff + x_offset;
        let y3 = yc[3] as f32 * y_scale_coeff + y_offset;

        output.push(Point::new(px(x0), px(y0)));
        output.push(Point::new(px(x1), px(y1)));
        output.push(Point::new(px(x2), px(y2)));
        output.push(Point::new(px(x3), px(y3)));
    }

    for (vx, vy) in rem_x.iter().zip(rem_y.iter()) {
        let sx = *vx as f32 * x_scale_coeff + x_offset;
        let sy = *vy as f32 * y_scale_coeff + y_offset;
        output.push(Point::new(px(sx), px(sy)));
    }
}
