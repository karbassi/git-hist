# git-hist

[![Rust](https://github.com/karbassi/git-hist/workflows/Rust/badge.svg)](https://github.com/karbassi/git-hist/actions)
[![crates.io](https://img.shields.io/crates/v/git-hist.svg)](https://crates.io/crates/git-hist)
[![license: MIT](https://img.shields.io/badge/license-MIT-yellow.svg)](https://github.com/karbassi/git-hist/blob/main/LICENSE)

A CLI tool to quickly browse the git history of files **on a terminal**. This project is inspired by [git-history](https://github.com/pomber/git-history).

<div align="center">
    <img src="screenshots/screenshot_01.png" />
</div>

## Installation

```sh
cargo install git-hist
```

### From source

```sh
git clone https://github.com/karbassi/git-hist.git
cd git-hist
cargo install --path .
```

## Usage

```sh
git hist <file>
```

You can use `git-hist` as a git subcommand, so the hyphen is not required.

### Keymap

- <kbd>Left</kbd> / <kbd>Right</kbd> : Go to a previous/next commit.
- <kbd>Up</kbd> / <kbd>Down</kbd> or mouse scrolls: Scroll up/down.
- <kbd>PageUp</kbd> / <kbd>PageDown</kbd> : Scroll page up/down.
- <kbd>Home</kbd> / <kbd>End</kbd> : Scroll to the top/bottom.
- <kbd>q</kbd>, <kbd>Ctrl</kbd>+<kbd>c</kbd>, <kbd>Ctrl</kbd>+<kbd>d</kbd> : Exit.

### Options

```
Usage: git-hist [OPTIONS] <file>

Arguments:
  <file>  Set a target file path

Options:
      --full-hash             Show full commit hashes instead of abbreviated commit hashes
      --beyond-last-line      Set whether the view will scroll beyond the last line
      --emphasize-diff        Set whether the view will emphasize different parts
      --name-of <user>        Use whether authors or committers for names [default: author]
                              [possible values: author, committer]
      --date-of <user>        Use whether authors or committers for dates [default: author]
                              [possible values: author, committer]
      --date-format <format>  Set date format [default: [%Y-%m-%d]]
      --tab-size <size>       Set the number of spaces for a tab character (\t) [default: 4]
  -h, --help                  Print help
  -V, --version               Print version
```

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [mise](https://mise.jdx.dev/) (optional, for task running and git hooks)

### Setup

```sh
git clone https://github.com/karbassi/git-hist.git
cd git-hist
mise install        # installs Rust toolchain and configures git hooks
```

### Tasks

```sh
mise run fmt        # fix code formatting
mise run clippy     # run clippy lints
mise run test       # run test suite
mise run lint       # fmt + clippy
mise run check      # fmt + clippy + test
```

A pre-commit hook runs `cargo fmt` and `cargo clippy` automatically on every commit.

## Contributors

- [Ark](https://github.com/arkark) — original author
- [Ali Karbassi](https://github.com/karbassi)

## License

MIT
