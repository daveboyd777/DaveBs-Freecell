// Tests for stats-view.mjs. Run with `node --test dashboard/` (Node 18+'s
// built-in test runner and assert module -- no npm install, no build
// step, matching this dashboard's zero-dependency, static-file design).

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  SUPPORTED_SCHEMA_VERSION,
  validateExport,
  cumulativeWinRate,
  moveCountBuckets,
  formatStreak,
} from "./stats-view.mjs";

function sampleHistory() {
  return [
    { seed: 1, won: true, moves: 42 },
    { seed: 2, won: false, moves: 15 },
    { seed: 3, won: true, moves: 130 },
    { seed: 4, won: true, moves: 60 },
  ];
}

test("validateExport accepts a well-formed export", () => {
  const data = { version: SUPPORTED_SCHEMA_VERSION, history: [] };
  assert.equal(validateExport(data), data);
});

test("validateExport rejects a non-object", () => {
  assert.throws(() => validateExport(null), /not an object/);
  assert.throws(() => validateExport("nope"), /not an object/);
});

test("validateExport rejects a mismatched or missing version", () => {
  assert.throws(() => validateExport({ version: 2, history: [] }), /Unsupported stats schema version/);
  assert.throws(() => validateExport({ history: [] }), /Unsupported stats schema version/);
});

test("validateExport rejects a missing history array", () => {
  assert.throws(
    () => validateExport({ version: SUPPORTED_SCHEMA_VERSION }),
    /Missing or invalid "history"/,
  );
});

test("cumulativeWinRate is empty for empty history", () => {
  assert.deepEqual(cumulativeWinRate([]), []);
});

test("cumulativeWinRate computes running win percentage in play order", () => {
  const points = cumulativeWinRate(sampleHistory());
  assert.equal(points.length, 4);
  assert.deepEqual(
    points.map((p) => Math.round(p.winRate * 100) / 100),
    [100, 50, 66.67, 75],
  );
  // Original per-game fields pass through untouched.
  assert.equal(points[0].seed, 1);
  assert.equal(points[0].gameNumber, 1);
  assert.equal(points[1].won, false);
  assert.equal(points[2].moves, 130);
});

test("moveCountBuckets returns a single empty bucket for empty history", () => {
  const buckets = moveCountBuckets([]);
  assert.equal(buckets.length, 1);
  assert.deepEqual(buckets[0], { from: 0, to: 10, won: 0, lost: 0, games: [] });
});

test("moveCountBuckets buckets by move count with won/lost split, bucket size 10", () => {
  const buckets = moveCountBuckets(sampleHistory());
  // max moves = 130 -> bucket_count = 130/10 + 1 = 14 buckets (0..140)
  assert.equal(buckets.length, 14);
  assert.deepEqual(buckets[4], { from: 40, to: 50, won: 1, lost: 0, games: [sampleHistory()[0]] });
  assert.deepEqual(buckets[1], { from: 10, to: 20, won: 0, lost: 1, games: [sampleHistory()[1]] });
  assert.deepEqual(buckets[6], { from: 60, to: 70, won: 1, lost: 0, games: [sampleHistory()[3]] });
  assert.deepEqual(buckets[13], { from: 130, to: 140, won: 1, lost: 0, games: [sampleHistory()[2]] });
});

test("moveCountBuckets honors a custom bucket size", () => {
  const buckets = moveCountBuckets(sampleHistory(), 50);
  assert.equal(buckets.length, 3); // 130 / 50 + 1 = 3
  assert.equal(buckets[0].won + buckets[0].lost, 2); // moves 42, 15
  assert.equal(buckets[1].won + buckets[1].lost, 1); // moves 60
  assert.equal(buckets[2].won + buckets[2].lost, 1); // moves 130
});

test("formatStreak formats each StreakExport variant", () => {
  assert.equal(formatStreak({ type: "winning", length: 3 }), "Winning streak: 3");
  assert.equal(formatStreak({ type: "losing", length: 2 }), "Losing streak: 2");
  assert.equal(formatStreak({ type: "none" }), "No streak yet");
  assert.equal(formatStreak(undefined), "No streak yet");
});
