mod obd;
mod recording;
mod signal;
mod source;
mod telemetry;
mod ui;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, KeyCode, KeyEventKind};

use source::Source;
use source::elm327::Elm327;
use source::mock::Mock;
use source::replay::Replay;
use telemetry::{Event, Telemetry};

const FRAME: Duration = Duration::from_millis(100);
const USAGE: &str = "usage: dogecar-ui [--mock] [--replay <file.csv>] [--elm <serial port>] [--baud <rate>]";

fn main() -> Result<()> {
    let sources = parse_args()?;

    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let handles: Vec<_> = sources
        .into_iter()
        .map(|s| source::spawn(s, tx.clone(), stop.clone()))
        .collect();
    drop(tx);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &rx);
    ratatui::restore();

    stop.store(true, Ordering::Relaxed);
    for handle in handles {
        let _ = handle.join();
    }
    result
}

fn run(terminal: &mut DefaultTerminal, rx: &Receiver<Event>) -> Result<()> {
    let dashboard = ui::Dashboard::new();
    let mut telemetry = Telemetry::default();
    loop {
        for event in rx.try_iter() {
            telemetry.apply(event);
        }
        terminal.draw(|frame| dashboard.draw(frame, &telemetry))?;

        if event::poll(FRAME)?
            && let event::Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        {
            return Ok(());
        }
    }
}

fn parse_args() -> Result<Vec<Box<dyn Source>>> {
    let mut sources: Vec<Box<dyn Source>> = Vec::new();
    let mut elm_path = None;
    let mut baud = 38400;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mock" => sources.push(Box::new(Mock::new())),
            "--replay" => sources.push(Box::new(Replay::new(args.next().context(USAGE)?.into()))),
            "--elm" => elm_path = Some(args.next().context(USAGE)?),
            "--baud" => baud = args.next().context(USAGE)?.parse().context("invalid baud rate")?,
            _ => bail!("unknown argument {arg}\n{USAGE}"),
        }
    }
    if let Some(path) = elm_path {
        sources.push(Box::new(Elm327::new(path, baud)));
    }
    if sources.is_empty() {
        bail!(USAGE);
    }
    Ok(sources)
}
