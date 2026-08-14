//! Scaffolding for issue #6 (a future ratatui terminal UI driven by
//! [`freecell::Store`]).
//!
//! This is a minimal placeholder proving the workspace wiring -- a real
//! `Store`, a real ratatui/crossterm render loop -- not the actual card
//! rendering, move input, or `Store::subscribe` wiring, which land in #6/#7.

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use freecell::Store;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Alignment,
    widgets::{Block, Borders, Paragraph},
};
use std::io::{self, Stdout};
use std::time::Duration;

/// Restores the terminal (raw mode off, leave the alternate screen) when
/// dropped -- including on an early `?` return or a panic unwind -- so a
/// failure partway through setup or the render loop can't leave the user's
/// terminal stuck in raw mode / the alternate screen. Best-effort: errors
/// from the restore calls themselves are swallowed, since a `Drop` impl has
/// nowhere to report them.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn main() -> io::Result<()> {
    let store = Store::new(617);

    enable_raw_mode()?;
    // Constructed immediately after raw mode is enabled, so it restores the
    // terminal even if EnterAlternateScreen or Terminal::new fails below.
    let _guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    run(&mut terminal, &store)
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, store: &Store) -> io::Result<()> {
    loop {
        terminal.draw(|frame| {
            let text = format!(
                "FreeCell TUI scaffold (issue #6)\n\nDeal #{}   moves: {}\n\nPress q to quit.",
                store.game().seed().unwrap_or(0),
                store.game().moves_played()
            );
            let block = Block::default()
                .title("DaveB's Freecell")
                .borders(Borders::ALL);
            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, frame.area());
        })?;

        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && key.code == KeyCode::Char('q')
        {
            return Ok(());
        }
    }
}
