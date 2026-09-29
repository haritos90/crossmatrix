// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Alexander Kharitonov

//! Grid and encoder tests.

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

/// Grid as text rows.
fn text(s: &Screen) -> Vec<String> {
    s.cells
        .chunks(usize::from(s.width.max(1)))
        .map(|row| {
            row.iter()
                .filter(|c| **c != Cell::CONT)
                .map(|c| c.ch)
                .collect()
        })
        .collect()
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
    assert_eq!(text(&s)[0], "  日      ");
    s.set(2, 0, Cell::BLANK);
    assert_eq!(render(&mut s), "\x1b[1;3H  ");
    assert_eq!(text(&s)[0], " ".repeat(10));
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
fn zero_size_is_safe() {
    let mut s = Screen::new(0, 0);
    s.set(0, 0, Cell::new('a', GREEN));
    s.set_banner(Some("x"));
    assert_eq!(render(&mut s), "\x1b[0m\x1b[2J");
}
