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

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Cells per character, 1 or 2.
    pub fn width(&self) -> u16 {
        self.width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_matches_cmatrix() {
        let set = Charset::ascii();
        assert_eq!(set.len(), 90);
        assert_eq!(set.width(), 1);
    }

    #[test]
    fn katakana_has_digits() {
        let set = Charset::katakana();
        assert_eq!(set.len(), 66);
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
        assert_eq!(set.len(), 4);
        assert_eq!(set.width(), 2);
    }

    #[test]
    fn pick_stays_in_set() {
        let set = Charset::custom("ab").unwrap();
        let mut rng = Rng::with_seed(1);
        assert!((0..100).all(|_| matches!(set.pick(&mut rng), 'a' | 'b')));
    }
}
