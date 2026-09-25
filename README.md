# crossmatrix

Matrix digital rain for the terminal. Linux, macOS, Windows.

Versus cmatrix 2.0: 5-30x less CPU, 2-30x less output, 25% less memory.

## Requirements

- Terminal with ANSI escape support; Windows 10 or later.
- `-c`: font with half-width katakana, e.g. Noto Sans Mono CJK.
- `-C #RRGGBB`: truecolor terminal.
- Build from source: Rust 1.85 or later.

## Installation

Binary: archive for the platform from [releases](../../releases/latest); put `crossmatrix` on `PATH`.

From source, in a clone:

    cargo install --path .

## Usage

    crossmatrix [OPTIONS]
    crossmatrix -ab -u 2 -C red
    crossmatrix -c -C '#00ff41' -T 10

| Option | Effect |
|---|---|
| `-a`, `--async` | Asynchronous scroll |
| `-b`, `--bold` | Bold characters on |
| `-B`, `--all-bold` | All characters bold |
| `-n`, `--no-bold` | No bold characters (default) |
| `-c`, `--katakana` | Half-width katakana and digits |
| `-U`, `--chars CHARS` | Custom characters, overrides `-c` |
| `-C`, `--color COLORS` | Color or comma list (default green) |
| `-r`, `--rainbow` | Rainbow colors |
| `-m`, `--lambda` | Lambda characters |
| `-k`, `--mutate` | Characters change while falling |
| `-o`, `--old-style` | Old-style scrolling |
| `-M`, `--message TEXT` | Centered message |
| `-L`, `--lock` | Ignore quit keys; `L` `L` `L` unlocks |
| `-s`, `--screensaver` | Exit on first keystroke |
| `-u`, `--delay 0-10` | Frame delay, 10 ms units (default 4) |
| `-T`, `--timeout SECS` | Exit after SECS seconds |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

Colors: `green` `red` `blue` `white` `yellow` `cyan` `magenta` `black` `default` `#RRGGBB`.

| Key | Effect |
|---|---|
| `q`, Ctrl-C, Ctrl-\\, Ctrl-Z | Quit, unless locked |
| `a` | Toggle asynchronous scroll |
| `b` `B` `n` | Bold: some, all, none |
| `0`-`9` | Frame delay |
| `!` `@` `#` `$` `%` `^` `&` `)` | Red, green, yellow, blue, magenta, cyan, white, black |
| `r` | Toggle rainbow |
| `m` | Toggle lambda |
| `p` | Pause |
| `L` | Lock; `L` `L` `L` unlocks |

## Development

    cargo test
    cargo clippy --all-targets
    cargo build --release

Release: push tag `vX.Y.Z` matching the `Cargo.toml` version.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

Based on [cmatrix](https://github.com/abishekvashok/cmatrix) by Chris Allegretta and Abishek V Ashok, and on its community issues and pull requests.
