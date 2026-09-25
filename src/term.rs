// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 1999-2017 Chris Allegretta
// Copyright (C) 2017-Present Abishek V Ashok
// Copyright (C) 2026 Alexander Kharitonov

//! Terminal setup and restore.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::{cursor, execute, style, terminal};

/// Set while the terminal is ours.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Raw mode and alternate screen until drop.
pub struct Terminal(());

impl Terminal {
    pub fn enter(out: &mut impl Write) -> io::Result<Terminal> {
        terminal::enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        let guard = Terminal(());
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            hook(info);
        }));
        execute!(
            out,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            terminal::DisableLineWrap
        )?;
        Ok(guard)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        restore();
    }
}

/// Runs once, also from panics.
fn restore() {
    if !ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
    let _ = execute!(
        io::stdout(),
        style::ResetColor,
        terminal::Clear(terminal::ClearType::All),
        terminal::EnableLineWrap,
        cursor::Show,
        terminal::LeaveAlternateScreen,
    );
    let _ = terminal::disable_raw_mode();
}
