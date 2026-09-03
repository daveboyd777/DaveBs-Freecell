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

1. Write reducer tests mirroring the existing 16-test specification — done (#2)
2. Extract `GameState` (cascades/freecells/foundations) from `Game` — done (#3)
3. Implement `reduce` by delegating to the existing move logic — done (#2)
4. Add `Store` with subscribe/dispatch and undo/redo stacks — done (#3, #4, #24),
   with three deliberate deviations from the sketch above:
   - **`Store` wraps `Game` and calls `reduce`/`reduce_in_place`**, rather than
     reimplementing dispatch logic against a bare `GameState`. `Game`'s own
     `past`/`future: Vec<GameState>` (generalized from its original single
     `history` stack in #4) already serve as the stacks the sketch describes
     as separate `Store` fields, so there is no second, Store-level undo/redo
     mechanism that could drift out of sync. `Store::dispatch` forwards any
     `Action` — including the new `Action::Redo` — generically, so redo
     required zero Store-specific code once `Game` grew a `future` stack.
     Subscribers fire only on a *successful* dispatch (a rejected action
     produced no transition to observe), and there is no unsubscribe API
     for v1.
   - **`Store::dispatch` uses `reduce_in_place`, not `reduce`.** `reduce`
     clones the entire `Game` (including its `past`/`future`) per call, so
     per-dispatch cost grew with moves already played (#24, fixed by adding
     the efficient in-place sibling; `reduce` itself is unchanged and still
     used by tests/replay call sites that want an immutable transform).
   - **A dispatched non-undo/redo action clears `future`** via `Game::do_move`
     (and therefore `Game::auto_play`, which calls it in a loop) rather than
     in `Store` — this is the literal issue #4 requirement, implemented once
     at the source of every position-changing action instead of duplicated
     across call sites.
5. Port the CLI to dispatch actions instead of calling methods — done (#5).
   `main.rs` now builds a `Store` and dispatches `Action` for every command
   (moves, undo, redo, autoplay, restart, new deal), gaining a `y`/redo
   command for free. A new `replay(seed, actions) -> Result<Game, ActionError>`
   generalizes the `(seed, Vec<Action>)` replay pattern already proven in
   `tests/reducer_tests.rs`; the CLI calls it live on every win, printing the
   action log and verifying the replay reproduces the win — not just an
   assertion in tests, a runtime proof every time a game finishes.

**Phase 1 is now complete.**

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

0. Workspace scaffolding — done. The repo is now a Cargo workspace: `freecell`
   (root package: engine + text CLI, unchanged) plus two new member crates,
   `tui/` (`freecell-tui`: ratatui + crossterm) and `gui/` (`freecell-gui`:
   egui + eframe). Kept separate from `freecell` because their dependency
   graphs don't coexist in one crate — `crossterm` doesn't target
   `wasm32-unknown-unknown` (needed by step 4), and neither UI toolkit
   belongs in the pure engine's own dependency tree. Each currently holds
   only a minimal placeholder `main.rs` (constructs a real `Store`, renders a
   placeholder screen) proving the wiring; CI's `check` job now runs
   `--workspace` so both stay green as they grow. Actual card rendering and
   input land in steps 1–4.
1. ratatui front-end as a store subscriber (keyboard + mouse) — done (#6, #7)
2. Card rendering with color, selection highlights, and legal-move hints —
   done (#7), later joined by real vector suit pips (issue #8's follow-up)
3. egui/eframe desktop app sharing the same store — done (#8)
4. WASM build published from CI to GitHub Pages — done (#9)

**Phase 2 is now complete.** (A locally-buildable Android debug build was
later added on top of the same `gui` crate -- see "Beyond the roadmap"
below, since it wasn't part of this phase's original plan.)

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

**Two-track visualization — the deliberate Rust ⇄ JavaScript fork:**

The statistics module is a *data layer*, not a renderer. It exposes its
results through a stable, versioned JSON schema (`freecell stats --json`),
and two independent presentation tracks consume that schema:

| Track | Stack | What it's for |
|-------|-------|---------------|
| **A — In-app (pure Rust)** | plotters inside egui | Charts embedded in the desktop/WASM game itself. One language, ships with the app. |
| **B — Web dashboard (JavaScript)** | D3.js / Observable Plot on GitHub Pages | Interactive analytics over game history: hover a win-rate point to see the deal, click through to a replay (possible because a game is fully described by `(seed, action log)`). Maximum expressiveness, browser-native. |

Separation-of-concerns rules that keep the fork safe:

1. **All computation stays in Rust.** Game logic, streaks, win rates, and
   solver results are computed once, in the tested `stats` module.
   JavaScript renders; it never calculates.
2. **The JSON schema is the contract.** It carries a version field, is
   specified by Rust tests (schema snapshot tests), and a change to it is
   a breaking change reviewed like engine code.
3. **Renderers are disposable; the data layer is not.** Either track can
   be rewritten or dropped without touching Rust internals — and future
   renderers (a different JS library, a native mobile view) just consume
   the same JSON.

**Steps:**

1. Test-first `stats` module: streak logic, win-rate math, serialization
2. Store subscriber that records every finished game
3. Solver crate module with its own test suite (known-solvable /
   known-unsolvable deals, e.g. #11982)
4. Hint and post-game-analysis commands wired into both UIs
5. Versioned JSON export (`freecell stats --json`) with schema tests —
   the hinge point the JavaScript track hangs on
6. D3.js / Observable Plot dashboard on GitHub Pages consuming the JSON,
   including deal-level drill-down and replay links -- done (#20). Lives
   in `dashboard/` (plain static HTML/CSS/JS, D3 via CDN, no build step)
   and deploys alongside the WASM game at `/dashboard/`. Since
   `GameResult` only records a deal's seed and outcome, not its full
   action log, "replay" here means opening the WASM game on that same
   numbered deal (`?seed=`, parsed by `gui/src/main.rs`) rather than
   replaying the exact recorded moves -- still fully in the spirit of
   "a game is fully described by `(seed, action log)`", since a fresh
   deal from that seed *is* that game's starting position.

**Phase 3 is now complete.**

## Phase 4 — Release and maintenance automation

Already in place from this commit:

- **CI** (GitHub Actions): `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test` on every push/PR
- **Dependabot**: weekly Cargo and GitHub Actions update PRs, auto-labeled
- **Release workflow** (#15): pushing a `v*` tag builds `freecell`,
  `freecell-tui`, and `freecell-gui` release binaries on Windows, macOS,
  and Linux, packages each platform into an archive with the README and
  LICENSE, and publishes them all to a GitHub Release --
  `.github/workflows/release.yml`, giving the README a real download link
- **Code coverage** (#16): `cargo-llvm-cov` runs alongside the rest of CI
  and uploads an lcov report to Codecov, which renders the README badge
  -- `fail_ci_if_error: false`, so this never blocks CI even before the
  repo is activated on codecov.io
- **Dependabot auto-merge** (#17): `.github/workflows/dependabot-auto-merge.yml`
  enables GitHub's native auto-merge on a Dependabot PR once it's a
  patch-level bump (`dependabot/fetch-metadata`'s `update-type`); GitHub
  still waits for CI to go green before actually merging it. Minor/major
  bumps are left for manual review.

**Phase 4 is now complete.**

## Beyond the roadmap

Ad hoc additions made outside the original phased plan above, not tied to
a specific phase:

- **Android (local debug build only)**: `gui/` doubles as a `cdylib` with
  its own `android_main` entry point, built into a sideload-only debug
  APK via `cargo apk build --lib` -- not published to any store, not
  wired into CI or the release workflow. Uses `android-native-activity`
  (matching `cargo-apk`'s own manifest default) rather than
  `android-game-activity`, since the latter needs Java glue `cargo-apk`
  doesn't automate. The statistics charts window is unavailable on this
  target -- `plotters`' font rendering has no Android backend. See the
  README's "Android (local debug build only)" section for build steps.
- **Design papers** (`docs/papers/`): in-depth write-ups of *why*,
  complementing this document's *what/when*: engine and workspace
  architecture, hint/solver design, and a design-notes survey of
  FreeCell solution strategies.
- **Podcast script** (`docs/podcast-script.md`): a conversational
  transcript about the project, meant to be fed into a text-to-speech or
  AI podcast-generation tool of the reader's choice.
