# DaveB's Freecell

[![CI](https://github.com/daveboyd777/DaveBs-Freecell/actions/workflows/ci.yml/badge.svg)](https://github.com/daveboyd777/DaveBs-Freecell/actions/workflows/ci.yml)

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
cargo test            # run the engine specification
cargo fmt --check     # formatting
cargo clippy          # lints
```

Project layout:

```
src/lib.rs            game engine (no I/O)
src/main.rs           terminal interface
tests/game_tests.rs   executable specification of the engine
```

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the continuous-improvement plan: Redux-style
state management, a richer visual interface, and self-analysis / statistics
modules tracking the classic FreeCell stats.

## Maintenance automation

- **GitHub Actions CI** builds, lints, and runs the full test suite on every
  push and pull request.
- **Dependabot** watches Cargo dependencies and the CI workflow itself,
  opening automatic update pull requests weekly.
- **CodeRabbit** (AI code review, free for open-source) reviews every pull
  request — configuration in [`.coderabbit.yaml`](.coderabbit.yaml).

## License

[MIT](LICENSE) — © 2026 Dave Boyd, Softflo Technology
