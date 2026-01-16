use d3rs::scale::{LinearScale, Scale};
use serde::{Deserialize, Serialize};
use crate::gaps::GapIndex;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum AxisFormat {
    Numeric,
    Time(TimeUnit),
}

impl Default for AxisFormat {
    fn default() -> Self {
        Self::Numeric
    }
}

/// Axis management types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct AxisId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AxisEdge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone)]
pub struct ChartAxis<T> {
    pub axis: gpui::Entity<T>,
    pub edge: AxisEdge,
    /// Size in pixels (width for vertical axes, height for horizontal)
    pub size: gpui::Pixels,
}

/// State for a single axis (X or Y).
#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
pub struct AxisRange {
    pub min: f64,
    pub max: f64,
    pub min_limit: Option<f64>,
    pub max_limit: Option<f64>,
    pub cached_ticks: Vec<f64>,
    pub last_tick_domain: (f64, f64),
}

impl AxisRange {
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            cached_ticks: vec![],
            last_tick_domain: (0.0, 0.0),
            ..Default::default()
        }
    }

    pub fn span(&self) -> f64 {
        self.max - self.min
    }

    pub fn ticks(&mut self, count: usize, gaps: Option<&GapIndex>) -> &[f64] {
        let (min, max) = self.clamped_bounds();
        let domain_changed = (min - self.last_tick_domain.0).abs() > (max - min) * 0.001
            || (max - self.last_tick_domain.1).abs() > (max - min) * 0.001;

        if domain_changed || self.cached_ticks.is_empty() {
            if let Some(gaps) = gaps {
                let l_min = gaps.to_logical(min as i64) as f64;
                let l_max = gaps.to_logical(max as i64) as f64;
                let logical_ticks = LinearScale::new()
                    .domain(l_min, l_max)
                    .range(0.0, 1.0)
                    .ticks(count);

                let mut cursor = gaps.cursor();
                self.cached_ticks = logical_ticks
                    .into_iter()
                    .map(|t| cursor.to_real(t as i64) as f64)
                    .filter(|&t| !gaps.is_inside(t as i64))
                    .collect();
            } else {
                self.cached_ticks = LinearScale::new()
                    .domain(min, max)
                    .range(0.0, 1.0)
                    .ticks(count);
            }
            self.last_tick_domain = (min, max);
        }
        &self.cached_ticks
    }

    pub fn update_ticks_if_needed(&mut self, count: usize, gaps: Option<&GapIndex>) {
        let _ = self.ticks(count, gaps);
    }

    /// Returns the clamped bounds for rendering.
    pub fn clamped_bounds(&self) -> (f64, f64) {
        let mut c_min = self.min;
        let mut c_max = self.max;
        if let Some(l) = self.min_limit {
            if c_min < l {
                c_min = l;
            }
            if c_max < l {
                c_max = l;
            }
        }
        if let Some(l) = self.max_limit {
            if c_max > l {
                c_max = l;
            }
            if c_min > l {
                c_min = l;
            }
        }
        (c_min, c_max)
    }

    /// Pure zoom without constraints to preserve the pivot point.
    pub fn zoom_at(&mut self, pivot_data: f64, pivot_pct: f64, factor: f64) {
        let new_span = self.span() * factor;
        self.min = pivot_data - new_span * pivot_pct;
        self.max = self.min + new_span;
        self.cached_ticks.clear();
    }

    /// Pan with optional clamping (handled manually if needed).
    pub fn pan(&mut self, delta_data: f64) {
        self.min += delta_data;
        self.max += delta_data;
        self.cached_ticks.clear();
    }

    /// Applies limits intelligently to preserve the pivot.
    pub fn clamp(&mut self) {
        let (Some(min_l), Some(max_l)) = (self.min_limit, self.max_limit) else {
            // Case where only one or no limit is present: simple clamping
            if let Some(l) = self.min_limit {
                if self.min < l {
                    let s = self.span();
                    self.min = l;
                    self.max = l + s;
                }
            }
            if let Some(l) = self.max_limit {
                if self.max > l {
                    let s = self.span();
                    self.max = l;
                    self.min = l - s;
                }
            }
            self.cached_ticks.clear();
            return;
        };

        // Case of two limits: intelligent span management
        let limit_span = max_l - min_l;
        let current_span = self.span();

        if current_span <= limit_span {
            if self.min < min_l {
                self.min = min_l;
                self.max = min_l + current_span;
            } else if self.max > max_l {
                self.max = max_l;
                self.min = max_l - current_span;
            }
        } else if self.min > min_l {
            self.min = min_l;
            self.max = min_l + current_span;
        } else if self.max < max_l {
            self.max = max_l;
            self.min = max_l - current_span;
        }
        self.cached_ticks.clear();
    }
}

/// Consolidated state for a chart view.
#[derive(Clone, Debug, Default)]
pub struct AxisDomain {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
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
}
