# DaveB's Freecell: Hint and Solver Design

This paper covers `src/solver.rs` and `src/analysis.rs`: how the game
determines whether a position is winnable, how that powers a hint
command, and how a finished game is graded against the solver's own
assessment of the original deal.

## 1. The solver

### 1.1 Algorithm

Depth-first search with a transposition table, in the spirit of
fc-solve: from a `GameState`, generate legal successor moves, recurse,
and remember every `GameState` already visited (and its outcome) so the
same position reached by a different move order is never re-explored.
FreeCell's search space is large enough that the transposition table is
not an optimization so much as a requirement -- without it, transposed
move orders (which are extremely common: moving card A then B is often
equivalent to B then A) blow up the search combinatorially.

A **safe-autoplay heuristic** reduces branching before the "real" search
even starts: any card that can go to a foundation without ever being
needed again (a strict superset of "no card of lower rank and opposite
color could still need it exposed") is played automatically. This is
standard practice for FreeCell solvers and dramatically prunes the
search tree in the common case where a run of autoplay moves is forced
and undoing any of them can never help.

### 1.2 Result type: `Solvability`

```rust
enum Solvability {
    Solvable(Vec<Action>),
    Unsolvable,
    Unknown,
}
```

The third variant is the honest one: a bounded search that exhausts its
budget without finding a win *and* without proving no win exists reports
`Unknown`, not `Unsolvable`. Every caller (hints, grading, the CLI/TUI/
GUI) is written to handle `Unknown` as its own case rather than treating
it as a `false` -- getting this wrong would silently misreport "you
can't win from here" for positions that simply weren't searched deeply
enough.

### 1.3 The stack-overflow bug, and its fix

A DFS solver is naturally recursive, and Windows' default 1MB thread
stack is not enough for the search depth some real deals require --
deal #617 and #42 were found to crash via stack overflow during
development, not via any logical bug. The fix has two parts:

1. **Run the solver on a dedicated thread with a 64MB stack**
   (`std::thread::Builder::new().stack_size(64 * 1024 * 1024)`) on
   native targets, where spawning a thread with a custom stack size is
   possible.
2. **An unconditional `MAX_SEARCH_DEPTH` cap** (10,000 native / 2,000
   wasm32) as a second line of defense, since `std::thread::spawn`
   doesn't functionally give you a bigger stack on `wasm32-unknown-unknown`
   (there is no OS thread to spawn with a custom stack size in that
   environment) -- the depth cap is what actually protects the WASM
   build, where the dedicated-thread trick is unavailable.

The lesson generalized from this bug: *any* unbounded-recursion solver
needs an explicit, tested depth bound independent of whatever stack
size happens to be available at runtime, because that size is not
something the program controls or can rely on being generous.

## 2. Hints: the same solver, a smaller budget

`analysis::hint(state) -> Option<(Loc, Loc)>` runs the solver with a
deliberately smaller `SolverConfig` than the default (20,000 states vs.
the default's 20 million), and returns just the first move of a found
solution (or `None` if the search comes back `Unknown` or `Unsolvable`).
The budget was tuned empirically against known-hard *fresh* deals
(#617 and #42) -- the worst case for `hint`, since mid-game positions
have fewer cards still in play and solve faster -- where it comes back
`Unknown` in well under a second (~0.6-0.9s in an unoptimized debug
build) rather than the several seconds a much larger budget costs on
the same deals. This balances two goals in tension: a hint that takes
several seconds to appear feels broken in an interactive UI, but too
small a budget means a hint search that could have found a winning move
instead falls back to a shrug.

This is the central design decision inherited from the roadmap for
issue #13: **hints and grading are explicit, on-demand actions**, not
automatic. None of the three UIs have (or needed to gain) async/
background-thread infrastructure; a hint or report is computed
synchronously when the player asks for it, with UI copy that sets the
expectation ("this may take a moment") rather than the program silently
freezing. Given the solver's real measured cost (single-digit seconds
for a hard deal like #11982), adding a full async/progress-spinner
layer purely to compute an optional hint the player didn't yet ask for
would have been a large scope increase in service of a UX property
(instant response) the feature doesn't actually need.

## 3. Post-game grading: `GameReport`

```rust
struct GameReport {
    moves_played: usize,
    best_line: Solvability,
    first_unsolvable_move: Option<usize>,
    foundations: [u8; 4],
}
```

* `best_line` is `solve(&history[0])` -- the *original deal*, solved
  with the full (larger) search budget, since grading is a one-shot,
  explicitly requested action where a few extra seconds is an acceptable
  cost for a materially better answer. Keeping the full `Solvability`
  (not just a move count) means a UI can also show the optimal line
  itself, not only its length.
* `foundations` is read directly off the final state -- no solving
  needed, since it's just "which foundations stalled."
* `first_unsolvable_move` is the interesting one, covered next.

### 3.1 Monotonicity makes "where did it go wrong" cheap

The key insight: **along one continuous attempt's real move history,
solvability is monotonic.** Once a position reached via legal play is
unsolvable, every later position in that same sequence is also
unsolvable -- if a later position were solvable, the earlier one would
be too, by simply making the moves that got there first. Solvability
along the sequence therefore starts `Solvable` and, if it ever flips,
stays `Unsolvable` from that point on. It never flips back.

That's exactly the shape binary search wants. Rather than solving every
position in the history (`O(moves)` solver calls, each potentially
seconds long), `grade` bisects: `O(log moves)` solver calls find the
exact index where solvability flipped. For a 60-move game, that's the
difference between ~60 solver invocations and ~6.

`first_unsolvable_move` is `None` in two distinct cases, both reported
honestly rather than conflated:

* The final position is still `Solvable` -- the player quit or the game
  is ongoing while still winnable; nothing "went wrong."
* Any solver call along the bisection returns `Unknown` -- the search
  budget wasn't enough to be sure, so the report says so instead of
  guessing an index that might be wrong.

An index of `0` is the degenerate but real case of an #11982-class deal:
never winnable from the very first position, so the "mistake" was
dealing that hand at all, not anything the player did.

## 4. What would change if extended

The clearest extension point already scoped out (roadmap) is
per-history-game grading feeding into the JSON export -- deliberately
*not* done, because computing `grade` for every historical game on every
`freecell stats --json` export would mean re-running the solver once per
past game on every export, which is a materially different (and
potentially very slow, unboundedly so as history grows) feature. If ever
wanted, it should cache each game's `GameReport` at the moment that game
finishes (when the solver call is a one-time cost paid once, like the
live hint/report commands already are) rather than recomputing it
retroactively.
