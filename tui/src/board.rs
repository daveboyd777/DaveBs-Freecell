//! Pure board layout and hit-testing (issue #6: "rendering must be a pure
//! function of GameState").
//!
//! This module holds the parts of the TUI that can be unit tested without a
//! real terminal: which color a card's suit should render as, and -- given
//! only the terminal area -- the clickable rectangle for every free cell,
//! the foundations, and every cascade column. The actual `ratatui::Frame`
//! draw calls in `main.rs` stay thin and read this module's output plus
//! `GameState`'s own card data directly.

use freecell::{Card, Loc};
use ratatui::layout::Rect;

/// Cascade columns, free cells: matches the engine's fixed board shape.
pub const CASCADE_COUNT: usize = 8;
pub const FREE_CELL_COUNT: usize = 4;

/// Which color a card should render in. Hearts/diamonds are red; clubs and
/// spades use the terminal's default foreground (typically light/white).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardColor {
    Red,
    Black,
}

/// The color a card's suit should render as -- a pure function of the card,
/// independent of terminal state.
pub fn card_color(card: Card) -> CardColor {
    if card.suit.is_red() {
        CardColor::Red
    } else {
        CardColor::Black
    }
}

/// The clickable rectangle for every free cell, the foundations (addressed
/// collectively, matching [`Loc::Foundation`]), and every cascade column,
/// computed once per frame from the board's terminal area.
#[derive(Debug, Clone, Copy)]
pub struct BoardLayout {
    pub free_cells: [Rect; FREE_CELL_COUNT],
    pub foundations: Rect,
    pub cascades: [Rect; CASCADE_COUNT],
}

/// Compute the clickable layout for `area` (the board region of the
/// terminal, i.e. excluding the status line and footer). Pure: the same
/// `area` always yields the same layout.
///
/// The top row (height 3, to fit a bordered one-line cell) splits into 8
/// equal columns: the first 4 are free cells, the last 4 together are the
/// foundations (one rect, since a move never targets a specific pile --
/// [`Loc::Foundation`] picks the pile from the card's suit). Everything
/// below that row is split into 8 equal cascade columns.
pub fn layout(area: Rect) -> BoardLayout {
    let col_width = (area.width / CASCADE_COUNT as u16).max(1);
    let top_height = area.height.min(3);

    let mut free_cells = [Rect::default(); FREE_CELL_COUNT];
    for (i, cell) in free_cells.iter_mut().enumerate() {
        *cell = Rect {
            x: area.x + i as u16 * col_width,
            y: area.y,
            width: col_width,
            height: top_height,
        };
    }

    let foundations = Rect {
        x: area.x + FREE_CELL_COUNT as u16 * col_width,
        y: area.y,
        width: col_width * FREE_CELL_COUNT as u16,
        height: top_height,
    };

    let mut cascades = [Rect::default(); CASCADE_COUNT];
    let cascade_y = area.y + top_height;
    let cascade_height = area.height.saturating_sub(top_height);
    for (i, cascade) in cascades.iter_mut().enumerate() {
        *cascade = Rect {
            x: area.x + i as u16 * col_width,
            y: cascade_y,
            width: col_width,
            height: cascade_height,
        };
    }

    BoardLayout {
        free_cells,
        foundations,
        cascades,
    }
}

fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

/// Map a terminal click at `(x, y)` to the [`Loc`] it falls within, or
/// `None` if the click missed every clickable region.
pub fn hit_test(layout: &BoardLayout, x: u16, y: u16) -> Option<Loc> {
    for (i, &rect) in layout.free_cells.iter().enumerate() {
        if contains(rect, x, y) {
            return Some(Loc::Free(i));
        }
    }
    if contains(layout.foundations, x, y) {
        return Some(Loc::Foundation);
    }
    for (i, &rect) in layout.cascades.iter().enumerate() {
        if contains(rect, x, y) {
            return Some(Loc::Cascade(i));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use freecell::Suit;

    fn area(x: u16, y: u16, width: u16, height: u16) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn card_color_is_red_for_hearts_and_diamonds() {
        assert_eq!(card_color(Card::new(1, Suit::Hearts)), CardColor::Red);
        assert_eq!(card_color(Card::new(1, Suit::Diamonds)), CardColor::Red);
    }

    #[test]
    fn card_color_is_black_for_clubs_and_spades() {
        assert_eq!(card_color(Card::new(1, Suit::Clubs)), CardColor::Black);
        assert_eq!(card_color(Card::new(1, Suit::Spades)), CardColor::Black);
    }

    #[test]
    fn layout_places_four_free_cells_then_foundations_across_the_top_row() {
        let l = layout(area(0, 0, 80, 20));
        assert_eq!(l.free_cells.len(), FREE_CELL_COUNT);
        // Free cells and foundations all start at the same row.
        for cell in l.free_cells {
            assert_eq!(cell.y, 0);
        }
        assert_eq!(l.foundations.y, 0);
        // Foundations sit to the right of the fourth free cell.
        assert_eq!(l.foundations.x, l.free_cells[3].x + l.free_cells[3].width);
    }

    #[test]
    fn layout_places_eight_cascades_below_the_top_row() {
        let l = layout(area(0, 0, 80, 20));
        assert_eq!(l.cascades.len(), CASCADE_COUNT);
        for cascade in l.cascades {
            assert!(cascade.y >= l.free_cells[0].y + l.free_cells[0].height);
        }
        // Cascades line up under their corresponding top-row column.
        assert_eq!(l.cascades[0].x, l.free_cells[0].x);
    }

    #[test]
    fn hit_test_finds_free_cell_b() {
        let l = layout(area(0, 0, 80, 20));
        let target = l.free_cells[1];
        let loc = hit_test(&l, target.x, target.y);
        assert_eq!(loc, Some(Loc::Free(1)));
    }

    #[test]
    fn hit_test_finds_foundations_regardless_of_which_quarter_is_clicked() {
        let l = layout(area(0, 0, 80, 20));
        let left_edge = hit_test(&l, l.foundations.x, l.foundations.y);
        let right_edge = hit_test(
            &l,
            l.foundations.x + l.foundations.width - 1,
            l.foundations.y,
        );
        assert_eq!(left_edge, Some(Loc::Foundation));
        assert_eq!(right_edge, Some(Loc::Foundation));
    }

    #[test]
    fn hit_test_finds_cascade_five() {
        let l = layout(area(0, 0, 80, 20));
        let target = l.cascades[4];
        let loc = hit_test(&l, target.x, target.y + 1);
        assert_eq!(loc, Some(Loc::Cascade(4)));
    }

    #[test]
    fn hit_test_returns_none_outside_every_region() {
        let l = layout(area(0, 0, 80, 20));
        assert_eq!(hit_test(&l, 1000, 1000), None);
    }

    #[test]
    fn layout_is_a_pure_function_of_area() {
        let a = layout(area(2, 3, 80, 24));
        let b = layout(area(2, 3, 80, 24));
        assert_eq!(a.free_cells, b.free_cells);
        assert_eq!(a.foundations, b.foundations);
        assert_eq!(a.cascades, b.cascades);
    }
}
