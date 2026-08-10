use crossterm::{
    event::{self, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dirs::data_dir;
use kunger_lib::persistence::{self, ScanRepository};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders};
use std::io;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match setup_terminal() {
        Ok(_) => {
            eprintln!("[SIMPLE] Terminal setup succeeded");
        }
        Err(e) => {
            eprintln!("[SIMPLE] ERROR in setup_terminal: {}", e);
            return Err(Box::new(e));
        }
    }

    let result = run_app();

    if let Err(e) = restore_terminal() {
        eprintln!("[SIMPLE] ERROR in restore_terminal: {}", e);
    }

    result
}

fn setup_terminal() -> io::Result<()> {
    eprintln!("[SIMPLE] Setting up terminal...");
    enable_raw_mode()?;
    eprintln!("[SIMPLE] Raw mode enabled");

    let mut stdout = io::stdout();
    match execute!(stdout, EnterAlternateScreen) {
        Ok(_) => eprintln!("[SIMPLE] Entered alternate screen"),
        Err(e) => eprintln!("[SIMPLE] WARNING: Could not enter alternate screen: {}", e),
    }

    eprintln!("[SIMPLE] Terminal setup complete");
    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, LeaveAlternateScreen);
    disable_raw_mode()?;
    Ok(())
}

fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[SIMPLE] Loading database...");
    let db_path = data_dir()
        .ok_or("Could not find data directory")?
        .join("kunger")
        .join("kunger.db");

    let conn = persistence::db::open(&db_path)?;
    let repository = Arc::new(persistence::SqliteScanRepository::new(conn));

    let all_items = repository.latest_items()?;
    eprintln!("[SIMPLE] Loaded {} items", all_items.len());

    if all_items.is_empty() {
        eprintln!("No software items found. Please run a scan first.");
        return Ok(());
    }

    eprintln!("[SIMPLE] Creating terminal...");
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;

    eprintln!("[SIMPLE] Entering render loop...");
    let mut page = 0;
    let page_size = 10;

    loop {
        eprintln!("[SIMPLE] Drawing frame for page {}...", page);

        terminal.draw(|f| {
            let items = &all_items
                [(page * page_size)..std::cmp::min((page + 1) * page_size, all_items.len())];

            // Simple vertical layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(10),
                    Constraint::Length(2),
                ])
                .split(f.area());

            // Header
            let header_text = format!("Kunger CLI - {} items total", all_items.len());
            let header = ratatui::widgets::Paragraph::new(header_text)
                .block(Block::default().borders(Borders::ALL).title("Header"))
                .alignment(Alignment::Center);
            f.render_widget(header, chunks[0]);

            // Items table
            let mut rows = vec![];
            for item in items {
                let row = format!(
                    "  {} | {:?} | {}",
                    item.display_name,
                    item.category,
                    item.version.as_ref().map(|v| v.as_str()).unwrap_or("—")
                );
                rows.push(ratatui::text::Line::raw(row));
            }

            let items_display = ratatui::widgets::Paragraph::new(rows).block(
                Block::default().borders(Borders::ALL).title(format!(
                    "Items (page {}/{}))",
                    page + 1,
                    (all_items.len() + page_size - 1) / page_size
                )),
            );
            f.render_widget(items_display, chunks[1]);

            // Footer
            let footer = ratatui::widgets::Paragraph::new(
                "Press 'n' for next, 'p' for previous, 'q' to quit",
            )
            .alignment(Alignment::Center);
            f.render_widget(footer, chunks[2]);
        })?;

        eprintln!("[SIMPLE] Frame drawn, waiting for input...");

        if event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('n') => {
                        page = (page + 1).min((all_items.len() + page_size - 1) / page_size - 1)
                    }
                    KeyCode::Char('p') => page = page.saturating_sub(1),
                    _ => {}
                }
            }
        }
    }

    eprintln!("[SIMPLE] Exiting...");
    Ok(())
}
