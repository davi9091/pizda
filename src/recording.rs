//! On-disk format for recorded telemetry: what the logger will write and
//! what the replay source reads, so a real drive can be replayed later and
//! hand-written mock scenarios use the same format.
//!
//! Plain CSV, one sample per row, in time order:
//!
//! ```text
//! # comments and blank lines are ignored
//! t,signal,value
//! 0.000,rpm,850
//! 0.000,coolant_temp_c,82
//! 0.105,rpm,910
//! 0.210,speed_kph,12
//! ```
//!
//! - `t`: seconds since the start of the recording. Only differences matter,
//!   so a clip cut from a longer log doesn't need re-zeroing.
//! - `signal`: a [`Signal::key`].
//! - `value`: in the signal's unit (see [`Signal::unit`]).
//!
//! One row per sample (rather than a column per signal) because signals
//! arrive at different rates. It's also append-only for the logger, and easy
//! to write by hand: a file where every row has the same `t` is a static
//! snapshot, e.g. "engine overheating at idle".

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};

use crate::signal::Signal;

pub const HEADER: &str = "t,signal,value";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    pub t: Duration,
    pub signal: Signal,
    pub value: f64,
}

pub fn load(path: &Path) -> Result<Vec<Record>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&text).with_context(|| format!("in {}", path.display()))
}

pub fn parse(text: &str) -> Result<Vec<Record>> {
    let mut records: Vec<Record> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line == HEADER {
            continue;
        }
        let record = parse_line(line).with_context(|| format!("line {}", i + 1))?;
        if records.last().is_some_and(|prev| record.t < prev.t) {
            bail!("line {}: time goes backwards", i + 1);
        }
        records.push(record);
    }
    Ok(records)
}

fn parse_line(line: &str) -> Result<Record> {
    let [t, signal, value] = line
        .split(',')
        .map(str::trim)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| anyhow!("expected 3 fields: {HEADER}"))?;

    let t: f64 = t.parse().with_context(|| format!("bad time {t:?}"))?;
    Ok(Record {
        t: Duration::try_from_secs_f64(t).with_context(|| format!("bad time {t}"))?,
        signal: Signal::from_key(signal).ok_or_else(|| anyhow!("unknown signal {signal:?}"))?,
        value: value.parse().with_context(|| format!("bad value {value:?}"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recording() {
        let text = "# idle\nt,signal,value\n\n0,rpm,850\n0.5, coolant_temp_c ,82.5\n";
        let records = parse(text).unwrap();
        assert_eq!(
            records,
            [
                Record { t: Duration::ZERO, signal: Signal::Rpm, value: 850.0 },
                Record { t: Duration::from_millis(500), signal: Signal::CoolantTempC, value: 82.5 },
            ]
        );
    }

    #[test]
    fn rejects_bad_rows() {
        assert!(parse("0,boost_psi,10").is_err());
        assert!(parse("0,rpm").is_err());
        assert!(parse("-1,rpm,800").is_err());
        assert!(parse("1,rpm,800\n0,rpm,900").is_err());
    }

    #[test]
    fn signal_keys_roundtrip() {
        for signal in Signal::ALL {
            assert_eq!(Signal::from_key(signal.key()), Some(signal));
        }
    }
}
