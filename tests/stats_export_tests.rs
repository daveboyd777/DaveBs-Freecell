//! Schema snapshot tests for the versioned `freecell stats --json` export
//! (issue #19). `StatsExport` is a stable, external contract per
//! ROADMAP.md's two-track visualization architecture -- these tests pin
//! down its exact JSON shape (field names, types, and the `version`
//! number) so any accidental drift (a rename, a type change, a dropped
//! field) fails loudly here instead of silently breaking whatever
//! consumes it (in-app charts, issue #14; the web dashboard, issue #20).
//!
//! Comparisons go through `serde_json::Value` (via the `json!` macro)
//! rather than raw string equality, so these tests don't depend on key
//! ordering or whitespace -- only on the actual shape.

use freecell::stats::{GameResult, Stats, StatsExport, STATS_EXPORT_VERSION};
use serde_json::json;

#[test]
fn export_schema_version_is_1() {
    // A change to this constant is a breaking change to the schema and
    // must come with a new snapshot test alongside this one, per
    // ROADMAP.md's "a change to it is a breaking change reviewed like
    // engine code."
    assert_eq!(STATS_EXPORT_VERSION, 1);
}

#[test]
fn empty_stats_export_matches_the_expected_schema_exactly() {
    let export = StatsExport::from_stats(&Stats::default());
    let value = serde_json::to_value(&export).expect("StatsExport serializes");

    assert_eq!(
        value,
        json!({
            "version": 1,
            "games_played": 0,
            "games_won": 0,
            "games_lost": 0,
            "win_percentage": 0.0,
            "current_streak": { "type": "none" },
            "longest_winning_streak": 0,
            "longest_losing_streak": 0,
            "history": []
        })
    );
}

#[test]
fn populated_stats_export_matches_the_expected_schema_exactly() {
    let mut stats = Stats::default();
    stats.record(GameResult {
        seed: 1,
        won: true,
        moves: 42,
    });
    stats.record(GameResult {
        seed: 617,
        won: false,
        moves: 10,
    });
    stats.record(GameResult {
        seed: 1,
        won: true,
        moves: 30,
    });

    let export = StatsExport::from_stats(&stats);
    let value = serde_json::to_value(&export).expect("StatsExport serializes");

    assert_eq!(
        value,
        json!({
            "version": 1,
            "games_played": 3,
            "games_won": 2,
            "games_lost": 1,
            // Matches Stats::win_percentage's exact expression
            // (games_won as f64 / games_played as f64 * 100.0) --
            // floating-point operations aren't associative, so computing
            // this any other way (e.g. 200.0 / 3.0) can differ in the
            // last bit.
            "win_percentage": 2.0 / 3.0 * 100.0,
            "current_streak": { "type": "winning", "length": 1 },
            "longest_winning_streak": 1,
            "longest_losing_streak": 1,
            "history": [
                { "seed": 1, "won": true, "moves": 42 },
                { "seed": 617, "won": false, "moves": 10 },
                { "seed": 1, "won": true, "moves": 30 }
            ]
        })
    );
}

#[test]
fn a_losing_streak_is_tagged_losing_in_the_export() {
    let mut stats = Stats::default();
    stats.record(GameResult {
        seed: 1,
        won: false,
        moves: 5,
    });
    stats.record(GameResult {
        seed: 2,
        won: false,
        moves: 5,
    });

    let export = StatsExport::from_stats(&stats);
    let value = serde_json::to_value(&export).expect("StatsExport serializes");
    assert_eq!(
        value["current_streak"],
        json!({ "type": "losing", "length": 2 })
    );
}

#[test]
fn export_round_trips_through_json() {
    let mut stats = Stats::default();
    stats.record(GameResult {
        seed: 42,
        won: true,
        moves: 7,
    });
    let export = StatsExport::from_stats(&stats);

    let json = export.to_json().expect("serializes");
    let restored: StatsExport = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(export, restored);
}
