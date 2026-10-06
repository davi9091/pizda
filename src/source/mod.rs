//! Data inputs. Each source runs on its own thread and reports through a
//! [`Sink`]; nothing downstream knows which kind of source produced a value.

pub mod elm327;
pub mod mock;
pub mod replay;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::signal::Signal;
use crate::telemetry::{Event, SourceStatus};

const RETRY_DELAY: Duration = Duration::from_secs(2);

pub trait Source: Send {
    fn name(&self) -> &'static str;

    /// Connect and stream data into `sink` until `sink.should_stop()`.
    ///
    /// Returning an error causes `run` to be called again after a delay, so
    /// implementations should (re)connect from scratch at the start.
    fn run(&mut self, sink: &Sink) -> anyhow::Result<()>;
}

pub struct Sink {
    source: &'static str,
    tx: Sender<Event>,
    stop: Arc<AtomicBool>,
}

impl Sink {
    pub fn emit(&self, signal: Signal, value: f64) {
        let _ = self.tx.send(Event::Sample { signal, value, at: Instant::now() });
    }

    pub fn status(&self, status: SourceStatus) {
        let _ = self.tx.send(Event::Status { source: self.source, status });
    }

    pub fn should_stop(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}

/// Run `source` on its own thread, restarting it whenever it fails.
pub fn spawn(mut source: Box<dyn Source>, tx: Sender<Event>, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    let sink = Sink { source: source.name(), tx, stop };
    thread::spawn(move || {
        while !sink.should_stop() {
            match source.run(&sink) {
                Ok(()) => break,
                Err(e) => sink.status(SourceStatus::Error(format!("{e:#}"))),
            }
            let retry_at = Instant::now() + RETRY_DELAY;
            while Instant::now() < retry_at && !sink.should_stop() {
                thread::sleep(Duration::from_millis(100));
            }
        }
    })
}
