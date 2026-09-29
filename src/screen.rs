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

    /// Bytes bringing the terminal up to date.
    pub fn render(&mut self) -> &[u8] {
        self.out.clear();
        self.order.clear();
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
}

#[cfg(test)]
mod tests;
