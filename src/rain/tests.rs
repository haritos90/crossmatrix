// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Alexander Kharitonov

//! Stream simulation tests.

use super::*;

fn rain(width: u16, height: u16) -> (Rain, Rng) {
    let mut rng = Rng::with_seed(7);
    (Rain::new(width, height, Charset::ascii(), &mut rng), rng)
}

fn visible(r: &Rain) -> Vec<(u16, u16, Slot)> {
    r.slots().filter(|(_, _, s)| !s.sym.is_gap()).collect()
}

fn heads(r: &Rain, x: u16) -> Vec<u16> {
    r.slots()
        .filter(|&(cx, _, s)| cx == x && s.head)
        .map(|(_, y, _)| y)
        .collect()
}

#[test]
fn starts_empty() {
    let (r, _) = rain(40, 20);
    assert!(visible(&r).is_empty());
    assert_eq!(r.slots().count(), 20 * 20);
}

#[test]
fn streams_appear_and_fall() {
    let (mut r, mut rng) = rain(40, 20);
    for _ in 0..40 {
        r.step(1, false, false, &mut rng);
    }
    assert!(!visible(&r).is_empty());
    let x = (0..40)
        .step_by(2)
        .find(|&x| heads(&r, x).iter().any(|&y| y < 19))
        .unwrap();
    let before = heads(&r, x);
    r.step(1, false, false, &mut rng);
    let after = heads(&r, x);
    for y in before.into_iter().filter(|&y| y < 19) {
        assert!(after.contains(&(y + 1)));
    }
}

#[test]
fn head_tops_its_stream() {
    let (mut r, mut rng) = rain(40, 20);
    for _ in 0..200 {
        r.step(1, false, true, &mut rng);
        for (x, y, s) in r.slots() {
            if s.head && y > 0 {
                assert!(matches!(r.slot(x, y - 1).sym, Sym::Char(_)));
            }
        }
    }
}

#[test]
fn changed_lists_exact_diff() {
    let (mut r, mut rng) = rain(30, 15);
    for _ in 0..50 {
        let before: Vec<_> = r.slots().collect();
        r.step(1, false, true, &mut rng);
        let after: Vec<_> = r.slots().collect();
        let diff: Vec<_> = before
            .iter()
            .zip(&after)
            .filter(|(a, b)| a != b)
            .map(|(_, b)| (b.0, b.1))
            .collect();
        let mut changed = r.changed().to_vec();
        changed.sort_unstable();
        let mut diff = diff;
        diff.sort_unstable();
        assert_eq!(changed, diff);
    }
}

#[test]
fn async_skips_slow_columns() {
    let (mut r, mut rng) = rain(40, 20);
    for _ in 0..100 {
        r.step(1, true, false, &mut rng);
    }
    assert!(visible(&r).is_empty());
}

#[test]
fn swap_charset_repicks_or_restarts() {
    let (mut r, mut rng) = rain(40, 20);
    for _ in 0..40 {
        r.step(1, false, false, &mut rng);
    }
    let mut other = Charset::custom("x").unwrap();
    assert!(!r.swap_charset(&mut other, &mut rng));
    assert_eq!(other, Charset::ascii());
    let shown = visible(&r);
    assert!(!shown.is_empty());
    assert!(shown.iter().all(|(_, _, s)| s.sym == Sym::Char('x')));
    let mut wide = Charset::custom("日").unwrap();
    assert!(r.swap_charset(&mut wide, &mut rng));
    assert!(visible(&r).is_empty());
}

#[test]
fn tiny_sizes_survive() {
    for (w, h) in [
        (0, 0),
        (1, 1),
        (2, 1),
        (1, 2),
        (3, 3),
        (4, 4),
        (0, 5),
        (5, 0),
    ] {
        let (mut r, mut rng) = rain(w, h);
        for _ in 0..100 {
            r.step(1, false, true, &mut rng);
        }
        r.resize(h, w, &mut rng);
        r.step(1, false, true, &mut rng);
    }
}

#[test]
fn wide_columns_fit() {
    let mut rng = Rng::with_seed(3);
    let set = Charset::custom("日月").unwrap();
    let mut r = Rain::new(9, 10, set, &mut rng);
    for _ in 0..50 {
        r.step(1, false, false, &mut rng);
    }
    let xs: Vec<u16> = r.slots().map(|(x, _, _)| x).collect();
    assert!(xs.iter().all(|x| x % 4 == 0 && x + 2 <= 9));
    assert_eq!(*xs.iter().max().unwrap(), 4);
}
