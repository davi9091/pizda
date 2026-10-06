//! ELM327 adapter over a serial port (USB or paired Bluetooth rfcomm).

use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serialport::{ClearBuffer, SerialPort};

use super::{Sink, Source};
use crate::obd::{self, Pid};
use crate::signal::Signal;
use crate::telemetry::SourceStatus;

/// ISO 15765-4 CAN, 11-bit, 500 kbps, which is what the RX-8 uses.
/// Use "0" (auto-detect) for other cars.
const PROTOCOL: &str = "6";
const RESET_TIMEOUT: Duration = Duration::from_secs(5);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(1);
/// The first OBD request triggers protocol setup, which can be slow.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// A PID that answers NO DATA this many times in a row stops being polled.
const MAX_MISSES: u32 = 3;
const BATTERY_EVERY: u32 = 20;

pub struct Elm327 {
    path: String,
    baud: u32,
}

impl Elm327 {
    pub fn new(path: String, baud: u32) -> Self {
        Self { path, baud }
    }
}

struct Poll {
    pid: &'static Pid,
    every: u32,
    misses: u32,
}

impl Source for Elm327 {
    fn name(&self) -> &'static str {
        "elm327"
    }

    fn run(&mut self, sink: &Sink) -> Result<()> {
        sink.status(SourceStatus::Connecting);
        let mut link = Link::open(&self.path, self.baud)?;
        let version = link.init()?;
        sink.status(SourceStatus::Connected(version));

        let mut plan: Vec<Poll> = obd::POLL_PLAN
            .iter()
            .filter_map(|&(code, every)| Some(Poll { pid: obd::pid(code)?, every, misses: 0 }))
            .collect();

        let mut tick: u32 = 0;
        while !sink.should_stop() {
            for poll in plan.iter_mut().filter(|p| tick % p.every == 0) {
                match link.query_pid(poll.pid.code)? {
                    Some(data) => {
                        poll.misses = 0;
                        if let Some(value) = poll.pid.decode(&data) {
                            sink.emit(poll.pid.signal, value);
                        }
                    }
                    None => poll.misses += 1,
                }
            }
            plan.retain(|p| p.misses < MAX_MISSES);
            if plan.is_empty() {
                bail!("ECU stopped responding");
            }

            if tick % BATTERY_EVERY == 0
                && let Some(volts) = link.battery_voltage()?
            {
                sink.emit(Signal::BatteryVolts, volts);
            }
            tick = tick.wrapping_add(1);
        }
        Ok(())
    }
}

struct Link {
    port: Box<dyn SerialPort>,
}

impl Link {
    fn open(path: &str, baud: u32) -> Result<Self> {
        let port = serialport::new(path, baud)
            .timeout(Duration::from_millis(100))
            .open()
            .with_context(|| format!("opening {path}"))?;
        Ok(Self { port })
    }

    /// Reset and configure the adapter, then check the ECU answers.
    /// Returns the adapter's version string.
    fn init(&mut self) -> Result<String> {
        let _ = self.port.clear(ClearBuffer::All);
        let reset = self.command("ATZ", RESET_TIMEOUT)?;
        let version = reset
            .iter()
            .find(|l| l.starts_with("ELM"))
            .cloned()
            .unwrap_or_else(|| "unknown adapter".into());

        let setup = [
            "ATE0".to_string(), // echo off
            "ATL0".into(),      // no linefeeds
            "ATS0".into(),      // no spaces between bytes
            "ATH0".into(),      // no CAN headers
            format!("ATSP{PROTOCOL}"),
        ];
        for cmd in &setup {
            let reply = self.command(cmd, COMMAND_TIMEOUT)?;
            if !reply.iter().any(|l| l == "OK") {
                bail!("{cmd} failed: {}", reply.join(" | "));
            }
        }

        let reply = self.command("0100", CONNECT_TIMEOUT)?;
        if parse_pid_response(0x00, &reply)?.is_none() {
            bail!("ECU not responding (ignition off?)");
        }
        Ok(version)
    }

    /// Query a mode 01 PID. `Ok(None)` means the ECU doesn't answer it.
    fn query_pid(&mut self, pid: u8) -> Result<Option<Vec<u8>>> {
        // Trailing "1": return after the first ECU reply instead of waiting
        // for more, which roughly doubles the polling rate.
        let reply = self.command(&format!("01{pid:02X}1"), COMMAND_TIMEOUT)?;
        parse_pid_response(pid, &reply)
    }

    /// Supply voltage on OBD pin 16, measured by the adapter itself.
    fn battery_voltage(&mut self) -> Result<Option<f64>> {
        let reply = self.command("ATRV", COMMAND_TIMEOUT)?;
        Ok(reply.first().and_then(|l| l.trim_end_matches('V').parse().ok()))
    }

    /// Send a command and collect the reply lines up to the `>` prompt.
    fn command(&mut self, cmd: &str, timeout: Duration) -> Result<Vec<String>> {
        self.port.write_all(cmd.as_bytes())?;
        self.port.write_all(b"\r")?;

        let deadline = Instant::now() + timeout;
        let mut buf = Vec::new();
        let mut chunk = [0u8; 64];
        while !buf.contains(&b'>') {
            match self.port.read(&mut chunk) {
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == io::ErrorKind::TimedOut => {}
                Err(e) => return Err(e).context("reading from adapter"),
            }
            if Instant::now() > deadline {
                bail!("no response to {cmd}");
            }
        }

        Ok(String::from_utf8_lossy(&buf)
            .split(['\r', '\n', '>', '\0'])
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }
}

/// Extract the data bytes from a reply to `01 <pid>`.
fn parse_pid_response(pid: u8, lines: &[String]) -> Result<Option<Vec<u8>>> {
    let prefix = format!("41{pid:02X}");
    for line in lines {
        let hex: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if let Some(data) = hex.strip_prefix(&prefix) {
            return decode_hex(data).map(Some);
        }
        if hex.starts_with("7F01") || hex == "NODATA" {
            return Ok(None);
        }
    }
    bail!("unexpected reply to 01{pid:02X}: {}", lines.join(" | "))
}

fn decode_hex(s: &str) -> Result<Vec<u8>> {
    if s.len() % 2 != 0 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("bad hex: {s}");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).context("bad hex"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_data() {
        let r = parse_pid_response(0x0C, &lines(&["410C1AF8"])).unwrap();
        assert_eq!(r, Some(vec![0x1A, 0xF8]));
    }

    #[test]
    fn skips_searching_and_spaces() {
        let r = parse_pid_response(0x0C, &lines(&["SEARCHING...", "41 0C 1A F8"])).unwrap();
        assert_eq!(r, Some(vec![0x1A, 0xF8]));
    }

    #[test]
    fn no_data_is_none() {
        assert_eq!(parse_pid_response(0x42, &lines(&["NO DATA"])).unwrap(), None);
    }

    #[test]
    fn bus_errors_are_errors() {
        assert!(parse_pid_response(0x0C, &lines(&["CAN ERROR"])).is_err());
        assert!(parse_pid_response(0x0C, &lines(&["UNABLE TO CONNECT"])).is_err());
    }
}
