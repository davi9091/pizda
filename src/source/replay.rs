//! Plays back a recording (see [`crate::recording`]) in real time, looping.

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};

use super::{Sink, Source};
use crate::recording;
use crate::telemetry::SourceStatus;

pub struct Replay {
    path: PathBuf,
    name: &'static str,
}

impl Replay {
    pub fn new(path: PathBuf) -> Self {
        // Named after the file so several replays get separate status entries.
        // Leaked once per source for the life of the process.
        let file = path.file_name().unwrap_or_default().to_string_lossy();
        let name = Box::leak(format!("replay:{file}").into_boxed_str());
        Self { path, name }
    }
}

impl Source for Replay {
    fn name(&self) -> &'static str {
        self.name
    }

    fn run(&mut self, sink: &Sink) -> Result<()> {
        // Loaded on every (re)start, so fixing a broken file needs no app restart.
        let records = recording::load(&self.path)?;
        let (Some(first), Some(last)) = (records.first(), records.last()) else {
            bail!("{} has no samples", self.path.display());
        };
        sink.status(SourceStatus::Connected(format!("{} samples", records.len())));

        // All samples at one instant: a static snapshot, emit once and hold.
        if first.t == last.t {
            for r in &records {
                sink.emit(r.signal, r.value);
            }
            while !sink.should_stop() {
                thread::sleep(Duration::from_millis(100));
            }
            return Ok(());
        }

        loop {
            let start = Instant::now();
            for r in &records {
                let due = start + (r.t - first.t);
                while let Some(wait) = due.checked_duration_since(Instant::now()) {
                    if sink.should_stop() {
                        return Ok(());
                    }
                    thread::sleep(wait.min(Duration::from_millis(100)));
                }
                sink.emit(r.signal, r.value);
            }
        }
    }
}
