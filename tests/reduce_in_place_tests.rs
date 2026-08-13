//! Test-first specification for `reduce_in_place` (issue #24).
//!
//! `reduce_in_place(&mut Game, Action)` must behave identically to
//! `reduce(&Game, Action) -> Game` for every action, success and failure
//! alike -- it exists purely to avoid `reduce`'s full-`Game` (and thus
//! full-`history`) clone per call, not to change semantics.

use freecell::{reduce, reduce_in_place, Action, ActionError, Card, Game, Loc, MoveError, Suit};

fn c(s: &str) -> Card {
    let bytes = s.as_bytes();
    let rank = match bytes[0] {
        b'A' => 1,
        b'T' => 10,
        b'J' => 11,
        b'Q' => 12,
        b'K' => 13,
        d @ b'2'..=b'9' => d - b'0',
        _ => panic!("bad rank in {s}"),
    };
    let suit = match bytes[1] {
        b'C' => Suit::Clubs,
        b'D' => Suit::Diamonds,
        b'H' => Suit::Hearts,
        b'S' => Suit::Spades,
        _ => panic!("bad suit in {s}"),
    };
    Card::new(rank, suit)
}

fn cascade(cards: &[&str]) -> Vec<Card> {
    cards.iter().map(|s| c(s)).collect()
}

#[test]
fn deal_action_produces_the_numbered_game_in_place() {
    let mut game = Game::from_parts(Default::default(), [None; 4], [0; 4]);
    reduce_in_place(&mut game, Action::Deal { seed: 1 }).expect("deal always succeeds");
    assert_eq!(game.cascades(), Game::deal(1).cascades());
    assert_eq!(game.seed(), Some(1));
}

#[test]
fn move_action_mutates_the_game_in_place() {
    let mut game = Game::from_parts(
        [
            cascade(&["KC", "7H"]),
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
    reduce_in_place(
        &mut game,
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
    )
    .expect("legal move");

    assert_eq!(game.freecells()[0], Some(c("7H")));
    assert_eq!(game.cascades()[0], cascade(&["KC"]));
    assert_eq!(game.moves_played(), 1);
}

#[test]
fn illegal_move_action_reports_the_error_and_leaves_game_untouched() {
    let mut game = Game::from_parts(
        [
            cascade(&["KC"]),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [Some(c("2C")), None, None, None],
        [0; 4],
    );
    let before = game.clone();
    let err = reduce_in_place(
        &mut game,
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
    )
    .unwrap_err();

    assert_eq!(err, ActionError::Move(MoveError::OccupiedFreeCell));
    assert_eq!(game.cascades(), before.cascades());
    assert_eq!(game.freecells(), before.freecells());
    assert_eq!(game.moves_played(), before.moves_played());
}

#[test]
fn autoplay_action_sends_every_playable_card_home_in_place() {
    let mut game = Game::from_parts(
        [
            cascade(&["KD", "AC"]),
            cascade(&["2H"]),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [Some(c("AH")), None, None, None],
        [0; 4],
    );
    reduce_in_place(&mut game, Action::AutoPlay).expect("autoplay always succeeds");
    assert_eq!(game.foundations()[Suit::Clubs as usize], 1);
    assert_eq!(game.foundations()[Suit::Hearts as usize], 2);
    assert_eq!(game.freecells()[0], None);
    assert_eq!(game.cascades()[0], cascade(&["KD"]));
}

#[test]
fn undo_action_steps_back_one_state_in_place() {
    let mut game = Game::from_parts(
        [
            cascade(&["KC", "7H"]),
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
    let before = game.clone();
    reduce_in_place(
        &mut game,
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
    )
    .unwrap();
    reduce_in_place(&mut game, Action::Undo).expect("one move to undo");

    assert_eq!(game.cascades(), before.cascades());
    assert_eq!(game.freecells(), before.freecells());
    assert_eq!(game.moves_played(), 0);
}

#[test]
fn undo_action_with_no_history_is_an_error_and_does_not_mutate() {
    let mut game = Game::deal(617);
    let before = game.clone();
    assert_eq!(
        reduce_in_place(&mut game, Action::Undo).unwrap_err(),
        ActionError::NothingToUndo
    );
    assert_eq!(game.cascades(), before.cascades());
}

#[test]
fn restart_action_redeals_the_same_numbered_game_in_place() {
    let mut game = Game::deal(617);
    reduce_in_place(
        &mut game,
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
    )
    .unwrap();
    reduce_in_place(&mut game, Action::Restart).expect("numbered deals can restart");

    assert_eq!(game.cascades(), Game::deal(617).cascades());
    assert_eq!(game.seed(), Some(617));
    assert_eq!(game.moves_played(), 0);
}

#[test]
fn restart_action_on_a_constructed_position_is_an_error() {
    let mut game = Game::from_parts(Default::default(), [None; 4], [0; 4]);
    assert_eq!(
        reduce_in_place(&mut game, Action::Restart).unwrap_err(),
        ActionError::UnknownDeal
    );
}

#[test]
fn redo_action_replays_the_undone_move_in_place() {
    let mut game = Game::from_parts(
        [
            cascade(&["KC", "7H"]),
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
    reduce_in_place(
        &mut game,
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
    )
    .unwrap();
    let after_move = game.cascades().clone();
    reduce_in_place(&mut game, Action::Undo).unwrap();
    reduce_in_place(&mut game, Action::Redo).expect("one undone move to redo");

    assert_eq!(game.cascades(), &after_move);
}

#[test]
fn redo_action_with_no_future_is_an_error_and_does_not_mutate() {
    let mut game = Game::deal(617);
    let before = game.clone();
    assert_eq!(
        reduce_in_place(&mut game, Action::Redo).unwrap_err(),
        ActionError::NothingToRedo
    );
    assert_eq!(game.cascades(), before.cascades());
}

/// The whole point of `reduce_in_place`: it must be observably identical to
/// `reduce` for every action, so swapping one for the other (as `Store` did
/// in issue #24) changes performance, not behavior.
#[test]
fn reduce_in_place_matches_reduce_across_every_action_type() {
    let actions = [
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(1),
        },
        Action::Undo,
        Action::Redo,
        Action::AutoPlay,
        Action::Undo,
        Action::Restart,
        Action::Deal { seed: 42 },
    ];

    let mut via_in_place = Game::deal(11982);
    let mut via_reduce = Game::deal(11982);

    for action in actions {
        let in_place_result = reduce_in_place(&mut via_in_place, action);
        let reduce_result = reduce(&via_reduce, action);

        match (&in_place_result, &reduce_result) {
            (Ok(()), Ok(next)) => {
                via_reduce = next.clone();
                assert_eq!(via_in_place.cascades(), via_reduce.cascades());
                assert_eq!(via_in_place.freecells(), via_reduce.freecells());
                assert_eq!(via_in_place.foundations(), via_reduce.foundations());
                assert_eq!(via_in_place.seed(), via_reduce.seed());
            }
            (Err(e1), Err(e2)) => assert_eq!(e1, e2),
            _ => panic!(
                "reduce_in_place and reduce disagreed on {action:?}: {in_place_result:?} vs {reduce_result:?}"
            ),
        }
    }
}
