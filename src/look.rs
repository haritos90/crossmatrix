// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 1999-2017 Chris Allegretta
// Copyright (C) 2017-Present Abishek V Ashok
// Copyright (C) 2026 Alexander Kharitonov

//! Rain slot to screen cell.

use fastrand::Rng;

use crate::rain::{Slot, Sym};
use crate::screen::{Cell, Color, Style};

/// Bold modes: -n, -b, -B.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bold {
    Off,
    Mixed,
    All,
}

/// Rainbow colors, as cmatrix.
pub const RAINBOW: [Color; 6] = [
    Color::Green,
    Color::Blue,
    Color::Default,
    Color::Yellow,
    Color::Cyan,
    Color::Magenta,
];

/// Appearance settings.
#[derive(Clone, Debug)]
pub struct Look {
    pub palette: Vec<Color>,
    pub rainbow: bool,
    pub bold: Bold,
    pub lambda: bool,
}

impl Look {
    /// Rainbow and palette colors drawn per call.
    pub fn cell(&self, slot: Slot, rng: &mut Rng) -> Cell {
        if slot.head && !self.rainbow {
            let style = Style {
                color: Color::White,
                bold: self.bold != Bold::Off,
            };
            return match slot.sym {
                Sym::Char(c) => Cell::new(c, style),
                Sym::Empty | Sym::Blank => Cell::BLANK,
            };
        }
        match slot.sym {
            Sym::Char(c) => {
                let bold = match self.bold {
                    Bold::Off => false,
                    Bold::Mixed => u32::from(c) % 2 == 0,
                    Bold::All => true,
                };
                let ch = if self.lambda { 'λ' } else { c };
                Cell::new(
                    ch,
                    Style {
                        color: self.color(rng),
                        bold,
                    },
                )
            }
            Sym::Empty | Sym::Blank => Cell::BLANK,
        }
    }

    fn color(&self, rng: &mut Rng) -> Color {
        let palette: &[Color] = if self.rainbow {
            &RAINBOW
        } else {
            &self.palette
        };
        match palette {
            [] => Color::Green,
            [one] => *one,
            _ => palette[rng.usize(..palette.len())],
        }
    }
}

#[cfg(test)]
mod tests;
