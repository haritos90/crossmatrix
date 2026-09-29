// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 1999-2017 Chris Allegretta
// Copyright (C) 2017-Present Abishek V Ashok
// Copyright (C) 2026 Alexander Kharitonov

//! Matrix digital rain for the terminal.

mod app;
mod charset;
mod cli;
mod look;
mod rain;
mod screen;
mod term;

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal;

use app::{App, Flow, Key};
use cli::{Command, Options};

/// Fallback size poll period.
const SIZE_CHECK: Duration = Duration::from_secs(1);

fn main() -> ExitCode {
    match cli::parse(std::env::args_os().skip(1)) {
        Ok(Command::Help) => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("crossmatrix {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Run(options)) => match run(options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("crossmatrix: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("crossmatrix: {e}\nTry 'crossmatrix --help'.");
            ExitCode::from(2)
        }
    }
}

fn run(options: Options) -> io::Result<()> {
    let mut out = io::stdout();
    if !out.is_terminal() {
        return Err(io::Error::other("stdout is not a terminal"));
    }
    let end = options.timeout.map(|t| Instant::now() + t);
    let _term = term::Terminal::enter(&mut out)?;
    // Resize events tracked before sizing.
    event::poll(Duration::ZERO)?;
    let (width, height) = terminal::size()?;
    let mut app = App::new(options, width, height, fastrand::Rng::new());
    let mut next = Instant::now();
    let mut size_check = next + SIZE_CHECK;
    // Main rendering loop.
    loop {
        let now = Instant::now();
        let mut wake = size_check;
        if !app.paused() {
            wake = wake.min(next);
        }
        if let Some(end) = end {
            wake = wake.min(end);
        }
        let mut resized = false;
        if event::poll(wake.saturating_duration_since(now))? {
            loop {
                match event::read()? {
                    Event::Key(k) if k.kind != KeyEventKind::Release => {
                        if app.key(map_key(k)) == Flow::Quit {
                            return Ok(());
                        }
                    }
                    Event::Resize(..) => resized = true,
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
        let now = Instant::now();
        if end.is_some_and(|end| now >= end) {
            return Ok(());
        }
        if resized || now >= size_check {
            let (w, h) = terminal::size()?;
            if (w, h) != app.size() {
                app.resize(w, h);
            }
            size_check = now + SIZE_CHECK;
        }
        next = next.min(now + app.interval());
        if !app.paused() && now >= next {
            app.step();
            next += app.interval();
            if next < now {
                next = now + app.interval();
            }
        }
        let frame = app.render();
        if !frame.is_empty() {
            out.write_all(frame)?;
            out.flush()?;
        }
    }
}

/// Control chords quit; others ignored.
fn map_key(k: KeyEvent) -> Key {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Char('c' | 'z' | '\\' | '4') if ctrl => Key::Quit,
        KeyCode::Char(_) if ctrl => Key::Other,
        KeyCode::Char(c) => Key::Char(c),
        _ => Key::Other,
    }
}
