# DaveB's Freecell: Engine and Architecture Design

This paper documents the design of the game engine and the workspace
architecture around it, as implemented. It is a snapshot of *why* the
code is shaped the way it is, not just *what* it does -- see the crate
docs and `ROADMAP.md` for the incremental history that produced it.

## 1. Workspace layout

The repository is a Cargo workspace with four independently evolving
pieces:

```text
freecell (root package)   Engine (no I/O) + text CLI + stats module
tui/        freecell-tui  ratatui terminal UI
gui/        freecell-gui  egui/eframe desktop + WASM UI, plus charts
dashboard/                Static JS/D3 web stats dashboard (no Rust)
```

`tui` and `gui` are separate crates rather than features of `freecell`
because their dependency graphs don't coexist cleanly in one `Cargo.toml`:
`crossterm` (TUI) doesn't target `wasm32-unknown-unknown`, and neither UI
toolkit belongs in the pure engine's own dependency tree. `dashboard/` is
plain static HTML/CSS/JS with no build step at all, deployed as a
sibling of the WASM build's output on GitHub Pages -- see Section 5.

This separation is the load-bearing architectural decision of the whole
project: **the engine crate has zero UI dependencies**, and every UI is a
thin, symmetric consumer of the same public API. New UIs (a future
mobile port, a different terminal toolkit) are additive, not invasive.

## 2. The engine: pure functions over immutable state

### 2.1 `GameState` vs. `Game`

`GameState` is the *data*: three cascades (columns), four free cells,
four foundations, nothing else. It has no history and no notion of "the
current game" -- it's a value, comparable and cloneable, and every
move-legality query (`can_move`, `movable_run_len`, `is_won`) is a pure
function of it alone. This is what every UI's rendering code reads
directly, every frame/refresh, with no caching.

`Game` wraps `GameState` with the *process* of playing: `past`/`future`
stacks of prior/undone states (undo and redo), and the deal seed. `Game`
never contains move-legality logic itself -- it delegates to `GameState`
and only manages the history stacks around it.

### 2.2 Actions and the reducer

Every way the game state can change is one value of an `Action` enum
(`Deal`, `Move`, `AutoPlay`, `Undo`, `Redo`, `Restart`). `reduce(state,
action) -> Result<GameState, MoveError>` is a pure function: no I/O, no
mutation of its input, and (critically) no privileged knowledge of
*which* UI dispatched it. A CLI text command, a TUI keypress, and a GUI
mouse click all end up constructing the same `Action` value and calling
the same reducer.

Two reducer entry points exist for a deliberate performance reason:

* `reduce` clones the whole `Game` (including `past`/`future`) and
  returns a new one -- simple, and what tests/`replay` use.
* `reduce_in_place` mutates a `&mut Game` -- avoids paying for a clone of
  ever-growing history on every single move, which is what all three
  live UIs' `Store::dispatch` actually uses.

Both implement identical semantics; `reduce` is kept as the small,
obviously-correct reference implementation `reduce_in_place` is tested
against.

### 2.3 The `Store`: subscribers, not observers of a mutable object

`Store` owns a `Game`, dispatches `Action`s through `reduce_in_place`,
and after each *successful* dispatch, calls every subscriber with
`(&GameState, &Action)`. This is the one piece of "infrastructure" in the
engine, and it exists specifically so that cross-cutting concerns don't
need to be woven into move logic:

* The action-log recorder (used for the `(seed, actions)` replay proof)
  is a subscriber that just appends to a `Vec<Action>`.
* `StatsRecorder` (Section 3) is a subscriber that infers move counts,
  win/loss, and deal boundaries purely from the `(state, action)` stream.

Neither of these needed a single line of the reducer or `GameState`
changed to be added. A subscriber that failed to record something would
be a bug in the subscriber, never in game logic -- the failure domains
are fully separated.

Two properties worth calling out because they weren't the "obvious"
design and were arrived at by fixing real bugs during development:

* Subscribers only fire on a **successful** dispatch. A rejected move
  (illegal per the rules) produces no `(state, action)` pair -- there is
  nothing to observe, since nothing changed.
* A dispatched action that isn't `Undo`/`Redo` clears the `future` stack.
  This is implemented once, inside `Game::do_move`/`auto_play`, not
  duplicated at each of the three call sites that can trigger it.

### 2.4 Determinism and replay

A game is fully specified by `(seed, Vec<Action>)`. `replay(seed,
&actions) -> Result<Game, ActionError>` deals from the seed and applies
every action via the reducer, and every UI calls this on a win to
*prove* (not just assert) that replaying the recorded action log
reproduces the exact winning game. This same primitive underpins:

* The engine's own reducer test suite (`tests/reducer_tests.rs`).
* The web dashboard's "replay" links (Section 5) -- with a caveat: the
  *persisted* stats history only stores `(seed, won, moves)`, not the
  full action log, so a dashboard replay link re-deals the same numbered
  game rather than replaying the exact recorded moves. The
  `(seed, actions)` guarantee above is what makes that substitution
  faithful: dealing from that seed *is* that game's exact starting
  position.

## 3. Statistics: a subscriber, not a special case

`StatsRecorder` is a `Store` subscriber (Section 2.3) that reconstructs
everything it needs -- moves played, current deal's seed, whether a win
was already recorded -- purely from the `(state, action)` stream, mirrored
against how `Game` itself derives the same values internally (e.g.
`moves_played` is `past.len()`, which changes by exactly one per
`Move`/`Undo`/`Redo`).

A loss is recorded when a deal with at least one move played is
abandoned (a new `Deal`/`Restart` while unfinished) or when the process
exits mid-game (`finalize_on_exit`, called from each UI's own shutdown
path -- a quit command, Ctrl+C, or a window close). Both paths converge
on the same `record` method, so there is exactly one place a
`GameResult` gets appended to history.

`Stats` (the accumulated history) is a `Vec<GameResult>` plus pure
functions over it: `win_percentage`, `current_streak`,
`longest_winning_streak`/`longest_losing_streak`, `deal_history`. None of
these are cached or incrementally maintained -- they're recomputed from
the full history on demand, which is simple and, at realistic personal
play-history sizes, fast enough not to matter.

### 3.1 Two schemas, one deliberate seam

`Stats`'s own `Serialize`/`Deserialize` impl (used by `Stats::save`/
`load` for on-disk persistence) and `StatsExport` (the `freecell stats
--json` schema) are *different types*, not the same struct reused. The
internal format is free to change alongside the app itself, since only
this app ever reads it back. `StatsExport` is versioned
(`STATS_EXPORT_VERSION`) and covered by schema-snapshot tests, because
external consumers -- the in-app charts and the web dashboard -- depend
on it, and a silent shape change there would break them without a
compile error to catch it.

## 4. Solver-adjacent design: covered in its own paper

The solver, `hint`, and post-game `grade`/`GameReport` are a large
enough topic (search algorithm, the stack-overflow fix, the
monotonicity-based bisection trick) that they have their own paper:
`docs/papers/hint-and-solver-design.md`.

## 5. The two-track visualization split

The roadmap's Phase 3 made a deliberate architectural fork at the
statistics boundary: **Track A** (plotters charts embedded in
egui/WASM) and **Track B** (the D3.js web dashboard) are both pure
renderers of `StatsExport`, in two different languages, kept honest by
one rule: *computation stays in Rust; rendering-language code only
derives display-layer transforms (bucketing, cumulative sums) that a
human could verify by eye against the same input*. Concretely, both
`gui/src/charts.rs` (Rust) and `dashboard/stats-view.mjs` (JavaScript)
independently implement the *same* cumulative-win-rate and
move-count-bucketing logic against the same `history` array -- neither
one is a shared library the other calls, but they're required to agree,
and `dashboard/stats-view.test.mjs` exists specifically to keep the
JavaScript side pinned to the documented behavior.

This means either renderer could be deleted and rewritten from scratch
without touching a single line of Rust game logic -- the contract is the
JSON schema, not any shared code.

## 6. What this design gets right, and its costs

**Right:** the engine/UI separation, and the `Store`-subscriber pattern
specifically, meant that issues #10-#20 (stats, persistence, solver,
hints, JSON export, charts, and the web dashboard) were each additive:
none of them required touching `GameState`, `reduce`, or existing UI
code paths, only adding new subscribers or new read-only accessors.

**Cost:** `reduce` cloning the entire `Game` on every call is
`O(moves played)` per dispatch, which is why `reduce_in_place` exists as
a separate, tested-equivalent hot path. This is the one place the
"pure functions over immutable snapshots" ideal was traded for
performance, and it's contained to a single function rather than
threaded through the rest of the design.
