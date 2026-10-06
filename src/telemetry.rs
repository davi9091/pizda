use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::signal::Signal;

/// Everything a source can tell the rest of the app.
#[derive(Debug, Clone)]
pub enum Event {
    Sample {
        signal: Signal,
        value: f64,
        at: Instant,
    },
    Status {
        source: &'static str,
        status: SourceStatus,
    },
}

#[derive(Debug, Clone)]
pub enum SourceStatus {
    Connecting,
    /// Connected; the string is a human-readable detail, e.g. the adapter version.
    Connected(String),
    Error(String),
}

#[derive(Debug, Clone, Copy)]
pub struct Reading {
    pub value: f64,
    pub at: Instant,
}

/// A reading older than this is shown as stale.
pub const STALE_AFTER: Duration = Duration::from_secs(2);

/// How far back each signal's history is kept, for graphs.
pub const HISTORY: Duration = Duration::from_secs(60);

impl Reading {
    pub fn is_stale(&self) -> bool {
        self.at.elapsed() > STALE_AFTER
    }
}

/// Recent state of the car, built up from source events.
#[derive(Debug, Default)]
pub struct Telemetry {
    series: HashMap<Signal, VecDeque<Reading>>,
    sources: BTreeMap<&'static str, SourceStatus>,
}

impl Telemetry {
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Sample { signal, value, at } => {
                let series = self.series.entry(signal).or_default();
                series.push_back(Reading { value, at });
                // Always keep the latest reading, however old.
                while series.len() > 1
                    && series.front().is_some_and(|r| at.saturating_duration_since(r.at) > HISTORY)
                {
                    series.pop_front();
                }
            }
            Event::Status { source, status } => {
                self.sources.insert(source, status);
            }
        }
    }

    pub fn latest(&self, signal: Signal) -> Option<Reading> {
        self.series.get(&signal)?.back().copied()
    }

    /// Readings from the last [`HISTORY`], oldest first.
    pub fn history(&self, signal: Signal) -> impl Iterator<Item = &Reading> {
        self.series.get(&signal).into_iter().flatten()
    }

    pub fn sources(&self) -> impl Iterator<Item = (&'static str, &SourceStatus)> {
        self.sources.iter().map(|(name, status)| (*name, status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(value: f64, at: Instant) -> Event {
        Event::Sample { signal: Signal::Rpm, value, at }
    }

    #[test]
    fn trims_old_history_but_keeps_latest() {
        let t0 = Instant::now();
        let mut t = Telemetry::default();
        t.apply(sample(1.0, t0));
        t.apply(sample(2.0, t0 + HISTORY / 2));
        t.apply(sample(3.0, t0 + HISTORY * 2));
        let values: Vec<f64> = t.history(Signal::Rpm).map(|r| r.value).collect();
        assert_eq!(values, [3.0]);
        assert_eq!(t.latest(Signal::Rpm).unwrap().value, 3.0);
    }
}
