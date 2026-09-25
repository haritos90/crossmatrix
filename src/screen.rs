// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 1999-2017 Chris Allegretta
// Copyright (C) 2017-Present Abishek V Ashok
// Copyright (C) 2026 Alexander Kharitonov

//! Cell grid with dirty tracking and ANSI output.

use std::io::Write as _;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Spaces cheaper than a cursor move up to here.
const MAX_SKIP: u16 = 3;

/// Foreground colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Rgb(u8, u8, u8),
}

impl Color {
    /// Sort rank; equal colors group together.
    fn rank(self) -> u32 {
        match self {
            Color::Default => 1,
            Color::Black => 2,
            Color::Red => 3,
            Color::Green => 4,
            Color::Yellow => 5,
            Color::Blue => 6,
            Color::Magenta => 7,
            Color::Cyan => 8,
            Color::White => 9,
            Color::Rgb(r, g, b) => 16 + (u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)),
        }
    }

    fn sgr(self, out: &mut Vec<u8>) {
        let code: &[u8] = match self {
            Color::Default => b"39",
            Color::Black => b"30",
            Color::Red => b"31",
            Color::Green => b"32",
            Color::Yellow => b"33",
            Color::Blue => b"34",
            Color::Magenta => b"35",
            Color::Cyan => b"36",
            Color::White => b"37",
            Color::Rgb(r, g, b) => {
                let _ = write!(out, "38;2;{r};{g};{b}");
                return;
            }
        };
        out.extend_from_slice(code);
    }
}

/// Color and intensity of a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub color: Color,
    pub bold: bool,
}

impl Style {
    /// Terminal defaults.
    pub const PLAIN: Style = Style {
        color: Color::Default,
        bold: false,
    };
}

/// One screen position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
    pub wide: bool,
}

impl Cell {
    pub const BLANK: Cell = Cell {
        ch: ' ',
        style: Style::PLAIN,
        wide: false,
    };
    /// Right half of a wide cell.
    const CONT: Cell = Cell {
        ch: '\0',
        style: Style::PLAIN,
        wide: false,
    };

    /// Spaces collapse to `BLANK`.
    pub fn new(ch: char, style: Style) -> Cell {
        if ch == ' ' {
            return Cell::BLANK;
        }
        Cell {
            ch,
            style,
            wide: ch.width() == Some(2),
        }
    }

    /// Style group; blanks first.
    fn key(self) -> u64 {
        if self.ch == ' ' {
            return 0;
        }
        u64::from(self.style.color.rank() << 1 | u32::from(self.style.bold))
    }
}

/// Clipped message box.
#[derive(Debug)]
struct Banner {
    x0: u16,
    x1: u16,
    y0: u16,
    y1: u16,
    lines: Vec<(u16, u16, String)>,
}

impl Banner {
    /// Centered like cmatrix, left edge even.
    fn place(text: &str, width: u16, height: u16) -> Option<Banner> {
        if text.is_empty() || width == 0 || height == 0 {
            return None;
        }
        let text: String = text
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        let (w, h) = (i32::from(width), i32::from(height));
        let row = h / 2;
        let mut left = w / 2 - text.width() as i32 / 2 - 2;
        let mut pad = 2;
        if left.rem_euclid(2) == 1 {
            left -= 1;
            pad += 1;
        }
        let body = format!("{}{text}  ", " ".repeat(pad));
        let span = body.width() as i32;
        let blank = " ".repeat(span as usize);
        let lines = [(row - 1, &blank), (row, &body), (row + 1, &blank)]
            .into_iter()
            .filter(|&(y, _)| (0..h).contains(&y))
            .filter_map(|(y, line)| clip(line, left, w).map(|(x, s)| (x, y as u16, s)))
            .collect();
        Some(Banner {
            x0: left.clamp(0, w) as u16,
            x1: (left + span).clamp(0, w) as u16,
            y0: (row - 1).clamp(0, h) as u16,
            y1: (row + 2).clamp(0, h) as u16,
            lines,
        })
    }

    fn covers(&self, x: u16, y: u16) -> bool {
        (self.x0..self.x1).contains(&x) && (self.y0..self.y1).contains(&y)
    }
}

/// Visible part of `line` starting at column `left`.
fn clip(line: &str, left: i32, width: i32) -> Option<(u16, String)> {
    let mut start = None;
    let mut out = String::new();
    let mut col = left;
    for c in line.chars() {
        let w = c.width().unwrap_or(0) as i32;
        if col >= 0 && col + w <= width {
            start.get_or_insert(col as u16);
            out.push(c);
        }
        col += w;
    }
    start.map(|x| (x, out))
}

/// Desired screen state and its encoder.
pub struct Screen {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
    queued: Vec<bool>,
    dirty: Vec<u32>,
    order: Vec<u64>,
    text: Option<String>,
    banner: Option<Banner>,
    repaint: bool,
    scrolls: u16,
    stale_banner: bool,
    pen: Style,
    cursor: Option<(u16, u16)>,
    out: Vec<u8>,
}

impl Screen {
    pub fn new(width: u16, height: u16) -> Screen {
        let mut screen = Screen {
            width: 0,
            height: 0,
            cells: Vec::new(),
            queued: Vec::new(),
            dirty: Vec::new(),
            order: Vec::new(),
            text: None,
            banner: None,
            repaint: true,
            scrolls: 0,
            stale_banner: false,
            pen: Style::PLAIN,
            cursor: None,
            out: Vec::new(),
        };
        screen.resize(width, height);
        screen
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Blank grid; next render repaints.
    pub fn resize(&mut self, width: u16, height: u16) {
        let n = usize::from(width) * usize::from(height);
        self.width = width;
        self.height = height;
        self.cells.clear();
        self.cells.resize(n, Cell::BLANK);
        self.queued.clear();
        self.queued.resize(n, false);
        self.dirty.clear();
        self.banner = self
            .text
            .as_deref()
            .and_then(|t| Banner::place(t, width, height));
        self.repaint = true;
        self.scrolls = 0;
        self.stale_banner = false;
    }

    /// Centered message over the rain.
    pub fn set_banner(&mut self, text: Option<&str>) {
        if self.text.as_deref() == text {
            return;
        }
        self.text = text.map(str::to_owned);
        self.banner = text.and_then(|t| Banner::place(t, self.width, self.height));
        self.repaint = true;
    }

    /// Wide cells also claim the next column.
    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if x >= self.width || y >= self.height {
            return;
        }
        let cell = if cell.wide && x + 1 >= self.width {
            Cell::BLANK
        } else {
            cell
        };
        let i = usize::from(y) * usize::from(self.width) + usize::from(x);
        let old = self.cells[i];
        if old == cell {
            return;
        }
        self.put(i, cell);
        if cell.wide {
            self.put(i + 1, Cell::CONT);
        } else if old.wide {
            self.put(i + 1, Cell::BLANK);
        }
    }

    fn put(&mut self, i: usize, cell: Cell) {
        if self.cells[i] == cell {
            return;
        }
        self.cells[i] = cell;
        self.queue(i);
    }

    fn queue(&mut self, i: usize) {
        if !self.queued[i] {
            self.queued[i] = true;
            self.dirty.push(i as u32);
        }
    }

    /// Move column `x` down one row, colors kept.
    pub fn shift_down(&mut self, x: u16) {
        if x >= self.width {
            return;
        }
        let w = usize::from(self.width);
        for y in (1..self.height).rev() {
            let above = self.cells[usize::from(y - 1) * w + usize::from(x)];
            self.set(x, y, above);
        }
        self.set(x, 0, Cell::BLANK);
    }

    /// Whole grid down one row via IL.
    pub fn scroll_down(&mut self) -> bool {
        if self.repaint || self.height < 2 {
            return false;
        }
        let (w, n) = (usize::from(self.width), self.cells.len());
        self.cells.copy_within(..n - w, w);
        self.cells[..w].fill(Cell::BLANK);
        let pending = std::mem::take(&mut self.dirty);
        for &i in &pending {
            self.queued[i as usize] = false;
        }
        for i in pending {
            let below = i as usize + w;
            if below < n {
                self.queue(below);
            }
        }
        // Banner scrolled too: redraw it and the row below.
        if let Some((x0, x1, y1)) = self.banner.as_ref().map(|b| (b.x0, b.x1, b.y1)) {
            if y1 < self.height {
                let row = usize::from(y1) * w;
                for x in x0..x1 {
                    self.queue(row + usize::from(x));
                }
            }
            self.stale_banner = true;
        }
        self.scrolls += 1;
        true
    }

    /// Bytes bringing the terminal up to date.
    pub fn render(&mut self) -> &[u8] {
        self.out.clear();
        self.order.clear();
        let scrolls = std::mem::take(&mut self.scrolls);
        let stale_banner = std::mem::take(&mut self.stale_banner);
        if scrolls > 0 && !self.repaint {
            self.out.extend_from_slice(b"\x1b[H");
            if scrolls == 1 {
                self.out.extend_from_slice(b"\x1b[L");
            } else {
                let _ = write!(self.out, "\x1b[{scrolls}L");
            }
            self.cursor = None;
        }
        if self.repaint {
            self.repaint = false;
            self.out.extend_from_slice(b"\x1b[0m\x1b[2J");
            self.pen = Style::PLAIN;
            self.cursor = None;
            for (i, &cell) in self.cells.iter().enumerate() {
                if cell != Cell::BLANK && cell != Cell::CONT {
                    self.order.push(cell.key() << 32 | i as u64);
                }
            }
            self.paint_order();
            self.paint_banner();
        } else {
            for &i in &self.dirty {
                self.order
                    .push(self.cells[i as usize].key() << 32 | u64::from(i));
            }
            self.paint_order();
            if stale_banner {
                self.paint_banner();
            }
        }
        for &i in &self.dirty {
            self.queued[i as usize] = false;
        }
        self.dirty.clear();
        &self.out
    }

    fn paint_order(&mut self) {
        self.order.sort_unstable();
        for k in 0..self.order.len() {
            self.paint((self.order[k] & 0xffff_ffff) as usize);
        }
    }

    fn paint(&mut self, i: usize) {
        let cell = self.cells[i];
        let w = usize::from(self.width);
        let (x, y) = ((i % w) as u16, (i / w) as u16);
        if cell == Cell::CONT || self.masked(x, y) {
            return;
        }
        self.move_to(x, y);
        if cell.ch != ' ' && cell.style != self.pen {
            self.sgr(cell.style);
        }
        let mut buf = [0; 4];
        self.out
            .extend_from_slice(cell.ch.encode_utf8(&mut buf).as_bytes());
        let next = x + if cell.wide { 2 } else { 1 };
        self.cursor = (next < self.width).then_some((next, y));
    }

    fn masked(&self, x: u16, y: u16) -> bool {
        self.banner.as_ref().is_some_and(|b| b.covers(x, y))
    }

    /// Cells in `[from, to)` of row `y` show nothing.
    fn blank_span(&self, from: u16, to: u16, y: u16) -> bool {
        let row = usize::from(y) * usize::from(self.width);
        (from..to).all(|x| self.cells[row + usize::from(x)] == Cell::BLANK && !self.masked(x, y))
    }

    fn move_to(&mut self, x: u16, y: u16) {
        if let Some((cx, cy)) = self.cursor {
            if cy == y && x >= cx {
                let gap = x - cx;
                if gap == 0 {
                    return;
                }
                if gap <= MAX_SKIP && self.blank_span(cx, x, y) {
                    self.out.extend(std::iter::repeat_n(b' ', usize::from(gap)));
                } else if gap == 1 {
                    self.out.extend_from_slice(b"\x1b[C");
                } else {
                    let _ = write!(self.out, "\x1b[{gap}C");
                }
                return;
            }
        }
        let _ = write!(self.out, "\x1b[{};{}H", y + 1, x + 1);
    }

    /// Only the changed attributes.
    fn sgr(&mut self, style: Style) {
        self.out.extend_from_slice(b"\x1b[");
        let bold = style.bold != self.pen.bold;
        if bold {
            self.out
                .extend_from_slice(if style.bold { b"1" } else { b"22" });
        }
        if style.color != self.pen.color {
            if bold {
                self.out.push(b';');
            }
            style.color.sgr(&mut self.out);
        }
        self.out.push(b'm');
        self.pen = style;
    }

    fn paint_banner(&mut self) {
        let Some(banner) = self.banner.take() else {
            return;
        };
        if self.pen != Style::PLAIN {
            self.out.extend_from_slice(b"\x1b[0m");
            self.pen = Style::PLAIN;
        }
        for (x, y, text) in &banner.lines {
            let _ = write!(self.out, "\x1b[{};{}H{text}", y + 1, x + 1);
        }
        self.cursor = None;
        self.banner = Some(banner);
    }

    /// Grid as text rows.
    #[cfg(test)]
    pub fn text(&self) -> Vec<String> {
        self.cells
            .chunks(usize::from(self.width.max(1)))
            .map(|row| {
                row.iter()
                    .filter(|c| **c != Cell::CONT)
                    .map(|c| c.ch)
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREEN: Style = Style {
        color: Color::Green,
        bold: false,
    };
    const WHITE: Style = Style {
        color: Color::White,
        bold: false,
    };

    fn settled(width: u16, height: u16) -> Screen {
        let mut s = Screen::new(width, height);
        s.render();
        s
    }

    fn render(s: &mut Screen) -> String {
        String::from_utf8(s.render().to_vec()).unwrap()
    }

    #[test]
    fn first_render_clears() {
        let mut s = Screen::new(4, 2);
        assert_eq!(render(&mut s), "\x1b[0m\x1b[2J");
    }

    #[test]
    fn unchanged_frame_writes_nothing() {
        let mut s = settled(10, 5);
        assert_eq!(render(&mut s), "");
        s.set(3, 2, Cell::BLANK);
        assert_eq!(render(&mut s), "");
    }

    #[test]
    fn single_cell() {
        let mut s = settled(10, 5);
        s.set(3, 2, Cell::new('x', GREEN));
        assert_eq!(render(&mut s), "\x1b[3;4H\x1b[32mx");
        s.set(3, 2, Cell::new('x', GREEN));
        assert_eq!(render(&mut s), "");
    }

    #[test]
    fn pen_persists_across_frames() {
        let mut s = settled(10, 5);
        s.set(0, 0, Cell::new('a', GREEN));
        render(&mut s);
        s.set(0, 1, Cell::new('b', GREEN));
        assert_eq!(render(&mut s), "\x1b[2;1Hb");
    }

    #[test]
    fn short_gap_uses_spaces() {
        let mut s = settled(10, 5);
        s.set(0, 0, Cell::new('a', GREEN));
        s.set(2, 0, Cell::new('b', GREEN));
        assert_eq!(render(&mut s), "\x1b[1;1H\x1b[32ma b");
    }

    #[test]
    fn long_gap_uses_forward_move() {
        let mut s = settled(20, 5);
        s.set(0, 0, Cell::new('a', GREEN));
        s.set(10, 0, Cell::new('b', GREEN));
        assert_eq!(render(&mut s), "\x1b[1;1H\x1b[32ma\x1b[9Cb");
    }

    #[test]
    fn gap_over_text_moves_cursor() {
        let mut s = settled(10, 5);
        s.set(1, 0, Cell::new('z', WHITE));
        render(&mut s);
        s.set(0, 0, Cell::new('a', WHITE));
        s.set(2, 0, Cell::new('b', WHITE));
        assert_eq!(render(&mut s), "\x1b[1;1Ha\x1b[Cb");
    }

    #[test]
    fn styles_group_together() {
        let mut s = settled(10, 5);
        s.set(0, 0, Cell::new('a', GREEN));
        s.set(0, 1, Cell::new('b', WHITE));
        s.set(0, 2, Cell::new('c', GREEN));
        assert_eq!(
            render(&mut s),
            "\x1b[1;1H\x1b[32ma\x1b[3;1Hc\x1b[2;1H\x1b[37mb"
        );
    }

    #[test]
    fn blanks_keep_pen() {
        let mut s = settled(10, 5);
        s.set(0, 0, Cell::new('a', GREEN));
        render(&mut s);
        s.set(0, 0, Cell::BLANK);
        assert_eq!(render(&mut s), "\x1b[1;1H ");
    }

    #[test]
    fn bold_transitions() {
        let mut s = settled(10, 5);
        s.set(
            0,
            0,
            Cell::new(
                'a',
                Style {
                    color: Color::Green,
                    bold: true,
                },
            ),
        );
        assert_eq!(render(&mut s), "\x1b[1;1H\x1b[1;32ma");
        s.set(0, 1, Cell::new('b', GREEN));
        assert_eq!(render(&mut s), "\x1b[2;1H\x1b[22mb");
    }

    #[test]
    fn rgb_color() {
        let mut s = settled(10, 5);
        s.set(
            0,
            0,
            Cell::new(
                'a',
                Style {
                    color: Color::Rgb(0, 255, 65),
                    bold: false,
                },
            ),
        );
        assert_eq!(render(&mut s), "\x1b[1;1H\x1b[38;2;0;255;65ma");
    }

    #[test]
    fn last_column_forgets_cursor() {
        let mut s = settled(3, 2);
        s.set(2, 0, Cell::new('a', GREEN));
        s.set(2, 1, Cell::new('b', GREEN));
        assert_eq!(render(&mut s), "\x1b[1;3H\x1b[32ma\x1b[2;3Hb");
    }

    #[test]
    fn wide_cell_claims_next_column() {
        let mut s = settled(10, 2);
        s.set(2, 0, Cell::new('日', GREEN));
        assert_eq!(render(&mut s), "\x1b[1;3H\x1b[32m日");
        assert_eq!(s.text()[0], "  日      ");
        s.set(2, 0, Cell::BLANK);
        assert_eq!(render(&mut s), "\x1b[1;3H  ");
        assert_eq!(s.text()[0], " ".repeat(10));
    }

    #[test]
    fn wide_cell_never_splits_at_edge() {
        let mut s = settled(3, 1);
        s.set(2, 0, Cell::new('日', GREEN));
        assert_eq!(render(&mut s), "");
    }

    #[test]
    fn repaint_draws_everything() {
        let mut s = settled(10, 5);
        s.set(1, 1, Cell::new('a', GREEN));
        render(&mut s);
        s.resize(10, 5);
        assert_eq!(render(&mut s), "\x1b[0m\x1b[2J");
        s.set(1, 1, Cell::new('a', GREEN));
        s.set_banner(Some("x"));
        let out = render(&mut s);
        assert!(out.starts_with("\x1b[0m\x1b[2J\x1b[2;2H\x1b[32ma\x1b[0m"));
    }

    #[test]
    fn banner_layout() {
        let mut s = settled(20, 5);
        s.set_banner(Some("hi"));
        let out = render(&mut s);
        assert_eq!(
            out,
            "\x1b[0m\x1b[2J\x1b[2;7H       \x1b[3;7H   hi  \x1b[4;7H       "
        );
    }

    #[test]
    fn banner_masks_rain() {
        let mut s = settled(20, 5);
        s.set_banner(Some("hi"));
        render(&mut s);
        s.set(10, 2, Cell::new('a', GREEN));
        s.set(0, 0, Cell::new('b', GREEN));
        assert_eq!(render(&mut s), "\x1b[1;1H\x1b[32mb");
    }

    #[test]
    fn banner_clips() {
        let mut s = settled(4, 1);
        s.set_banner(Some("longer"));
        let out = render(&mut s);
        assert_eq!(out, "\x1b[0m\x1b[2J\x1b[1;1Honge");
    }

    #[test]
    fn shift_moves_cells_with_style() {
        let mut s = settled(4, 3);
        s.set(0, 0, Cell::new('a', GREEN));
        s.set(0, 1, Cell::new('b', WHITE));
        render(&mut s);
        s.shift_down(0);
        assert_eq!(s.text(), ["    ", "a   ", "b   "]);
        assert_eq!(
            render(&mut s),
            "\x1b[1;1H \x1b[2;1H\x1b[32ma\x1b[3;1H\x1b[37mb"
        );
    }

    #[test]
    fn scroll_uses_insert_line() {
        let mut s = settled(4, 3);
        s.set(0, 0, Cell::new('a', GREEN));
        s.set(2, 1, Cell::new('b', GREEN));
        render(&mut s);
        assert!(s.scroll_down());
        s.set(0, 0, Cell::new('c', GREEN));
        assert_eq!(render(&mut s), "\x1b[H\x1b[L\x1b[1;1Hc");
        assert_eq!(s.text(), ["c   ", "a   ", "  b "]);
    }

    #[test]
    fn scroll_keeps_pending_cells() {
        let mut s = settled(4, 3);
        s.set(1, 0, Cell::new('a', GREEN));
        s.set(1, 2, Cell::new('z', GREEN));
        assert!(s.scroll_down());
        assert_eq!(render(&mut s), "\x1b[H\x1b[L\x1b[2;2H\x1b[32ma");
    }

    #[test]
    fn no_scroll_before_repaint() {
        let mut s = Screen::new(10, 5);
        assert!(!s.scroll_down());
    }

    #[test]
    fn scroll_redraws_banner() {
        let mut s = settled(20, 5);
        s.set_banner(Some("hi"));
        s.set(8, 3, Cell::new('a', GREEN));
        render(&mut s);
        assert!(s.scroll_down());
        let out = render(&mut s);
        assert!(out.starts_with("\x1b[H\x1b[L\x1b[5;7H"), "{out:?}");
        assert!(out.contains("\x1b[32ma"), "{out:?}");
        assert!(out.ends_with("\x1b[3;7H   hi  \x1b[4;7H       "), "{out:?}");
        assert_eq!(s.text()[4], "        a           ");
    }

    #[test]
    fn zero_size_is_safe() {
        let mut s = Screen::new(0, 0);
        s.set(0, 0, Cell::new('a', GREEN));
        s.set_banner(Some("x"));
        assert_eq!(render(&mut s), "\x1b[0m\x1b[2J");
    }
}
