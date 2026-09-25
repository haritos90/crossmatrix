// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Alexander Kharitonov

//! Binary behavior without a terminal.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_crossmatrix"))
        .args(args)
        .output()
        .unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn help() {
    let o = run(&["--help"]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).starts_with("Usage: crossmatrix"));
}

#[test]
fn version() {
    let o = run(&["-V"]);
    assert!(o.status.success());
    let expected = format!("crossmatrix {}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(String::from_utf8_lossy(&o.stdout), expected);
}

#[test]
fn usage_errors_exit_2() {
    for args in [&["-C", "purple"][..], &["-l"], &["-u", "99"], &["-T", "0"]] {
        let o = run(args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(stderr(&o).contains("--help"), "{args:?}");
    }
}

#[test]
fn needs_terminal() {
    let o = run(&[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("not a terminal"));
}
