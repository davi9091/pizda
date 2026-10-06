//! OBD-II mode 01 PID definitions and decoding.
//!
//! Transport-agnostic: an ELM327 source and a future SocketCAN/ISO-TP source
//! both get the same data bytes back and decode them here.

use crate::signal::Signal;

pub struct Pid {
    pub code: u8,
    pub signal: Signal,
    decode: fn(&[u8]) -> Option<f64>,
}

impl Pid {
    /// Decode the data bytes that follow `41 <pid>` in a response.
    pub fn decode(&self, data: &[u8]) -> Option<f64> {
        (self.decode)(data)
    }
}

fn a(d: &[u8]) -> Option<f64> {
    d.first().map(|&a| a as f64)
}

fn ab(d: &[u8]) -> Option<f64> {
    match d {
        [a, b, ..] => Some(*a as f64 * 256.0 + *b as f64),
        _ => None,
    }
}

pub const PIDS: &[Pid] = &[
    Pid { code: 0x04, signal: Signal::EngineLoadPct, decode: |d| a(d).map(|a| a * 100.0 / 255.0) },
    Pid { code: 0x05, signal: Signal::CoolantTempC, decode: |d| a(d).map(|a| a - 40.0) },
    Pid { code: 0x06, signal: Signal::ShortFuelTrimPct, decode: |d| a(d).map(|a| (a - 128.0) * 100.0 / 128.0) },
    Pid { code: 0x07, signal: Signal::LongFuelTrimPct, decode: |d| a(d).map(|a| (a - 128.0) * 100.0 / 128.0) },
    Pid { code: 0x0C, signal: Signal::Rpm, decode: |d| ab(d).map(|v| v / 4.0) },
    Pid { code: 0x0D, signal: Signal::SpeedKph, decode: a },
    Pid { code: 0x0E, signal: Signal::TimingAdvanceDeg, decode: |d| a(d).map(|a| a / 2.0 - 64.0) },
    Pid { code: 0x0F, signal: Signal::IntakeAirTempC, decode: |d| a(d).map(|a| a - 40.0) },
    Pid { code: 0x10, signal: Signal::MafGramsPerSec, decode: |d| ab(d).map(|v| v / 100.0) },
    Pid { code: 0x11, signal: Signal::ThrottlePct, decode: |d| a(d).map(|a| a * 100.0 / 255.0) },
];

pub fn pid(code: u8) -> Option<&'static Pid> {
    PIDS.iter().find(|p| p.code == code)
}

/// What to poll and how often: `(pid, every)` where `every = 1` means each
/// cycle and `every = 10` means every 10th cycle. Fast-changing values first.
pub const POLL_PLAN: &[(u8, u32)] = &[
    (0x0C, 1),  // rpm
    (0x0D, 1),  // speed
    (0x11, 1),  // throttle
    (0x04, 2),  // load
    (0x0E, 2),  // timing
    (0x10, 2),  // maf
    (0x06, 5),  // stft
    (0x05, 10), // coolant
    (0x0F, 10), // iat
    (0x07, 20), // ltft
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_rpm() {
        assert_eq!(pid(0x0C).unwrap().decode(&[0x1A, 0xF8]), Some(1726.0));
    }

    #[test]
    fn decodes_coolant() {
        assert_eq!(pid(0x05).unwrap().decode(&[0x7B]), Some(83.0));
    }

    #[test]
    fn short_data_is_none() {
        assert_eq!(pid(0x0C).unwrap().decode(&[0x1A]), None);
    }

    #[test]
    fn poll_plan_is_valid() {
        for &(code, every) in POLL_PLAN {
            assert!(pid(code).is_some(), "unknown pid {code:02X}");
            assert!(every > 0);
        }
    }
}
