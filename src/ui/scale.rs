//! Display ranges and warning thresholds per signal: the one place to tune
//! what counts as "normal" on screen.

use ratatui::style::{Color, Style};

use crate::signal::Signal;
use crate::telemetry::Reading;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Zone {
    Normal,
    Warn,
    Critical,
}

impl Zone {
    pub fn color(self) -> Color {
        match self {
            Zone::Normal => Color::Reset,
            Zone::Warn => Color::Yellow,
            Zone::Critical => Color::Red,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Scale {
    pub min: f64,
    pub max: f64,
    /// Values at or above these are highlighted.
    pub warn: Option<f64>,
    pub critical: Option<f64>,
}

impl Scale {
    const fn new(min: f64, max: f64) -> Self {
        Self { min, max, warn: None, critical: None }
    }

    const fn warn_at(self, warn: f64, critical: f64) -> Self {
        Self { warn: Some(warn), critical: Some(critical), ..self }
    }

    /// Position of `value` within the range, 0.0..=1.0.
    pub fn ratio(&self, value: f64) -> f64 {
        ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    pub fn zone(&self, value: f64) -> Zone {
        if self.critical.is_some_and(|c| value >= c) {
            Zone::Critical
        } else if self.warn.is_some_and(|w| value >= w) {
            Zone::Warn
        } else {
            Zone::Normal
        }
    }
}

// Placeholder values, to be tuned.
pub fn scale(signal: Signal) -> Scale {
    match signal {
        Signal::Rpm => Scale::new(0.0, 9000.0).warn_at(7500.0, 8500.0),
        Signal::SpeedKph => Scale::new(0.0, 240.0),
        Signal::ThrottlePct | Signal::EngineLoadPct => Scale::new(0.0, 100.0),
        Signal::CoolantTempC => Scale::new(40.0, 120.0).warn_at(103.0, 110.0),
        Signal::IntakeAirTempC => Scale::new(0.0, 80.0).warn_at(50.0, 65.0),
        Signal::TimingAdvanceDeg => Scale::new(-10.0, 50.0),
        Signal::MafGramsPerSec => Scale::new(0.0, 250.0),
        Signal::ShortFuelTrimPct | Signal::LongFuelTrimPct => Scale::new(-25.0, 25.0),
        Signal::BatteryVolts => Scale::new(10.0, 16.0),
    }
}

/// Text style for a signal's current reading: dimmed when missing or stale,
/// coloured by zone otherwise.
pub fn value_style(signal: Signal, reading: Option<Reading>) -> Style {
    match reading {
        Some(r) if !r.is_stale() => Style::new().fg(scale(signal).zone(r.value).color()),
        _ => Style::new().fg(Color::DarkGray),
    }
}

/// A reading formatted for display, or a dash when there's none.
pub fn format_value(signal: Signal, reading: Option<Reading>) -> String {
    match reading {
        Some(r) => format!("{:.*}", signal.precision(), r.value),
        None => "--".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones() {
        let s = Scale::new(0.0, 100.0).warn_at(70.0, 90.0);
        assert_eq!(s.zone(50.0), Zone::Normal);
        assert_eq!(s.zone(70.0), Zone::Warn);
        assert_eq!(s.zone(95.0), Zone::Critical);
        assert_eq!(Scale::new(0.0, 100.0).zone(1e9), Zone::Normal);
    }

    #[test]
    fn ratio_clamps() {
        let s = Scale::new(40.0, 120.0);
        assert_eq!(s.ratio(80.0), 0.5);
        assert_eq!(s.ratio(0.0), 0.0);
        assert_eq!(s.ratio(200.0), 1.0);
    }
}
