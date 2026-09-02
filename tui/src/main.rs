//! ratatui terminal UI for DaveB's Freecell (issues #6 and #7).
//!
//! Board rendering (`board` module plus this file's draw calls) reads
//! [`freecell::GameState`] directly every frame; the only UI-only state
//! that feeds into it is `App::selected` (issue #7), which drives the
//! selected-run highlight and legal-destination dimming and nothing else --
//! the keyboard input buffer, status messages, and help visibility never
//! affect what a card looks like or where it sits.
//!
//! Legal-destination dimming and the selected-run highlight (issue #7) are
//! computed by asking [`freecell::GameState::can_move`] and
//! [`freecell::GameState::movable_run_len`] -- the engine's own move
//! validation -- rather than reimplementing any move rule here.
//!
//! Keyboard input reuses the exact same command grammar as the text CLI
//! (`src/main.rs`): type a command, press Enter. Mouse input is additive:
//! click a location to select it as a move source, click a second location
//! to dispatch the move (or click the same location again to deselect).

mod board;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use freecell::stats::{Stats, StatsRecorder};
use freecell::{Action, ActionError, Loc, Store, parse_move, replay};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::cell::RefCell;
use std::io::{self, Stdout};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Restores the terminal (raw mode off, mouse capture off, leave the
/// alternate screen) when dropped -- including on an early `?` return or a
/// panic unwind -- so a failure partway through setup or the render loop
/// can't leave the user's terminal stuck. Best-effort: errors from the
/// restore calls themselves are swallowed, since a `Drop` impl has nowhere
/// to report them.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
    }
}

fn random_seed() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1);
    nanos % 32000 + 1
}

/// Render a location the same way the CLI's move grammar spells it, for the
/// action-log panel (e.g. `Loc::Free(1)` -> "b").
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
/// (selection, keyboard input buffer, status messages, help visibility).
struct App {
    store: Store,
    original_seed: u32,
    log: Rc<RefCell<Vec<Action>>>,
    /// The replay-proof message, computed once and cached the moment a win
    /// is detected in `dispatch` (rather than recomputed by `draw_footer`
    /// every frame -- replaying a long action log on every draw would be
    /// wasteful once the win screen is up). `None` before a win, or after
    /// undo/restart/a new deal steps back out of one.
    replay_result: Option<String>,
    selected: Option<Loc>,
    input: String,
    status: Option<String>,
    show_help: bool,
    /// Store subscriber that records every finished game (issue #11),
    /// shared with the CLI and GUI via `freecell::stats::StatsRecorder`.
    /// Kept as a field (rather than only captured by the subscriber
    /// closure in `App::new`) so `finalize_stats` can call
    /// `StatsRecorder::finalize_on_exit` from the TUI's own shutdown
    /// paths -- quitting with 'q', Ctrl+C, or `run` otherwise returning.
    stats: Rc<RefCell<StatsRecorder>>,
}

impl App {
    fn new(seed: u32) -> Self {
        let mut store = Store::new(seed);
        let log: Rc<RefCell<Vec<Action>>> = Rc::new(RefCell::new(Vec::new()));
        let log_for_subscriber = Rc::clone(&log);
        // The Store-subscriber wiring the issue title asks for: every
        // successfully dispatched action is recorded here, independent of
        // the board rendering (which reads `store.state()` directly).
        store.subscribe(move |_state, action| {
            log_for_subscriber.borrow_mut().push(*action);
        });

        // Store subscriber that records every finished game to the OS data
        // directory (issue #11), shared with the CLI and GUI via
        // `freecell::stats::StatsRecorder` so all three contribute to the
        // same persisted history.
        let stats_path = freecell::stats::default_stats_path();
        let persisted = stats_path
            .as_deref()
            .map(Stats::load_or_default)
            .unwrap_or_default();
        let stats = Rc::new(RefCell::new(StatsRecorder::new(
            seed, persisted, stats_path,
        )));
        let stats_for_subscriber = Rc::clone(&stats);
        store.subscribe(move |state, action| {
            stats_for_subscriber.borrow_mut().observe(state, action);
        });

        Self {
            store,
            original_seed: seed,
            log,
            replay_result: None,
            selected: None,
            input: String::new(),
            status: None,
            show_help: false,
            stats,
        }
    }

    /// Record the in-progress game as a loss if it's a genuine, unfinished
    /// attempt (issue #11's quit-detection gap): call this from every one
    /// of the TUI's own shutdown paths. Idempotent, so it's safe to call
    /// from more than one of them for the same exit.
    fn finalize_stats(&self) {
        self.stats.borrow_mut().finalize_on_exit();
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

    /// Interpret one submitted command line, mirroring the text CLI's
    /// grammar exactly (`src/main.rs`): single-letter shortcuts, `n
    /// [seed]`, then the shared two-character move parser as a fallback.
    fn run_command(&mut self, line: &str) {
        let line = line.trim().to_lowercase();
        match line.as_str() {
            "" => return,
            "u" | "undo" => return self.dispatch(Action::Undo),
            "y" | "redo" => return self.dispatch(Action::Redo),
            "a" | "auto" => {
                let before = self.store.game().moves_played();
                self.dispatch(Action::AutoPlay);
                // `dispatch` already set `self.status` to an error message
                // if the store rejected the action (AutoPlay itself never
                // fails, but keep this defensive in case that changes); only
                // overwrite it with the count on an actual success.
                if self.status.is_none() {
                    let sent = self.store.game().moves_played() - before;
                    self.status = Some(format!("Sent {sent} card(s) home."));
                }
                return;
            }
            "r" | "restart" => return self.dispatch(Action::Restart),
            "h" | "hint" => {
                self.status = Some(match freecell::analysis::hint(self.store.state()) {
                    Some((from, to)) => format!("Hint: try {}{}", loc_char(from), loc_char(to)),
                    None => "No hint available right now.".to_string(),
                });
                return;
            }
            "g" | "report" => {
                let report = freecell::analysis::grade(self.store.game());
                self.status = Some(describe_report(&report));
                return;
            }
            _ => {}
        }

        if let Some(rest) = line.strip_prefix('n') {
            let seed = rest.trim().parse::<u32>().unwrap_or_else(|_| random_seed());
            return self.dispatch(Action::Deal { seed });
        }

        match parse_move(&line) {
            Some((from, to)) => self.dispatch(Action::Move { from, to }),
            None => self.status = Some(format!("Unrecognized command '{line}'.")),
        }
    }

    /// Click-to-select-then-click-to-move mouse handling. Highlighting the
    /// selection and dimming illegal destinations (issue #7) happens in
    /// `draw_board`, driven by the `selected` field this sets.
    fn handle_click(&mut self, board_area: Rect, x: u16, y: u16) {
        let layout = board::layout(board_area);
        let Some(loc) = board::hit_test(&layout, x, y) else {
            return;
        };
        match self.selected.take() {
            None => self.selected = Some(loc),
            Some(from) if from == loc => self.selected = None,
            Some(from) => self.dispatch(Action::Move { from, to: loc }),
        }
    }
}

fn main() -> io::Result<()> {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or_else(random_seed);
    let mut app = App::new(seed);

    enable_raw_mode()?;
    // Constructed immediately after raw mode is enabled, so it restores the
    // terminal even if the setup calls below fail.
    let _guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = run(&mut terminal, &mut app);
    // Covers any exit from `run` that isn't already one of the explicit
    // quit paths inside it (e.g. propagating a terminal I/O error via
    // `?`); idempotent with those, so this never double-records.
    app.finalize_stats();
    result
}

/// Regions of the terminal, recomputed fresh from the current terminal size
/// on every draw *and* on every mouse click, so the board's clickable
/// layout always matches what was last rendered without needing to persist
/// any layout state between frames.
fn regions(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(6),
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame.area(), frame, app))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') if app.input.is_empty() => {
                    app.finalize_stats();
                    return Ok(());
                }
                // Raw mode (`enable_raw_mode`, above) disables the
                // terminal's own Ctrl+C-to-SIGINT handling, so it arrives
                // here as a plain key event rather than terminating the
                // process -- handle it as an explicit quit rather than
                // (per the catch-all below) inserting a literal 'c'.
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.finalize_stats();
                    return Ok(());
                }
                KeyCode::Char('?') if app.input.is_empty() => app.show_help = !app.show_help,
                KeyCode::Enter => {
                    let line = std::mem::take(&mut app.input);
                    app.run_command(&line);
                }
                KeyCode::Esc => {
                    app.input.clear();
                    app.selected = None;
                }
                KeyCode::Backspace => {
                    app.input.pop();
                }
                KeyCode::Char(c) => app.input.push(c),
                _ => {}
            },
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                let size = terminal.size()?;
                let area = Rect {
                    x: 0,
                    y: 0,
                    width: size.width,
                    height: size.height,
                };
                let (_, board_area, _) = regions(area);
                app.handle_click(board_area, mouse.column, mouse.row);
            }
            _ => {}
        }
    }
}

fn draw(area: Rect, frame: &mut ratatui::Frame, app: &App) {
    let (status_area, board_area, footer_area) = regions(area);
    draw_status(frame, status_area, app);
    draw_board(frame, board_area, app);
    draw_footer(frame, footer_area, app);
}

fn draw_status(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let seed = app.store.game().seed().unwrap_or(app.original_seed);
    let moves = app.store.game().moves_played();
    let won = if app.store.state().is_won() {
        "  *** WON ***"
    } else {
        ""
    };
    let text = format!("DaveB's Freecell -- Game #{seed}   moves: {moves}{won}");
    frame.render_widget(Paragraph::new(text), area);
}

fn card_span(card: freecell::Card, highlighted: bool) -> Span<'static> {
    let mut style = match board::card_color(card) {
        board::CardColor::Red => Style::default().fg(Color::Red),
        // Leave the foreground unset so the terminal's own default applies,
        // matching `CardColor::Black`'s documented intent -- hard-coding
        // white here would be unreadable on a light-background terminal.
        board::CardColor::Black => Style::default(),
    };
    if highlighted {
        // Invert fg/bg rather than introducing a new color, so the
        // selected run reads clearly under any terminal color theme.
        style = style.add_modifier(Modifier::REVERSED);
    }
    Span::styled(format!("{card} "), style)
}

/// How a board slot's border should render, given the current selection.
/// `Illegal` is only ever produced for a slot other than the selected one
/// (see `slot_style`) -- the selected slot is always `Selected`, never run
/// through the legality check.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotStyle {
    Selected,
    Illegal,
    Normal,
}

/// Classify a candidate destination `loc` relative to `app.selected`, using
/// [`freecell::GameState::can_move`] as the single source of truth for
/// legality (issue #7) -- this function never reimplements a move rule.
/// Always `Normal` when nothing is selected.
fn slot_style(app: &App, loc: Loc) -> SlotStyle {
    match app.selected {
        Some(selected) if selected == loc => SlotStyle::Selected,
        Some(selected) => {
            if app.store.state().can_move(selected, loc).is_ok() {
                SlotStyle::Normal
            } else {
                SlotStyle::Illegal
            }
        }
        None => SlotStyle::Normal,
    }
}

fn draw_board(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let layout = board::layout(area);
    let state = app.store.state();

    for (i, &rect) in layout.free_cells.iter().enumerate() {
        let slot = slot_style(app, Loc::Free(i));
        let title = format!("{}", (b'a' + i as u8) as char);
        let content: Line = match state.freecells()[i] {
            Some(card) => Line::from(vec![card_span(card, false)]),
            None => Line::from("--"),
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style(slot));
        frame.render_widget(Paragraph::new(content).block(block), rect);
    }

    {
        let sub = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 4); 4])
            .split(layout.foundations);
        // Foundations are addressed collectively (`Loc::Foundation` alone
        // picks the pile by suit), so every displayed pile shares one
        // legality classification rather than four independent ones.
        let slot = slot_style(app, Loc::Foundation);
        for (i, &rect) in sub.iter().enumerate() {
            let rank = state.foundations()[i];
            let suit_char = ['C', 'D', 'H', 'S'][i];
            let content = if rank == 0 {
                format!("{suit_char}-")
            } else {
                format!("{suit_char}{}", rank_char(rank))
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style(slot));
            frame.render_widget(Paragraph::new(content).block(block), rect);
        }
    }

    for (i, &rect) in layout.cascades.iter().enumerate() {
        let loc = Loc::Cascade(i);
        let slot = slot_style(app, loc);
        // Only the selected cascade highlights its movable tail run; every
        // other cascade highlights nothing.
        let run_len = if app.selected == Some(loc) {
            state.movable_run_len(loc)
        } else {
            0
        };
        let lines = cascade_lines(&state.cascades()[i], rect, run_len);
        let block = Block::default()
            .title(format!("{}", i + 1))
            .borders(Borders::ALL)
            .border_style(border_style(slot));
        frame.render_widget(Paragraph::new(lines).block(block), rect);
    }
}

/// Render a cascade's cards for a `height`-row bordered cell, truncating
/// from the top (with a "+N more" indicator) rather than the bottom when it
/// doesn't fit: the bottom (frontmost) card is the only one that can ever be
/// moved, so it must stay visible even when the column is deep.
///
/// `run_len` (0 unless this cascade is the current selection) highlights
/// the trailing `run_len` cards -- the movable run [`freecell::GameState::
/// movable_run_len`] reports for this column. Since truncation always keeps
/// the tail visible, the highlighted run stays visible together with any
/// "+N more" indicator (a run deeper than the visible area is the same
/// pre-existing edge case truncation already accepts).
fn cascade_lines(cascade: &[freecell::Card], rect: Rect, run_len: usize) -> Vec<Line<'static>> {
    let available_rows = rect.height.saturating_sub(2) as usize; // minus borders
    if cascade.is_empty() || available_rows == 0 {
        return Vec::new();
    }
    // A card at absolute index `i` (0-based from the top of the column) is
    // part of the highlighted run when it's within the last `run_len` cards.
    let is_highlighted = |i: usize| cascade.len() - i <= run_len;
    if cascade.len() <= available_rows {
        return cascade
            .iter()
            .enumerate()
            .map(|(i, &card)| Line::from(vec![card_span(card, is_highlighted(i))]))
            .collect();
    }
    if available_rows == 1 {
        // No room for both an indicator and a card: prioritize the playable
        // bottom card.
        let i = cascade.len() - 1;
        return vec![Line::from(vec![card_span(cascade[i], is_highlighted(i))])];
    }
    let visible = available_rows - 1;
    let hidden = cascade.len() - visible;
    let mut lines = vec![Line::from(format!("+{hidden} more"))];
    lines.extend(
        cascade[cascade.len() - visible..]
            .iter()
            .enumerate()
            .map(|(offset, &card)| {
                let i = hidden + offset;
                Line::from(vec![card_span(card, is_highlighted(i))])
            }),
    );
    lines
}

fn border_style(slot: SlotStyle) -> Style {
    match slot {
        SlotStyle::Selected => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        SlotStyle::Illegal => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
        SlotStyle::Normal => Style::default(),
    }
}

fn rank_char(rank: u8) -> char {
    const RANK_CHARS: [char; 14] = [
        '-', 'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K',
    ];
    RANK_CHARS[rank as usize]
}

fn draw_footer(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    if app.show_help {
        let text = "Moves: two chars, source then destination -- 1-8 cascades, a-d free cells, h/f foundations (e.g. 35, 1a, 2h).\n\
             Commands: a auto-play, u undo, y redo, r restart, n [seed] new game, q quit.\n\
             \x20         h hint (may take a moment), g report (may take a moment).\n\
             Mouse: click a location to select it, click again to move there (or the same spot to deselect).\n\
             Press ? to close help.";
        frame.render_widget(
            Paragraph::new(text).block(Block::default().title("Help").borders(Borders::ALL)),
            area,
        );
        return;
    }

    let mut lines = vec![Line::from(format!("> {}", app.input))];
    if let Some(status) = &app.status {
        lines.push(Line::from(status.clone()));
    }
    let recent: Vec<Action> = app
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
        lines.push(Line::from(format!("Log: {log_line}")));
    }
    if let Some(result) = &app.replay_result {
        lines.push(Line::from(result.clone()));
    }
    lines.push(Line::from("Press ? for help."));

    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

/// Summarize a `freecell::analysis::GameReport` (issue #13) into one
/// compact status line: moves played vs. the solver's best line from the
/// original deal, where a losing attempt went wrong (if anywhere), and
/// which foundations stalled.
fn describe_report(report: &freecell::analysis::GameReport) -> String {
    use freecell::solver::Solvability;
    let best_line = match &report.best_line {
        Solvability::Solvable(moves) => format!("best line {}", moves.len()),
        Solvability::Unsolvable => "never winnable".to_string(),
        Solvability::Unknown => "best line unknown".to_string(),
    };
    let went_wrong = match report.first_unsolvable_move {
        Some(0) => "unwinnable from the start".to_string(),
        Some(i) => format!("went wrong at move {i}"),
        None => "still winnable".to_string(),
    };
    const SUIT_CHARS: [char; 4] = ['C', 'D', 'H', 'S'];
    let foundations: Vec<String> = report
        .foundations
        .iter()
        .enumerate()
        .map(|(i, &r)| format!("{}{}", SUIT_CHARS[i], rank_char(r)))
        .collect();
    format!(
        "{} moves | {best_line} | {went_wrong} | {}",
        report.moves_played,
        foundations.join(" ")
    )
}

/// The `(seed, actions)` replay proof issue #5 asks for, adapted for the
/// TUI footer: replaying the action log from the original seed must
/// reproduce the exact current game.
fn replay_summary(app: &App) -> String {
    let actions = app.log.borrow();
    match replay(app.original_seed, &actions) {
        Ok(rebuilt) if &rebuilt == app.store.game() => {
            "Replay verified: (seed, actions) reproduces this win.".to_string()
        }
        Ok(_) => "Replay produced a different game (this is a bug).".to_string(),
        Err(ActionError::Move(e)) => format!("Replay failed: {e} (this is a bug)."),
        Err(e) => format!("Replay failed: {e} (this is a bug)."),
    }
}
