//! Test-first specification for the FreeCell game engine.
//!
//! These tests define the public API of the `freecell` library:
//! - `Card`, `Suit` — playing cards
//! - `Game::deal(seed)` — Microsoft-compatible numbered deals
//! - `Game::from_parts(...)` — arbitrary positions for testing
//! - `Loc` — move source/destination (cascade, free cell, foundation)
//! - `Game::do_move(from, to)` — validated moves incl. multi-card supermoves
//! - `Game::undo()`, `Game::is_won()`

use freecell::{replay, Action, Card, Game, Loc, Suit};

/// Parse "JD" / "TC" / "AS" style shorthand into a Card.
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

// ---------------------------------------------------------------- dealing

#[test]
fn deal_produces_52_unique_cards_in_standard_layout() {
    let game = Game::deal(617);
    let lengths: Vec<usize> = game.cascades().iter().map(|col| col.len()).collect();
    assert_eq!(lengths, vec![7, 7, 7, 7, 6, 6, 6, 6]);

    let mut seen = std::collections::HashSet::new();
    for col in game.cascades() {
        for card in col {
            assert!(
                (1..=13).contains(&card.rank),
                "rank out of range: {}",
                card.rank
            );
            assert!(seen.insert(*card), "duplicate card dealt: {card:?}");
        }
    }
    assert_eq!(seen.len(), 52);

    assert!(game.freecells().iter().all(|f| f.is_none()));
    assert!(game.foundations().iter().all(|&f| f == 0));
}

#[test]
fn deal_1_matches_the_classic_microsoft_layout() {
    // Game #1 from the original Microsoft FreeCell deal algorithm.
    let game = Game::deal(1);
    let expected_rows = [
        ["JD", "2D", "9H", "JC", "5D", "7H", "7C", "5H"],
        ["KD", "KC", "9S", "5S", "AD", "QC", "KH", "3H"],
        ["2S", "KS", "9D", "QD", "JS", "AS", "AH", "3C"],
        ["4C", "5C", "TS", "QH", "4H", "AC", "4D", "7S"],
        ["3S", "TD", "4S", "TH", "8H", "2C", "JH", "7D"],
        ["6D", "8S", "8D", "QS", "6C", "3D", "8C", "TC"],
    ];
    for (row, names) in expected_rows.iter().enumerate() {
        for (col, name) in names.iter().enumerate() {
            assert_eq!(
                game.cascades()[col][row],
                c(name),
                "mismatch at row {row}, column {col}"
            );
        }
    }
    // Seventh row only reaches the first four columns.
    let last = ["6S", "9C", "2H", "6H"];
    for (col, name) in last.iter().enumerate() {
        assert_eq!(game.cascades()[col][6], c(name));
    }
}

#[test]
fn same_seed_deals_same_game() {
    assert_eq!(Game::deal(11982).cascades(), Game::deal(11982).cascades());
}

// ---------------------------------------------------------------- free cells

#[test]
fn top_card_can_move_to_an_empty_free_cell() {
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
        [None, None, None, None],
        [0, 0, 0, 0],
    );
    let moved = game
        .do_move(Loc::Cascade(0), Loc::Free(1))
        .expect("move should succeed");
    assert_eq!(moved, 1);
    assert_eq!(game.freecells()[1], Some(c("7H")));
    assert_eq!(game.cascades()[0], cascade(&["KC"]));
}

#[test]
fn a_card_cannot_move_to_an_occupied_free_cell() {
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
        [Some(c("2C")), None, None, None],
        [0, 0, 0, 0],
    );
    assert!(game.do_move(Loc::Cascade(0), Loc::Free(0)).is_err());
    // State unchanged after a rejected move.
    assert_eq!(game.cascades()[0], cascade(&["KC", "7H"]));
    assert_eq!(game.freecells()[0], Some(c("2C")));
}

#[test]
fn a_free_cell_card_can_return_to_a_valid_cascade() {
    let mut game = Game::from_parts(
        [
            cascade(&["8S"]),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [Some(c("7H")), None, None, None],
        [0, 0, 0, 0],
    );
    game.do_move(Loc::Free(0), Loc::Cascade(0))
        .expect("7H onto 8S is legal");
    assert_eq!(game.cascades()[0], cascade(&["8S", "7H"]));
    assert_eq!(game.freecells()[0], None);
}

// ---------------------------------------------------------------- foundations

#[test]
fn only_an_ace_starts_a_foundation() {
    let mut game = Game::from_parts(
        [
            cascade(&["AH"]),
            cascade(&["2H"]),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [None, None, None, None],
        [0, 0, 0, 0],
    );
    assert!(
        game.do_move(Loc::Cascade(1), Loc::Foundation).is_err(),
        "2H before AH must fail"
    );
    game.do_move(Loc::Cascade(0), Loc::Foundation)
        .expect("AH starts the hearts foundation");
    assert_eq!(game.foundations()[Suit::Hearts as usize], 1);
    // Now the 2H goes up.
    game.do_move(Loc::Cascade(1), Loc::Foundation)
        .expect("2H follows AH");
    assert_eq!(game.foundations()[Suit::Hearts as usize], 2);
}

#[test]
fn foundations_are_per_suit() {
    let mut game = Game::from_parts(
        [
            cascade(&["2S"]),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [None, None, None, None],
        [0, 0, 1, 0], // hearts foundation holds the ace
    );
    assert!(
        game.do_move(Loc::Cascade(0), Loc::Foundation).is_err(),
        "2S cannot ride the hearts foundation"
    );
}

// ---------------------------------------------------------------- cascade rules

#[test]
fn cascade_stacking_requires_descending_rank_and_alternating_color() {
    let mut game = Game::from_parts(
        [
            cascade(&["8S"]), // black 8
            cascade(&["7H"]), // red 7  -> legal on 8S
            cascade(&["7D"]), // red 7  -> legal on 8S
            cascade(&["7S"]), // black 7 -> illegal on 8S (same color)
            cascade(&["6H"]), // red 6  -> illegal on 8S (wrong rank)
            vec![],
            vec![],
            vec![],
        ],
        [None, None, None, None],
        [0, 0, 0, 0],
    );
    assert!(
        game.do_move(Loc::Cascade(3), Loc::Cascade(0)).is_err(),
        "same color must fail"
    );
    assert!(
        game.do_move(Loc::Cascade(4), Loc::Cascade(0)).is_err(),
        "rank gap must fail"
    );
    game.do_move(Loc::Cascade(1), Loc::Cascade(0))
        .expect("red 7 on black 8");
    assert_eq!(game.cascades()[0], cascade(&["8S", "7H"]));
}

#[test]
fn any_card_can_move_to_an_empty_cascade() {
    let mut game = Game::from_parts(
        [
            cascade(&["4D", "9C"]),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [None, None, None, None],
        [0, 0, 0, 0],
    );
    game.do_move(Loc::Cascade(0), Loc::Cascade(5))
        .expect("9C to empty cascade");
    assert_eq!(game.cascades()[5], cascade(&["9C"]));
}

// ---------------------------------------------------------------- supermoves

#[test]
fn an_ordered_run_moves_as_a_unit_when_capacity_allows() {
    // 9H-8S-7D is an ordered alternating run; all four free cells empty
    // gives capacity (4+1) = 5, so a 3-card move is fine.
    let mut game = Game::from_parts(
        [
            cascade(&["KC", "9H", "8S", "7D"]),
            cascade(&["TS"]),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [None, None, None, None],
        [0, 0, 0, 0],
    );
    let moved = game
        .do_move(Loc::Cascade(0), Loc::Cascade(1))
        .expect("run onto TS");
    assert_eq!(moved, 3);
    assert_eq!(game.cascades()[1], cascade(&["TS", "9H", "8S", "7D"]));
    assert_eq!(game.cascades()[0], cascade(&["KC"]));
}

#[test]
fn supermove_fails_when_free_cells_cannot_cover_it() {
    // All free cells full and no empty cascades: capacity is 1, so the
    // 3-card run cannot move even though the target card matches.
    let mut game = Game::from_parts(
        [
            cascade(&["KC", "9H", "8S", "7D"]),
            cascade(&["TS"]),
            cascade(&["2C"]),
            cascade(&["2D"]),
            cascade(&["2H"]),
            cascade(&["2S"]),
            cascade(&["3C"]),
            cascade(&["3D"]),
        ],
        [Some(c("KD")), Some(c("KH")), Some(c("KS")), Some(c("QC"))],
        [0, 0, 0, 0],
    );
    assert!(game.do_move(Loc::Cascade(0), Loc::Cascade(1)).is_err());
}

#[test]
fn empty_cascades_double_supermove_capacity() {
    // 3 free cells full, 1 free -> base capacity 2; one empty cascade
    // doubles it to 4, enough for the 3-card run.
    let mut game = Game::from_parts(
        [
            cascade(&["KC", "9H", "8S", "7D"]),
            cascade(&["TS"]),
            vec![], // empty cascade used as a waystation
            cascade(&["2D"]),
            cascade(&["2H"]),
            cascade(&["2S"]),
            cascade(&["3C"]),
            cascade(&["3D"]),
        ],
        [Some(c("KD")), Some(c("KH")), Some(c("KS")), None],
        [0, 0, 0, 0],
    );
    let moved = game
        .do_move(Loc::Cascade(0), Loc::Cascade(1))
        .expect("supermove via empty column");
    assert_eq!(moved, 3);
    assert_eq!(game.cascades()[1], cascade(&["TS", "9H", "8S", "7D"]));
}

// ---------------------------------------------------------------- win & undo

#[test]
fn game_is_won_when_all_foundations_reach_king() {
    let won = Game::from_parts(
        [
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [None, None, None, None],
        [13, 13, 13, 13],
    );
    assert!(won.is_won());

    let not_won = Game::from_parts(
        [
            cascade(&["KH"]),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [None, None, None, None],
        [13, 13, 12, 13],
    );
    assert!(!not_won.is_won());
}

#[test]
fn undo_restores_the_previous_position() {
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
        [None, None, None, None],
        [0, 0, 0, 0],
    );
    let before = game.cascades().clone();
    game.do_move(Loc::Cascade(0), Loc::Free(0)).unwrap();
    assert!(game.undo(), "undo after a move should succeed");
    assert_eq!(game.cascades(), &before);
    assert_eq!(game.freecells()[0], None);
    assert!(!game.undo(), "nothing left to undo");
}

#[test]
fn rejected_moves_are_not_undoable() {
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
        [0, 0, 0, 0],
    );
    assert!(game.do_move(Loc::Cascade(0), Loc::Free(0)).is_err());
    assert!(!game.undo(), "a failed move must not create an undo entry");
}

// ---------------------------------------------------------------- redo

#[test]
fn redo_replays_a_move_that_was_undone() {
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
        [None, None, None, None],
        [0, 0, 0, 0],
    );
    game.do_move(Loc::Cascade(0), Loc::Free(0)).unwrap();
    let after_move = game.cascades().clone();
    game.undo();

    assert!(game.redo(), "redo after an undo should succeed");
    assert_eq!(game.cascades(), &after_move);
    assert_eq!(game.freecells()[0], Some(c("7H")));
    assert_eq!(game.moves_played(), 1);
}

#[test]
fn redo_with_nothing_undone_is_a_no_op() {
    let mut game = Game::deal(617);
    assert!(!game.redo(), "nothing to redo without an undo first");
}

#[test]
fn a_new_move_after_undo_clears_the_redo_stack() {
    let mut game = Game::from_parts(
        [
            cascade(&["KC", "7H"]),
            cascade(&["8S"]),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        [None, None, None, None],
        [0, 0, 0, 0],
    );
    game.do_move(Loc::Cascade(0), Loc::Free(0)).unwrap();
    game.undo();
    // A different move now, instead of redoing the free-cell move.
    game.do_move(Loc::Cascade(1), Loc::Free(1)).unwrap();

    assert!(
        !game.redo(),
        "a fresh move after undo must invalidate the old redo target"
    );
}

#[test]
fn undo_then_redo_round_trips_to_the_exact_same_state() {
    let mut game = Game::deal(11982);
    let before = game.cascades().clone();
    game.do_move(Loc::Cascade(0), Loc::Free(0)).unwrap();
    game.undo();
    assert_eq!(game.cascades(), &before);
    game.redo();
    // And undo remains available after a redo.
    assert!(game.undo(), "undo should work again after a redo");
    assert_eq!(game.cascades(), &before);
}

// ---------------------------------------------------------------- replay

#[test]
fn replay_reconstructs_the_exact_same_game_as_playing_it_live() {
    let seed = 617;
    let moves = [
        (Loc::Cascade(0), Loc::Free(0)),
        (Loc::Cascade(1), Loc::Free(1)),
    ];

    let mut live = Game::deal(seed);
    for &(from, to) in &moves {
        live.do_move(from, to).unwrap();
    }

    let actions: Vec<Action> = moves
        .iter()
        .map(|&(from, to)| Action::Move { from, to })
        .collect();
    let rebuilt = replay(seed, &actions).expect("both moves are legal");

    assert_eq!(rebuilt.cascades(), live.cascades());
    assert_eq!(rebuilt.freecells(), live.freecells());
    assert_eq!(
        rebuilt, live,
        "replay must match the full Game, not just its position"
    );
}

#[test]
fn replay_with_no_actions_is_just_the_bare_deal() {
    let rebuilt = replay(11982, &[]).expect("empty replay always succeeds");
    assert_eq!(rebuilt, Game::deal(11982));
}
