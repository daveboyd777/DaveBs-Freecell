//! Hints and post-game self-analysis (issue #13), built on the solver
//! (issue #12): suggest a move during play, and grade a finished (or
//! abandoned) attempt against the solver's best line, where it went wrong,
//! and which foundations stalled.
//!
//! Both are explicit, on-demand operations rather than automatic
//! background work: the solver's runtime is not bounded to "instant" (a
//! hard position can take single-digit seconds even with the state budget
//! below), and none of the three UIs have any async/background-thread
//! infrastructure. [`hint`] uses a smaller, interactive-friendly search
//! budget to stay responsive; [`grade`] is a one-shot, explicitly
//! requested action, so it uses the solver's default (larger) budget.

use crate::solver::{self, Solvability, SolverConfig};
use crate::{Game, GameState, Loc};

/// The search budget [`hint`] uses: small enough to stay responsive on a
/// typical mid-game position, at the cost of occasionally being unable to
/// find one on a genuinely hard position (returns `None` rather than
/// blocking for seconds).
///
/// Tuned empirically against known-hard *fresh* deals (the worst case for
/// `hint`, since mid-game positions have fewer cards still in play and
/// solve faster): at 20,000 states, deals #617 and #42 both come back
/// `Unknown` in well under a second (~0.6-0.9s in an unoptimized debug
/// build) rather than the several seconds a much larger budget costs on
/// the same deals. Most positions -- and virtually all realistic mid-game
/// ones -- resolve far faster than this worst case.
fn hint_config() -> SolverConfig {
    SolverConfig { max_states: 20_000 }
}

/// Suggest a next move for `state`, or `None` if the search was
/// inconclusive within the interactive budget, or the position is
/// genuinely unsolvable (in which case there is no good move to suggest).
pub fn hint(state: &GameState) -> Option<(Loc, Loc)> {
    match solver::solve_with_config(state, hint_config()) {
        Solvability::Solvable(moves) => moves.into_iter().next(),
        Solvability::Unsolvable | Solvability::Unknown => None,
    }
}

/// A post-game (or post-abandonment) self-analysis report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameReport {
    /// How many moves have been played in this attempt.
    pub moves_played: usize,
    /// The solver's assessment of the original deal, before any moves.
    /// `Solvable` carries the solver's own best line, so a report can show
    /// not just its length but the line itself.
    pub best_line: Solvability,
    /// The index into the attempt's move history (0 = the original deal)
    /// of the first position that was no longer winnable -- i.e. where a
    /// losing game actually went wrong. `None` when the final position is
    /// still solvable (the attempt was abandoned while still winnable, so
    /// there is nothing to blame), or when the search was inconclusive.
    pub first_unsolvable_move: Option<usize>,
    /// The final position's foundation ranks -- "which foundations
    /// stalled."
    pub foundations: [u8; 4],
}

/// Grade `game`'s current attempt (from its last deal/restart to now),
/// whether it has been won, lost, or is still in progress.
pub fn grade(game: &Game) -> GameReport {
    let history: Vec<&GameState> = game
        .history()
        .iter()
        .chain(std::iter::once(game.state()))
        .collect();

    GameReport {
        moves_played: history.len() - 1,
        best_line: solver::solve(history[0]),
        first_unsolvable_move: first_unsolvable_move(&history),
        foundations: *history[history.len() - 1].foundations(),
    }
}

/// Binary search `history` (a real sequence of positions reached by legal
/// play, oldest first) for the first index that is unsolvable. See
/// [`bisect_unsolvable_boundary`] for the underlying algorithm and why it's
/// correct; this just supplies it with real solver calls.
fn first_unsolvable_move(history: &[&GameState]) -> Option<usize> {
    bisect_unsolvable_boundary(history.len(), |i| solver::solve(history[i]))
}

/// The core of [`first_unsolvable_move`], factored out so it can be tested
/// directly against a synthetic sequence of [`Solvability`] values instead
/// of needing real, valid 52-card positions with a known transition point
/// (laborious to hand-construct, since [`GameState::is_won`] requires all
/// 52 cards to genuinely be reachable).
///
/// Binary searches indices `0..len` for the first one whose `probe` call
/// returns `Solvability::Unsolvable`, relying on solvability being
/// monotonic non-increasing along real play: if position N (reached by
/// actual legal moves) is unsolvable, every later position in the same
/// sequence is too -- if a later position were solvable, N would be as
/// well, by making the moves that reach it and then following its winning
/// line. So along one continuous attempt, results start `Solvable` and, if
/// they ever flip, stay `Unsolvable` from then on: a classic
/// binary-search-friendly boundary, found in `O(log len)` calls to `probe`
/// instead of checking every index.
///
/// Returns `None` if the last index is still solvable (nothing went wrong)
/// or if any call to `probe` is inconclusive (`Solvability::Unknown`) --
/// reported honestly rather than guessed.
fn bisect_unsolvable_boundary(
    len: usize,
    mut probe: impl FnMut(usize) -> Solvability,
) -> Option<usize> {
    if !matches!(probe(len - 1), Solvability::Unsolvable) {
        return None;
    }

    let (mut lo, mut hi) = (0usize, len - 1);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match probe(mid) {
            Solvability::Unsolvable => hi = mid,
            Solvability::Solvable(_) => lo = mid + 1,
            Solvability::Unknown => return None,
        }
    }
    Some(lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A boundary in the middle of the range is found exactly, using far
    /// fewer probes than a linear scan would need.
    #[test]
    fn bisect_finds_a_boundary_in_the_middle() {
        let solvable_up_to = 6; // indices 0..=5 solvable, 6..=9 unsolvable
        let mut probes = 0;
        let result = bisect_unsolvable_boundary(10, |i| {
            probes += 1;
            if i <= solvable_up_to {
                Solvability::Solvable(vec![])
            } else {
                Solvability::Unsolvable
            }
        });
        assert_eq!(result, Some(solvable_up_to + 1));
        assert!(probes < 10, "expected fewer probes than a linear scan");
    }

    #[test]
    fn bisect_returns_none_when_the_end_is_still_solvable() {
        let result = bisect_unsolvable_boundary(5, |_| Solvability::Solvable(vec![]));
        assert_eq!(result, None);
    }

    #[test]
    fn bisect_returns_zero_when_unsolvable_from_the_start() {
        let result = bisect_unsolvable_boundary(5, |_| Solvability::Unsolvable);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn bisect_gives_up_honestly_on_an_inconclusive_probe() {
        // Unsolvable at the end, but an early probe the search happens to
        // need is inconclusive -- must not guess.
        let result = bisect_unsolvable_boundary(8, |i| match i {
            7 => Solvability::Unsolvable,
            3 => Solvability::Unknown,
            _ => Solvability::Solvable(vec![]),
        });
        assert_eq!(result, None);
    }
}
