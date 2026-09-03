# FreeCell Solution Strategies: Design Notes

This is a design-notes survey of FreeCell solving strategies — the
recurring patterns behind both this project's solver
(`docs/papers/hint-and-solver-design.md` covers its implementation) and
how skilled human players approach the game. It's grounded in what the
solver and stats modules actually compute today; it is *not* a
data-driven statistical study (no large-scale corpus of solved deals was
generated for it) — see "Future work" for what that would take.

## 1. Why FreeCell strategy is worth categorizing at all

Unlike most solitaire variants, FreeCell is fully open information (all
52 cards are visible from the start) and the overwhelming majority of
deals are solvable — only a small, known set of classic-numbering deals,
most famously **#11982**, are proven unsolvable. That combination means
FreeCell strategy isn't really about *hidden-information risk
management* the way, say, Klondike is. It's a **search and
sequencing problem**: the cards you need are visible from move one, and
winning is almost entirely about *move order* — which cards to free up
first, when to spend a free cell versus hold it in reserve, and when to
commit to sending a card to a foundation versus keeping it in play as a
landing spot for something else.

That framing is exactly why a DFS-with-transposition-table solver (see
the hint/solver paper) is a *natural* fit for the game, and why the
strategies below are really just named shorthand for search heuristics
a human plays out mentally rather than a program plays out
exhaustively.

## 2. Core strategies

### 2.1 Safe autoplay

The most basic strategy, and the only one this project's solver treats
as a hard rule rather than a heuristic (see
`is_safe_to_autoplay` in `src/solver.rs`): a card can be sent to its
foundation the moment neither opposite-color foundation could still
need it as a landing spot for a lower card. This is *provably* safe — it
never discards a winning line — which is why it's baked into the solver
itself rather than left as something the search has to discover, and
why the engine's own `AutoPlay` action exists as a first-class move a
player can invoke at any time.

The reason this isn't "just play everything you can immediately" is the
opposite-color caveat: sending a 6 of hearts home the instant it's legal
can strand a black 5 that needed it as a resting spot. Recognizing when
autoplay is genuinely free versus when it forecloses options is the
first thing every stronger strategy below has to account for.

### 2.2 Free cells and empty columns as one shared resource

The supermove capacity formula this engine implements —
`(1 + empty free cells) × 2^(empty columns)` — isn't just an
implementation detail; it's the mathematical expression of FreeCell's
central strategic tension. Free cells and empty cascade columns are
*both* forms of the same underlying resource (temporary storage), but
empty columns are worth exponentially more than free cells because they
can hold an entire ordered run, not just one card.

The strategic implication: a strong player treats "empty a column"
as a much higher-value goal than "empty a free cell," and is often
willing to spend several free cells' worth of temporary storage
specifically to finish clearing a column, because the resulting empty
column pays that cost back many times over in supermove capacity.

### 2.3 Foundation-timing tradeoffs

Because safe autoplay (2.1) is the *only* provably-safe foundation move,
every other foundation placement is a judgment call with a real
opportunity cost: a card on a foundation is permanently unavailable as a
landing spot for a lower opposite-color card still buried in a cascade.
This shows up directly in this project's post-game analysis
(`analysis::grade`, see the hint/solver paper): the `foundations` field
of a `GameReport` shows exactly which foundations stalled and at what
rank, which is the after-the-fact evidence of exactly this tradeoff
having gone wrong — a foundation that stalled early while cascades still
held unplayable cards of the needed color is a strong signal that a
card was sent home before it was truly safe to lose as a landing spot.

### 2.4 Cascade excavation order ("dig from the bottom up")

Since cascades are strictly LIFO (only the top card of a cascade is
directly accessible), the achievable move sequences are heavily
constrained by *which* card in a cascade you need versus how deeply
it's buried. A common human heuristic is to identify the single most
constraining buried card early — often a low-rank card of a color/suit
combination several other cards depend on — and plan a sequence that
excavates toward it, rather than greedily playing whatever's on top of
each cascade right now.

This is exactly the kind of longer-horizon planning that makes FreeCell
resistant to a purely greedy algorithm and is why the solver needs real
search (with backtracking via the transposition table) rather than a
one-pass heuristic solver: a locally-good move (clearing whatever's
currently accessible) can easily be globally bad if it buries the card
that excavation actually needed next.

### 2.5 Ordered-run construction as compression

Building a long, correctly-ordered descending/alternating-color run in
a single cascade is a strategy in its own right, distinct from simply
"making progress": a long run compresses many cards into one supermove
unit, and — per 2.2's capacity formula — a large run can be relocated in
a single conceptual move once enough free cells/empty columns exist,
even though the engine still executes it as a sequence of individual
`do_move` calls under the hood (see the engine paper's note on
`GameState` being the sole source of truth the supermove capacity check
consults). Constructing these runs deliberately, rather than as an
incidental byproduct of other moves, is a recognizable mid-game
strategic phase distinct from the early "safe autoplay + immediate
excavation" phase and the late "foundation cleanup" phase.

## 3. How the solver's own behavior maps onto these strategies

The solver (`src/solver.rs`) doesn't "know" any of the strategies above
by name — it's exhaustive search, not a strategy-classifier — but its
two real optimizations are directly explained by them:

- **The transposition table exists because of 2.4/2.5**: many different
  excavation/run-building orders reach the *same* resulting position
  (moving card A before B is frequently equivalent to B before A), so
  without memoization the search would redundantly re-explore
  strategically-identical states reached via different move orders.
- **Safe autoplay (2.1) is applied unconditionally, not searched**,
  specifically because it's the one strategy from this list that's
  *provably* correct rather than merely usually-good — the solver
  spends zero search budget on the question of whether to take a safe
  autoplay move, applying it immediately instead, which is a direct,
  measurable contributor to why the default 20-million-state budget
  comfortably proves deal #11982 unsolvable in about 7.5 seconds (see
  the hint/solver paper's timing note) rather than needing an
  astronomically larger budget.

## 4. Categorizing a *finished* game with what's already computed

`StatsExport`'s `history: Vec<GameResult>` (seed, won, moves) is
deliberately minimal — no move-by-move log is persisted (see the engine
paper's note on this constraining what a "replay" can mean). That means
the *strategy analysis above is a code-and-domain-knowledge exercise*,
not something derivable from the currently-persisted stats data: you
cannot look at a `GameResult` and tell which of the strategies in
section 2 were used, only the outcome (won/lost) and a rough proxy for
game complexity (move count, in the move-count distribution chart's
buckets).

What *can* be said from existing data: `analysis::grade`'s
`first_unsolvable_move` (see the hint/solver paper) identifies exactly
which move number in a *specific, freshly-analyzed* game first made the
position unsolvable — which is a categorization of *that one game's*
outcome ("a clean loss from an already-lost deal" at move 0 vs. "a real
strategic mistake" at some later move), but this is computed live,
per-game, on demand — not a field already sitting in the persisted
history for every past game.

## 5. Future work: an actual statistical study

Turning this from design notes into a genuine data-driven study of
"which strategies win more often" would require, at minimum:

1. **Collecting a real corpus**: run the solver across a large, defined
   sample of numbered deals (e.g. the classic Microsoft 1-32000 range,
   or a random sample of that size) and record each one's `Solvability`
   and, for solvable deals, the solver's own winning line.
2. **Extracting move-sequence features** from each solved line: how many
   safe-autoplay moves fired automatically, how many moves targeted an
   empty-column creation versus other destinations, the maximum ordered
   run length ever built, and how early/late foundation moves happened
   relative to the total game length.
3. **Clustering** deals by those features into named strategy profiles
   (a deal solved almost entirely by long-run construction looks very
   different, feature-wise, from one that's mostly a careful excavation
   puzzle with short runs), and correlating cluster membership with
   solver search cost (states explored) as a proxy for "how hard was
   this deal to find a strategy for."
4. **Cross-referencing against real play**, using this project's own
   `history` data (win rate and move count per deal actually played) to
   see whether deals in a given cluster are harder for the solver,
   harder for people, or both — those aren't guaranteed to correlate,
   and finding out whether they do would be the actual novel result of
   this kind of study.

None of this is implemented; it's scoped out here specifically so a
future issue proposing it has a concrete starting point rather than
starting from nothing.
