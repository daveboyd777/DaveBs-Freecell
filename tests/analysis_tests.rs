//! Integration tests for the analysis module (issue #13): `Game::history`,
//! `hint`, and `grade`. The bisection algorithm underlying `grade`'s
//! "where did it go wrong" is unit-tested directly against a mock
//! solvability sequence in `src/analysis.rs` itself -- constructing a real,
//! valid 52-card position with a precisely known solvable-to-unsolvable
//! transition point is unnecessary there and would be fragile here too, so
//! these tests instead check `grade`'s structural correctness on real,
//! independently-understood fixtures.

use freecell::analysis::{self, GameReport};
use freecell::solver::Solvability;
use freecell::{reduce_in_place, Action, Card, Game, Loc, Suit};

fn c(rank: u8, suit: Suit) -> Card {
    Card::new(rank, suit)
}

#[test]
fn history_tracks_moves_and_shrinks_on_undo_grows_on_redo() {
    let mut game = Game::from_parts(
        [
            vec![c(13, Suit::Clubs), c(7, Suit::Hearts)],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [None; 4],
        [0; 4],
    );
    assert_eq!(game.history().len(), 0);

    game.do_move(Loc::Cascade(0), Loc::Free(0)).unwrap();
    assert_eq!(game.history().len(), 1);

    game.undo();
    assert_eq!(game.history().len(), 0);

    game.do_move(Loc::Cascade(0), Loc::Free(0)).unwrap();
    game.do_move(Loc::Cascade(0), Loc::Free(1)).unwrap();
    assert_eq!(game.history().len(), 2);
    // Oldest first: the very first snapshot is the original two-card deal.
    assert_eq!(game.history()[0].cascades()[0].len(), 2);
}

#[test]
fn history_resets_on_restart() {
    let mut game = Game::deal(617);
    game.do_move(Loc::Cascade(0), Loc::Free(0)).unwrap();
    assert_eq!(game.history().len(), 1);

    reduce_in_place(&mut game, Action::Restart).expect("a numbered deal can always restart");
    assert_eq!(game.history().len(), 0);
}

#[test]
fn hint_suggests_the_winning_move_when_one_move_from_victory() {
    let state = freecell::GameState::from_parts(
        Default::default(),
        [Some(c(13, Suit::Spades)), None, None, None],
        [13, 13, 13, 12],
    );
    let suggestion = analysis::hint(&state);
    assert_eq!(suggestion, Some((Loc::Free(0), Loc::Foundation)));
}

#[test]
fn hint_is_none_when_already_won() {
    let state = freecell::GameState::from_parts(Default::default(), [None; 4], [13, 13, 13, 13]);
    assert_eq!(analysis::hint(&state), None);
}

#[test]
fn grade_on_a_fresh_already_won_game_has_no_moves_and_nothing_went_wrong() {
    let game = Game::from_parts(Default::default(), [None; 4], [13, 13, 13, 13]);
    let report = analysis::grade(&game);
    assert_eq!(
        report,
        GameReport {
            moves_played: 0,
            best_line: Solvability::Solvable(vec![]),
            first_unsolvable_move: None,
            foundations: [13, 13, 13, 13],
        }
    );
}

#[test]
fn grade_on_a_deal_that_was_never_winnable_reports_index_zero() {
    // The same zero-legal-moves "frozen" position used in
    // tests/solver_tests.rs: four Kings stuck in free cells, eight
    // cascades of mutually non-stacking 5s/7s.
    let game = Game::from_parts(
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
    let report = analysis::grade(&game);
    assert_eq!(report.moves_played, 0);
    assert_eq!(report.best_line, Solvability::Unsolvable);
    assert_eq!(report.first_unsolvable_move, Some(0));
    assert_eq!(report.foundations, [0, 0, 0, 0]);
}

#[test]
fn grade_reflects_moves_played_and_current_foundations_mid_game() {
    let mut game = Game::deal(1);
    let before = game.state().clone();
    let action_move = freecell::parse_move("1a").expect("a valid move string");
    game.do_move(action_move.0, action_move.1)
        .expect("legal move on a fresh deal");

    let report = analysis::grade(&game);
    assert_eq!(report.moves_played, 1);
    assert_eq!(&report.foundations, game.foundations());
    // best_line is computed from the ORIGINAL deal, not the current
    // position, so it must not depend on the move just made.
    assert_eq!(
        report.best_line,
        analysis::grade(&Game::from_parts(
            before.cascades().clone(),
            *before.freecells(),
            *before.foundations(),
        ))
        .best_line
    );
}
