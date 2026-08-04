# Roadmap — Continuous Improvement Plan

This document is the working plan for evolving DaveB's Freecell from a
terminal game into a self-analyzing, visually rich FreeCell with modern
state-management architecture. Phases are ordered so each one ships something
playable and keeps the test-first discipline: every phase starts with its
tests.

---

## Phase 1 — Redux-style state management

Refactor the engine around a single-store, action/reducer pattern. Rust's
ownership model makes this a natural fit: state transitions become pure
functions over immutable snapshots.

**Design:**

```rust
/// Every way the game state can change, as data.
enum Action {
    Deal { seed: u32 },
    Move { from: Loc, to: Loc },
    AutoPlay,
    Undo,
    Restart,
}

/// Pure reducer: no I/O, no mutation of the input.
fn reduce(state: &GameState, action: &Action) -> Result<GameState, MoveError>;

/// The store owns the state, applies actions, and notifies subscribers.
struct Store {
    state: GameState,
    past: Vec<GameState>,     // time travel: undo
    future: Vec<GameState>,   // time travel: redo (new capability)
    subscribers: Vec<Box<dyn Fn(&GameState, &Action)>>,
}
```

**Why:**

- The UI (Phase 2) and the statistics module (Phase 3) both become *pure
  subscribers* — they observe `(state, action)` pairs without touching game
  logic. The stats recorder is literally just a subscriber that counts.
- Time-travel debugging falls out for free: the existing undo history
  generalizes to undo/redo, and any bug report can be replayed as a list of
  actions from the deal seed.
- Replays and saved games become trivial to serialize: `(seed, Vec<Action>)`
  is the complete game.

**Steps:**

1. Write reducer tests mirroring the existing 16-test specification
2. Extract `GameState` (cascades/freecells/foundations) from `Game`
3. Implement `reduce` by delegating to the existing move logic
4. Add `Store` with subscribe/dispatch and undo/redo stacks
5. Port the CLI to dispatch actions instead of calling methods

## Phase 2 — Visualization

**Chosen libraries** (best-of-breed in the Rust ecosystem, evaluated against
Bevy, macroquad, and iced):

| Layer | Library | Rationale |
|-------|---------|-----------|
| Terminal UI | [ratatui](https://ratatui.rs) | The de-facto standard Rust TUI. Colored suits, card-shaped cells, mouse support, cross-platform — a huge upgrade from line-printed text with zero windowing burden. |
| Native GUI | [egui](https://github.com/emilk/egui) | Immediate-mode GUI that pairs perfectly with a Redux store (redraw = pure function of state). Ships as a single crate via `eframe`, compiles to Windows/macOS/Linux **and WebAssembly**, so the same code can publish to a browser via GitHub Pages. |
| Charts | [plotters](https://github.com/plotters-rs/plotters) | The standard Rust charting crate; renders the Phase 3 statistics (win-rate trend, move distributions) inside egui or to PNG/SVG. |

Bevy was rejected as overkill (a full game engine with heavy compile times for
a card game); macroquad lacks the widget layer needed for stats screens; iced
is excellent but egui's immediate-mode model fits the store-subscriber
architecture better.

**Steps:**

1. ratatui front-end as a store subscriber (keyboard + mouse)
2. Card rendering with color, selection highlights, and legal-move hints
3. egui/eframe desktop app sharing the same store
4. WASM build published from CI to GitHub Pages

## Phase 3 — Self-analysis and statistics

A `stats` module recording the **classic FreeCell statistics** plus modern
self-analysis, persisted as JSON in the platform data directory.

**Classic stats (as tracked by Microsoft FreeCell):**

- Games played / won / lost, win percentage
- Current winning streak, current losing streak
- Longest winning streak, longest losing streak
- Per-deal history: which numbered deals were attempted, won, lost

**Extended self-analysis:**

- Moves per win vs. the deal's known minimum; undo counts; time per game
- **Solver** (depth-first with transposition tables, in the spirit of
  fc-solve): determines whether the current position is still winnable,
  powers a hint command, and grades each finished game ("won in 87 moves;
  solver's best line was 52")
- Post-game report: where the losing move happened, which foundations
  stalled
- `freecell stats` subcommand and, in Phase 2's UIs, a charts screen
  (win-rate trend, move-count distribution) rendered with plotters

**Steps:**

1. Test-first `stats` module: streak logic, win-rate math, serialization
2. Store subscriber that records every finished game
3. Solver crate module with its own test suite (known-solvable /
   known-unsolvable deals, e.g. #11982)
4. Hint and post-game-analysis commands wired into both UIs

## Phase 4 — Release and maintenance automation

Already in place from this commit:

- **CI** (GitHub Actions): `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test` on every push/PR
- **Dependabot**: weekly Cargo and GitHub Actions update PRs, auto-labeled

Planned:

- Release workflow: tagged pushes build Windows/macOS/Linux binaries and
  attach them to GitHub Releases (gives README a real "download an exe" path)
- Code coverage reporting (cargo-llvm-cov) with a badge
- Auto-merge for green patch-level Dependabot PRs
