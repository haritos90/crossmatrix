// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 1999-2017 Chris Allegretta
// Copyright (C) 2017-Present Abishek V Ashok
// Copyright (C) 2026 Alexander Kharitonov

//! Animation state and key handling.

use std::time::Duration;

use fastrand::Rng;

use crate::cli::Options;
use crate::look::{Bold, Look};
use crate::rain::Rain;
use crate::screen::{Color, Screen};

/// Lock banner without -M, as cmatrix.
const LOCKED: &str = "Computer locked.";
/// Floor for -u 0.
const MIN_FRAME: Duration = Duration::from_millis(5);
/// L presses that unlock.
const UNLOCK: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    /// Ctrl-C, Ctrl-\, Ctrl-Z.
    Quit,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Run,
    Quit,
}

pub struct App {
    rain: Rain,
    screen: Screen,
    look: Look,
    rng: Rng,
    asynch: bool,
    mutate: bool,
    paused: bool,
    screensaver: bool,
    locked: bool,
    presses: u8,
    delay: u8,
    tick: u8,
    message: Option<String>,
}

impl App {
    pub fn new(o: Options, width: u16, height: u16, mut rng: Rng) -> App {
        let rain = Rain::new(width, height, o.old_style, o.charset, &mut rng);
        let look = Look {
            palette: o.palette,
            rainbow: o.rainbow,
            bold: o.bold,
            lambda: o.lambda,
        };
        let mut app = App {
            rain,
            screen: Screen::new(width, height),
            look,
            rng,
            asynch: o.asynch,
            mutate: o.mutate,
            paused: false,
            screensaver: o.screensaver,
            locked: o.lock,
            presses: 0,
            delay: o.delay,
            tick: 0,
            message: o.message,
        };
        app.show_banner();
        app
    }

    pub fn size(&self) -> (u16, u16) {
        self.screen.size()
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn interval(&self) -> Duration {
        (Duration::from_millis(10) * u32::from(self.delay)).max(MIN_FRAME)
    }

    /// Upstream bindings; -s quits on anything.
    pub fn key(&mut self, key: Key) -> Flow {
        if self.screensaver {
            return Flow::Quit;
        }
        if key != Key::Char('L') {
            self.presses = 0;
        }
        match key {
            Key::Quit | Key::Char('q') => {
                if !self.locked {
                    return Flow::Quit;
                }
            }
            Key::Char('L') => self.lock_key(),
            Key::Char('a') => self.asynch = !self.asynch,
            Key::Char('b') => self.set_bold(Bold::Mixed),
            Key::Char('B') => self.set_bold(Bold::All),
            Key::Char('n') => self.set_bold(Bold::Off),
            Key::Char(d) if d.is_ascii_digit() => self.delay = d as u8 - b'0',
            Key::Char('r') => {
                self.look.rainbow = !self.look.rainbow;
                self.restyle();
            }
            Key::Char('m') => {
                self.look.lambda = !self.look.lambda;
                self.restyle();
            }
            Key::Char('p' | 'P') => self.paused = !self.paused,
            Key::Char(c) => {
                if let Some(color) = key_color(c) {
                    self.look.palette = vec![color];
                    self.look.rainbow = false;
                    self.restyle();
                }
            }
            Key::Other => {}
        }
        Flow::Run
    }

    fn lock_key(&mut self) {
        if !self.locked {
            self.locked = true;
        } else {
            self.presses += 1;
            if self.presses == UNLOCK {
                self.locked = false;
                self.presses = 0;
            }
        }
        self.show_banner();
    }

    fn show_banner(&mut self) {
        let text = self.message.as_deref().or(self.locked.then_some(LOCKED));
        self.screen.set_banner(text);
    }

    fn set_bold(&mut self, bold: Bold) {
        self.look.bold = bold;
        self.restyle();
    }

    /// Recolor every visible slot.
    fn restyle(&mut self) {
        for (x, y, slot) in self.rain.slots() {
            let cell = self.look.cell(slot, &mut self.rng);
            self.screen.set(x, y, cell);
        }
    }

    /// One frame of rain.
    pub fn step(&mut self) {
        self.tick = self.tick % 4 + 1;
        self.rain
            .step(self.tick, self.asynch, self.mutate, &mut self.rng);
        for &(x, y) in self.rain.changed() {
            let cell = self.look.cell(self.rain.slot(x, y), &mut self.rng);
            self.screen.set(x, y, cell);
        }
    }

    /// Restart the rain for a new size.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.screen.resize(width, height);
        self.rain.resize(width, height, &mut self.rng);
    }

    /// Terminal bytes for this frame.
    pub fn render(&mut self) -> &[u8] {
        self.screen.render()
    }
}

/// Color keys, as cmatrix plus `)`.
fn key_color(c: char) -> Option<Color> {
    Some(match c {
        '!' => Color::Red,
        '@' => Color::Green,
        '#' => Color::Yellow,
        '$' => Color::Blue,
        '%' => Color::Magenta,
        '^' => Color::Cyan,
        '&' => Color::White,
        ')' => Color::Black,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charset::Charset;

    fn app(o: Options) -> App {
        App::new(o, 40, 12, Rng::with_seed(9))
    }

    fn text(app: &mut App) -> String {
        String::from_utf8(app.render().to_vec()).unwrap()
    }

    #[test]
    fn quit_keys() {
        for key in [Key::Char('q'), Key::Quit] {
            assert_eq!(app(Options::default()).key(key), Flow::Quit);
        }
        assert_eq!(app(Options::default()).key(Key::Char('x')), Flow::Run);
    }

    #[test]
    fn lock_ignores_quit() {
        let mut a = app(Options {
            lock: true,
            ..Options::default()
        });
        assert_eq!(a.key(Key::Char('q')), Flow::Run);
        assert_eq!(a.key(Key::Quit), Flow::Run);
    }

    #[test]
    fn three_l_unlock() {
        let mut a = app(Options {
            lock: true,
            ..Options::default()
        });
        a.key(Key::Char('L'));
        a.key(Key::Char('L'));
        a.key(Key::Char('x'));
        a.key(Key::Char('L'));
        a.key(Key::Char('L'));
        assert_eq!(a.key(Key::Char('q')), Flow::Run);
        for _ in 0..3 {
            a.key(Key::Char('L'));
        }
        assert_eq!(a.key(Key::Char('q')), Flow::Quit);
    }

    #[test]
    fn l_key_locks() {
        let mut a = app(Options::default());
        a.key(Key::Char('L'));
        assert_eq!(a.key(Key::Char('q')), Flow::Run);
    }

    #[test]
    fn lock_banner_follows_state() {
        let mut a = app(Options {
            lock: true,
            ..Options::default()
        });
        assert!(text(&mut a).contains(LOCKED));
        for _ in 0..3 {
            a.key(Key::Char('L'));
        }
        assert!(!text(&mut a).contains(LOCKED));
        let mut a = app(Options {
            lock: true,
            message: Some("hi".into()),
            ..Options::default()
        });
        assert!(text(&mut a).contains("hi"));
    }

    #[test]
    fn screensaver_quits_on_any_key() {
        let mut a = app(Options {
            screensaver: true,
            lock: true,
            ..Options::default()
        });
        assert_eq!(a.key(Key::Other), Flow::Quit);
    }

    #[test]
    fn delay_keys() {
        let mut a = app(Options::default());
        assert_eq!(a.interval(), Duration::from_millis(40));
        a.key(Key::Char('9'));
        assert_eq!(a.interval(), Duration::from_millis(90));
        a.key(Key::Char('0'));
        assert_eq!(a.interval(), MIN_FRAME);
    }

    #[test]
    fn toggles() {
        let mut a = app(Options::default());
        a.key(Key::Char('p'));
        assert!(a.paused());
        a.key(Key::Char('P'));
        assert!(!a.paused());
        a.key(Key::Char('r'));
        assert!(a.look.rainbow);
        a.key(Key::Char('r'));
        assert!(!a.look.rainbow);
        a.key(Key::Char('a'));
        assert!(a.asynch);
    }

    #[test]
    fn color_keys_leave_rainbow() {
        let mut a = app(Options {
            rainbow: true,
            ..Options::default()
        });
        a.key(Key::Char(')'));
        assert!(!a.look.rainbow);
        assert_eq!(a.look.palette, [Color::Black]);
        a.key(Key::Char('$'));
        assert_eq!(a.look.palette, [Color::Blue]);
    }

    #[test]
    fn restyle_repaints_visible() {
        let mut a = app(Options {
            charset: Charset::custom("x").unwrap(),
            ..Options::default()
        });
        for _ in 0..30 {
            a.step();
        }
        text(&mut a);
        a.key(Key::Char('!'));
        assert!(text(&mut a).contains("\x1b[31m"));
    }

    #[test]
    fn paused_frames_write_nothing() {
        let mut a = app(Options::default());
        for _ in 0..30 {
            a.step();
        }
        text(&mut a);
        a.key(Key::Char('p'));
        assert_eq!(text(&mut a), "");
    }

    #[test]
    fn resize_restarts() {
        let mut a = app(Options::default());
        for _ in 0..30 {
            a.step();
        }
        a.resize(10, 3);
        assert_eq!(a.size(), (10, 3));
        assert_eq!(text(&mut a), "\x1b[0m\x1b[2J");
    }
}
