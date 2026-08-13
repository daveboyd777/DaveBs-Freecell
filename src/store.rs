//! The Store — owns a [`Game`], applies dispatched [`Action`]s, and notifies
//! subscribers with `(&GameState, &Action)` pairs after every successful
//! dispatch (issue #3).
//!
//! `Store::dispatch` delegates to the existing, already-tested [`reduce`]
//! function rather than reimplementing action handling: `Game`'s own
//! `history` already serves as the "past" stack the design sketch in
//! `ROADMAP.md` describes as a separate `Store` field, so there is no
//! second undo mechanism to keep in sync.
//!
//! Deliberately deferred to issue #4 ("Add redo (time-travel) support"):
//! a `future`/redo stack. Adding one here unused would either trip
//! `cargo clippy -D warnings` (dead code) or half-ship #4's public API
//! inside this PR.

use crate::{reduce, Action, ActionError, Game, GameState};

/// A subscriber observes every successful `(state, action)` transition.
/// Subscribers are never called for a rejected dispatch — a rejected action
/// produced no transition to observe.
type Subscriber = Box<dyn Fn(&GameState, &Action)>;

pub struct Store {
    game: Game,
    subscribers: Vec<Subscriber>,
}

impl Store {
    /// Start a new store from a numbered deal.
    pub fn new(seed: u32) -> Store {
        Store::from_game(Game::deal(seed))
    }

    /// Start a new store from an arbitrary [`Game`] (e.g.
    /// `Store::from_game(Game::from_parts(...))` for tests).
    pub fn from_game(game: Game) -> Store {
        Store {
            game,
            subscribers: Vec::new(),
        }
    }

    /// The full underlying [`Game`] (state, history, and seed).
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The current position, with no history or seed attached. Convenience
    /// alias for `self.game().state()`.
    pub fn state(&self) -> &GameState {
        self.game.state()
    }

    /// Apply an action via the pure [`reduce`] reducer. On success, replaces
    /// the store's `Game` and notifies every subscriber, in registration
    /// order, with the new state and the action that produced it. On
    /// failure, the store is left untouched and no subscriber is notified.
    pub fn dispatch(&mut self, action: Action) -> Result<(), ActionError> {
        let next = reduce(&self.game, action)?;
        self.game = next;
        for subscriber in &self.subscribers {
            subscriber(self.game.state(), &action);
        }
        Ok(())
    }

    /// Register a subscriber. Subscribers are called in registration order
    /// after every successful `dispatch`. There is no unsubscribe for v1 —
    /// subscribers are expected to live for the store's lifetime.
    pub fn subscribe(&mut self, f: impl Fn(&GameState, &Action) + 'static) {
        self.subscribers.push(Box::new(f));
    }
}
