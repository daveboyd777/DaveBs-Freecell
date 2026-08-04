//! FreeCell game engine.
//!
//! Deals are compatible with the classic Microsoft FreeCell numbering, so
//! `Game::deal(1)` produces the same layout as "Game #1" in the original.

use std::fmt;

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

type Snapshot = ([Vec<Card>; 8], [Option<Card>; 4], [u8; 4]);

#[derive(Debug, Clone)]
pub struct Game {
    cascades: [Vec<Card>; 8],
    freecells: [Option<Card>; 4],
    /// Top rank per suit, indexed by `Suit as usize`; 0 means empty.
    foundations: [u8; 4],
    history: Vec<Snapshot>,
    /// The deal number, when this game came from `Game::deal`.
    seed: Option<u32>,
}

impl Game {
    /// Deal a numbered game using the classic Microsoft FreeCell algorithm.
    pub fn deal(seed: u32) -> Game {
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

        Game {
            cascades,
            freecells: [None; 4],
            foundations: [0; 4],
            history: Vec::new(),
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
            cascades,
            freecells,
            foundations,
            history: Vec::new(),
            seed: None,
        }
    }

    /// The deal number this game was dealt from, when known.
    pub fn seed(&self) -> Option<u32> {
        self.seed
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

    /// Undo the last successful move. Returns false when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some((cascades, freecells, foundations)) => {
                self.cascades = cascades;
                self.freecells = freecells;
                self.foundations = foundations;
                true
            }
            None => false,
        }
    }

    /// How many moves have been played (and can be undone).
    pub fn moves_played(&self) -> usize {
        self.history.len()
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

    /// Perform a move, returning the number of cards moved.
    ///
    /// Cascade-to-cascade moves transfer the longest ordered run that legally
    /// fits the destination, subject to the standard supermove capacity
    /// `(1 + empty free cells) * 2^(empty cascades)` — the destination column,
    /// if empty, does not count toward that doubling.
    pub fn do_move(&mut self, from: Loc, to: Loc) -> Result<usize, MoveError> {
        let snapshot = (self.cascades.clone(), self.freecells, self.foundations);
        let moved = self.try_move(from, to)?;
        self.history.push(snapshot);
        Ok(moved)
    }

    fn try_move(&mut self, from: Loc, to: Loc) -> Result<usize, MoveError> {
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

        // Longest ordered (descending, alternating-color) run at the tail.
        let mut run_len = 1;
        while run_len < source.len() {
            let upper = &source[source.len() - run_len - 1];
            let lower = &source[source.len() - run_len];
            if lower.stacks_on(upper) {
                run_len += 1;
            } else {
                break;
            }
        }

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

/// Every way the game state can change, expressed as plain data.
///
/// A finished game is fully described by its deal seed plus the sequence of
/// actions applied to it, which makes games serializable, replayable, and
/// (with the Phase 1 Store) time-travel debuggable.
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
    /// Re-deal the current numbered game from scratch.
    Restart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionError {
    /// The underlying move was illegal.
    Move(MoveError),
    /// Undo was dispatched with no moves to undo.
    NothingToUndo,
    /// Restart was dispatched on a position with no deal number
    /// (e.g. one built with `Game::from_parts`).
    UnknownDeal,
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::Move(e) => e.fmt(f),
            ActionError::NothingToUndo => f.write_str("nothing to undo"),
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
        Action::Restart => match game.seed() {
            Some(seed) => Ok(Game::deal(seed)),
            None => Err(ActionError::UnknownDeal),
        },
    }
}
