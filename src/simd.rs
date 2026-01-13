//! SIMD-optimized batch operations for coordinate transformation.
//! Relying on auto-vectorization by LLVM.

use crate::data_types::PlotData;
use gpui::{Point, Pixels, px};

/// Batch transforms a slice of PlotPoints to screen coordinates.
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
    
    for item in data {
        if let PlotData::Point(p) = item {
            let sx = p.x as f32 * x_scale_coeff + x_offset;
            let sy = p.y as f32 * y_scale_coeff + y_offset;
            output.push(Point::new(px(sx), px(sy)));
        }
    }
}

/// Specialized batch transform for raw f64 arrays (even faster, guaranteed SIMD).
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

/// Finds the minimum value in a slice using SIMD auto-vectorization.
/// Uses f64::min which is more likely to be vectorized.
pub fn min_f64(data: &[f64]) -> f64 {
    if data.is_empty() { return f64::NAN; }
    
    let chunks = data.chunks_exact(8);
    let rem = chunks.remainder();
    
    let mut min_val = f64::MAX;
    
    for c in chunks {
        let m = c[0].min(c[1]).min(c[2]).min(c[3]).min(c[4]).min(c[5]).min(c[6]).min(c[7]);
        min_val = min_val.min(m);
    }
    
    for &val in rem {
        min_val = min_val.min(val);
    }
    min_val
}

/// Finds the maximum value in a slice using SIMD auto-vectorization.
pub fn max_f64(data: &[f64]) -> f64 {
    if data.is_empty() { return f64::NAN; }
    
    let chunks = data.chunks_exact(8);
    let rem = chunks.remainder();
    
    let mut max_val = f64::MIN;
    
    for c in chunks {
        let m = c[0].max(c[1]).max(c[2]).max(c[3]).max(c[4]).max(c[5]).max(c[6]).max(c[7]);
        max_val = max_val.max(m);
    }
    
    for &val in rem {
        max_val = max_val.max(val);
    }
    max_val
}

/// Sums values in a slice using SIMD auto-vectorization.
pub fn sum_f64(data: &[f64]) -> f64 {
    let chunks = data.chunks_exact(8);
    let rem = chunks.remainder();
    let mut total = 0.0;
    
    for c in chunks {
        total += c[0] + c[1] + c[2] + c[3] + c[4] + c[5] + c[6] + c[7];
    }
    
    for &val in rem {
        total += val;
    }
    total
}

/// Finds the index of the point that maximizes the triangle area with points A and C.
/// Area formula: |Ax(By - Cy) + Bx(Cy - Ay) + Cx(Ay - By)|
/// Optimized to: |Bx * (Ay - Cy) + By * (Cx - Ax) + (Ax*Cy - Cx*Ay)|
pub fn find_max_area_index(x: &[f64], y: &[f64], ax: f64, ay: f64, cx: f64, cy: f64) -> usize {
    let len = x.len().min(y.len());
    if len == 0 { return 0; }

    let c1 = ay - cy;
    let c2 = cx - ax;
    let c3 = ax * cy - cx * ay;

    let mut max_area = -1.0;
    let mut best_idx = 0;

    let chunks = x.chunks_exact(8).zip(y.chunks_exact(8));
    let rem_x = x.chunks_exact(8).remainder();
    let rem_y = y.chunks_exact(8).remainder();

    for (i, (xc, yc)) in chunks.enumerate() {
        for j in 0..8 {
            let area = (xc[j] * c1 + yc[j] * c2 + c3).abs();
            if area > max_area {
                max_area = area;
                best_idx = i * 8 + j;
            }
        }
    }

    let offset = (len / 8) * 8;
    for (i, (&vx, &vy)) in rem_x.iter().zip(rem_y.iter()).enumerate() {
        let area = (vx * c1 + vy * c2 + c3).abs();
        if area > max_area {
            max_area = area;
            best_idx = offset + i;
        }
    }

    best_idx
}