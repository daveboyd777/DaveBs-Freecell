//! Solvability analysis (issue #12): a depth-first solver with a
//! transposition table, in the spirit of fc-solve, that determines whether
//! a position is still winnable.
//!
//! This module adds no new move-legality logic: it works entirely in terms
//! of the existing [`GameState`]/[`Loc`]/[`GameState::do_move`] and only
//! decides *which* legal moves to try and *when* to stop, exactly like the
//! rest of the engine keeps move rules in one place.

use crate::{Card, GameState, Loc, Suit};
use std::collections::HashSet;

/// The result of a solvability search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Solvability {
    /// The position can be won; the moves are a complete, legal sequence
    /// (verified by the solver's own tests to actually reach a won state
    /// when replayed through real `do_move` calls) from the given
    /// position to a win.
    Solvable(Vec<(Loc, Loc)>),
    /// Every reachable position was explored and none of them win.
    Unsolvable,
    /// The search budget (`SolverConfig::max_states`) was exhausted before
    /// a definitive answer was reached. Not a claim that the position is
    /// unsolvable -- just that this search gave up early.
    Unknown,
}

/// Tuning knobs for [`solve_with_config`].
#[derive(Debug, Clone, Copy)]
pub struct SolverConfig {
    /// The maximum number of candidate moves the search will attempt
    /// before giving up and returning [`Solvability::Unknown`]. Bounds
    /// both worst-case runtime and the transposition table's memory use.
    pub max_states: u64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        // Comfortably above the state count needed to prove the famously
        // hard deal #11982 unsolvable: that search completes as a genuine
        // exhaustive proof (never hits this budget) in ~7.5s in an
        // unoptimized debug build (tests/solver_tests.rs measures and
        // prints the real figure on every run). 20 million gives ample
        // headroom for harder positions without an unbounded worst case.
        SolverConfig {
            max_states: 20_000_000,
        }
    }
}

/// Determine whether `state` can still be won, using
/// [`SolverConfig::default`].
pub fn solve(state: &GameState) -> Solvability {
    solve_with_config(state, SolverConfig::default())
}

/// Determine whether `state` can still be won.
///
/// `dfs` is recursive, with one stack frame per move currently on the
/// path, and a long unbroken chain of moves before the first backtrack can
/// run deep enough to overflow a platform's default stack (observed in
/// practice on Windows' comparatively small 1 MiB default while tuning
/// issue #13's smaller interactive search budget). On native targets this
/// runs the search on a dedicated thread with a generous stack as a cheap
/// extra safety margin; [`MAX_SEARCH_DEPTH`] bounds recursion depth
/// directly and unconditionally either way.
#[cfg(not(target_arch = "wasm32"))]
pub fn solve_with_config(state: &GameState, config: SolverConfig) -> Solvability {
    let state = state.clone();
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || run_dfs(state, config))
        .expect("failed to spawn solver thread")
        .join()
        .expect("solver thread panicked")
}

/// wasm32 equivalent of the native `solve_with_config` above. Does *not*
/// spawn a dedicated thread: `std::thread::Builder::spawn` compiles for
/// `wasm32-unknown-unknown` but doesn't actually work there (there is no
/// real OS thread to create), and a library crate has no portable way to
/// request a larger stack for itself from the wasm host at build time
/// either. [`MAX_SEARCH_DEPTH`] is correspondingly more conservative on
/// this target, since it is the *only* safety margin here.
#[cfg(target_arch = "wasm32")]
pub fn solve_with_config(state: &GameState, config: SolverConfig) -> Solvability {
    run_dfs(state.clone(), config)
}

fn run_dfs(mut state: GameState, config: SolverConfig) -> Solvability {
    let mut path = Vec::new();
    let mut visited = HashSet::new();
    let mut budget = config.max_states;
    match dfs(&mut state, &mut path, &mut visited, &mut budget) {
        DfsOutcome::Solved => Solvability::Solvable(path),
        DfsOutcome::Exhausted => Solvability::Unsolvable,
        DfsOutcome::BudgetExceeded => Solvability::Unknown,
    }
}

/// A hard cap on `dfs` recursion depth (i.e. `path.len()`), independent of
/// `SolverConfig::max_states`: a real FreeCell solution or forced dead-end
/// chain is never anywhere close to this deep (typical solutions are tens
/// to a few hundred moves), so this is purely a stack-safety backstop, not
/// a practical limitation in either case. Hitting it aborts the search
/// with [`Solvability::Unknown`] rather than a wrong answer. Smaller on
/// wasm32, where `solve_with_config` has no dedicated larger-stack thread
/// to fall back on as an additional margin.
#[cfg(not(target_arch = "wasm32"))]
const MAX_SEARCH_DEPTH: usize = 10_000;
#[cfg(target_arch = "wasm32")]
const MAX_SEARCH_DEPTH: usize = 2_000;

enum DfsOutcome {
    Solved,
    Exhausted,
    BudgetExceeded,
}

/// A transposition-table key: like [`GameState`], but with the free cells
/// sorted into a canonical order first. Two positions that differ only in
/// *which* free cell holds a given card are logically identical -- free
/// cells are interchangeable -- but `GameState`'s `[Option<Card>; 4]` is
/// ordered, so without this canonicalization the search would treat those
/// as distinct positions and re-explore work it's already done.
#[derive(PartialEq, Eq, Hash)]
struct StateKey {
    cascades: [Vec<Card>; 8],
    free_cells: [Option<Card>; 4],
    foundations: [u8; 4],
}

fn state_key(state: &GameState) -> StateKey {
    let mut free_cells = *state.freecells();
    free_cells.sort_by_key(|c| match c {
        Some(card) => (0_u8, card.suit as u8, card.rank),
        None => (1_u8, 0, 0),
    });
    StateKey {
        cascades: state.cascades().clone(),
        free_cells,
        foundations: *state.foundations(),
    }
}

/// The two foundation piles of the opposite color to `suit`.
fn opposite_color_foundations(suit: Suit, foundations: &[u8; 4]) -> (u8, u8) {
    if suit.is_red() {
        (
            foundations[Suit::Clubs as usize],
            foundations[Suit::Spades as usize],
        )
    } else {
        (
            foundations[Suit::Diamonds as usize],
            foundations[Suit::Hearts as usize],
        )
    }
}

/// The standard, provably-safe FreeCell "autoplay" rule: a card of rank
/// `R` may be sent to its foundation unconditionally once *both*
/// foundations of the opposite color are at rank `R - 1` or higher. At
/// that point, the only cards that could ever need to rest on this card
/// in a cascade -- opposite-color cards of rank `R - 1` -- are already
/// home, so it can never be missed later. This is strictly safe (it never
/// discards a winning line), unlike [`crate::Game::auto_play`], which
/// sends home *any* currently-playable card and so is not reused here.
fn is_safe_to_autoplay(card: Card, foundations: &[u8; 4]) -> bool {
    if foundations[card.suit as usize] + 1 != card.rank {
        return false; // not even legal right now
    }
    let (a, b) = opposite_color_foundations(card.suit, foundations);
    let threshold = card.rank - 1;
    a >= threshold && b >= threshold
}

/// Repeatedly send home every provably-safe card, mutating `state` in
/// place, and return the moves performed (in order) so callers can append
/// them to a recorded solution path.
fn apply_safe_autoplay(state: &mut GameState) -> Vec<(Loc, Loc)> {
    let mut moves = Vec::new();
    loop {
        let mut progressed = false;
        for i in 0..8 {
            if let Some(&card) = state.cascades()[i].last() {
                if is_safe_to_autoplay(card, state.foundations()) {
                    let from = Loc::Cascade(i);
                    state
                        .do_move(from, Loc::Foundation)
                        .expect("checked safe to autoplay");
                    moves.push((from, Loc::Foundation));
                    progressed = true;
                }
            }
        }
        for i in 0..4 {
            if let Some(card) = state.freecells()[i] {
                if is_safe_to_autoplay(card, state.foundations()) {
                    let from = Loc::Free(i);
                    state
                        .do_move(from, Loc::Foundation)
                        .expect("checked safe to autoplay");
                    moves.push((from, Loc::Foundation));
                    progressed = true;
                }
            }
        }
        if !progressed {
            return moves;
        }
    }
}

/// Every legal move worth trying from `state`, roughly best-first ordered:
/// (remaining, not-provably-safe) foundation moves first, then
/// cascade-to-cascade moves, then free-cell-to-cascade moves, then moves
/// that consume a free cell last -- using up a free cell reduces future
/// flexibility, so trying it last tends to find solutions faster and
/// exposes dead ends sooner. Legality is decided purely by attempting
/// `do_move` on a clone, the same dry-run pattern [`GameState::can_move`]
/// already uses, so this never reimplements a move rule.
fn candidate_moves(state: &GameState) -> Vec<(Loc, Loc)> {
    let mut to_foundation = Vec::new();
    let mut cascade_to_cascade = Vec::new();
    let mut free_to_cascade = Vec::new();
    let mut to_free_cell = Vec::new();

    let first_empty_free_cell = state.freecells().iter().position(Option::is_none);

    for i in 0..8 {
        if state.cascades()[i].is_empty() {
            continue;
        }
        let from = Loc::Cascade(i);
        if state.can_move(from, Loc::Foundation).is_ok() {
            to_foundation.push((from, Loc::Foundation));
        }
        for j in 0..8 {
            if i != j && state.can_move(from, Loc::Cascade(j)).is_ok() {
                cascade_to_cascade.push((from, Loc::Cascade(j)));
            }
        }
        // Trying every empty free cell for the same source card would
        // only ever produce logically identical resulting positions (the
        // transposition table's canonical free-cell ordering already
        // treats them as one), so only the first is attempted.
        if let Some(j) = first_empty_free_cell {
            if state.can_move(from, Loc::Free(j)).is_ok() {
                to_free_cell.push((from, Loc::Free(j)));
            }
        }
    }

    for i in 0..4 {
        if state.freecells()[i].is_none() {
            continue;
        }
        let from = Loc::Free(i);
        if state.can_move(from, Loc::Foundation).is_ok() {
            to_foundation.push((from, Loc::Foundation));
        }
        for j in 0..8 {
            if state.can_move(from, Loc::Cascade(j)).is_ok() {
                free_to_cascade.push((from, Loc::Cascade(j)));
            }
        }
    }

    to_foundation
        .into_iter()
        .chain(cascade_to_cascade)
        .chain(free_to_cascade)
        .chain(to_free_cell)
        .collect()
}

fn dfs(
    state: &mut GameState,
    path: &mut Vec<(Loc, Loc)>,
    visited: &mut HashSet<StateKey>,
    budget: &mut u64,
) -> DfsOutcome {
    if path.len() >= MAX_SEARCH_DEPTH {
        return DfsOutcome::BudgetExceeded;
    }

    let auto_moves = apply_safe_autoplay(state);
    let auto_count = auto_moves.len();
    path.extend(auto_moves);

    if state.is_won() {
        return DfsOutcome::Solved;
    }

    if !visited.insert(state_key(state)) {
        // Already explored this exact position (via some other move
        // order) and it led nowhere.
        path.truncate(path.len() - auto_count);
        return DfsOutcome::Exhausted;
    }

    for (from, to) in candidate_moves(state) {
        if *budget == 0 {
            path.truncate(path.len() - auto_count);
            return DfsOutcome::BudgetExceeded;
        }
        *budget -= 1;

        let snapshot = state.clone();
        state
            .do_move(from, to)
            .expect("candidate_moves only yields legal moves");
        path.push((from, to));

        match dfs(state, path, visited, budget) {
            DfsOutcome::Solved => return DfsOutcome::Solved,
            DfsOutcome::BudgetExceeded => {
                path.truncate(path.len() - auto_count);
                return DfsOutcome::BudgetExceeded;
            }
            DfsOutcome::Exhausted => {
                path.pop();
                *state = snapshot;
            }
        }
    }

    path.truncate(path.len() - auto_count);
    DfsOutcome::Exhausted
}
