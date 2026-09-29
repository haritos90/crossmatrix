// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Alexander Kharitonov

//! Key, lock, and frame tests.

use super::*;
use crate::rain::Sym;

fn app(o: Options) -> App {
    App::new(o, 40, 12, Rng::with_seed(9))
}

fn text(app: &mut App) -> String {
    String::from_utf8(app.render().to_vec()).unwrap()
}

fn chars(app: &App) -> Vec<char> {
    app.rain
        .slots()
        .filter_map(|(_, _, s)| match s.sym {
            Sym::Char(c) => Some(c),
            _ => None,
        })
        .collect()
}

fn kana(c: char) -> bool {
    ('\u{ff66}'..='\u{ff9d}').contains(&c)
}

#[test]
fn quit_keys() {
    for key in [Key::Char('q'), Key::Quit] {
        assert_eq!(app(Options::default()).key(key), Flow::Quit);
    }
    assert_eq!(app(Options::default()).key(Key::Char('x')), Flow::Run);
}

#[test]
fn lock_ignores_keys() {
    let mut a = app(Options {
        lock: true,
        ..Options::default()
    });
    assert_eq!(a.key(Key::Char('q')), Flow::Run);
    assert_eq!(a.key(Key::Quit), Flow::Run);
    for c in "9!rpakcmB".chars() {
        a.key(Key::Char(c));
    }
    assert_eq!(a.interval(), Duration::from_millis(40));
    assert!(!(a.paused || a.asynch || a.mutate || a.look.rainbow || a.look.lambda));
    assert_eq!(a.look.palette, [Color::Green]);
    assert_eq!(a.look.bold, Bold::Off);
    assert_eq!(a.alt, Charset::katakana());
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
    a.key(Key::Char('k'));
    assert!(a.mutate);
}

#[test]
fn c_toggles_katakana() {
    let mut a = app(Options::default());
    for _ in 0..30 {
        a.step();
    }
    text(&mut a);
    a.key(Key::Char('c'));
    let shown = chars(&a);
    assert!(!shown.is_empty());
    assert!(shown.into_iter().all(|c| kana(c) || c.is_ascii_digit()));
    assert!(text(&mut a).contains(kana));
    a.key(Key::Char('c'));
    assert!(chars(&a).into_iter().all(|c| ('!'..='z').contains(&c)));
    let a = app(Options {
        charset: Charset::katakana(),
        ..Options::default()
    });
    assert_eq!(a.alt, Charset::ascii());
}

#[test]
fn c_restarts_on_width_change() {
    let mut a = app(Options {
        charset: Charset::custom("日月").unwrap(),
        ..Options::default()
    });
    for _ in 0..30 {
        a.step();
    }
    assert!(!chars(&a).is_empty());
    text(&mut a);
    a.key(Key::Char('c'));
    assert!(chars(&a).is_empty());
    assert_eq!(text(&mut a), "\x1b[0m\x1b[2J");
    assert_eq!(a.rain.slots().map(|(x, _, _)| x).max(), Some(38));
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
