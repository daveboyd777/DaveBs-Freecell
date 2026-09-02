//! Classic FreeCell statistics (issue #10): games played/won/lost, win
//! percentage, current and longest winning/losing streaks, and per-deal
//! history -- the stats tracked by classic Microsoft FreeCell.
//!
//! This module is only the data model and its math over a plain
//! `Vec<GameResult>`. Two closely related concerns are deliberately kept
//! out of it, each with its own issue: wiring a `Store` subscriber to
//! record every finished game live and persisting `Stats` to disk (#11),
//! and the versioned `freecell stats --json` CLI export (#19).

use serde::{Deserialize, Serialize};

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
}
