//! Test-first specification for the Redux-style layer (issue #2).
//!
//! `Action` describes every way the game can change, as plain data.
//! `reduce(&game, action)` is a PURE function: it never mutates its input,
//! never performs I/O, and returns either the next state or an error.

use freecell::{reduce, Action, ActionError, Card, Game, Loc, MoveError, Suit};

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
fn deal_action_produces_the_numbered_game() {
    let start = Game::from_parts(Default::default(), [None; 4], [0; 4]);
    let dealt = reduce(&start, Action::Deal { seed: 1 }).expect("deal always succeeds");
    assert_eq!(dealt.cascades(), Game::deal(1).cascades());
    assert_eq!(dealt.seed(), Some(1));
}

#[test]
fn move_action_returns_the_next_state_and_never_mutates_the_input() {
    let before = Game::from_parts(
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
    let after = reduce(
        &before,
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
    )
    .expect("legal move");

    // The new state reflects the move...
    assert_eq!(after.freecells()[0], Some(c("7H")));
    assert_eq!(after.cascades()[0], cascade(&["KC"]));
    // ...and the input state is untouched (purity).
    assert_eq!(before.freecells()[0], None);
    assert_eq!(before.cascades()[0], cascade(&["KC", "7H"]));
}

#[test]
fn illegal_move_action_reports_the_underlying_move_error() {
    let game = Game::from_parts(
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
    let err = reduce(
        &game,
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
    )
    .unwrap_err();
    assert_eq!(err, ActionError::Move(MoveError::OccupiedFreeCell));
}

#[test]
fn autoplay_action_sends_every_playable_card_home() {
    // AC and AH are on top; after the aces go up, 2H follows.
    let game = Game::from_parts(
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
    let after = reduce(&game, Action::AutoPlay).expect("autoplay always succeeds");
    assert_eq!(after.foundations()[Suit::Clubs as usize], 1);
    assert_eq!(after.foundations()[Suit::Hearts as usize], 2);
    assert_eq!(after.freecells()[0], None);
    assert_eq!(after.cascades()[0], cascade(&["KD"]));
    // Purity: the input still has everything in place.
    assert_eq!(game.foundations(), &[0, 0, 0, 0]);
}

#[test]
fn undo_action_steps_back_one_state() {
    let g0 = Game::from_parts(
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
    let g1 = reduce(
        &g0,
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
    )
    .unwrap();
    let g2 = reduce(&g1, Action::Undo).expect("one move to undo");
    assert_eq!(g2.cascades(), g0.cascades());
    assert_eq!(g2.freecells(), g0.freecells());
}

#[test]
fn undo_action_with_no_history_is_an_error() {
    let game = Game::deal(617);
    assert_eq!(
        reduce(&game, Action::Undo).unwrap_err(),
        ActionError::NothingToUndo
    );
}

#[test]
fn restart_action_redeal_the_same_numbered_game() {
    let dealt = Game::deal(617);
    let played = reduce(
        &dealt,
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
    )
    .unwrap();
    let restarted = reduce(&played, Action::Restart).expect("numbered deals can restart");
    assert_eq!(restarted.cascades(), dealt.cascades());
    assert_eq!(restarted.seed(), Some(617));
}

#[test]
fn restart_action_on_a_constructed_position_is_an_error() {
    // from_parts positions have no deal number to restart from.
    let game = Game::from_parts(Default::default(), [None; 4], [0; 4]);
    assert_eq!(
        reduce(&game, Action::Restart).unwrap_err(),
        ActionError::UnknownDeal
    );
}

#[test]
fn a_finished_game_is_replayable_from_its_action_log() {
    // The Redux payoff: (seed, actions) fully reconstructs a game.
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
    ];
    let mut live = Game::deal(11982);
    let mut replayed = Game::deal(11982);
    for a in &actions {
        live = reduce(&live, *a).unwrap();
    }
    for a in &actions {
        replayed = reduce(&replayed, *a).unwrap();
    }
    assert_eq!(live.cascades(), replayed.cascades());
    assert_eq!(live.freecells(), replayed.freecells());
    assert_eq!(live.foundations(), replayed.foundations());
}
