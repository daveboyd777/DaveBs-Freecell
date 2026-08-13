//! Test-first specification for the Store (issue #3).
//!
//! `Store` owns a `Game`, applies dispatched `Action`s via the existing
//! `reduce` reducer, and notifies subscribers with `(&GameState, &Action)`
//! pairs after every successful dispatch.

use std::cell::RefCell;
use std::rc::Rc;

use freecell::{Action, ActionError, Card, Game, Loc, MoveError, Store, Suit};

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
fn new_deals_the_same_game_as_game_deal() {
    let store = Store::new(617);
    assert_eq!(store.state().cascades(), Game::deal(617).cascades());
    assert_eq!(store.game().seed(), Some(617));
}

#[test]
fn successful_dispatch_updates_state_and_notifies_subscribers() {
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
    let mut store = Store::from_game(game);

    let seen: Rc<RefCell<Vec<(usize, Action)>>> = Rc::new(RefCell::new(Vec::new()));
    let seen_clone = Rc::clone(&seen);
    store.subscribe(move |state, action| {
        seen_clone
            .borrow_mut()
            .push((state.cascades()[0].len(), *action));
    });

    let action = Action::Move {
        from: Loc::Cascade(0),
        to: Loc::Free(0),
    };
    store.dispatch(action).expect("legal move");

    assert_eq!(store.state().freecells()[0], Some(c("7H")));
    assert_eq!(store.state().cascades()[0], cascade(&["KC"]));

    let log = seen.borrow();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (1, action)); // cascade 0 now has 1 card left
}

#[test]
fn failed_dispatch_leaves_state_untouched_and_does_not_notify() {
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
    let mut store = Store::from_game(game);

    let calls = Rc::new(RefCell::new(0));
    let calls_clone = Rc::clone(&calls);
    store.subscribe(move |_, _| *calls_clone.borrow_mut() += 1);

    let err = store
        .dispatch(Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        })
        .unwrap_err();

    assert_eq!(err, ActionError::Move(MoveError::OccupiedFreeCell));
    assert_eq!(store.state().cascades()[0], cascade(&["KC"]));
    assert_eq!(store.state().freecells()[0], Some(c("2C")));
    assert_eq!(*calls.borrow(), 0, "a rejected dispatch must not notify");
}

#[test]
fn multiple_subscribers_all_fire_in_registration_order() {
    let mut store = Store::new(11982);
    let order: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));

    let order_a = Rc::clone(&order);
    store.subscribe(move |_, _| order_a.borrow_mut().push("a"));
    let order_b = Rc::clone(&order);
    store.subscribe(move |_, _| order_b.borrow_mut().push("b"));

    store
        .dispatch(Action::AutoPlay)
        .expect("autoplay always succeeds");

    assert_eq!(*order.borrow(), vec!["a", "b"]);
}

#[test]
fn undo_dispatch_restores_the_previous_state_and_notifies() {
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
    let mut store = Store::from_game(game);
    let before = store.state().clone();

    store
        .dispatch(Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        })
        .unwrap();

    let calls = Rc::new(RefCell::new(0));
    let calls_clone = Rc::clone(&calls);
    store.subscribe(move |_, _| *calls_clone.borrow_mut() += 1);

    store.dispatch(Action::Undo).expect("one move to undo");

    assert_eq!(store.state(), &before);
    assert_eq!(*calls.borrow(), 1);
}

#[test]
fn undo_dispatch_with_no_history_is_an_error_and_does_not_notify() {
    let mut store = Store::new(617);
    let calls = Rc::new(RefCell::new(0));
    let calls_clone = Rc::clone(&calls);
    store.subscribe(move |_, _| *calls_clone.borrow_mut() += 1);

    assert_eq!(
        store.dispatch(Action::Undo).unwrap_err(),
        ActionError::NothingToUndo
    );
    assert_eq!(*calls.borrow(), 0);
}

/// Demonstrates the pattern the future stats module (issues #10/#11) will
/// use: a subscriber recording every dispatched action for later analysis,
/// without touching game logic.
#[test]
fn recorder_subscriber_pattern_records_every_successful_action() {
    let mut store = Store::new(617);
    let log: Rc<RefCell<Vec<Action>>> = Rc::new(RefCell::new(Vec::new()));
    let log_clone = Rc::clone(&log);
    store.subscribe(move |_state, action| log_clone.borrow_mut().push(*action));

    store.dispatch(Action::AutoPlay).unwrap();
    let _ = store.dispatch(Action::Move {
        from: Loc::Cascade(0),
        to: Loc::Free(0),
    }); // may succeed or fail depending on the deal; either is fine here

    // Only successful dispatches are recorded.
    let recorded = log.borrow();
    assert!(recorded.contains(&Action::AutoPlay));
    assert!(recorded.len() <= 2);
}
