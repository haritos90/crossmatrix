// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 1999-2017 Chris Allegretta
// Copyright (C) 2017-Present Abishek V Ashok
// Copyright (C) 2026 Alexander Kharitonov

//! Rain character sets.

use fastrand::Rng;
use unicode_width::UnicodeWidthChar;

/// Characters to draw and their widest cell count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Charset {
    chars: Vec<char>,
    width: u16,
}

impl Charset {
    /// ASCII `!` to `z`, as cmatrix.
    pub fn ascii() -> Charset {
        Charset {
            chars: ('!'..='z').collect(),
            width: 1,
        }
    }

    /// Half-width katakana and digits.
    pub fn katakana() -> Charset {
        Charset {
            chars: ('\u{ff66}'..='\u{ff9d}').chain('0'..='9').collect(),
            width: 1,
        }
    }

    /// Printable one or two cell characters.
    pub fn custom(text: &str) -> Option<Charset> {
        let chars: Vec<char> = text
            .chars()
            .filter(|c| !c.is_whitespace() && matches!(c.width(), Some(1 | 2)))
            .collect();
        let width = chars.iter().filter_map(|c| c.width()).max()?;
        Some(Charset {
            chars,
            width: width as u16,
        })
    }

    pub fn pick(&self, rng: &mut Rng) -> char {
        self.chars[rng.usize(..self.chars.len())]
    }

    /// Cells per character, 1 or 2.
    pub fn width(&self) -> u16 {
        self.width
    }
}

#[cfg(test)]
mod tests;
