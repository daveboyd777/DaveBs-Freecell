//! Test-first specification for the solver module (issue #12): a
//! depth-first solver with a transposition table that determines whether a
//! position is still winnable.

use freecell::solver::{self, Solvability};
use freecell::{Card, Game, GameState, Suit};
use std::time::Instant;

fn c(rank: u8, suit: Suit) -> Card {
    Card::new(rank, suit)
}

#[test]
fn an_already_won_position_is_solvable_with_no_moves_needed() {
    let state = GameState::from_parts(Default::default(), [None; 4], [13, 13, 13, 13]);
    assert_eq!(solver::solve(&state), Solvability::Solvable(vec![]));
}

#[test]
fn one_move_from_winning_is_solved_via_safe_autoplay() {
    // KS in a free cell, every other foundation already complete: sending
    // it home is provably safe (both opposite-color foundations are at
    // rank 13), so the solver's forced safe-autoplay pass alone should
    // find this without any real search.
    let state = GameState::from_parts(
        Default::default(),
        [Some(c(13, Suit::Spades)), None, None, None],
        [13, 13, 13, 12],
    );
    let Solvability::Solvable(moves) = solver::solve(&state) else {
        panic!("expected this position to be solvable");
    };

    let mut replayed = state;
    for (from, to) in moves {
        replayed
            .do_move(from, to)
            .expect("the solver's own moves must be legal");
    }
    assert!(replayed.is_won());
}

#[test]
fn a_completely_frozen_position_with_zero_legal_moves_is_unsolvable() {
    // Every free cell holds a King (rank 13: never a legal foundation
    // move from an empty foundation, and Kings can only land on an empty
    // cascade -- there are none here). Every cascade holds a single 5 or
    // 7: two ranks apart, so no cascade top can ever stack on another
    // (stacking needs adjacent ranks), and neither rank is an ace, so no
    // foundation move is available either. Zero legal moves from a
    // not-yet-won position is the simplest possible "unsolvable".
    let state = GameState::from_parts(
        [
            vec![c(5, Suit::Clubs)],
            vec![c(5, Suit::Diamonds)],
            vec![c(5, Suit::Hearts)],
            vec![c(5, Suit::Spades)],
            vec![c(7, Suit::Clubs)],
            vec![c(7, Suit::Diamonds)],
            vec![c(7, Suit::Hearts)],
            vec![c(7, Suit::Spades)],
        ],
        [
            Some(c(13, Suit::Clubs)),
            Some(c(13, Suit::Diamonds)),
            Some(c(13, Suit::Hearts)),
            Some(c(13, Suit::Spades)),
        ],
        [0; 4],
    );
    assert_eq!(solver::solve(&state), Solvability::Unsolvable);
}

#[test]
fn classic_deal_1_is_solvable_and_the_solution_replays_to_a_win() {
    let game = Game::deal(1);
    let Solvability::Solvable(moves) = solver::solve(game.state()) else {
        panic!("classic deal #1 is a well-known easy, solvable deal");
    };

    let mut state = game.state().clone();
    for (from, to) in moves {
        state
            .do_move(from, to)
            .expect("the solver's own moves must be legal");
    }
    assert!(state.is_won());
}

#[test]
fn classic_deal_11982_is_the_famously_unsolvable_deal() {
    let game = Game::deal(11982);
    let start = Instant::now();
    let result = solver::solve(game.state());
    eprintln!("deal #11982 solved in {:?}: {result:?}", start.elapsed());
    assert_eq!(result, Solvability::Unsolvable);
}
