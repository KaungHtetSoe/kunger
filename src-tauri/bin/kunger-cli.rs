use std::io;
use std::sync::Arc;
use crossterm::{
    event::{self, EnableMouseCapture},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::prelude::*;
use kunger_lib::persistence::{self, ScanRepository};
use kunger_lib::tui::{App, UI};
use kunger_lib::tui::handlers::{InputHandler, Action};
use dirs::data_dir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_terminal()?;
    let result = run_app();
    restore_terminal()?;
    result
}

fn setup_terminal() -> io::Result<()> {
    eprintln!("[DEBUG] Setting up terminal...");
    eprintln!("[DEBUG] TERM={}", std::env::var("TERM").unwrap_or_else(|_| "UNSET".to_string()));

    enable_raw_mode()?;
    eprintln!("[DEBUG] Raw mode enabled");

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    eprintln!("[DEBUG] Alternate screen entered, mouse capture enabled");

    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen)?;
    Ok(())
}

fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = data_dir()
        .ok_or("Could not find data directory")?
        .join("kunger")
        .join("kunger.db");

    let conn = persistence::db::open(&db_path)?;
    let repository = Arc::new(persistence::SqliteScanRepository::new(conn));

    let all_items = repository.latest_items()?;
    eprintln!("[DEBUG] Loaded {} items from database", all_items.len());

    if all_items.is_empty() {
        eprintln!("No software items found. Please run a scan first.");
        return Ok(());
    }

    let mut app = App::new(all_items);
    eprintln!("[DEBUG] App created");

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    eprintln!("[DEBUG] Terminal created");

    terminal.clear()?;
    eprintln!("[DEBUG] Terminal cleared");

    loop {
        eprintln!("[DEBUG] Drawing frame...");
        terminal.draw(|f| UI::render(f, &app))?;
        eprintln!("[DEBUG] Frame drawn, polling for input...");

        if event::poll(std::time::Duration::from_millis(100))? {
            let evt = event::read()?;
            eprintln!("[DEBUG] Got event: {:?}", evt);
            let action = InputHandler::handle_event(&mut app, evt);
            if action == Action::Quit {
                break;
            }
        }
    }

    Ok(())
}
