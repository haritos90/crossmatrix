# Matrix-style terminal app: crossmatrix

Matrix digital rain in the terminal for Linux, macOS and Windows. Inspired by [cmatrix](https://github.com/abishekvashok/cmatrix), this Rust implementation uses 5-30x less CPU, produces 2-30x less output, and requires 25% less memory.

## Requirements

- Terminal with ANSI escape support (for Windows users: Windows 10 or later).
- `-c`: font with half-width katakana, e.g. Noto Sans Mono CJK.
- `-C #RRGGBB`: truecolor terminal.
- Build from source: Rust 1.85 or later.

## Installation

### Binary from Github:

Choose your platform from [releases](../../releases/latest); put `crossmatrix` on `PATH`.

List of platforms:

| Platform             | Archive                           |
| -------------------- | --------------------------------- |
|  Linux x86_64        | x86_64-unknown-linux-musl.tar.gz  |
| Linux ARM64          | aarch64-unknown-linux-musl.tar.gz |
|  macOS Intel         | x86_64-apple-darwin.tar.gz        |
|  macOS Apple Silicon | aarch64-apple-darwin.tar.gz       |
| Windows x64          |  x86_64-pc-windows-msvc.zip       |
|  Windows ARM64       | aarch64-pc-windows-msvc.zip       |

### Build from source:

    git clone https://github.com/haritos90/crossmatrix.git
    cd crossmatrix
    cargo install --path .

## Usage

    crossmatrix [OPTIONS]
    crossmatrix -ab -u 2 -C red
    crossmatrix -c -C '#00ff41' -T 10

| Option                 | Effect                               |
| ---------------------- | ------------------------------------ |
| `-a`, `--async`        | Asynchronous scroll                  |
| `-b`, `--bold`         | Bold characters on                   |
| `-B`, `--all-bold`     | All characters bold                  |
| `-n`, `--no-bold`      | No bold characters (default)         |
| `-c`, `--katakana`     | Half-width katakana and digits       |
| `-U`, `--chars CHARS`  | Custom characters, overrides `-c`    |
| `-C`, `--color COLORS` | Color or comma list (default green)  |
| `-r`, `--rainbow`      | Rainbow colors                       |
| `-m`, `--lambda`       | Lambda characters                    |
| `-k`, `--mutate`       | Characters change while falling      |
| `-M`, `--message TEXT` | Centered message                     |
| `-L`, `--lock`         | Ignore all keys; `L` `L` `L` unlocks |
| `-s`, `--screensaver`  | Exit on first keystroke              |
| `-u`, `--delay 0-10`   | Frame delay, 10 ms units (default 4) |
| `-T`, `--timeout SECS` | Exit after SECS seconds              |
| `-h`, `--help`         | Print help                           |
| `-V`, `--version`      | Print version                        |

Colors: `green` `red` `blue` `white` `yellow` `cyan` `magenta` `black` `default` `#RRGGBB`.

| Key                             | Effect                                                          |
| ------------------------------- | --------------------------------------------------------------- |
| `q`, Ctrl-C, Ctrl-\\, Ctrl-Z    | Quit                                                            |
| `a`                             | Toggle asynchronous scroll                                      |
| `b` `B` `n`                     | Bold: some, all, none                                           |
| `0`-`9`                         | Frame delay                                                     |
| `!` `@` `#` `$` `%` `^` `&` `)` | Red, green (default), yellow, blue, magenta, cyan, white, black |
| `r`                             | Toggle rainbow                                                  |
| `m`                             | Toggle lambda                                                   |
| `c`                             | Toggle katakana                                                 |
| `k`                             | Toggle mutate                                                   |
| `p`                             | Pause                                                           |
| `L`                             | Lock; `L` `L` `L` unlocks                                       |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

## Credits

Inspired by [cmatrix](https://github.com/abishekvashok/cmatrix) by Chris Allegretta and Abishek V Ashok, and its community issues and pull requests.
