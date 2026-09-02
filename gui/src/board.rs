//! Pure layout math for the egui/eframe desktop GUI (issue #8), mirroring
//! `tui/src/board.rs`'s separation of concerns: this module knows nothing
//! about `GameState` or move legality, only how to turn the available
//! drawing area into card-sized rectangles and answer hit-tests against
//! them. `main.rs`'s draw code stays thin and reads `GameState` directly.

use egui::{Rect, Vec2, pos2, vec2};
use freecell::Loc;

pub const CASCADE_COUNT: usize = 8;
pub const FREE_CELL_COUNT: usize = 4;
pub const FOUNDATION_COUNT: usize = 4;

/// Logical size of one card, in egui points.
pub const CARD_SIZE: Vec2 = vec2(70.0, 96.0);
/// Horizontal gap between adjacent cells/columns.
pub const CELL_SPACING: f32 = 10.0;
/// Extra horizontal gap between the free cells and the foundations.
pub const GROUP_GAP: f32 = 24.0;
/// Vertical offset between successive overlapping cards in a cascade.
pub const CASCADE_OVERLAP: f32 = 28.0;
/// A generous cap on how many overlapping cards a cascade's *clickable*
/// region accounts for. A real cascade deeper than this still renders and
/// draws every card; only the hit-test region stops growing past this
/// depth -- an accepted edge case in the same spirit as the TUI's cascade
/// truncation (issue #6).
pub const CASCADE_VISUAL_CAPACITY: usize = 24;

/// The clickable/drawable rectangles for one frame's board area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardLayout {
    pub free_cells: [Rect; FREE_CELL_COUNT],
    pub foundations: [Rect; FOUNDATION_COUNT],
    /// Each cascade's full clickable column, from its top-card slot down to
    /// `CASCADE_VISUAL_CAPACITY` overlapped cards. `card_rect_in_cascade`
    /// and `cascade_occupied_rect` derive individual card/occupied-area
    /// rects from this.
    pub cascades: [Rect; CASCADE_COUNT],
}

/// Lay out the board within `available` (using its top-left corner as the
/// origin, growing right and down). Pure: the same `available` always
/// yields the same layout.
pub fn layout(available: Rect) -> BoardLayout {
    let origin = available.min;
    let step = CARD_SIZE.x + CELL_SPACING;

    let free_cells = std::array::from_fn(|i| {
        Rect::from_min_size(pos2(origin.x + i as f32 * step, origin.y), CARD_SIZE)
    });

    let foundation_x0 = origin.x + FREE_CELL_COUNT as f32 * step + GROUP_GAP;
    let foundations = std::array::from_fn(|i| {
        Rect::from_min_size(pos2(foundation_x0 + i as f32 * step, origin.y), CARD_SIZE)
    });

    let cascade_y = origin.y + CARD_SIZE.y + CELL_SPACING * 2.0;
    let cascade_height = CARD_SIZE.y + CASCADE_OVERLAP * (CASCADE_VISUAL_CAPACITY - 1) as f32;
    let cascades = std::array::from_fn(|i| {
        Rect::from_min_size(
            pos2(origin.x + i as f32 * step, cascade_y),
            vec2(CARD_SIZE.x, cascade_height),
        )
    });

    BoardLayout {
        free_cells,
        foundations,
        cascades,
    }
}

/// The rectangle for the `index`-th (0-based, top of column first) card in
/// a cascade, given that cascade's full column rect from [`BoardLayout`].
pub fn card_rect_in_cascade(column: Rect, index: usize) -> Rect {
    let y = column.min.y + index as f32 * CASCADE_OVERLAP;
    Rect::from_min_size(pos2(column.min.x, y), CARD_SIZE)
}

/// The bounding rect of the first `count` cards in a cascade, or just the
/// top card's empty-slot rect when `count == 0`. Used to draw a dimmed
/// overlay or selection outline over exactly the occupied area, rather than
/// the column's full click-target capacity.
pub fn cascade_occupied_rect(column: Rect, count: usize) -> Rect {
    if count == 0 {
        return Rect::from_min_size(column.min, CARD_SIZE);
    }
    let last = card_rect_in_cascade(column, count - 1);
    Rect::from_min_size(column.min, vec2(CARD_SIZE.x, last.max.y - column.min.y))
}

/// Map a click position to the [`Loc`] it falls within, or `None` if it
/// missed every clickable region. Free cells and foundations are checked
/// before cascades so they win if a layout ever made regions overlap.
pub fn hit_test(layout: &BoardLayout, pos: egui::Pos2) -> Option<Loc> {
    for (i, &rect) in layout.free_cells.iter().enumerate() {
        if rect.contains(pos) {
            return Some(Loc::Free(i));
        }
    }
    for &rect in &layout.foundations {
        if rect.contains(pos) {
            return Some(Loc::Foundation);
        }
    }
    for (i, &rect) in layout.cascades.iter().enumerate() {
        if rect.contains(pos) {
            return Some(Loc::Cascade(i));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2(x, y), vec2(w, h))
    }

    #[test]
    fn layout_places_free_cells_then_foundations_with_a_gap() {
        let l = layout(area(0.0, 0.0, 900.0, 700.0));
        for pair in l.free_cells.windows(2) {
            assert!(pair[0].min.x < pair[1].min.x);
        }
        assert!(
            l.foundations[0].min.x > l.free_cells[3].max.x,
            "foundations must start strictly right of the last free cell, with a gap"
        );
        for cell in l.free_cells.into_iter().chain(l.foundations) {
            assert_eq!(cell.min.y, l.free_cells[0].min.y);
        }
    }

    #[test]
    fn layout_places_cascades_below_the_top_row() {
        let l = layout(area(0.0, 0.0, 900.0, 700.0));
        for cascade in l.cascades {
            assert!(cascade.min.y >= l.free_cells[0].max.y);
        }
        // Cascades line up under their corresponding top-row column.
        assert_eq!(l.cascades[0].min.x, l.free_cells[0].min.x);
        for pair in l.cascades.windows(2) {
            assert!(pair[0].min.x < pair[1].min.x);
        }
    }

    #[test]
    fn hit_test_finds_free_cell_b() {
        let l = layout(area(0.0, 0.0, 900.0, 700.0));
        let target = l.free_cells[1];
        assert_eq!(hit_test(&l, target.center()), Some(Loc::Free(1)));
    }

    #[test]
    fn hit_test_finds_a_foundation_pile() {
        let l = layout(area(0.0, 0.0, 900.0, 700.0));
        let target = l.foundations[2];
        assert_eq!(hit_test(&l, target.center()), Some(Loc::Foundation));
    }

    #[test]
    fn hit_test_finds_cascade_five() {
        let l = layout(area(0.0, 0.0, 900.0, 700.0));
        let target = l.cascades[4];
        assert_eq!(hit_test(&l, target.center()), Some(Loc::Cascade(4)));
    }

    #[test]
    fn hit_test_returns_none_outside_every_region() {
        let l = layout(area(0.0, 0.0, 900.0, 700.0));
        assert_eq!(hit_test(&l, pos2(-10.0, -10.0)), None);
        assert_eq!(hit_test(&l, pos2(100_000.0, 100_000.0)), None);
    }

    #[test]
    fn card_rect_in_cascade_offsets_vertically_and_keeps_the_column_x() {
        let l = layout(area(0.0, 0.0, 900.0, 700.0));
        let column = l.cascades[0];
        let first = card_rect_in_cascade(column, 0);
        let third = card_rect_in_cascade(column, 2);
        assert_eq!(first.min, column.min);
        assert_eq!(third.min.x, column.min.x);
        assert_eq!(third.min.y, column.min.y + 2.0 * CASCADE_OVERLAP);
    }

    #[test]
    fn cascade_occupied_rect_matches_the_top_slot_when_empty() {
        let l = layout(area(0.0, 0.0, 900.0, 700.0));
        let column = l.cascades[0];
        let occupied = cascade_occupied_rect(column, 0);
        assert_eq!(occupied, Rect::from_min_size(column.min, CARD_SIZE));
    }

    #[test]
    fn cascade_occupied_rect_ends_at_the_last_card_when_nonempty() {
        let l = layout(area(0.0, 0.0, 900.0, 700.0));
        let column = l.cascades[0];
        let occupied = cascade_occupied_rect(column, 3);
        let last = card_rect_in_cascade(column, 2);
        assert_eq!(occupied.min, column.min);
        assert_eq!(occupied.max.y, last.max.y);
    }

    #[test]
    fn layout_is_a_pure_function_of_available() {
        let a = layout(area(3.0, 5.0, 900.0, 700.0));
        let b = layout(area(3.0, 5.0, 900.0, 700.0));
        assert_eq!(a, b);
    }
}
