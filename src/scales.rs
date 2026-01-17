use crate::gaps::GapIndex;
use d3rs::scale::LogScale;
use d3rs::scale::{LinearScale, Scale as D3Scale};
use std::sync::Arc;

#[derive(Clone)]
pub enum ChartScale {
    Linear(LinearScale, Option<Arc<GapIndex>>),
    Log(LogScale, Option<Arc<GapIndex>>),
}

impl ChartScale {
    pub fn new_linear(domain: (f64, f64), range: (f32, f32)) -> Self {
        let mut d_min = domain.0;
        let mut d_max = domain.1;
        if (d_max - d_min).abs() < f64::EPSILON {
            d_min -= 0.5;
            d_max += 0.5;
        }
        let scale = LinearScale::new()
            .domain(d_min, d_max)
            .range(range.0 as f64, range.1 as f64);
        Self::Linear(scale, None)
    }

    pub fn with_gaps(mut self, gaps: Option<Arc<GapIndex>>) -> Self {
        // Force refresh domain if gaps changed
        let (d_min, d_max) = self.domain();
        match &mut self {
            Self::Linear(_, g) => *g = gaps,
            Self::Log(_, g) => *g = gaps,
        }
        self.update_domain(d_min, d_max);
        self
    }

    pub fn gap_index(&self) -> Option<&Arc<GapIndex>> {
        match self {
            Self::Linear(_, g) => g.as_ref(),
            Self::Log(_, g) => g.as_ref(),
        }
    }

    pub fn map(&self, value: f64) -> f32 {
        let logical_value = if let Some(gaps) = self.gap_index() {
            gaps.to_logical(value as i64) as f64
        } else {
            value
        };

        let res = match self {
            Self::Linear(s, _) => s.scale(logical_value) as f32,
            Self::Log(s, _) => s.scale(logical_value) as f32,
        };
        if res.is_nan() || res.is_infinite() {
            0.0
        } else {
            res
        }
    }

    pub fn invert(&self, pixel: f32) -> f64 {
        let logical_value = match self {
            Self::Linear(s, _) => s.invert(pixel as f64).unwrap_or(0.0),
            Self::Log(s, _) => s.invert(pixel as f64).unwrap_or(0.0),
        };

        if let Some(gaps) = self.gap_index() {
            gaps.to_real(logical_value as i64) as f64
        } else {
            logical_value
        }
    }

    pub fn range(&self) -> (f32, f32) {
        match self {
            Self::Linear(s, _) => (s.range().0 as f32, s.range().1 as f32),
            Self::Log(s, _) => (s.range().0 as f32, s.range().1 as f32),
        }
    }

    pub fn domain(&self) -> (f64, f64) {
        let (l_min, l_max) = match self {
            Self::Linear(s, _) => (s.domain().0, s.domain().1),
            Self::Log(s, _) => (s.domain().0, s.domain().1),
        };

        if let Some(gaps) = self.gap_index() {
            (
                gaps.to_real(l_min as i64) as f64,
                gaps.to_real(l_max as i64) as f64,
            )
        } else {
            (l_min, l_max)
        }
    }

    pub fn ticks(&self, count: usize) -> Vec<f64> {
        let logical_ticks = match self {
            Self::Linear(s, _) => s.ticks(count),
            Self::Log(s, _) => s.ticks(count),
        };

        if let Some(gaps) = self.gap_index() {
            let mut cursor = gaps.cursor();
            logical_ticks
                .into_iter()
                .map(|t| cursor.to_real(t as i64) as f64)
                .filter(|&t| !gaps.is_inside(t as i64))
                .collect()
        } else {
            logical_ticks
        }
    }

    pub fn format_tick(&self, value: f64, format: &crate::data_types::AxisFormat) -> String {
        match format {
            crate::data_types::AxisFormat::Time(unit) => {
                let (d_min, d_max) = self.domain();
                let span = (d_max - d_min).abs();

                let span_sec = match unit {
                    crate::data_types::TimeUnit::Seconds => span,
                    crate::data_types::TimeUnit::Milliseconds => span / 1000.0,
                    crate::data_types::TimeUnit::Microseconds => span / 1_000_000.0,
                    crate::data_types::TimeUnit::Nanoseconds => span / 1_000_000_000.0,
                };

                let fmt = crate::utils::date_formatter::determine_date_format(span_sec);
                return crate::utils::date_formatter::format_timestamp(value, fmt, *unit);
            }
            crate::data_types::AxisFormat::Numeric => {
                // Keep heuristic ONLY for numeric fallback if it looks really like a timestamp
                if value.abs() > 100_000_000_000.0 {
                    let (d_min, d_max) = self.domain();
                    let span = (d_max - d_min).abs();
                    let span_sec = if value.abs() > 3_000_000_000_000.0 {
                        span / 1000.0
                    } else {
                        span
                    };
                    let fmt = crate::utils::date_formatter::determine_date_format(span_sec);
                    let unit = if value.abs() > 3_000_000_000_000.0 {
                        crate::data_types::TimeUnit::Milliseconds
                    } else {
                        crate::data_types::TimeUnit::Seconds
                    };
                    return crate::utils::date_formatter::format_timestamp(value, fmt, unit);
                }
            }
        }

        if value.abs() < 0.001 && value.abs() > 0.0 {
            format!("{:.4}", value)
        } else if value.abs() > 1000.0 {
            format!("{:.0}", value)
        } else {
            format!("{:.2}", value)
        }
    }

    pub fn update_domain(&mut self, min: f64, max: f64) {
        let (l_min, l_max) = if let Some(gaps) = self.gap_index() {
            (
                gaps.to_logical(min as i64) as f64,
                gaps.to_logical(max as i64) as f64,
            )
        } else {
            (min, max)
        };

        let mut d_min = l_min;
        let mut d_max = l_max;
        if (d_max - d_min).abs() < f64::EPSILON {
            d_min -= 0.5;
            d_max += 0.5;
        }
        match self {
            Self::Linear(s, _) => {
                *s = s.domain(d_min, d_max);
            }
            Self::Log(s, _) => {
                *s = s.domain(d_min, d_max);
            }
        }
    }

    pub fn update_range(&mut self, min: f32, max: f32) {
        match self {
            Self::Linear(s, _) => {
                s.range(min as f64, max as f64);
            }
            Self::Log(s, _) => {
                s.range(min as f64, max as f64);
            }
        }
    }

    /// Returns (m, c) such that screen = value * m + c
    /// Only exact for Linear scales. Log scales return approximation or fallback.
    pub fn get_linear_coeffs(&self) -> (f32, f32) {
        let (l_min, l_max) = match self {
            Self::Linear(s, _) => (s.domain().0, s.domain().1),
            Self::Log(s, _) => (s.domain().0, s.domain().1),
        };
        let (r_min, r_max) = self.range();

        let m = (r_max - r_min) as f64 / (l_max - l_min);
        let c = r_min as f64 - m * l_min;

        match self {
            Self::Linear(_, _) => (m as f32, c as f32),
            Self::Log(_, _) => (1.0, 0.0), // Fallback, manual map needed for log
        }
    }
}
