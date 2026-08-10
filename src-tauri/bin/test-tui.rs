use crossterm::{
    event::{self, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TEST] Starting minimal TUI test...");

    eprintln!("[TEST] Enabling raw mode...");
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    eprintln!("[TEST] Entering alternate screen...");
    execute!(stdout, EnterAlternateScreen)?;

    eprintln!("[TEST] Creating terminal...");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    eprintln!("[TEST] Clearing terminal...");
    terminal.clear()?;

    eprintln!("[TEST] Drawing test frame...");
    terminal.draw(|f| {
        let area = f.area();

        // Draw a simple box
        let block = ratatui::widgets::Block::default()
            .title("TUI TEST - Press 'q' to quit")
            .borders(ratatui::widgets::Borders::ALL);

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Draw some text
        let text = vec![
            ratatui::text::Line::raw("✓ Terminal initialized successfully!"),
            ratatui::text::Line::raw("✓ Ratatui can render!"),
            ratatui::text::Line::raw("✓ Crossterm working!"),
            ratatui::text::Line::raw(""),
            ratatui::text::Line::raw("If you see this text, TUI rendering works!"),
            ratatui::text::Line::raw("Press 'q' to quit"),
        ];

        let paragraph = ratatui::widgets::Paragraph::new(text).alignment(Alignment::Center);

        f.render_widget(paragraph, inner);
    })?;

    eprintln!("[TEST] Frame drawn successfully!");

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }) = event::read()?
            {
                break;
            }
        }
    }

    eprintln!("[TEST] Restoring terminal...");
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    eprintln!("[TEST] Test complete!");
    Ok(())
}
