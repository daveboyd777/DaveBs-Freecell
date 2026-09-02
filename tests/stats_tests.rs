//! Test-first specification for the stats module (issue #10): the classic
//! Microsoft FreeCell statistics, computed from a history of finished
//! games. Recording games live via a `Store` subscriber and persisting to
//! disk are separate concerns (issue #11); this only covers the `Stats`
//! data model and its math.

use freecell::stats::{GameResult, Stats, Streak};

fn result(seed: u32, won: bool, moves: u32) -> GameResult {
    GameResult { seed, won, moves }
}

#[test]
fn fresh_stats_have_no_games_and_no_streak() {
    let stats = Stats::default();
    assert_eq!(stats.games_played(), 0);
    assert_eq!(stats.games_won(), 0);
    assert_eq!(stats.games_lost(), 0);
    assert_eq!(stats.win_percentage(), 0.0);
    assert_eq!(stats.current_streak(), Streak::None);
}

#[test]
fn recording_games_updates_counts_and_win_percentage() {
    let mut stats = Stats::default();
    stats.record(result(1, true, 50));
    stats.record(result(2, true, 60));
    stats.record(result(3, false, 10));

    assert_eq!(stats.games_played(), 3);
    assert_eq!(stats.games_won(), 2);
    assert_eq!(stats.games_lost(), 1);
    assert!((stats.win_percentage() - (200.0 / 3.0)).abs() < 1e-9);
}

#[test]
fn current_streak_tracks_the_most_recent_run_of_outcomes() {
    let mut stats = Stats::default();
    stats.record(result(1, true, 10));
    stats.record(result(2, true, 10));
    assert_eq!(stats.current_streak(), Streak::Winning(2));

    stats.record(result(3, false, 10));
    assert_eq!(stats.current_streak(), Streak::Losing(1));

    stats.record(result(4, true, 10));
    assert_eq!(stats.current_streak(), Streak::Winning(1));
}

#[test]
fn longest_streaks_survive_after_the_current_streak_resets() {
    let mut stats = Stats::default();
    stats.record(result(1, true, 10));
    stats.record(result(2, true, 10));
    stats.record(result(3, true, 10));
    stats.record(result(4, false, 10));
    stats.record(result(5, false, 10));
    stats.record(result(6, true, 10));

    assert_eq!(stats.longest_winning_streak(), 3);
    assert_eq!(stats.longest_losing_streak(), 2);
    // The current streak reset to a single win; the longest-streak
    // records above must not have been erased by that reset.
    assert_eq!(stats.current_streak(), Streak::Winning(1));
}

#[test]
fn deal_history_returns_only_that_seeds_attempts_in_order() {
    let mut stats = Stats::default();
    stats.record(result(617, false, 20));
    stats.record(result(1, true, 40));
    stats.record(result(617, true, 30));

    let history = stats.deal_history(617);
    assert_eq!(
        history,
        vec![&result(617, false, 20), &result(617, true, 30)]
    );
    assert!(stats.deal_history(9999).is_empty());
}

#[test]
fn stats_round_trip_through_json() {
    let mut stats = Stats::default();
    stats.record(result(1, true, 42));
    stats.record(result(2, false, 15));

    let json = serde_json::to_string(&stats).expect("Stats serializes");
    let restored: Stats = serde_json::from_str(&json).expect("Stats deserializes");
    assert_eq!(stats, restored);
}
