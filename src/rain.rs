// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 1999-2017 Chris Allegretta
// Copyright (C) 2017-Present Abishek V Ashok
// Copyright (C) 2026 Alexander Kharitonov

//! Falling streams, ported from cmatrix.

use fastrand::Rng;

use crate::charset::Charset;

/// Slot content; cmatrix values in comments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sym {
    /// Untouched (-1).
    Empty,
    /// Gap (' ').
    Blank,
    /// Old-style white head (0).
    Glow,
    /// Old-style bar under head (1).
    Bar,
    Char(char),
}

impl Sym {
    fn is_gap(self) -> bool {
        matches!(self, Sym::Empty | Sym::Blank)
    }
}

/// One rain cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub sym: Sym,
    pub head: bool,
}

impl Slot {
    const EMPTY: Slot = Slot {
        sym: Sym::Empty,
        head: false,
    };
}

/// Stream length, 3 to rows - 1.
fn length(rows: usize, rng: &mut Rng) -> usize {
    rng.usize(..rows.saturating_sub(3).max(1)) + 3
}

/// Row 0 is hidden in new style.
struct Column {
    slots: Vec<Slot>,
    length: usize,
    spaces: usize,
    updates: u8,
}

impl Column {
    fn new(rows: usize, rng: &mut Rng) -> Column {
        let mut slots = vec![Slot::EMPTY; rows + 1];
        slots[1].sym = Sym::Blank;
        Column {
            slots,
            length: length(rows, rng),
            spaces: rng.usize(..rows) + 1,
            updates: rng.u8(1..=3),
        }
    }

    /// Heads extend down; tails follow.
    fn step_new(&mut self, rows: usize, set: &Charset, mutate: bool, rng: &mut Rng) {
        let m = &mut self.slots;
        let free = m[0].sym == Sym::Empty && m[1].sym == Sym::Blank;
        if free && self.spaces > 0 {
            self.spaces -= 1;
        } else if free {
            self.length = length(rows, rng);
            m[0].sym = Sym::Char(set.pick(rng));
            self.spaces = rng.usize(..rows) + 1;
        }
        let mut i = 0;
        let mut first_done = false;
        while i <= rows {
            while i <= rows && m[i].sym.is_gap() {
                i += 1;
            }
            if i > rows {
                break;
            }
            let tail = i;
            let mut run = 0;
            while i <= rows && !m[i].sym.is_gap() {
                m[i].head = false;
                if mutate && rng.u8(..8) == 0 {
                    m[i].sym = Sym::Char(set.pick(rng));
                }
                i += 1;
                run += 1;
            }
            if i > rows {
                m[tail].sym = Sym::Blank;
                continue;
            }
            m[i] = Slot {
                sym: Sym::Char(set.pick(rng)),
                head: true,
            };
            if run > self.length || first_done {
                m[tail].sym = Sym::Blank;
                m[0].sym = Sym::Empty;
            }
            first_done = true;
            i += 1;
        }
    }

    /// Whole column shifts down one row.
    fn step_old(&mut self, rows: usize, set: &Charset, rng: &mut Rng) {
        let m = &mut self.slots;
        for i in (1..rows).rev() {
            m[i].sym = m[i - 1].sym;
        }
        let n = set.len();
        let roll = rng.usize(..n + 8);
        let below = m[1].sym;
        m[0].sym = if below == Sym::Glow {
            Sym::Bar
        } else if below.is_gap() {
            if self.spaces > 0 {
                self.spaces -= 1;
                Sym::Blank
            } else {
                self.spaces = rng.usize(..rows) + 1;
                if rng.u8(..3) == 1 {
                    Sym::Glow
                } else {
                    Sym::Char(set.pick(rng))
                }
            }
        } else if roll > n && below != Sym::Bar {
            Sym::Blank
        } else {
            Sym::Char(set.pick(rng))
        };
    }
}

/// All columns of one screen.
pub struct Rain {
    width: u16,
    height: u16,
    old: bool,
    charset: Charset,
    columns: Vec<Column>,
    scratch: Vec<Slot>,
    changed: Vec<(u16, u16)>,
}

impl Rain {
    pub fn new(width: u16, height: u16, old: bool, charset: Charset, rng: &mut Rng) -> Rain {
        let mut rain = Rain {
            width,
            height,
            old,
            charset,
            columns: Vec::new(),
            scratch: Vec::new(),
            changed: Vec::new(),
        };
        rain.reset(rng);
        rain
    }

    /// Fresh streams, as cmatrix on resize.
    pub fn resize(&mut self, width: u16, height: u16, rng: &mut Rng) {
        self.width = width;
        self.height = height;
        self.reset(rng);
    }

    fn reset(&mut self, rng: &mut Rng) {
        let rows = usize::from(self.height);
        let glyph = self.charset.width();
        let count = if rows == 0 || self.width < glyph {
            0
        } else {
            usize::from((self.width - glyph) / self.pitch()) + 1
        };
        self.columns = (0..count).map(|_| Column::new(rows, rng)).collect();
        self.scratch = vec![Slot::EMPTY; rows + 1];
        self.changed.clear();
    }

    /// Column spacing: glyph plus equal gap.
    fn pitch(&self) -> u16 {
        2 * self.charset.width()
    }

    /// First visible slot.
    fn top(&self) -> usize {
        usize::from(!self.old)
    }

    /// One update; `tick` cycles 1 to 4.
    pub fn step(&mut self, tick: u8, asynch: bool, mutate: bool, rng: &mut Rng) {
        self.changed.clear();
        let (rows, top, pitch) = (usize::from(self.height), self.top(), self.pitch());
        for (k, col) in self.columns.iter_mut().enumerate() {
            if asynch && tick <= col.updates {
                continue;
            }
            self.scratch.copy_from_slice(&col.slots);
            if self.old {
                col.step_old(rows, &self.charset, rng);
            } else {
                col.step_new(rows, &self.charset, mutate, rng);
            }
            let x = k as u16 * pitch;
            for y in 0..rows {
                if col.slots[y + top] != self.scratch[y + top] {
                    self.changed.push((x, y as u16));
                }
            }
        }
    }

    /// Positions changed by the last step.
    pub fn changed(&self) -> &[(u16, u16)] {
        &self.changed
    }

    pub fn slot(&self, x: u16, y: u16) -> Slot {
        let col = self.columns.get(usize::from(x / self.pitch()));
        col.and_then(|c| c.slots.get(usize::from(y) + self.top()))
            .copied()
            .unwrap_or(Slot::EMPTY)
    }

    /// Every visible slot with its position.
    pub fn slots(&self) -> impl Iterator<Item = (u16, u16, Slot)> + '_ {
        let (rows, top, pitch) = (usize::from(self.height), self.top(), self.pitch());
        self.columns.iter().enumerate().flat_map(move |(k, c)| {
            let x = k as u16 * pitch;
            c.slots[top..top + rows]
                .iter()
                .enumerate()
                .map(move |(y, s)| (x, y as u16, *s))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rain(width: u16, height: u16, old: bool) -> (Rain, Rng) {
        let mut rng = Rng::with_seed(7);
        (
            Rain::new(width, height, old, Charset::ascii(), &mut rng),
            rng,
        )
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
        let (r, _) = rain(40, 20, false);
        assert!(visible(&r).is_empty());
        assert_eq!(r.slots().count(), 20 * 20);
    }

    #[test]
    fn streams_appear_and_fall() {
        let (mut r, mut rng) = rain(40, 20, false);
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
        let (mut r, mut rng) = rain(40, 20, false);
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
        let (mut r, mut rng) = rain(30, 15, false);
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
        let (mut r, mut rng) = rain(40, 20, false);
        for _ in 0..100 {
            r.step(1, true, false, &mut rng);
        }
        assert!(visible(&r).is_empty());
    }

    #[test]
    fn old_style_shifts_down() {
        let (mut r, mut rng) = rain(40, 20, true);
        for _ in 0..60 {
            r.step(1, false, false, &mut rng);
        }
        let before: Vec<_> = r.slots().collect();
        r.step(1, false, false, &mut rng);
        for (x, y, s) in before {
            if y < 19 {
                assert_eq!(r.slot(x, y + 1).sym, s.sym);
            }
        }
    }

    #[test]
    fn old_style_glow_then_bar() {
        let (mut r, mut rng) = rain(80, 30, true);
        let mut seen = false;
        for _ in 0..300 {
            r.step(1, false, false, &mut rng);
            for (x, y, s) in r.slots() {
                if s.sym == Sym::Glow && y > 0 && r.slot(x, y - 1).sym != Sym::Empty {
                    assert_eq!(r.slot(x, y - 1).sym, Sym::Bar);
                    seen = true;
                }
            }
        }
        assert!(seen);
    }

    #[test]
    fn tiny_sizes_survive() {
        for old in [false, true] {
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
                let (mut r, mut rng) = rain(w, h, old);
                for _ in 0..100 {
                    r.step(1, false, true, &mut rng);
                }
                r.resize(h, w, &mut rng);
                r.step(1, false, true, &mut rng);
            }
        }
    }

    #[test]
    fn wide_columns_fit() {
        let mut rng = Rng::with_seed(3);
        let set = Charset::custom("日月").unwrap();
        let mut r = Rain::new(9, 10, false, set, &mut rng);
        for _ in 0..50 {
            r.step(1, false, false, &mut rng);
        }
        let xs: Vec<u16> = r.slots().map(|(x, _, _)| x).collect();
        assert!(xs.iter().all(|x| x % 4 == 0 && x + 2 <= 9));
        assert_eq!(*xs.iter().max().unwrap(), 4);
    }
}
