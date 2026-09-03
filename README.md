# DaveB's Freecell

[![CI](https://github.com/daveboyd777/DaveBs-Freecell/actions/workflows/ci.yml/badge.svg)](https://github.com/daveboyd777/DaveBs-Freecell/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/daveboyd777/DaveBs-Freecell/graph/badge.svg)](https://codecov.io/gh/daveboyd777/DaveBs-Freecell)
[![Latest Release](https://img.shields.io/github/v/release/daveboyd777/DaveBs-Freecell)](https://github.com/daveboyd777/DaveBs-Freecell/releases/latest)

A faithful FreeCell card game for the terminal, written in Rust and built
test-first. Deals are bit-compatible with the classic Microsoft FreeCell
numbering, so game **#11982** here is the same famously-unwinnable game #11982
you remember.

```
Game #1   moves: 0
free  a:__ b:__ c:__ d:__    home  C- D- H- S-

   1   2   3   4   5   6   7   8
   JD  2D  9H  JC  5D  7H  7C  5H
   KD  KC  9S  5S  AD  QC  KH  3H
   2S  KS  9D  QD  JS  AS  AH  3C
   4C  5C  TS  QH  4H  AC  4D  7S
   3S  TD  4S  TH  8H  2C  JH  7D
   6D  8S  8D  QS  6C  3D  8C  TC
   6S  9C  2H  6H
>
```

## Features

- Classic Microsoft deal numbering (`freecell 617` deals game #617)
- Full rules engine: free cells, foundations, alternating-color cascades
- Multi-card **supermoves** with the standard capacity formula
  `(1 + empty free cells) x 2^(empty columns)`
- Unlimited undo, restart, auto-play to foundations
- 16-test engine specification written before the implementation (TDD)

## Download and install

### Prebuilt binaries

Grab the latest Windows/macOS/Linux archive (CLI, TUI, and native GUI) from
the [Releases page](https://github.com/daveboyd777/DaveBs-Freecell/releases/latest)
-- no Rust toolchain required. Or play the GUI directly in a browser, no
download at all: [daveboyd777.github.io/DaveBs-Freecell](https://daveboyd777.github.io/DaveBs-Freecell/).

### Prerequisite: Rust

Install Rust from <https://rustup.rs> (any recent stable toolchain).

### From source

```sh
git clone https://github.com/daveboyd777/DaveBs-Freecell.git
cd DaveBs-Freecell
cargo build --release
```

The executable lands at `target/release/freecell` (Windows:
`target\release\freecell.exe`). Or build and play in one step:

```sh
cargo run --release            # random deal
cargo run --release -- 11982   # a specific deal number
```

### Without git

Use GitHub's **Code → Download ZIP** button, unzip, then run
`cargo build --release` in the unzipped folder.

## How to play

Moves are two characters: **source** then **destination**.

| Key   | Meaning                    |
|-------|----------------------------|
| `1`-`8` | Cascade columns          |
| `a`-`d` | Free cells               |
| `h`   | Foundations ("home")       |

Examples: `35` moves the run from column 3 onto column 5 · `1a` parks
column 1's top card in free cell a · `2h` sends column 2's top card to its
foundation · `b4` returns free cell b's card to column 4.

Ordered runs (descending, alternating color) move together automatically
whenever enough free cells and empty columns are available.

| Command | Effect                                          |
|---------|-------------------------------------------------|
| `a`     | Auto-play every card that can go to a foundation |
| `u`     | Undo the last move                               |
| `r`     | Restart the current deal                         |
| `n 617` | New game (deal number optional)                  |
| `?`     | Help                                             |
| `q`     | Quit                                             |

## Development

The engine (`src/lib.rs`) is fully specified by `tests/game_tests.rs` —
dealing, move legality, supermove capacity, win detection, and undo semantics
were all written as failing tests first.

```sh
cargo test            # run the engine specification (freecell package only)
cargo fmt --all --check     # formatting, whole workspace
cargo clippy --workspace --all-targets -- -D warnings   # lints, whole workspace
cargo test --workspace      # tests, whole workspace (incl. tui/gui once they have any)
node --test dashboard/*.test.mjs   # web dashboard's pure data-transform tests
cargo llvm-cov --workspace --open   # coverage report (needs cargo-llvm-cov)
```

Project layout (a Cargo workspace):

```text
Cargo.toml            workspace root + the freecell package (engine + text CLI)
src/lib.rs            game engine (no I/O)
src/store.rs          Store: dispatches Action, notifies subscribers
src/main.rs           text CLI (dispatches Action through a Store)
tests/                engine, reducer, and Store test suites
tui/                  ratatui terminal UI (freecell-tui) -- WIP, Phase 2
gui/                  egui/eframe desktop + WASM UI (freecell-gui) -- WIP, Phase 2
dashboard/            static JS/D3 web stats dashboard, deployed alongside
                      the WASM build to GitHub Pages at /dashboard/
```

`tui/` and `gui/` are separate workspace members (not features of `freecell`)
because their dependencies don't coexist cleanly in one crate -- see the
comment at the top of the root `Cargo.toml`.

### Android (local debug build only)

`gui/` doubles as a `cdylib` with its own `android_main` entry point
(`gui/src/lib.rs`), so the exact same app can also be packaged as a
sideload-only Android debug APK -- not published to any app store, and not
built by CI. To reproduce locally:

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk
# Requires a JDK, and an Android SDK with platform-tools, platforms;android-34,
# build-tools;34.0.0, and ndk;27.0.12077973 (or compatible versions) installed,
# with ANDROID_HOME/ANDROID_NDK_HOME pointing at them.
cd gui
cargo apk build --lib   # --lib: this crate also has a native/wasm binary target,
                        # which cargo-apk otherwise gets confused trying to package too
```

The signed APK lands at `target/debug/apk/freecell_gui.apk`; install it with
`adb install` or `cargo apk run --lib` on a connected device/emulator. The
statistics charts window isn't available on this target -- `plotters`'
font rendering has no Android backend -- everything else (the board, moves,
hints, undo/redo) works the same as the desktop build, except stats don't
persist between runs (no OS data directory in this sandboxed environment,
the same graceful fallback the WASM build already uses).

## Web stats dashboard

The classic FreeCell stats (win rate, streaks, per-deal history) are also
browsable as interactive charts at
[daveboyd777.github.io/DaveBs-Freecell/dashboard/](https://daveboyd777.github.io/DaveBs-Freecell/dashboard/)
-- a static, JavaScript-only page (D3.js) that never computes anything
itself; it only renders a `freecell stats --json` export you load (drag a
file in, or try the built-in sample data). Clicking a game opens the WASM
game pre-loaded on that deal for a rematch. See `dashboard/` and
ROADMAP.md's "two-track visualization" section for the Rust/JavaScript
split this is built on.

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the continuous-improvement plan: Redux-style
state management, a richer visual interface, and self-analysis / statistics
modules tracking the classic FreeCell stats.

## Design papers

For in-depth write-ups of *why* specific parts are designed the way they
are (not just what they do), see [docs/papers/](docs/papers/README.md):
engine/architecture, and hint/solver design.

## Maintenance automation

- **GitHub Actions CI** builds, lints, and runs the full test suite on every
  push and pull request.
- **Code coverage** (cargo-llvm-cov, badge above) uploads to
  [Codecov](https://codecov.io/gh/daveboyd777/DaveBs-Freecell) on every push
  and pull request.
- **Release workflow** builds and publishes Windows/macOS/Linux binaries to
  GitHub Releases on every `v*` tag.
- **Dependabot** watches Cargo dependencies and the CI workflow itself,
  opening automatic update pull requests weekly; green patch-level updates
  auto-merge on their own once CI passes.
- **CodeRabbit** (AI code review, free for open-source) reviews every pull
  request — configuration in [`.coderabbit.yaml`](.coderabbit.yaml).

## License

[MIT](LICENSE) — © 2026 Dave Boyd, Softflo Technology
