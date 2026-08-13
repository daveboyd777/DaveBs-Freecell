//! The Store — owns a [`Game`], applies dispatched [`Action`]s, and notifies
//! subscribers with `(&GameState, &Action)` pairs after every successful
//! dispatch (issue #3).
//!
//! `Store::dispatch` applies actions via [`reduce_in_place`] rather than the
//! pure, by-reference [`reduce`]: `reduce` clones the entire input `Game`
//! (including its whole `past`/`future` stacks) per call, which made
//! dispatch cost grow with moves already played (issue #24).
//! `reduce_in_place` mutates the store's `Game` directly through its own
//! already-efficient methods instead.
//!
//! Undo *and* redo (issue #4) both come for free: `Game`'s own `past`/
//! `future` stacks already serve as the history the design sketch in
//! `ROADMAP.md` describes as separate `Store` fields, and `Store::dispatch`
//! forwards any [`Action`] — including `Action::Redo` — generically, with
//! no Store-specific redo code needed. This deliberately avoids a second,
//! Store-level undo/redo mechanism that could drift out of sync with
//! `Game`'s own.

use crate::{reduce_in_place, Action, ActionError, Game, GameState};

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

    /// The full underlying [`Game`] (state, past/future stacks, and seed).
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The current position, with no history or seed attached. Convenience
    /// alias for `self.game().state()`.
    pub fn state(&self) -> &GameState {
        self.game.state()
    }

    /// Apply an action via [`reduce_in_place`], which mutates the store's
    /// `Game` directly without cloning its past/future stacks (issue #24).
    /// On success, notifies every subscriber, in registration order, with
    /// the new state and the action that produced it. On failure, the store
    /// is left untouched and no subscriber is notified.
    pub fn dispatch(&mut self, action: Action) -> Result<(), ActionError> {
        reduce_in_place(&mut self.game, action)?;
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
