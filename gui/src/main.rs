//! egui/eframe desktop UI for DaveB's Freecell (issue #8).
//!
//! Shares the exact same `Store`/engine as the text CLI and the ratatui
//! TUI: no move rules live here. Board rendering reads `GameState` directly
//! every frame; the only UI-only state that feeds into it is
//! `FreecellApp::selected`, which drives legal-destination dimming and the
//! selected-run highlight the same way the TUI does (issue #7), by asking
//! [`freecell::GameState::can_move`] and
//! [`freecell::GameState::movable_run_len`] rather than reimplementing any
//! move rule.
//!
//! Mouse input is click-to-select-then-click-to-move, identical to the
//! TUI's mouse handling: click a location to select it, click a second
//! location to dispatch the move, click the same location again to
//! deselect. There is no textual move-command input in the GUI; a toolbar
//! of buttons covers undo/redo/auto-play/restart/new-game instead.

mod board;

use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Sense, Shape, Stroke, StrokeKind, pos2};
use freecell::{Action, GameState, Loc, Store, Suit, replay};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

fn random_seed() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1);
    nanos % 32000 + 1
}

/// Render a location the same way the CLI's/TUI's move grammar spells it,
/// for the action-log line (e.g. `Loc::Free(1)` -> "b").
fn loc_char(loc: Loc) -> char {
    match loc {
        Loc::Cascade(i) => (b'1' + i as u8) as char,
        Loc::Free(i) => (b'a' + i as u8) as char,
        Loc::Foundation => 'h',
    }
}

fn describe(action: Action) -> String {
    match action {
        Action::Deal { seed } => format!("Deal #{seed}"),
        Action::Move { from, to } => format!("Move {}{}", loc_char(from), loc_char(to)),
        Action::AutoPlay => "AutoPlay".to_string(),
        Action::Undo => "Undo".to_string(),
        Action::Redo => "Redo".to_string(),
        Action::Restart => "Restart".to_string(),
    }
}

/// Application state that is *not* part of [`freecell::GameState`]: the
/// running `Store`, the replay log, and purely presentational state
/// (selection, status message, the new-game seed text field).
struct FreecellApp {
    store: Store,
    original_seed: u32,
    log: Rc<RefCell<Vec<Action>>>,
    /// The replay-proof message, computed once and cached the moment a win
    /// is detected in `dispatch`, mirroring the TUI's caching (rather than
    /// recomputing -- and replaying the whole action log -- on every frame
    /// the win screen is up).
    replay_result: Option<String>,
    selected: Option<Loc>,
    status: Option<String>,
    seed_input: String,
}

impl FreecellApp {
    fn new(seed: u32) -> Self {
        let mut store = Store::new(seed);
        let log: Rc<RefCell<Vec<Action>>> = Rc::new(RefCell::new(Vec::new()));
        let log_for_subscriber = Rc::clone(&log);
        // Store-subscriber wiring (issue #3/#6's pattern, reused here):
        // every successfully dispatched action is recorded here,
        // independent of the board rendering (which reads `store.state()`
        // directly).
        store.subscribe(move |_state, action| {
            log_for_subscriber.borrow_mut().push(*action);
        });
        Self {
            store,
            original_seed: seed,
            log,
            replay_result: None,
            selected: None,
            status: None,
            seed_input: String::new(),
        }
    }

    fn dispatch(&mut self, action: Action) {
        match self.store.dispatch(action) {
            Ok(()) => self.status = None,
            Err(e) => self.status = Some(format!("Error: {e}")),
        }
        if self.store.state().is_won() {
            if self.replay_result.is_none() {
                let summary = replay_summary(self);
                self.replay_result = Some(summary);
            }
        } else {
            self.replay_result = None;
        }
    }

    /// Click-to-select-then-click-to-move handling, identical in spirit to
    /// the TUI's `handle_click` (issue #6/#7).
    fn handle_click(&mut self, loc: Loc) {
        match self.selected.take() {
            None => self.selected = Some(loc),
            Some(from) if from == loc => self.selected = None,
            Some(from) => self.dispatch(Action::Move { from, to: loc }),
        }
    }

    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Undo").clicked() {
                self.dispatch(Action::Undo);
            }
            if ui.button("Redo").clicked() {
                self.dispatch(Action::Redo);
            }
            if ui.button("Auto-Play").clicked() {
                let before = self.store.game().moves_played();
                self.dispatch(Action::AutoPlay);
                // `dispatch` already set `self.status` to an error message
                // if the store rejected the action; only overwrite it with
                // the count on an actual success (mirrors the TUI).
                if self.status.is_none() {
                    let sent = self.store.game().moves_played() - before;
                    self.status = Some(format!("Sent {sent} card(s) home."));
                }
            }
            if ui.button("Restart").clicked() {
                self.dispatch(Action::Restart);
            }
            ui.separator();
            ui.label("Seed:");
            ui.add(egui::TextEdit::singleline(&mut self.seed_input).desired_width(60.0));
            if ui.button("New Game").clicked() {
                let seed = self
                    .seed_input
                    .trim()
                    .parse::<u32>()
                    .unwrap_or_else(|_| random_seed());
                self.dispatch(Action::Deal { seed });
            }
        });
    }

    fn draw_status(&self, ui: &mut egui::Ui) {
        let seed = self.store.game().seed().unwrap_or(self.original_seed);
        let moves = self.store.game().moves_played();
        let mut text = format!("Game #{seed}   moves: {moves}");
        if self.store.state().is_won() {
            text.push_str("   *** WON ***");
        }
        ui.label(text);

        if let Some(status) = &self.status {
            ui.colored_label(Color32::from_rgb(200, 90, 0), status);
        }

        let recent: Vec<Action> = self
            .log
            .borrow()
            .iter()
            .rev()
            .take(3)
            .rev()
            .copied()
            .collect();
        if !recent.is_empty() {
            let log_line = recent
                .iter()
                .map(|&a| describe(a))
                .collect::<Vec<_>>()
                .join("  ");
            ui.label(format!("Log: {log_line}"));
        }

        if let Some(result) = &self.replay_result {
            ui.label(result);
        }
    }

    fn draw_board(&mut self, ui: &mut egui::Ui) {
        let area = ui.available_rect_before_wrap();
        let layout = board::layout(area);

        // One interactive region for the whole board; the exact clicked
        // `Loc` is resolved via `board::hit_test`, mirroring the TUI's
        // single hit-test-per-click approach (issue #6/#7) rather than one
        // widget per slot.
        let response = ui.interact(area, ui.id().with("gui-board"), Sense::click());
        let clicked = response
            .clicked()
            .then(|| response.interact_pointer_pos())
            .flatten()
            .and_then(|pos| board::hit_test(&layout, pos));

        let state = self.store.state();

        for (i, &rect) in layout.free_cells.iter().enumerate() {
            let loc = Loc::Free(i);
            let slot = slot_style(self.selected, state, loc);
            match state.freecells()[i] {
                Some(card) => draw_card(ui.painter(), rect, card, false),
                None => draw_empty_slot(ui.painter(), rect, slot == SlotStyle::Illegal),
            }
            draw_slot_overlay(ui.painter(), rect, slot);
        }

        // Foundations are addressed collectively (`Loc::Foundation` picks
        // the pile by suit), so every displayed pile shares one legality
        // classification, matching the TUI (issue #7).
        let foundation_slot = slot_style(self.selected, state, Loc::Foundation);
        for (i, &rect) in layout.foundations.iter().enumerate() {
            draw_foundation(ui.painter(), rect, i, state.foundations()[i]);
            draw_slot_overlay(ui.painter(), rect, foundation_slot);
        }

        for (i, &column) in layout.cascades.iter().enumerate() {
            let loc = Loc::Cascade(i);
            let slot = slot_style(self.selected, state, loc);
            let cards = &state.cascades()[i];
            let run_len = if self.selected == Some(loc) {
                state.movable_run_len(loc)
            } else {
                0
            };
            if cards.is_empty() {
                let slot_rect = board::card_rect_in_cascade(column, 0);
                draw_empty_slot(ui.painter(), slot_rect, slot == SlotStyle::Illegal);
            } else {
                for (idx, &card) in cards.iter().enumerate() {
                    let rect = board::card_rect_in_cascade(column, idx);
                    let highlighted = cards.len() - idx <= run_len;
                    draw_card(ui.painter(), rect, card, highlighted);
                }
            }
            let occupied = board::cascade_occupied_rect(column, cards.len());
            draw_slot_overlay(ui.painter(), occupied, slot);
        }

        if let Some(loc) = clicked {
            self.handle_click(loc);
        }
    }
}

/// How a board slot's overlay should render, given the current selection.
/// `Illegal` is only ever produced for a slot other than the selected one
/// (see `slot_style`) -- the selected slot is always `Selected`, never run
/// through the legality check.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotStyle {
    Selected,
    Illegal,
    Normal,
}

/// Classify a candidate destination `loc` relative to `selected`, using
/// [`freecell::GameState::can_move`] as the single source of truth for
/// legality (issue #7) -- this function never reimplements a move rule.
/// Always `Normal` when nothing is selected.
fn slot_style(selected: Option<Loc>, state: &GameState, loc: Loc) -> SlotStyle {
    match selected {
        Some(s) if s == loc => SlotStyle::Selected,
        Some(s) => {
            if state.can_move(s, loc).is_ok() {
                SlotStyle::Normal
            } else {
                SlotStyle::Illegal
            }
        }
        None => SlotStyle::Normal,
    }
}

fn draw_slot_overlay(painter: &egui::Painter, rect: egui::Rect, slot: SlotStyle) {
    match slot {
        SlotStyle::Selected => {
            painter.rect_stroke(
                rect,
                6.0,
                Stroke::new(3.0, Color32::from_rgb(255, 195, 0)),
                StrokeKind::Outside,
            );
        }
        SlotStyle::Illegal => {
            painter.rect_filled(rect, 6.0, Color32::from_black_alpha(120));
        }
        SlotStyle::Normal => {}
    }
}

fn draw_card(painter: &egui::Painter, rect: egui::Rect, card: freecell::Card, highlighted: bool) {
    let background = if highlighted {
        Color32::from_rgb(255, 244, 190)
    } else {
        Color32::WHITE
    };
    painter.rect_filled(rect, 6.0, background);
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, Color32::DARK_GRAY),
        StrokeKind::Inside,
    );
    let color = suit_color(card.suit);
    let rank = rank_label(card.rank);

    // Two-corner index (rank + a real suit pip) like an actual card, plus
    // a large center pip. The bottom-right corner isn't rotated 180
    // degrees -- `Painter::text` has no cheap glyph-flip -- but it still
    // reads as a card index rather than a flat two-character code.
    painter.text(
        rect.left_top() + egui::vec2(6.0, 4.0),
        Align2::LEFT_TOP,
        rank,
        FontId::proportional(16.0),
        color,
    );
    draw_pip(
        painter,
        rect.left_top() + egui::vec2(14.0, 32.0),
        7.0,
        card.suit,
        color,
    );

    painter.text(
        rect.right_bottom() - egui::vec2(6.0, 4.0),
        Align2::RIGHT_BOTTOM,
        rank,
        FontId::proportional(16.0),
        color,
    );
    draw_pip(
        painter,
        rect.right_bottom() - egui::vec2(14.0, 32.0),
        7.0,
        card.suit,
        color,
    );

    draw_pip(painter, rect.center(), 18.0, card.suit, color);
}

fn draw_empty_slot(painter: &egui::Painter, rect: egui::Rect, dimmed: bool) {
    let color = if dimmed {
        Color32::from_gray(90)
    } else {
        Color32::GRAY
    };
    painter.rect_stroke(rect, 6.0, Stroke::new(1.5, color), StrokeKind::Inside);
}

fn draw_foundation(painter: &egui::Painter, rect: egui::Rect, suit_index: usize, rank: u8) {
    painter.rect_filled(rect, 6.0, Color32::from_gray(235));
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, Color32::DARK_GRAY),
        StrokeKind::Inside,
    );
    let suit = suit_from_index(suit_index);
    if rank == 0 {
        // Empty pile: a dimmed outline-weight pip for the suit it will
        // eventually collect, no rank -- nothing has landed here yet.
        draw_pip(painter, rect.center(), 14.0, suit, Color32::from_gray(170));
        return;
    }
    let color = suit_color(suit);
    painter.text(
        rect.left_top() + egui::vec2(6.0, 4.0),
        Align2::LEFT_TOP,
        rank_label(rank),
        FontId::proportional(14.0),
        color,
    );
    draw_pip(painter, rect.center(), 16.0, suit, color);
}

/// Map a foundation pile index (`Suit as usize`, matching
/// [`freecell::GameState::foundations`]'s indexing) back to the `Suit` it
/// represents. Mirrors the CLI's/TUI's own hardcoded C/D/H/S ordering.
fn suit_from_index(i: usize) -> Suit {
    match i {
        0 => Suit::Clubs,
        1 => Suit::Diamonds,
        2 => Suit::Hearts,
        3 => Suit::Spades,
        _ => unreachable!("foundation index is always 0..4"),
    }
}

fn suit_color(suit: Suit) -> Color32 {
    if suit.is_red() {
        Color32::from_rgb(190, 20, 20)
    } else {
        Color32::BLACK
    }
}

/// The corner index label for a rank, spelling out "10" the way a real
/// card does rather than the CLI/TUI's single-character `T`.
fn rank_label(rank: u8) -> &'static str {
    const LABELS: [&str; 13] = [
        "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
    ];
    LABELS[(rank - 1) as usize]
}

/// Draw a real vector suit pip rather than a font glyph, so it renders
/// identically regardless of the active font's suit-symbol coverage. This
/// is the "full icon" upgrade from the plain two-character card code: a
/// genuine club/diamond/heart/spade shape built from circles and filled
/// triangles, centered at `center` and sized to fit within roughly a
/// `2*r`-wide box. No image assets or network fetch required.
fn draw_pip(painter: &egui::Painter, center: Pos2, r: f32, suit: Suit, color: Color32) {
    match suit {
        Suit::Diamonds => draw_diamond(painter, center, r, color),
        Suit::Hearts => draw_heart(painter, center, r, color),
        Suit::Spades => draw_spade(painter, center, r, color),
        Suit::Clubs => draw_club(painter, center, r, color),
    }
}

fn draw_diamond(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let points = vec![
        pos2(center.x, center.y - r),
        pos2(center.x + r * 0.7, center.y),
        pos2(center.x, center.y + r),
        pos2(center.x - r * 0.7, center.y),
    ];
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

fn draw_heart(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let lobe_r = r * 0.5;
    let lobe_y = center.y - lobe_r * 0.4;
    painter.circle_filled(pos2(center.x - lobe_r * 0.85, lobe_y), lobe_r, color);
    painter.circle_filled(pos2(center.x + lobe_r * 0.85, lobe_y), lobe_r, color);
    let points = vec![
        pos2(center.x - r * 0.95, lobe_y + lobe_r * 0.15),
        pos2(center.x + r * 0.95, lobe_y + lobe_r * 0.15),
        pos2(center.x, center.y + r),
    ];
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

fn draw_spade(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    // An upside-down heart plus a small stem: the classic spade shape.
    let lobe_r = r * 0.45;
    let lobe_y = center.y + r * 0.15;
    painter.circle_filled(pos2(center.x - lobe_r * 0.85, lobe_y), lobe_r, color);
    painter.circle_filled(pos2(center.x + lobe_r * 0.85, lobe_y), lobe_r, color);
    let points = vec![
        pos2(center.x - r * 0.85, lobe_y + lobe_r * 0.2),
        pos2(center.x + r * 0.85, lobe_y + lobe_r * 0.2),
        pos2(center.x, center.y - r),
    ];
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    let stem = egui::Rect::from_center_size(
        pos2(center.x, center.y + r * 0.8),
        egui::vec2(r * 0.22, r * 0.5),
    );
    painter.rect_filled(stem, 0.0, color);
}

fn draw_club(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let lobe_r = r * 0.42;
    painter.circle_filled(pos2(center.x, center.y - lobe_r * 0.9), lobe_r, color);
    painter.circle_filled(
        pos2(center.x - lobe_r * 0.95, center.y + lobe_r * 0.35),
        lobe_r,
        color,
    );
    painter.circle_filled(
        pos2(center.x + lobe_r * 0.95, center.y + lobe_r * 0.35),
        lobe_r,
        color,
    );
    let stem = egui::Rect::from_center_size(
        pos2(center.x, center.y + r * 0.8),
        egui::vec2(r * 0.2, r * 0.55),
    );
    painter.rect_filled(stem, 0.0, color);
}

/// The `(seed, actions)` replay proof issues #5/#6 ask for, adapted for the
/// GUI status area: replaying the action log from the original seed must
/// reproduce the exact current game.
fn replay_summary(app: &FreecellApp) -> String {
    let actions = app.log.borrow();
    match replay(app.original_seed, &actions) {
        Ok(rebuilt) if &rebuilt == app.store.game() => {
            "Replay verified: (seed, actions) reproduces this win.".to_string()
        }
        Ok(_) => "Replay produced a different game (this is a bug).".to_string(),
        Err(e) => format!("Replay failed: {e} (this is a bug)."),
    }
}

impl eframe::App for FreecellApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            self.draw_toolbar(ui);
            ui.separator();
            self.draw_status(ui);
            ui.separator();
            self.draw_board(ui);
        });
    }
}

fn main() -> eframe::Result<()> {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or_else(random_seed);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([880.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DaveB's Freecell",
        options,
        Box::new(move |_cc| Ok(Box::new(FreecellApp::new(seed)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_label_spells_out_ten_and_uses_single_letters_elsewhere() {
        assert_eq!(rank_label(1), "A");
        assert_eq!(rank_label(9), "9");
        assert_eq!(rank_label(10), "10");
        assert_eq!(rank_label(11), "J");
        assert_eq!(rank_label(13), "K");
    }

    #[test]
    fn suit_color_is_red_for_hearts_and_diamonds_only() {
        assert_eq!(suit_color(Suit::Hearts), Color32::from_rgb(190, 20, 20));
        assert_eq!(suit_color(Suit::Diamonds), Color32::from_rgb(190, 20, 20));
        assert_eq!(suit_color(Suit::Clubs), Color32::BLACK);
        assert_eq!(suit_color(Suit::Spades), Color32::BLACK);
    }

    #[test]
    fn suit_from_index_matches_the_foundations_array_ordering() {
        // Same C/D/H/S-by-index convention the CLI and TUI hardcode.
        assert_eq!(suit_from_index(0), Suit::Clubs);
        assert_eq!(suit_from_index(1), Suit::Diamonds);
        assert_eq!(suit_from_index(2), Suit::Hearts);
        assert_eq!(suit_from_index(3), Suit::Spades);
    }
}
