// Pure, DOM-free data transforms for the stats dashboard (issue #20).
//
// Kept separate from dashboard.js (which owns all D3/DOM wiring) so this
// module can be unit-tested with plain `node --test dashboard/`, no
// browser or build step required. Per the roadmap's two-track rule
// ("JavaScript renders; it never calculates"), everything here is either
// a straight pass-through of a value `freecell stats --json` already
// computed, or a display-only transform of `history` (bucketing,
// cumulative counts) -- the exact same kind of derivation
// `gui/src/charts.rs` does for the in-app charts, not a new statistic.

/// The only `StatsExport` schema version this dashboard understands.
/// Bump alongside `STATS_EXPORT_VERSION` in `src/stats.rs` -- and this
/// module's tests -- any time the Rust schema changes.
export const SUPPORTED_SCHEMA_VERSION = 1;

/// Mirrors `MOVES_PER_BUCKET` in `gui/src/charts.rs`, so the web
/// dashboard's move-count histogram buckets moves identically to the
/// in-app chart (Track A and Track B stay visually consistent).
export const MOVES_PER_BUCKET = 10;

/// Validates that `data` looks like a `StatsExport` this dashboard can
/// render, throwing a descriptive `Error` if not. Returns `data`
/// unchanged on success, so call sites can use it inline:
/// `render(validateExport(JSON.parse(text)))`.
export function validateExport(data) {
  if (data === null || typeof data !== "object") {
    throw new Error("That doesn't look like a stats JSON file (not an object).");
  }
  if (data.version !== SUPPORTED_SCHEMA_VERSION) {
    throw new Error(
      `Unsupported stats schema version ${JSON.stringify(data.version)} ` +
        `(this dashboard understands version ${SUPPORTED_SCHEMA_VERSION}). ` +
        "Export again with a matching build of freecell.",
    );
  }
  if (!Array.isArray(data.history)) {
    throw new Error('Missing or invalid "history" array.');
  }
  return data;
}

/// One point per game, in play order: the cumulative win percentage
/// after that game, plus the original `GameResult` fields, ready for the
/// win-rate trend chart and its hover/click drill-down. Mirrors
/// `draw_win_rate_trend`'s per-point calculation in `gui/src/charts.rs`.
export function cumulativeWinRate(history) {
  let wins = 0;
  return history.map((game, index) => {
    if (game.won) wins += 1;
    return {
      gameNumber: index + 1,
      seed: game.seed,
      won: game.won,
      moves: game.moves,
      winRate: (wins / (index + 1)) * 100,
    };
  });
}

/// Buckets `history` by move count into fixed-width `bucketSize` ranges
/// (default `MOVES_PER_BUCKET`, matching the in-app chart), each with its
/// won/lost counts and the list of games that landed in it -- the latter
/// is what powers the dashboard's click-to-drill-down (the in-app chart
/// has no equivalent, since it can't open a details panel). Always
/// returns at least one bucket, even for an empty `history`.
export function moveCountBuckets(history, bucketSize = MOVES_PER_BUCKET) {
  const maxMoves = history.reduce((max, game) => Math.max(max, game.moves), 0);
  const bucketCount = Math.max(1, Math.floor(maxMoves / bucketSize) + 1);
  const buckets = Array.from({ length: bucketCount }, (_, i) => ({
    from: i * bucketSize,
    to: (i + 1) * bucketSize,
    won: 0,
    lost: 0,
    games: [],
  }));
  for (const game of history) {
    const bucket = buckets[Math.floor(game.moves / bucketSize)];
    if (game.won) {
      bucket.won += 1;
    } else {
      bucket.lost += 1;
    }
    bucket.games.push(game);
  }
  return buckets;
}

/// One human-readable line for `StatsExport.current_streak`'s tagged-enum
/// shape (`{"type": "winning", "length": 3}` etc.) -- pure formatting of
/// a value Rust already computed, not a calculation of its own.
export function formatStreak(streak) {
  if (!streak || typeof streak !== "object") return "No streak yet";
  switch (streak.type) {
    case "winning":
      return `Winning streak: ${streak.length}`;
    case "losing":
      return `Losing streak: ${streak.length}`;
    default:
      return "No streak yet";
  }
}
