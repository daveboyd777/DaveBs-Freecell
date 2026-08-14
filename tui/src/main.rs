//! Scaffolding for issue #6 (ratatui terminal UI as a Store subscriber).
//!
//! This is a minimal placeholder proving the workspace wiring -- a real
//! `Store`, a real ratatui/crossterm render loop -- not the actual card
//! rendering or move input, which land in #6/#7.

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

fn main() -> io::Result<()> {
    let store = Store::new(617);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = run(&mut terminal, &store);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    result
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
