//! Tests for `StatsRecorder` (issue #11): turning a live `Store`'s stream
//! of dispatched actions into recorded `GameResult`s, driven through a
//! real `Store` exactly the way a UI's `Store::subscribe` closure does.

use freecell::stats::{GameResult, Stats, StatsRecorder};
use freecell::{Action, Card, Game, Loc, Store, Suit};
use std::cell::RefCell;
use std::rc::Rc;

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

fn store_with_recorder(game: Game, seed: u32) -> (Store, Rc<RefCell<StatsRecorder>>) {
    let mut store = Store::from_game(game);
    let recorder = Rc::new(RefCell::new(StatsRecorder::new(
        seed,
        Stats::default(),
        None,
    )));
    let for_subscriber = Rc::clone(&recorder);
    store.subscribe(move |state, action| for_subscriber.borrow_mut().observe(state, action));
    (store, recorder)
}

#[test]
fn winning_a_game_is_recorded_once_and_survives_undo_redo() {
    let game = Game::from_parts(
        Default::default(),
        [Some(c("KS")), None, None, None],
        [13, 13, 13, 12],
    );
    let (mut store, recorder) = store_with_recorder(game, 42);

    store
        .dispatch(Action::Move {
            from: Loc::Free(0),
            to: Loc::Foundation,
        })
        .expect("legal move");
    assert!(store.state().is_won());
    assert_eq!(recorder.borrow().stats().games_played(), 1);
    assert_eq!(recorder.borrow().stats().games_won(), 1);

    store.dispatch(Action::Undo).expect("undo the winning move");
    store
        .dispatch(Action::Redo)
        .expect("redo back into the win");
    assert_eq!(
        recorder.borrow().stats().games_played(),
        1,
        "cycling undo/redo through the same win must not double-record it"
    );
}

#[test]
fn abandoning_a_deal_with_moves_records_a_loss_but_zero_moves_does_not() {
    let game = Game::from_parts(
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
    let (mut store, recorder) = store_with_recorder(game, 7);

    store
        .dispatch(Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        })
        .expect("legal move");
    store
        .dispatch(Action::Deal { seed: 8 })
        .expect("dealing a new game always succeeds");

    let stats = recorder.borrow().stats().clone();
    assert_eq!(stats.games_played(), 1);
    assert_eq!(stats.games_lost(), 1);
    assert_eq!(
        stats.deal_history(7),
        vec![&GameResult {
            seed: 7,
            won: false,
            moves: 1
        }]
    );

    // Immediately abandoning the fresh deal 8 (zero moves played) must not
    // record an attempt that never really happened.
    store
        .dispatch(Action::Deal { seed: 9 })
        .expect("dealing a new game always succeeds");
    assert_eq!(recorder.borrow().stats().games_played(), 1);
}

#[test]
fn restart_records_a_loss_for_the_same_seed_and_starts_a_fresh_attempt() {
    let (mut store, recorder) = store_with_recorder(Game::deal(617), 617);

    // Moving the top card of any cascade to an empty free cell is always
    // legal, regardless of what the actual dealt cards are.
    store
        .dispatch(Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        })
        .expect("legal move");
    store
        .dispatch(Action::Restart)
        .expect("a numbered deal can always restart");

    let stats = recorder.borrow().stats().clone();
    assert_eq!(
        stats.deal_history(617),
        vec![&GameResult {
            seed: 617,
            won: false,
            moves: 1
        }]
    );

    // The restarted attempt is tracked fresh: abandoning it immediately
    // (zero moves) must not add a second record.
    store
        .dispatch(Action::Deal { seed: 1 })
        .expect("dealing a new game always succeeds");
    assert_eq!(recorder.borrow().stats().games_played(), 1);
}

#[test]
fn autoplay_counts_every_card_it_sends_home_as_moves() {
    // AC and AH sit on top; autoplay should send both home in one dispatch.
    let game = Game::from_parts(
        [
            cascade(&["KD", "AC"]),
            cascade(&["AH"]),
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
    let (mut store, recorder) = store_with_recorder(game, 11);

    store
        .dispatch(Action::AutoPlay)
        .expect("autoplay always succeeds");
    store
        .dispatch(Action::Deal { seed: 12 })
        .expect("dealing a new game always succeeds");

    let stats = recorder.borrow().stats().clone();
    assert_eq!(
        stats.deal_history(11),
        vec![&GameResult {
            seed: 11,
            won: false,
            moves: 2
        }]
    );
}
