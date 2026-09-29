// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Alexander Kharitonov

//! Cell styling tests.

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
fn gaps_are_blank() {
    let mut rng = Rng::with_seed(1);
    let l = look();
    assert_eq!(l.cell(slot(Sym::Blank, true), &mut rng), Cell::BLANK);
    assert_eq!(l.cell(slot(Sym::Empty, false), &mut rng), Cell::BLANK);
}
