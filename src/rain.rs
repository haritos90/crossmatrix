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

/// Row 0 is hidden.
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
    fn step(&mut self, rows: usize, set: &Charset, mutate: bool, rng: &mut Rng) {
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
}

/// All columns of one screen.
pub struct Rain {
    width: u16,
    height: u16,
    charset: Charset,
    columns: Vec<Column>,
    scratch: Vec<Slot>,
    changed: Vec<(u16, u16)>,
}

impl Rain {
    pub fn new(width: u16, height: u16, charset: Charset, rng: &mut Rng) -> Rain {
        let mut rain = Rain {
            width,
            height,
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

    /// Swap character sets; true on restart.
    pub fn swap_charset(&mut self, other: &mut Charset, rng: &mut Rng) -> bool {
        std::mem::swap(&mut self.charset, other);
        if self.charset.width() != other.width() {
            self.reset(rng);
            return true;
        }
        for slot in self.columns.iter_mut().flat_map(|c| c.slots.iter_mut()) {
            if let Sym::Char(_) = slot.sym {
                slot.sym = Sym::Char(self.charset.pick(rng));
            }
        }
        false
    }

    /// Column spacing: glyph plus equal gap.
    fn pitch(&self) -> u16 {
        2 * self.charset.width()
    }

    /// One update; `tick` cycles 1 to 4.
    pub fn step(&mut self, tick: u8, asynch: bool, mutate: bool, rng: &mut Rng) {
        self.changed.clear();
        let (rows, pitch) = (usize::from(self.height), self.pitch());
        for (k, col) in self.columns.iter_mut().enumerate() {
            if asynch && tick <= col.updates {
                continue;
            }
            let x = k as u16 * pitch;
            self.scratch.copy_from_slice(&col.slots);
            col.step(rows, &self.charset, mutate, rng);
            for y in 0..rows {
                if col.slots[y + 1] != self.scratch[y + 1] {
                    self.changed.push((x, y as u16));
                }
            }
        }
    }

    /// Positions the last step changed.
    pub fn changed(&self) -> &[(u16, u16)] {
        &self.changed
    }

    pub fn slot(&self, x: u16, y: u16) -> Slot {
        let col = self.columns.get(usize::from(x / self.pitch()));
        col.and_then(|c| c.slots.get(usize::from(y) + 1))
            .copied()
            .unwrap_or(Slot::EMPTY)
    }

    /// Every visible slot with its position.
    pub fn slots(&self) -> impl Iterator<Item = (u16, u16, Slot)> + '_ {
        let pitch = self.pitch();
        self.columns.iter().enumerate().flat_map(move |(k, c)| {
            let x = k as u16 * pitch;
            c.slots[1..]
                .iter()
                .enumerate()
                .map(move |(y, s)| (x, y as u16, *s))
        })
    }
}

#[cfg(test)]
mod tests;
