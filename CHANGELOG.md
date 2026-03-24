# Changelog

All notable changes to this project will be documented in this file.

## [1.1.0] - 2026-03-24

### Added
- Pre-commit hooks via mise for fmt and clippy checks
- mise tasks: `fmt`, `clippy`, `test`, `lint`, `check`
- Release workflow for cross-platform binaries (macOS ARM/Intel, Linux)
- Comprehensive TDD integration test suite with regression tests
- Submodule detection with actionable error messages
- Copilot code review enabled on PRs

### Changed
- Migrated from `tui` 0.18 to `ratatui` 0.29
- Upgraded `crossterm` from 0.24 to 0.28 (adds key repeat support)
- Upgraded `clap` from 3.2 to 4.x with derive API
- Updated Rust edition from 2018 to 2021
- Replaced `once_cell` with `std::sync::OnceLock`/`LazyLock`
- Moved `diff_height` calculation from Dashboard to State (breaks circular dependency)
- Reduced State boilerplate with `with_line_index`/`with_point` helpers
- Cleaned up dashboard reference display with chained iterators
- Line number column width now reflects current commit (no longer monotonically grows)
- Version flag changed from `-v` to `-V` (clap 4 convention)

### Fixed
- `--date-of` flag was ignored (dashboard always used `--name-of` for dates)
- Deleted file status displayed `new_path` instead of `old_path`
- `strip_prefix` panic when file path is outside the repository
- Revwalk panic on shallow or corrupt repositories
- Deprecated `chrono::Utc.timestamp()` replaced with `timestamp_opt()`
- `TurningPoint` two-phase initialization could panic on `.unwrap()`
- Production `assert!()` calls replaced with `debug_assert!` and `saturating_sub`
- Remaining `.unwrap()` calls in git history traversal replaced with `?`
- CI test suite now uses `fetch-depth: 0` for full clone
- `repo.workdir()` used instead of `repo.path().parent()` for worktree support

## [1.0.5] - 2024-12-28

### Fixed
- Diff status for `Delta::Deleted`

## [1.0.4] - 2024-06-18

### Fixed
- Beta version of clap compatibility
- Clippy errors
- Updated dependencies

## [1.0.3] - 2023-01-11

### Changed
- Sped up diff rendering

## [1.0.2] - 2022-08-18

### Fixed
- Tab visibility and added `--tab-size` option
- Clippy errors
- Updated dependencies

## [1.0.1] - 2022-08-04

### Fixed
- Vendored OpenSSL for git2

## [1.0.0] - 2022-08-04

### Changed
- Stable release

## [0.1.2] - 2022-08-04

### Added
- CLI options: `--full-hash`, `--name-of`, `--date-of`, `--beyond-last-line`, `--date-format`, `--emphasize-diff`, `--help`

## [0.1.1] - 2021-08-31

### Added
- Binary file detection and alert message

### Fixed
- Negative overflow with `saturating_sub`

## [0.1.0] - 2021-08-28

### Added
- Initial release
- Browse file history in terminal with TUI
- Commit navigation (left/right)
- Scrolling (up/down, page up/down, home/end)
- Colored diff output with line numbers
- Git reference display (branches, tags, HEAD)
- Rename tracking across commits
