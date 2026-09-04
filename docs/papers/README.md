# Design papers

In-depth write-ups of specific design decisions in DaveB's Freecell,
complementing `ROADMAP.md`'s incremental history with a "why it's shaped
this way" view of each area.

- [Engine and Architecture](engine-and-architecture.md) -- the workspace
  layout, the `GameState`/`Game`/`Store`/`Action` engine design, the
  statistics module as a `Store` subscriber, and the two-track (Rust +
  JavaScript) visualization split.
- [Hint and Solver Design](hint-and-solver-design.md) -- the DFS solver
  with a transposition table, the stack-overflow bug and its fix,
  why hints and post-game grading are synchronous and on-demand, and the
  monotonicity property that makes "where did it go wrong" a binary
  search instead of a linear scan.
- [FreeCell Solution Strategies](solution-strategies.md) -- a
  design-notes survey of solving strategies (safe autoplay, free
  cell/empty column tradeoffs, foundation timing, cascade excavation
  order, run construction), how the solver's own optimizations map onto
  them, and what a real data-driven study of this would require.
