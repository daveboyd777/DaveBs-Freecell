//! Classic FreeCell statistics (issue #10): games played/won/lost, win
//! percentage, current and longest winning/losing streaks, and per-deal
//! history -- the stats tracked by classic Microsoft FreeCell.
//!
//! This module is only the data model and its math over a plain
//! `Vec<GameResult>`. Two closely related concerns are deliberately kept
//! out of it, each with its own issue: wiring a `Store` subscriber to
//! record every finished game live and persisting `Stats` to disk (#11),
//! and the versioned `freecell stats --json` CLI export (#19).
//!
//! [`StatsRecorder`] additionally exposes `finalize_on_exit`, which each
//! UI calls from its shutdown path (a quit command, Ctrl+C, or a window
//! close) so abandoning a game by quitting outright -- not just by
//! starting a new deal or restarting -- is recorded as a loss too.

use crate::{Action, GameState};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One finished game: which numbered deal it was, whether it was won, and
/// how many moves it took.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameResult {
    pub seed: u32,
    pub won: bool,
    pub moves: u32,
}

/// The outcome and length of the run of games still in progress at the end
/// of a `Stats`'s history. `None` only when no games have been recorded
/// yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Streak {
    Winning(u32),
    Losing(u32),
    None,
}

/// The classic Microsoft FreeCell statistics, computed from a complete
/// history of finished games. Games are recorded in play order; every
/// other stat here is a pure function of that history, so there is
/// nothing else to keep in sync.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    history: Vec<GameResult>,
}

impl Stats {
    /// Record one finished game. Games must be recorded in the order they
    /// were played -- streaks and `current_streak` are computed from
    /// history order, not from any field on `GameResult` itself.
    pub fn record(&mut self, result: GameResult) {
        self.history.push(result);
    }

    pub fn games_played(&self) -> usize {
        self.history.len()
    }

    pub fn games_won(&self) -> usize {
        self.history.iter().filter(|g| g.won).count()
    }

    pub fn games_lost(&self) -> usize {
        self.games_played() - self.games_won()
    }

    /// `0.0` on an empty history, rather than dividing by zero.
    pub fn win_percentage(&self) -> f64 {
        if self.history.is_empty() {
            0.0
        } else {
            self.games_won() as f64 / self.games_played() as f64 * 100.0
        }
    }

    /// The outcome and length of the run of identical outcomes at the tail
    /// of the history -- i.e. the streak still "in progress" right now.
    pub fn current_streak(&self) -> Streak {
        let mut games = self.history.iter().rev();
        let Some(last) = games.next() else {
            return Streak::None;
        };
        let len = 1 + games.take_while(|g| g.won == last.won).count() as u32;
        if last.won {
            Streak::Winning(len)
        } else {
            Streak::Losing(len)
        }
    }

    /// The longest run of consecutive wins anywhere in the history, not
    /// just the current one -- a `current_streak` reset does not erase
    /// this record.
    pub fn longest_winning_streak(&self) -> u32 {
        self.longest_streak(true)
    }

    /// The longest run of consecutive losses anywhere in the history.
    pub fn longest_losing_streak(&self) -> u32 {
        self.longest_streak(false)
    }

    fn longest_streak(&self, won: bool) -> u32 {
        let mut longest = 0;
        let mut current = 0;
        for g in &self.history {
            if g.won == won {
                current += 1;
                longest = longest.max(current);
            } else {
                current = 0;
            }
        }
        longest
    }

    /// Every recorded attempt of numbered deal `seed`, in play order --
    /// the classic "per-deal history" stat.
    pub fn deal_history(&self, seed: u32) -> Vec<&GameResult> {
        self.history.iter().filter(|g| g.seed == seed).collect()
    }

    /// Load `Stats` from a JSON file at `path`.
    pub fn load(path: &Path) -> io::Result<Stats> {
        let text = fs::read_to_string(path)?;
        serde_json::from_str(&text).map_err(io::Error::from)
    }

    /// `Stats::load`, falling back to an empty `Stats` on any error (a
    /// missing file on first run, a permissions issue, or corrupt JSON) --
    /// a fresh history is always a safe default, so callers such as the
    /// three UIs' startup code don't each need their own fallback logic.
    pub fn load_or_default(path: &Path) -> Stats {
        Stats::load(path).unwrap_or_default()
    }

    /// Save `Stats` as JSON to `path`, creating any missing parent
    /// directories first (the OS data directory `default_stats_path`
    /// points at may not exist yet on a fresh install).
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(io::Error::from)?;
        fs::write(path, json)
    }
}

/// The default location `Stats::save`/`load` persist to: `stats.json` in
/// this app's OS-appropriate data directory (issue #11). `None` only when
/// the OS reports no usable home directory to derive one from.
pub fn default_stats_path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "DaveBs-Freecell")?;
    Some(dirs.data_dir().join("stats.json"))
}

/// Turns a [`crate::Store`] subscriber's `(GameState, Action)` stream into
/// recorded [`GameResult`]s -- the "Store subscriber that records every
/// finished game" issue #11 (and the roadmap's Phase 3 step 2) asks for.
///
/// This deliberately does not require any change to `Store::subscribe`'s
/// `Fn(&GameState, &Action)` signature: every value it needs (moves played
/// so far, the deal in progress, whether it's already been recorded as a
/// win) is reconstructed purely from the stream of `(state, action)` pairs
/// a subscriber already receives, mirrored against how [`crate::Game`]
/// itself derives the same values (e.g. `moves_played` is `past.len()`,
/// which changes by exactly one per `Move`/`Undo`/`Redo`, and by the
/// foundation-count delta per `AutoPlay`).
///
/// A loss is recorded either when a deal with at least one move played is
/// abandoned via `Action::Deal`/`Action::Restart` (via `observe`), or when
/// the process exits mid-game (via `finalize_on_exit`, which each UI calls
/// from its own shutdown path -- see that method's docs).
pub struct StatsRecorder {
    stats: Stats,
    path: Option<PathBuf>,
    seed: u32,
    moves: u32,
    foundation_total: u32,
    recorded: bool,
}

impl StatsRecorder {
    /// `initial_seed` is the deal the `Store` was constructed with --
    /// `Store::new`/`from_game` never dispatch a `Deal` action for it, so a
    /// subscriber has no other way to learn it. `path` is where `observe`
    /// saves after recording a result; pass `None` to keep everything
    /// in-memory (e.g. in tests).
    pub fn new(initial_seed: u32, stats: Stats, path: Option<PathBuf>) -> StatsRecorder {
        StatsRecorder {
            stats,
            path,
            seed: initial_seed,
            moves: 0,
            foundation_total: 0,
            recorded: false,
        }
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Feed one successful `(state, action)` dispatch from a `Store`
    /// subscriber. Call this from the subscriber closure registered via
    /// `Store::subscribe`.
    pub fn observe(&mut self, state: &GameState, action: &Action) {
        let new_total: u32 = state.foundations().iter().map(|&r| u32::from(r)).sum();
        match action {
            // A deal boundary: finalize whatever attempt was in progress,
            // then start tracking the new one. `Deal` switches to a new
            // seed; `Restart` re-plays the same one -- both equally leave
            // the previous attempt behind.
            Action::Deal { seed } => {
                self.finalize_abandoned_attempt();
                self.seed = *seed;
                self.moves = 0;
            }
            Action::Restart => {
                self.finalize_abandoned_attempt();
                self.moves = 0;
            }
            // Exactly one `past` entry per dispatch, matching
            // `Game::moves_played`'s `past.len()` (see `Game::do_move`).
            Action::Move { .. } => self.moves += 1,
            Action::Redo => self.moves += 1,
            Action::Undo => self.moves = self.moves.saturating_sub(1),
            // AutoPlay is one dispatch but zero-or-more `do_move` calls
            // internally; the foundation-count delta is exactly how many
            // of those succeeded, since AutoPlay only ever sends cards up.
            Action::AutoPlay => self.moves += new_total.saturating_sub(self.foundation_total),
        }
        self.foundation_total = new_total;

        if state.is_won() && !self.recorded {
            self.record(true);
        }
    }

    /// Record the in-progress attempt as a loss, if it's a genuine attempt
    /// (at least one move played) that hasn't already been recorded as a
    /// win. Called when a `Deal`/`Restart` leaves it behind, so `recorded`
    /// resets afterward regardless -- a new attempt is about to begin.
    fn finalize_abandoned_attempt(&mut self) {
        if !self.recorded && self.moves > 0 {
            self.record(false);
        }
        self.recorded = false;
    }

    /// Record the in-progress attempt as a loss, if it's a genuine attempt
    /// (at least one move played) that hasn't already been recorded as a
    /// win, and persist immediately.
    ///
    /// Call this once from each UI's shutdown path -- a quit command,
    /// Ctrl+C, or a window close -- so abandoning a game by quitting
    /// outright is recorded as a loss too, the same as abandoning it via a
    /// new deal or restart already is via `observe`. Unlike
    /// `finalize_abandoned_attempt`, this does *not* reset `recorded`
    /// afterward: the process is exiting, there is no next attempt to
    /// track, and leaving `recorded` set makes a second call -- e.g. if a
    /// UI calls this from both a signal handler and a fallback after its
    /// event loop returns -- a safe no-op instead of double-recording the
    /// same result.
    pub fn finalize_on_exit(&mut self) {
        if !self.recorded && self.moves > 0 {
            self.record(false);
        }
    }

    fn record(&mut self, won: bool) {
        self.stats.record(GameResult {
            seed: self.seed,
            won,
            moves: self.moves,
        });
        self.recorded = true;
        if let Some(path) = &self.path {
            // Best-effort: a save failure (e.g. a read-only data
            // directory) shouldn't interrupt play, only silently leave
            // that result unpersisted for this session.
            let _ = self.stats.save(path);
        }
    }
}
