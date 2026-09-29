// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Alexander Kharitonov

//! Option parsing tests.

use super::*;

fn run(args: &[&str]) -> Options {
    match parse(args.iter().copied()) {
        Ok(Command::Run(o)) => o,
        other => panic!("{args:?}: {other:?}"),
    }
}

fn fails(args: &[&str]) -> String {
    parse(args.iter().copied()).unwrap_err().to_string()
}

#[test]
fn defaults() {
    assert_eq!(run(&[]), Options::default());
}

#[test]
fn bundled_flags() {
    let o = run(&["-bak"]);
    assert!(o.asynch && o.mutate);
    assert_eq!(o.bold, Bold::Mixed);
}

#[test]
fn long_forms() {
    let o = run(&[
        "--async",
        "--rainbow",
        "--lambda",
        "--screensaver",
        "--lock",
    ]);
    assert!(o.asynch && o.rainbow && o.lambda && o.screensaver && o.lock);
}

#[test]
fn bold_precedence() {
    assert_eq!(run(&["-n", "-b"]).bold, Bold::Off);
    assert_eq!(run(&["-b", "-B"]).bold, Bold::All);
    assert_eq!(run(&["-B", "-b"]).bold, Bold::All);
    assert_eq!(run(&["--no-bold", "--all-bold"]).bold, Bold::Off);
}

#[test]
fn delay_forms() {
    for args in [
        &["-u2"][..],
        &["-u", "2"],
        &["--delay=2"],
        &["--delay", "2"],
    ] {
        assert_eq!(run(args).delay, 2);
    }
    assert_eq!(run(&["-u", "10"]).delay, 10);
    assert!(fails(&["-u", "11"]).contains("invalid delay"));
    assert!(fails(&["-u", "x"]).contains("invalid delay"));
    assert!(fails(&["-u", "-1"]).contains("invalid delay"));
}

#[test]
fn colors() {
    assert_eq!(run(&["-C", "red"]).palette, [Color::Red]);
    assert_eq!(run(&["-CRED"]).palette, [Color::Red]);
    assert_eq!(run(&["-C", "#00ff41"]).palette, [Color::Rgb(0, 255, 65)]);
    assert_eq!(
        run(&["--color", "00FF41"]).palette,
        [Color::Rgb(0, 255, 65)]
    );
    assert_eq!(
        run(&["-C", "red, white,blue"]).palette,
        [Color::Red, Color::White, Color::Blue]
    );
    assert_eq!(run(&["-C", "black"]).palette, [Color::Black]);
    assert!(fails(&["-C", "purple"]).contains("invalid color"));
    assert!(fails(&["-C", ""]).contains("invalid color"));
    assert!(fails(&["-C", "red,"]).contains("invalid color"));
    assert!(fails(&["-C", "#12345"]).contains("invalid color"));
}

#[test]
fn charsets() {
    assert_eq!(run(&[]).charset, Charset::ascii());
    assert_eq!(run(&["-c"]).charset, Charset::katakana());
    assert_eq!(
        run(&["-c", "-U", "01"]).charset,
        Charset::custom("01").unwrap()
    );
    assert_eq!(run(&["--chars=日月"]).charset.width(), 2);
    assert!(fails(&["-U", " "]).contains("no printable"));
}

#[test]
fn message_and_lock() {
    let o = run(&["-L", "-M", "hello world"]);
    assert!(o.lock);
    assert_eq!(o.message.as_deref(), Some("hello world"));
    assert_eq!(run(&["-L"]).message, None);
}

#[test]
fn timeouts() {
    assert_eq!(
        run(&["-T", "1.5"]).timeout,
        Some(Duration::from_millis(1500))
    );
    assert_eq!(run(&["--timeout=3"]).timeout, Some(Duration::from_secs(3)));
    for bad in ["0", "-1", "x", "inf", "NaN"] {
        assert!(fails(&["-T", bad]).contains("invalid timeout"), "{bad}");
    }
}

#[test]
fn help_and_version() {
    for args in [&["-h"][..], &["-?"], &["--help"], &["-a", "-h", "-l"]] {
        assert_eq!(parse(args.iter().copied()).unwrap(), Command::Help);
    }
    assert_eq!(parse(["-V"]).unwrap(), Command::Version);
    assert_eq!(parse(["--version"]).unwrap(), Command::Version);
}

#[test]
fn removed_and_unknown() {
    for flag in [
        "-l",
        "-f",
        "-x",
        "-t",
        "--tty",
        "-o",
        "--old-style",
        "stray",
    ] {
        assert!(parse([flag]).is_err(), "{flag}");
    }
}

#[test]
fn help_lists_every_option() {
    for flag in [
        "-a", "-b", "-B", "-n", "-c", "-U", "-C", "-r", "-m", "-k", "-M", "-L", "-s", "-u", "-T",
        "-h", "-V",
    ] {
        assert!(HELP.contains(&format!("  {flag}, --")), "{flag}");
    }
}
