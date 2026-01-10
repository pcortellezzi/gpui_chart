use d3rs::scale::{LinearScale, Scale as D3Scale};
use d3rs::scale::LogScale;
use chrono::{TimeZone, Utc};

#[derive(Clone)]
pub enum ChartScale {
    Linear(LinearScale),
    Log(LogScale),
}

impl ChartScale {
    pub fn new_linear(domain: (f64, f64), range: (f32, f32)) -> Self {
        let scale = LinearScale::new()
            .domain(domain.0, domain.1)
            .range(range.0 as f64, range.1 as f64);
        Self::Linear(scale)
    }

    pub fn map(&self, value: f64) -> f32 {
        match self {
            Self::Linear(s) => s.scale(value) as f32,
            Self::Log(s) => s.scale(value) as f32,
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

    pub fn format_tick(&self, value: f64) -> String {
        if value > 1_000_000_000_000.0 {
            if let Some(dt) = Utc.timestamp_millis_opt(value as i64).single() {
                return dt.format("%H:%M").to_string();
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
        match self {
            Self::Linear(s) => { s.domain(min, max); },
            Self::Log(s) => { s.domain(min, max); },
        }
    }

    pub fn update_range(&mut self, min: f32, max: f32) {
        match self {
            Self::Linear(s) => { s.range(min as f64, max as f64); },
            Self::Log(s) => { s.range(min as f64, max as f64); },
        }
    }
}