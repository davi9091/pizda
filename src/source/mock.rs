//! Simulated car for developing away from the car.

use std::thread;
use std::time::{Duration, Instant};

use super::{Sink, Source};
use crate::signal::Signal;
use crate::telemetry::SourceStatus;

pub struct Mock {
    start: Instant,
}

impl Mock {
    pub fn new() -> Self {
        Self { start: Instant::now() }
    }
}

impl Source for Mock {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn run(&mut self, sink: &Sink) -> anyhow::Result<()> {
        sink.status(SourceStatus::Connected("simulated".into()));
        while !sink.should_stop() {
            let t = self.start.elapsed().as_secs_f64();
            let throttle = 50.0 * (1.0 - (t * 0.5).cos());
            sink.emit(Signal::ThrottlePct, throttle);
            sink.emit(Signal::Rpm, 900.0 + throttle * 80.0);
            sink.emit(Signal::SpeedKph, throttle * 1.2);
            sink.emit(Signal::EngineLoadPct, throttle * 0.8 + 15.0);
            sink.emit(Signal::CoolantTempC, (20.0 + t * 2.0).min(88.0));
            sink.emit(Signal::IntakeAirTempC, 28.0);
            sink.emit(Signal::TimingAdvanceDeg, 10.0 + throttle / 10.0);
            sink.emit(Signal::MafGramsPerSec, 3.0 + throttle * 1.5);
            sink.emit(Signal::ShortFuelTrimPct, 2.0 * (t * 3.0).sin());
            sink.emit(Signal::LongFuelTrimPct, -1.6);
            sink.emit(Signal::BatteryVolts, 14.1);
            thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }
}
