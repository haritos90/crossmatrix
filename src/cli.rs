// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 1999-2017 Chris Allegretta
// Copyright (C) 2017-Present Abishek V Ashok
// Copyright (C) 2026 Alexander Kharitonov

//! Command line options.

use std::ffi::OsString;
use std::time::Duration;

use lexopt::prelude::*;

use crate::charset::Charset;
use crate::look::Bold;
use crate::screen::Color;

pub const HELP: &str = "\
Usage: crossmatrix [OPTIONS]

Options:
  -a, --async          Asynchronous scroll
  -b, --bold           Bold characters on
  -B, --all-bold       All characters bold
  -n, --no-bold        No bold characters (default)
  -c, --katakana       Half-width katakana and digits
  -U, --chars CHARS    Custom characters, overrides -c
  -C, --color COLORS   Color or comma list (default green)
  -r, --rainbow        Rainbow colors
  -m, --lambda         Lambda characters
  -k, --mutate         Characters change while falling
  -M, --message TEXT   Centered message
  -L, --lock           Ignore all keys; L L L unlocks
  -s, --screensaver    Exit on first keystroke
  -u, --delay 0-10     Frame delay, 10 ms units (default 4)
  -T, --timeout SECS   Exit after SECS seconds
  -h, --help           Print help
  -V, --version        Print version

Colors: green red blue white yellow cyan magenta black default #RRGGBB

Keys:
  q Ctrl-C Ctrl-\\ Ctrl-Z  Quit
  a                       Toggle asynchronous scroll
  b B n                   Bold: some, all, none
  0-9                     Frame delay
  ! @ # $ % ^ & )         Red green yellow blue magenta cyan white black
  r                       Toggle rainbow
  m                       Toggle lambda
  c                       Toggle katakana
  k                       Toggle mutate
  p                       Pause
  L                       Lock; L L L unlocks
";

/// Run settings.
#[derive(Debug, PartialEq)]
pub struct Options {
    pub asynch: bool,
    pub bold: Bold,
    pub charset: Charset,
    pub palette: Vec<Color>,
    pub rainbow: bool,
    pub lambda: bool,
    pub mutate: bool,
    pub screensaver: bool,
    pub lock: bool,
    pub message: Option<String>,
    pub delay: u8,
    pub timeout: Option<Duration>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            asynch: false,
            bold: Bold::Off,
            charset: Charset::ascii(),
            palette: vec![Color::Green],
            rainbow: false,
            lambda: false,
            mutate: false,
            screensaver: false,
            lock: false,
            message: None,
            delay: 4,
            timeout: None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Run(Options),
    Help,
    Version,
}

/// Arguments without the program name.
pub fn parse<I>(args: I) -> Result<Command, lexopt::Error>
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    let mut o = Options::default();
    let (mut some, mut all, mut none, mut kana) = (false, false, false, false);
    let mut chars = None;
    let mut parser = lexopt::Parser::from_args(args);
    while let Some(arg) = parser.next()? {
        match arg {
            Short('a') | Long("async") => o.asynch = true,
            Short('b') | Long("bold") => some = true,
            Short('B') | Long("all-bold") => all = true,
            Short('n') | Long("no-bold") => none = true,
            Short('c') | Long("katakana") => kana = true,
            Short('U') | Long("chars") => chars = Some(parser.value()?.string()?),
            Short('C') | Long("color") => o.palette = palette(&parser.value()?.string()?)?,
            Short('r') | Long("rainbow") => o.rainbow = true,
            Short('m') | Long("lambda") => o.lambda = true,
            Short('k') | Long("mutate") => o.mutate = true,
            Short('M') | Long("message") => o.message = Some(parser.value()?.string()?),
            Short('L') | Long("lock") => o.lock = true,
            Short('s') | Long("screensaver") => o.screensaver = true,
            Short('u') | Long("delay") => o.delay = delay(&parser.value()?.string()?)?,
            Short('T') | Long("timeout") => o.timeout = Some(timeout(&parser.value()?.string()?)?),
            Short('h' | '?') | Long("help") => return Ok(Command::Help),
            Short('V') | Long("version") => return Ok(Command::Version),
            _ => return Err(arg.unexpected()),
        }
    }
    o.bold = match (none, all, some) {
        (true, _, _) => Bold::Off,
        (_, true, _) => Bold::All,
        (_, _, true) => Bold::Mixed,
        _ => Bold::Off,
    };
    o.charset = match chars {
        Some(text) => Charset::custom(&text).ok_or("no printable characters in --chars")?,
        None if kana => Charset::katakana(),
        None => Charset::ascii(),
    };
    Ok(Command::Run(o))
}

fn palette(text: &str) -> Result<Vec<Color>, lexopt::Error> {
    text.split(',')
        .map(|name| color(name.trim()).ok_or_else(|| format!("invalid color: '{name}'").into()))
        .collect()
}

/// Name or hex, case-insensitive.
pub fn color(name: &str) -> Option<Color> {
    Some(match name.to_ascii_lowercase().as_str() {
        "green" => Color::Green,
        "red" => Color::Red,
        "blue" => Color::Blue,
        "white" => Color::White,
        "yellow" => Color::Yellow,
        "cyan" => Color::Cyan,
        "magenta" => Color::Magenta,
        "black" => Color::Black,
        "default" => Color::Default,
        hex => return rgb(hex),
    })
}

/// `#RRGGBB` or `RRGGBB`.
fn rgb(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some(Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn delay(text: &str) -> Result<u8, lexopt::Error> {
    match text.parse::<u8>() {
        Ok(d) if d <= 10 => Ok(d),
        _ => Err(format!("invalid delay: '{text}' (0-10)").into()),
    }
}

fn timeout(text: &str) -> Result<Duration, lexopt::Error> {
    text.parse::<f64>()
        .ok()
        .filter(|s| *s > 0.0)
        .and_then(|s| Duration::try_from_secs_f64(s).ok())
        .ok_or_else(|| format!("invalid timeout: '{text}'").into())
}

#[cfg(test)]
mod tests;
