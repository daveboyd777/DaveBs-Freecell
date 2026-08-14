//! ratatui terminal UI for DaveB's Freecell (issue #6).
//!
//! Board rendering (`board` module plus this file's draw calls) is a pure
//! function of [`freecell::GameState`] -- no UI-only state (selection,
//! input buffer, messages) affects what a card looks like or where it sits.
//! Selection *highlighting* and legal-move dimming are issue #7, not this
//! file: clicking or typing a location here just tracks/dispatches moves.
//!
//! Keyboard input reuses the exact same command grammar as the text CLI
//! (`src/main.rs`): type a command, press Enter. Mouse input is additive:
//! click a location to select it as a move source, click a second location
//! to dispatch the move (or click the same location again to deselect).

mod board;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
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
    replay_shown: bool,
    selected: Option<Loc>,
    input: String,
    status: Option<String>,
    show_help: bool,
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
        Self {
            store,
            original_seed: seed,
            log,
            replay_shown: false,
            selected: None,
            input: String::new(),
            status: None,
            show_help: false,
        }
    }

    fn dispatch(&mut self, action: Action) {
        match self.store.dispatch(action) {
            Ok(()) => self.status = None,
            Err(e) => self.status = Some(format!("Error: {e}")),
        }
        if !self.store.state().is_won() {
            self.replay_shown = false;
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
                let sent = self.store.game().moves_played() - before;
                self.status = Some(format!("Sent {sent} card(s) home."));
                return;
            }
            "r" | "restart" => return self.dispatch(Action::Restart),
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

    /// Click-to-select-then-click-to-move mouse handling. No highlighting
    /// of the selection or legal destinations here -- that's issue #7.
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

    run(&mut terminal, &mut app)
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
                KeyCode::Char('q') if app.input.is_empty() => return Ok(()),
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

fn card_span(card: freecell::Card) -> Span<'static> {
    let style = match board::card_color(card) {
        board::CardColor::Red => Style::default().fg(Color::Red),
        board::CardColor::Black => Style::default().fg(Color::White),
    };
    Span::styled(format!("{card} "), style)
}

fn draw_board(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let layout = board::layout(area);
    let state = app.store.state();

    for (i, &rect) in layout.free_cells.iter().enumerate() {
        let selected = app.selected == Some(Loc::Free(i));
        let title = format!("{}", (b'a' + i as u8) as char);
        let content: Line = match state.freecells()[i] {
            Some(card) => Line::from(vec![card_span(card)]),
            None => Line::from("--"),
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style(selected));
        frame.render_widget(Paragraph::new(content).block(block), rect);
    }

    {
        let sub = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 4); 4])
            .split(layout.foundations);
        let selected = app.selected == Some(Loc::Foundation);
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
                .border_style(border_style(selected));
            frame.render_widget(Paragraph::new(content).block(block), rect);
        }
    }

    for (i, &rect) in layout.cascades.iter().enumerate() {
        let selected = app.selected == Some(Loc::Cascade(i));
        let lines: Vec<Line> = state.cascades()[i]
            .iter()
            .map(|&card| Line::from(vec![card_span(card)]))
            .collect();
        let block = Block::default()
            .title(format!("{}", i + 1))
            .borders(Borders::ALL)
            .border_style(border_style(selected));
        frame.render_widget(Paragraph::new(lines).block(block), rect);
    }
}

fn border_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
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
    if app.store.state().is_won() && !app.replay_shown {
        lines.push(Line::from(replay_summary(app)));
    }
    lines.push(Line::from("Press ? for help."));

    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL)),
        area,
    );
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
