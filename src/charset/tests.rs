// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Alexander Kharitonov

//! Character set tests.

use super::*;

#[test]
fn ascii_matches_cmatrix() {
    let set = Charset::ascii();
    assert_eq!(set.chars.len(), 90);
    assert_eq!(set.width(), 1);
}

#[test]
fn katakana_has_digits() {
    let set = Charset::katakana();
    assert_eq!(set.chars.len(), 66);
    assert!(set.chars.contains(&'ｱ'));
    assert!(set.chars.contains(&'7'));
}

#[test]
fn custom_filters_unprintable() {
    let set = Charset::custom("0 1\t\u{301}\n").unwrap();
    assert_eq!(set.chars, ['0', '1']);
    assert_eq!(set.width(), 1);
    assert!(Charset::custom(" \u{200b}").is_none());
    assert!(Charset::custom("").is_none());
}

#[test]
fn custom_wide() {
    let set = Charset::custom("01日🔥").unwrap();
    assert_eq!(set.chars.len(), 4);
    assert_eq!(set.width(), 2);
}

#[test]
fn pick_stays_in_set() {
    let set = Charset::custom("ab").unwrap();
    let mut rng = Rng::with_seed(1);
    assert!((0..100).all(|_| matches!(set.pick(&mut rng), 'a' | 'b')));
}
