//! FreeCell game engine.
//!
//! Deals are compatible with the classic Microsoft FreeCell numbering, so
//! `Game::deal(1)` produces the same layout as "Game #1" in the original.

use std::fmt;

pub mod analysis;
pub mod solver;
pub mod stats;
pub mod store;
pub use store::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Clubs = 0,
    Diamonds = 1,
    Hearts = 2,
    Spades = 3,
}

impl Suit {
    pub fn is_red(self) -> bool {
        matches!(self, Suit::Diamonds | Suit::Hearts)
    }

    fn from_index(i: usize) -> Suit {
        match i {
            0 => Suit::Clubs,
            1 => Suit::Diamonds,
            2 => Suit::Hearts,
            3 => Suit::Spades,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub rank: u8, // 1 = Ace .. 13 = King
    pub suit: Suit,
}

impl Card {
    pub fn new(rank: u8, suit: Suit) -> Card {
        assert!((1..=13).contains(&rank), "rank must be 1..=13");
        Card { rank, suit }
    }

    /// True when `self` may sit on `other` in a cascade:
    /// one rank lower and the opposite color.
    fn stacks_on(&self, other: &Card) -> bool {
        self.rank + 1 == other.rank && self.suit.is_red() != other.suit.is_red()
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const RANKS: [char; 13] = [
            'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K',
        ];
        const SUITS: [char; 4] = ['C', 'D', 'H', 'S'];
        write!(
            f,
            "{}{}",
            RANKS[(self.rank - 1) as usize],
            SUITS[self.suit as usize]
        )
    }
}

/// A move source or destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loc {
    Cascade(usize),
    Free(usize),
    /// Foundations are addressed collectively; the card's suit picks the pile.
    Foundation,
}

/// Parse the shared two-character move command grammar: source then
/// destination, `1`-`8` for cascade columns, `a`-`d` for free cells, `h` or
/// `f` for foundations (e.g. "1a", "35", "ah", "2h"). Whitespace between
/// (or around) the two characters is ignored. Foundations are never a valid
/// source (`h`/`f` as the first character is rejected), since a card only
/// ever leaves a foundation via undo/redo, not a dispatched move.
///
/// Shared by the text CLI (`src/main.rs`) and the ratatui TUI (`tui/`) so
/// the command grammar has one implementation and one set of tests.
pub fn parse_move(cmd: &str) -> Option<(Loc, Loc)> {
    let mut chars = cmd.chars().filter(|c| !c.is_whitespace());
    let from = parse_loc(chars.next()?)?;
    let to = parse_loc(chars.next()?)?;
    if chars.next().is_some() {
        return None;
    }
    if from == Loc::Foundation {
        return None;
    }
    Some((from, to))
}

fn parse_loc(c: char) -> Option<Loc> {
    match c {
        '1'..='8' => Some(Loc::Cascade(c as usize - '1' as usize)),
        'a'..='d' => Some(Loc::Free(c as usize - 'a' as usize)),
        'h' | 'f' => Some(Loc::Foundation),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    EmptySource,
    OccupiedFreeCell,
    NotOneHigherSameSuit,
    NoMatchingRun,
    NotEnoughCapacity,
    InvalidLocation,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            MoveError::EmptySource => "there is no card there to move",
            MoveError::OccupiedFreeCell => "that free cell is occupied",
            MoveError::NotOneHigherSameSuit => "foundations build up by suit, ace first",
            MoveError::NoMatchingRun => "that card cannot go on that pile",
            MoveError::NotEnoughCapacity => "not enough free cells to move that many cards",
            MoveError::InvalidLocation => "invalid location",
        };
        f.write_str(msg)
    }
}

/// A plain-data snapshot of a FreeCell position: the cascades, free cells,
/// and foundations. `GameState` carries no history and no behavior beyond
/// the rules needed to validate and apply a single move — it is the value
/// type that [`Store`] hands to subscribers and that [`Game`] wraps with
/// undo history and a deal seed.
///
/// Extracted from the original monolithic `Game` struct (issue #3) so the
/// position itself is cheap to snapshot, compare, and observe independently
/// of move-history bookkeeping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    cascades: [Vec<Card>; 8],
    freecells: [Option<Card>; 4],
    /// Top rank per suit, indexed by `Suit as usize`; 0 means empty.
    foundations: [u8; 4],
}

impl GameState {
    /// Deal a numbered game using the classic Microsoft FreeCell algorithm.
    pub fn deal(seed: u32) -> GameState {
        // Borland C rand(): state = state * 214013 + 2531011 (mod 2^31),
        // returning bits 16..30.
        let mut state: u32 = seed;
        let mut rand = move || {
            state = state.wrapping_mul(214013).wrapping_add(2531011) & 0x7FFF_FFFF;
            state >> 16
        };

        let mut deck: Vec<Card> = (0..52)
            .map(|n| Card::new((n / 4 + 1) as u8, Suit::from_index(n % 4)))
            .collect();

        let mut cascades: [Vec<Card>; 8] = Default::default();
        for i in 0..52 {
            let j = rand() as usize % (52 - i);
            deck.swap(j, 51 - i);
            cascades[i % 8].push(deck[51 - i]);
        }

        GameState {
            cascades,
            freecells: [None; 4],
            foundations: [0; 4],
        }
    }

    /// Build an arbitrary position. Used by tests and available for tooling;
    /// no consistency check is performed.
    pub fn from_parts(
        cascades: [Vec<Card>; 8],
        freecells: [Option<Card>; 4],
        foundations: [u8; 4],
    ) -> GameState {
        GameState {
            cascades,
            freecells,
            foundations,
        }
    }

    pub fn cascades(&self) -> &[Vec<Card>; 8] {
        &self.cascades
    }

    pub fn freecells(&self) -> &[Option<Card>; 4] {
        &self.freecells
    }

    pub fn foundations(&self) -> &[u8; 4] {
        &self.foundations
    }

    pub fn is_won(&self) -> bool {
        self.foundations.iter().all(|&r| r == 13)
    }

    /// Non-mutating dry run of [`GameState::do_move`]: reports whether `from
    /// -> to` would succeed, and how many cards would move, without
    /// changing `self`. This is the single source of truth for move
    /// legality that UIs (e.g. the TUI's legal-destination dimming, issue
    /// #7) should query instead of reimplementing any rule -- it is
    /// implemented purely in terms of `do_move` on a throwaway clone, so
    /// there is exactly one place the actual rules live.
    pub fn can_move(&self, from: Loc, to: Loc) -> Result<usize, MoveError> {
        self.clone().do_move(from, to)
    }

    /// How many trailing cards at `loc` would move together if `loc` were
    /// chosen as a move source, independent of any destination (i.e.
    /// ignoring destination-specific capacity/matching limits that
    /// `do_move`/`can_move` additionally enforce). Used for highlighting
    /// the selected run in a UI: `Loc::Cascade` returns the longest
    /// ordered, alternating-color run at that column's tail (0 if empty or
    /// out of range); `Loc::Free` returns 1 if occupied else 0;
    /// `Loc::Foundation` is always 0, since a foundation is never a valid
    /// move source.
    pub fn movable_run_len(&self, loc: Loc) -> usize {
        match loc {
            Loc::Cascade(i) => self.cascades.get(i).map_or(0, |c| tail_run_len(c)),
            Loc::Free(i) => usize::from(self.freecells.get(i).is_some_and(Option::is_some)),
            Loc::Foundation => 0,
        }
    }

    /// Perform a move, returning the number of cards moved.
    ///
    /// Cascade-to-cascade moves transfer the longest ordered run that legally
    /// fits the destination, subject to the standard supermove capacity
    /// `(1 + empty free cells) * 2^(empty cascades)` — the destination column,
    /// if empty, does not count toward that doubling.
    ///
    /// Atomic: on `Err`, `self` is left exactly as it was before the call.
    pub fn do_move(&mut self, from: Loc, to: Loc) -> Result<usize, MoveError> {
        match to {
            Loc::Free(cell) => {
                if cell >= 4 {
                    return Err(MoveError::InvalidLocation);
                }
                if self.freecells[cell].is_some() {
                    return Err(MoveError::OccupiedFreeCell);
                }
                let card = self.take_single(from)?;
                self.freecells[cell] = Some(card);
                Ok(1)
            }
            Loc::Foundation => {
                let card = self.peek_single(from)?;
                let pile = &mut self.foundations[card.suit as usize];
                if card.rank != *pile + 1 {
                    return Err(MoveError::NotOneHigherSameSuit);
                }
                *pile += 1;
                self.take_single(from)
                    .expect("peeked card must be takeable");
                Ok(1)
            }
            Loc::Cascade(dst) => {
                if dst >= 8 {
                    return Err(MoveError::InvalidLocation);
                }
                match from {
                    Loc::Free(_) => {
                        let card = self.peek_single(from)?;
                        self.check_cascade_target(dst, &card)?;
                        let card = self
                            .take_single(from)
                            .expect("peeked card must be takeable");
                        self.cascades[dst].push(card);
                        Ok(1)
                    }
                    Loc::Cascade(src) => self.move_run(src, dst),
                    Loc::Foundation => Err(MoveError::InvalidLocation),
                }
            }
        }
    }

    /// Move the longest legal run from cascade `src` onto cascade `dst`.
    fn move_run(&mut self, src: usize, dst: usize) -> Result<usize, MoveError> {
        if src >= 8 || src == dst {
            return Err(MoveError::InvalidLocation);
        }
        let source = &self.cascades[src];
        if source.is_empty() {
            return Err(MoveError::EmptySource);
        }

        let run_len = tail_run_len(source);
        let dest_top = self.cascades[dst].last().copied();
        // How many cards must move: onto a card, exactly the sub-run that
        // fits it; onto an empty column, as much of the run as capacity allows.
        let count = match dest_top {
            Some(top) => {
                let want = (1..=run_len).find(|&n| {
                    let card = source[source.len() - n];
                    card.stacks_on(&top)
                });
                match want {
                    Some(n) => n,
                    None => return Err(MoveError::NoMatchingRun),
                }
            }
            None => run_len.min(self.capacity(true)),
        };

        if count > self.capacity(dest_top.is_none()) {
            return Err(MoveError::NotEnoughCapacity);
        }

        let split = self.cascades[src].len() - count;
        let mut tail = self.cascades[src].split_off(split);
        self.cascades[dst].append(&mut tail);
        Ok(count)
    }

    /// Supermove capacity: (1 + empty free cells) * 2^(empty cascades).
    /// When the destination is an empty cascade it must not count itself.
    fn capacity(&self, dest_is_empty_cascade: bool) -> usize {
        let free = self.freecells.iter().filter(|c| c.is_none()).count();
        let mut empties = self.cascades.iter().filter(|c| c.is_empty()).count();
        if dest_is_empty_cascade {
            empties = empties.saturating_sub(1);
        }
        (free + 1) << empties
    }

    fn check_cascade_target(&self, dst: usize, card: &Card) -> Result<(), MoveError> {
        match self.cascades[dst].last() {
            Some(top) if !card.stacks_on(top) => Err(MoveError::NoMatchingRun),
            _ => Ok(()),
        }
    }

    fn peek_single(&self, from: Loc) -> Result<Card, MoveError> {
        match from {
            Loc::Cascade(i) if i < 8 => self.cascades[i]
                .last()
                .copied()
                .ok_or(MoveError::EmptySource),
            Loc::Free(i) if i < 4 => self.freecells[i].ok_or(MoveError::EmptySource),
            _ => Err(MoveError::InvalidLocation),
        }
    }

    fn take_single(&mut self, from: Loc) -> Result<Card, MoveError> {
        match from {
            Loc::Cascade(i) if i < 8 => self.cascades[i].pop().ok_or(MoveError::EmptySource),
            Loc::Free(i) if i < 4 => self.freecells[i].take().ok_or(MoveError::EmptySource),
            _ => Err(MoveError::InvalidLocation),
        }
    }
}

/// The longest ordered (descending, alternating-color) run at the tail of a
/// cascade column, i.e. how many cards would move together in a
/// cascade-to-cascade move before capacity/destination-matching limits are
/// applied. Shared by [`GameState::move_run`] (which additionally enforces
/// those limits) and [`GameState::movable_run_len`] (which reports it
/// as-is, for UI highlighting) so the run-detection rule itself has one
/// implementation. Returns 0 for an empty slice, 1 for a single card.
fn tail_run_len(cascade: &[Card]) -> usize {
    let mut run_len = match cascade.len() {
        0 => return 0,
        _ => 1,
    };
    while run_len < cascade.len() {
        let upper = &cascade[cascade.len() - run_len - 1];
        let lower = &cascade[cascade.len() - run_len];
        if lower.stacks_on(upper) {
            run_len += 1;
        } else {
            break;
        }
    }
    run_len
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    state: GameState,
    /// States reachable by [`Game::undo`], newest last.
    past: Vec<GameState>,
    /// States reachable by [`Game::redo`], newest last. Populated only by
    /// `undo`; cleared by any other state-changing action ([`Game::do_move`],
    /// and therefore [`Game::auto_play`], which calls it in a loop) so a
    /// fresh move after an undo does not leave a stale, misleading redo
    /// target around — the generalization issue #4 asks for.
    future: Vec<GameState>,
    /// The deal number, when this game came from `Game::deal`.
    seed: Option<u32>,
}

impl Game {
    /// Deal a numbered game using the classic Microsoft FreeCell algorithm.
    pub fn deal(seed: u32) -> Game {
        Game {
            state: GameState::deal(seed),
            past: Vec::new(),
            future: Vec::new(),
            seed: Some(seed),
        }
    }

    /// Build an arbitrary position. Used by tests and available for tooling;
    /// no consistency check is performed.
    pub fn from_parts(
        cascades: [Vec<Card>; 8],
        freecells: [Option<Card>; 4],
        foundations: [u8; 4],
    ) -> Game {
        Game {
            state: GameState::from_parts(cascades, freecells, foundations),
            past: Vec::new(),
            future: Vec::new(),
            seed: None,
        }
    }

    /// The deal number this game was dealt from, when known.
    pub fn seed(&self) -> Option<u32> {
        self.seed
    }

    /// The current position as a plain [`GameState`] snapshot, with no
    /// history or seed attached.
    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn cascades(&self) -> &[Vec<Card>; 8] {
        self.state.cascades()
    }

    pub fn freecells(&self) -> &[Option<Card>; 4] {
        self.state.freecells()
    }

    pub fn foundations(&self) -> &[u8; 4] {
        self.state.foundations()
    }

    pub fn is_won(&self) -> bool {
        self.state.is_won()
    }

    /// Undo the last successful move. Returns false when there is nothing to undo.
    /// On success, the undone state becomes available to [`Game::redo`].
    pub fn undo(&mut self) -> bool {
        match self.past.pop() {
            Some(state) => {
                self.future.push(std::mem::replace(&mut self.state, state));
                true
            }
            None => false,
        }
    }

    /// Redo the last undone move. Returns false when there is nothing to
    /// redo — either nothing has been undone yet, or a new action since the
    /// last undo cleared the redo stack.
    pub fn redo(&mut self) -> bool {
        match self.future.pop() {
            Some(state) => {
                self.past.push(std::mem::replace(&mut self.state, state));
                true
            }
            None => false,
        }
    }

    /// How many moves have been played (and can be undone).
    pub fn moves_played(&self) -> usize {
        self.past.len()
    }

    /// Every ancestor of the current position since the last deal/restart,
    /// oldest first -- i.e. the same states [`Game::undo`] can still step
    /// back through. Does not include the current position itself; pair
    /// with [`Game::state`] for the complete sequence from the deal to now
    /// (used by [`crate::analysis::grade`], issue #13).
    pub fn history(&self) -> &[GameState] {
        &self.past
    }

    /// Repeatedly send every playable card to the foundations.
    /// Returns the number of cards sent home.
    pub fn auto_play(&mut self) -> usize {
        let mut sent = 0;
        loop {
            let mut progressed = false;
            for i in 0..8 {
                if self.do_move(Loc::Cascade(i), Loc::Foundation).is_ok() {
                    progressed = true;
                    sent += 1;
                }
            }
            for i in 0..4 {
                if self.do_move(Loc::Free(i), Loc::Foundation).is_ok() {
                    progressed = true;
                    sent += 1;
                }
            }
            if !progressed {
                return sent;
            }
        }
    }

    /// Perform a move, returning the number of cards moved. See
    /// [`GameState::do_move`] for the move rules; this wrapper additionally
    /// records an undo snapshot on success and clears the redo stack (a new
    /// move invalidates whatever was previously undone).
    pub fn do_move(&mut self, from: Loc, to: Loc) -> Result<usize, MoveError> {
        let snapshot = self.state.clone();
        let moved = self.state.do_move(from, to)?;
        self.past.push(snapshot);
        self.future.clear();
        Ok(moved)
    }
}

/// Every way the game state can change, expressed as plain data.
///
/// A finished game is fully described by its deal seed plus the sequence of
/// actions applied to it, which makes games serializable, replayable, and
/// (with the Store) time-travel debuggable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Start a numbered deal.
    Deal { seed: u32 },
    /// Move a card or run between locations.
    Move { from: Loc, to: Loc },
    /// Send every playable card to the foundations.
    AutoPlay,
    /// Step back one move.
    Undo,
    /// Step forward one move previously undone. Any other state-changing
    /// action clears what `Redo` would have replayed (issue #4).
    Redo,
    /// Re-deal the current numbered game from scratch.
    Restart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionError {
    /// The underlying move was illegal.
    Move(MoveError),
    /// Undo was dispatched with no moves to undo.
    NothingToUndo,
    /// Redo was dispatched with no undone move to replay.
    NothingToRedo,
    /// Restart was dispatched on a position with no deal number
    /// (e.g. one built with `Game::from_parts`).
    UnknownDeal,
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::Move(e) => e.fmt(f),
            ActionError::NothingToUndo => f.write_str("nothing to undo"),
            ActionError::NothingToRedo => f.write_str("nothing to redo"),
            ActionError::UnknownDeal => f.write_str("this position has no deal number to restart"),
        }
    }
}

impl From<MoveError> for ActionError {
    fn from(e: MoveError) -> ActionError {
        ActionError::Move(e)
    }
}

/// Pure reducer: compute the next state from the current state and an action.
///
/// Never mutates its input, never performs I/O. `AutoPlay` always succeeds
/// (sending zero cards home is not an error); every other action reports
/// failures without changing anything.
///
/// This is the immutable, by-reference API: callers keep a usable `game`
/// after the call (relied on by [`tests/reducer_tests.rs`]'s purity checks)
/// and get back an independent `Game`. Each call clones the *entire* input
/// `Game`, including its `past`/`future` stacks, so repeated dispatch
/// through `reduce` costs grow with moves already played. [`Store`] uses
/// [`reduce_in_place`] instead for that reason; prefer `reduce` for tests,
/// replay/comparison, or any call site that genuinely wants an immutable
/// transform.
pub fn reduce(game: &Game, action: Action) -> Result<Game, ActionError> {
    match action {
        Action::Deal { seed } => Ok(Game::deal(seed)),
        Action::Move { from, to } => {
            let mut next = game.clone();
            next.do_move(from, to)?;
            Ok(next)
        }
        Action::AutoPlay => {
            let mut next = game.clone();
            next.auto_play();
            Ok(next)
        }
        Action::Undo => {
            let mut next = game.clone();
            if next.undo() {
                Ok(next)
            } else {
                Err(ActionError::NothingToUndo)
            }
        }
        Action::Redo => {
            let mut next = game.clone();
            if next.redo() {
                Ok(next)
            } else {
                Err(ActionError::NothingToRedo)
            }
        }
        Action::Restart => match game.seed() {
            Some(seed) => Ok(Game::deal(seed)),
            None => Err(ActionError::UnknownDeal),
        },
    }
}

/// Efficient sibling of [`reduce`]: applies `action` to `game` in place via
/// `Game`'s own mutating methods (`do_move`, `undo`, `redo`, `auto_play`),
/// instead of cloning the whole `Game` (and its `past`/`future` stacks)
/// first. Same semantics as `reduce` for every action — same success/failure
/// results, and atomic on failure: `game` is left exactly as it was when
/// `Err` is returned.
///
/// Intended for [`Store::dispatch`], whose per-move cost must not scale with
/// how many moves have already been played. `reduce` itself is unaffected
/// and unchanged — issue #24 tracks fixing the dispatch cost, not the pure
/// reducer's contract.
pub fn reduce_in_place(game: &mut Game, action: Action) -> Result<(), ActionError> {
    match action {
        Action::Deal { seed } => {
            *game = Game::deal(seed);
            Ok(())
        }
        Action::Move { from, to } => {
            game.do_move(from, to)?;
            Ok(())
        }
        Action::AutoPlay => {
            game.auto_play();
            Ok(())
        }
        Action::Undo => {
            if game.undo() {
                Ok(())
            } else {
                Err(ActionError::NothingToUndo)
            }
        }
        Action::Redo => {
            if game.redo() {
                Ok(())
            } else {
                Err(ActionError::NothingToRedo)
            }
        }
        Action::Restart => match game.seed() {
            Some(seed) => {
                *game = Game::deal(seed);
                Ok(())
            }
            None => Err(ActionError::UnknownDeal),
        },
    }
}

/// Reconstruct a [`Game`] from a deal seed and the full sequence of actions
/// applied to it: `Game::deal(seed)` followed by dispatching every action in
/// `actions`, in order, through [`reduce_in_place`].
///
/// This is the replay contract issue #5 asks for: a finished (or in-progress)
/// game is fully described by `(seed, Vec<Action>)`. Because `Deal` and
/// `Restart` both reset to an absolute position rather than a relative delta,
/// they can appear anywhere in `actions` (e.g. a player redealing or backing
/// out to a fresh deal mid-session) and replay still reproduces the exact
/// final state — the whole session's action log is always a valid replay
/// from its original seed, with no need to reset or trim it.
///
/// Uses `reduce_in_place` rather than `reduce`, for the same reason `Store`
/// does (issue #24): `reduce` clones the entire `Game` — including its
/// growing `past`/`future` stacks — per action, which would make replaying a
/// long action log (e.g. the CLI's live on-win check) cost O(n²) instead of
/// linear.
///
/// Returns the first error encountered, if any action in the sequence was
/// illegal (this should not happen when replaying a log of actions that were
/// each already successfully dispatched once).
pub fn replay(seed: u32, actions: &[Action]) -> Result<Game, ActionError> {
    let mut game = Game::deal(seed);
    for &action in actions {
        reduce_in_place(&mut game, action)?;
    }
    Ok(game)
}
