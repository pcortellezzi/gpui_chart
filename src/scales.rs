use d3rs::scale::LogScale;
use d3rs::scale::{LinearScale, Scale as D3Scale};

#[derive(Clone)]
pub enum ChartScale {
    Linear(LinearScale),
    Log(LogScale),
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
        Self::Linear(scale)
    }

    pub fn map(&self, value: f64) -> f32 {
        let res = match self {
            Self::Linear(s) => s.scale(value) as f32,
            Self::Log(s) => s.scale(value) as f32,
        };
        if res.is_nan() || res.is_infinite() {
            0.0
        } else {
            res
        }
    }

    pub fn invert(&self, pixel: f32) -> f64 {
        match self {
            Self::Linear(s) => s.invert(pixel as f64).unwrap_or(0.0),
            Self::Log(s) => s.invert(pixel as f64).unwrap_or(0.0),
        }
    }

    pub fn range(&self) -> (f32, f32) {
        match self {
            Self::Linear(s) => (s.range().0 as f32, s.range().1 as f32),
            Self::Log(s) => (s.range().0 as f32, s.range().1 as f32),
        }
    }

    pub fn domain(&self) -> (f64, f64) {
        match self {
            Self::Linear(s) => (s.domain().0, s.domain().1),
            Self::Log(s) => (s.domain().0, s.domain().1),
        }
    }

    pub fn ticks(&self, count: usize) -> Vec<f64> {
        match self {
            Self::Linear(s) => s.ticks(count),
            Self::Log(s) => s.ticks(count),
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
                    let span_sec = if value.abs() > 3_000_000_000_000.0 { span / 1000.0 } else { span };
                    let fmt = crate::utils::date_formatter::determine_date_format(span_sec);
                    let unit = if value.abs() > 3_000_000_000_000.0 { crate::data_types::TimeUnit::Milliseconds } else { crate::data_types::TimeUnit::Seconds };
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
        let mut d_min = min;
        let mut d_max = max;
        if (d_max - d_min).abs() < f64::EPSILON {
            d_min -= 0.5;
            d_max += 0.5;
        }
        match self {
            Self::Linear(s) => {
                s.domain(d_min, d_max);
            }
            Self::Log(s) => {
                s.domain(d_min, d_max);
            }
        }
    }

    pub fn update_range(&mut self, min: f32, max: f32) {
        match self {
            Self::Linear(s) => {
                s.range(min as f64, max as f64);
            }
            Self::Log(s) => {
                s.range(min as f64, max as f64);
            }
        }
    }

    /// Returns (m, c) such that screen = value * m + c
    /// Only exact for Linear scales. Log scales return approximation or fallback.
    pub fn get_linear_coeffs(&self) -> (f32, f32) {
        let (d_min, d_max) = self.domain();
        let (r_min, r_max) = self.range();
        
        let m = (r_max - r_min) as f64 / (d_max - d_min);
        let c = r_min as f64 - m * d_min;
        
        match self {
            Self::Linear(_) => (m as f32, c as f32),
            Self::Log(_) => (1.0, 0.0), // Fallback, manual map needed for log
        }
    }
}
