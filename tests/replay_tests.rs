//! Test-first specification for `replay` (issue #5).
//!
//! "Serialize finished games as (seed, Vec<Action>) to prove replays work":
//! `replay(seed, &actions)` must reconstruct the exact same `Game` that
//! dispatching those actions live (via `Store` or `reduce`) produces.

use freecell::{replay, Action, Game, Loc, Store};

#[test]
fn replay_with_no_actions_reproduces_the_bare_deal() {
    let rebuilt = replay(617, &[]).expect("empty replay always succeeds");
    assert_eq!(rebuilt.cascades(), Game::deal(617).cascades());
    assert_eq!(rebuilt.seed(), Some(617));
}

#[test]
fn replay_reconstructs_a_sequence_of_moves() {
    let seed = 11982;
    let actions = [
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(1),
        },
    ];

    let mut live = Game::deal(seed);
    for &a in &actions {
        live = freecell::reduce(&live, a).unwrap();
    }

    let rebuilt = replay(seed, &actions).expect("all moves legal");
    assert_eq!(rebuilt.cascades(), live.cascades());
    assert_eq!(rebuilt.freecells(), live.freecells());
}

#[test]
fn replay_matches_a_live_store_across_undo_redo_and_autoplay() {
    let seed = 617;
    let mut store = Store::new(seed);
    let mut log: Vec<Action> = Vec::new();

    let actions = [
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
        Action::Undo,
        Action::Redo,
        Action::AutoPlay,
    ];
    for &a in &actions {
        if store.dispatch(a).is_ok() {
            log.push(a);
        }
    }

    let rebuilt = replay(seed, &log).expect("logged actions were all legal once already");
    assert_eq!(rebuilt.cascades(), store.state().cascades());
    assert_eq!(rebuilt.freecells(), store.state().freecells());
    assert_eq!(rebuilt.foundations(), store.state().foundations());
}

#[test]
fn replay_handles_a_restart_mid_sequence() {
    let seed = 617;
    let actions = [
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
        Action::Restart,
        Action::Move {
            from: Loc::Cascade(1),
            to: Loc::Free(0),
        },
    ];

    let rebuilt = replay(seed, &actions).expect("restart mid-sequence is legal");
    // After Restart, the game is back at the bare deal for `seed`, then one
    // more move is applied -- so this must NOT equal the pre-restart state.
    let expected = freecell::reduce(
        &Game::deal(seed),
        Action::Move {
            from: Loc::Cascade(1),
            to: Loc::Free(0),
        },
    )
    .unwrap();
    assert_eq!(rebuilt.cascades(), expected.cascades());
    assert_eq!(rebuilt.seed(), Some(seed));
}

#[test]
fn replay_handles_a_deal_to_a_different_seed_mid_sequence() {
    // A `Deal { seed: other }` mid-log is an absolute reset, just like
    // `Restart` -- replay from the ORIGINAL seed must still reproduce the
    // final state, because Deal ignores whatever came before it.
    let original_seed = 1;
    let other_seed = 42;
    let actions = [
        Action::Move {
            from: Loc::Cascade(0),
            to: Loc::Free(0),
        },
        Action::Deal { seed: other_seed },
    ];

    let rebuilt = replay(original_seed, &actions).expect("deal mid-sequence is legal");
    assert_eq!(rebuilt.cascades(), Game::deal(other_seed).cascades());
    assert_eq!(rebuilt.seed(), Some(other_seed));
}

#[test]
fn replay_reports_the_first_illegal_action_as_an_error() {
    let seed = 617;
    // The free cell is filled by the first move; repeating the exact same
    // move again is illegal (cascade top card is now different anyway, but
    // more directly: moving onto an already-occupied free cell fails).
    let occupy = Action::Move {
        from: Loc::Cascade(0),
        to: Loc::Free(0),
    };
    let actions = [occupy, occupy];

    let err = replay(seed, &actions).unwrap_err();
    // Whatever the exact cause, replay must surface it rather than silently
    // continuing from a stale state.
    assert!(matches!(err, freecell::ActionError::Move(_)));
}
