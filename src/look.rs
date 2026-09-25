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
        let heavy = self.bold != Bold::Off;
        if slot.sym == Sym::Glow || (slot.head && !self.rainbow) {
            let style = Style {
                color: Color::White,
                bold: heavy,
            };
            return match slot.sym {
                Sym::Glow => Cell::new('&', style),
                Sym::Bar => Cell::new('|', style),
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
            Sym::Bar => Cell::new(
                '|',
                Style {
                    color: self.color(rng),
                    bold: heavy,
                },
            ),
            Sym::Glow | Sym::Empty | Sym::Blank => Cell::BLANK,
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
mod tests {
    use super::*;

    fn look() -> Look {
        Look {
            palette: vec![Color::Green],
            rainbow: false,
            bold: Bold::Off,
            lambda: false,
        }
    }

    fn slot(sym: Sym, head: bool) -> Slot {
        Slot { sym, head }
    }

    #[test]
    fn head_is_white() {
        let mut rng = Rng::with_seed(1);
        let c = look().cell(slot(Sym::Char('x'), true), &mut rng);
        assert_eq!(
            (c.ch, c.style.color, c.style.bold),
            ('x', Color::White, false)
        );
    }

    #[test]
    fn no_bold_means_no_bold_head() {
        let mut rng = Rng::with_seed(1);
        let l = Look {
            bold: Bold::Off,
            ..look()
        };
        assert!(!l.cell(slot(Sym::Char('x'), true), &mut rng).style.bold);
        let l = Look {
            bold: Bold::Mixed,
            ..look()
        };
        assert!(l.cell(slot(Sym::Char('x'), true), &mut rng).style.bold);
    }

    #[test]
    fn mixed_bold_by_parity() {
        let mut rng = Rng::with_seed(1);
        let l = Look {
            bold: Bold::Mixed,
            ..look()
        };
        assert!(l.cell(slot(Sym::Char('b'), false), &mut rng).style.bold);
        assert!(!l.cell(slot(Sym::Char('a'), false), &mut rng).style.bold);
    }

    #[test]
    fn lambda_spares_head() {
        let mut rng = Rng::with_seed(1);
        let l = Look {
            lambda: true,
            ..look()
        };
        assert_eq!(l.cell(slot(Sym::Char('x'), false), &mut rng).ch, 'λ');
        assert_eq!(l.cell(slot(Sym::Char('x'), true), &mut rng).ch, 'x');
    }

    #[test]
    fn rainbow_colors_heads_too() {
        let mut rng = Rng::with_seed(1);
        let l = Look {
            rainbow: true,
            ..look()
        };
        for _ in 0..50 {
            let c = l.cell(slot(Sym::Char('x'), true), &mut rng);
            assert!(RAINBOW.contains(&c.style.color));
        }
    }

    #[test]
    fn palette_picks_members() {
        let mut rng = Rng::with_seed(1);
        let l = Look {
            palette: vec![Color::Red, Color::Rgb(1, 2, 3)],
            ..look()
        };
        let colors: Vec<Color> = (0..50)
            .map(|_| l.cell(slot(Sym::Char('x'), false), &mut rng).style.color)
            .collect();
        assert!(colors.contains(&Color::Red) && colors.contains(&Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn old_style_symbols() {
        let mut rng = Rng::with_seed(1);
        let l = look();
        let glow = l.cell(slot(Sym::Glow, false), &mut rng);
        assert_eq!((glow.ch, glow.style.color), ('&', Color::White));
        assert_eq!(l.cell(slot(Sym::Bar, false), &mut rng).ch, '|');
        assert_eq!(l.cell(slot(Sym::Blank, true), &mut rng), Cell::BLANK);
        assert_eq!(l.cell(slot(Sym::Empty, false), &mut rng), Cell::BLANK);
    }
}
