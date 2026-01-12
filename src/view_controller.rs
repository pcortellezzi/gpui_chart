use crate::data_types::AxisRange;

/// ViewController handles the business logic of interactions (zoom, pan, resize)
/// independently of the GPUI infrastructure to facilitate testing.
pub struct ViewController;

impl ViewController {
    /// Calculates and applies a pan on an axis based on a pixel delta.
    pub fn pan_axis(range: &mut AxisRange, delta_pixels: f32, total_pixels: f32, is_y: bool) {
        if total_pixels <= 0.0 {
            return;
        }
        let span = range.span();
        let ratio = span / total_pixels as f64;

        // In Y, a positive delta (downwards) should decrease the values (data).
        // In X, a positive delta (rightwards) should decrease the origin values (shift to the left).
        // But here AxisRange::pan adds delta_data.
        // If we move the mouse 10px to the right, we want to "pull" the chart,
        // so the domain values must DECREASE to show what is on the left.
        let delta_data = if is_y {
            delta_pixels as f64 * ratio
        } else {
            -delta_pixels as f64 * ratio
        };

        range.pan(delta_data);
        range.clamp();
    }

    /// Zooms on an axis at a specific pivot point (expressed as a percentage of the domain).
    pub fn zoom_axis_at(range: &mut AxisRange, pivot_pct: f64, factor: f64) {
        let mut new_factor = factor;
        const MIN_SPAN: f64 = 1e-9;

        if range.span() * factor < MIN_SPAN {
            new_factor = MIN_SPAN / range.span();
        }

        let pivot_data = range.min + range.span() * pivot_pct;
        range.zoom_at(pivot_data, pivot_pct, new_factor);
        range.clamp();
    }

    /// Resizes two adjacent panes by adjusting their respective weights.
    /// Guarantees that a pane does not disappear completely (minimum weight).
    pub fn resize_panes(weights: &mut [f32], index: usize, delta_pixels: f32, total_height: f32) {
        if index + 1 >= weights.len() || total_height <= 0.0 {
            return;
        }

        let total_weight: f32 = weights.iter().sum();
        let dw = (delta_pixels / total_height) * total_weight;

        const MIN_WEIGHT: f32 = 0.05;
        let w1 = weights[index];
        let w2 = weights[index + 1];

        // We limit dw to respect the minima on both sides
        let actual_dw = if dw > 0.0 {
            // We lower the splitter: w1 increases, w2 decreases
            dw.min(w2 - MIN_WEIGHT).max(0.0)
        } else {
            // We raise the splitter: w1 decreases, w2 increases
            dw.max(-(w1 - MIN_WEIGHT)).min(0.0)
        };

        weights[index] = w1 + actual_dw;
        weights[index + 1] = w2 - actual_dw;
    }

    /// Calculates the new bounds for an auto-fit with an optional margin.
    pub fn compute_auto_fit(min: f64, max: f64, margin_pct: f64) -> (f64, f64) {
        if min == f64::INFINITY || max == f64::NEG_INFINITY {
            return (0.0, 100.0);
        }

        let span = if (max - min).abs() < f64::EPSILON {
            1.0 // Avoids a zero span
        } else {
            max - min
        };

        (min - span * margin_pct, max + span * margin_pct)
    }

    /// Applies an auto-fit on a given axis.
    pub fn auto_fit_axis(range: &mut AxisRange, data_min: f64, data_max: f64, margin_pct: f64) {
        let (new_min, new_max) = Self::compute_auto_fit(data_min, data_max, margin_pct);
        range.min = new_min;
        range.max = new_max;
        range.clamp();
    }

    /// Centers the axis on a given value, optionally respecting strict limits.
    pub fn move_to_center(range: &mut AxisRange, center_data: f64, clamp_to: Option<(f64, f64)>) {
        let span = range.span();
        range.min = center_data - span / 2.0;
        range.max = center_data + span / 2.0;

        if let Some((limit_min, limit_max)) = clamp_to {
            let limit_span = limit_max - limit_min;
            if span <= limit_span {
                if range.min < limit_min {
                    range.min = limit_min;
                    range.max = limit_min + span;
                } else if range.max > limit_max {
                    range.max = limit_max;
                    range.min = limit_max - span;
                }
            } else {
                range.min = limit_min;
                range.max = limit_max;
            }
        }

        range.clamp();
    }

    /// Calculates a zoom factor based on a pixel delta.
    pub fn compute_zoom_factor(delta: f32, sensitivity: f32) -> f64 {
        let factor = 1.0 + (delta.abs() / sensitivity) as f64;
        if delta > 0.0 {
            1.0 / factor
        } else {
            factor
        }
    }

    /// Calculates the new velocity with friction for inertia.
    pub fn apply_friction(velocity: &mut f64, friction: f64, dt: f64) {
        *velocity *= friction.powf(dt * 60.0);
        if velocity.abs() < 0.01 {
            *velocity = 0.0;
        }
    }

    /// Maps a pixel position to a value in a given domain.
    pub fn map_pixels_to_value(
        pixels: f32,
        total_pixels: f32,
        min_val: f64,
        max_val: f64,
        invert: bool,
    ) -> f64 {
        if total_pixels <= 0.0 {
            return min_val;
        }
        let pct = (pixels / total_pixels).clamp(0.0, 1.0) as f64;
        let effective_pct = if invert { 1.0 - pct } else { pct };
        min_val + (max_val - min_val) * effective_pct
    }
}