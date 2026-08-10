use std::io;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor::Show,
    event::{self},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kunger_lib::inventory::InventoryService;
use kunger_lib::persistence::{self, ScanRepository, SqliteScanRepository};
use kunger_lib::tui::handlers::{Action, InputHandler};
use kunger_lib::tui::{App, UI};
use ratatui::prelude::*;
use tokio_util::sync::CancellationToken;

const SCAN_TIMEOUT: Duration = Duration::from_secs(30);

enum ScanEvent {
    Completed,
    Cancelled,
    Failed(String),
}

struct TerminalSession {
    alternate_screen: bool,
}

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;

        let mut session = Self {
            alternate_screen: false,
        };
        let mut stdout = io::stdout();
        match execute!(stdout, EnterAlternateScreen) {
            Ok(()) => session.alternate_screen = true,
            Err(error) => eprintln!(
                "Warning: alternate screen is unavailable; rendering in the current terminal: {error}"
            ),
        }

        Ok(session)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = restore_terminal_output(&mut stdout, self.alternate_screen);
        let _ = disable_raw_mode();
    }
}

fn restore_terminal_output(writer: &mut impl io::Write, alternate_screen: bool) -> io::Result<()> {
    if alternate_screen {
        execute!(writer, LeaveAlternateScreen)?;
    }
    execute!(writer, Show)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = persistence::db::default_database_path()?;
    let conn = persistence::db::open(&db_path)?;
    let repository = Arc::new(SqliteScanRepository::new(conn));
    let items = repository.latest_items()?;

    let session = TerminalSession::enter()?;
    let result = run_app(items, repository);
    drop(session);
    result
}

fn run_app(
    items: Vec<kunger_lib::domain::SoftwareItem>,
    repository: Arc<SqliteScanRepository>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(items);
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;

    let (scan_sender, scan_receiver) = mpsc::channel();
    let mut cancellation: Option<CancellationToken> = None;

    loop {
        while let Ok(scan_event) = scan_receiver.try_recv() {
            cancellation = None;
            match scan_event {
                ScanEvent::Completed => match repository.latest_items() {
                    Ok(items) => {
                        let count = items.len();
                        app.replace_items(items);
                        app.complete_scan(format!("Scan complete: {count} items found."));
                    }
                    Err(error) => {
                        app.fail_scan(format!("Scan completed, but reload failed: {error}"))
                    }
                },
                ScanEvent::Cancelled => app.fail_scan("Scan cancelled."),
                ScanEvent::Failed(error) => app.fail_scan(format!("Scan failed: {error}")),
            }
        }

        terminal.draw(|frame| UI::render(frame, &app))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let event = event::read()?;
        let action = InputHandler::handle_event(&mut app, event);
        match action {
            Action::StartScan => {
                if cancellation.is_none() {
                    app.start_scan();
                    cancellation = Some(start_scan(Arc::clone(&repository), scan_sender.clone()));
                }
            }
            Action::CancelScan => {
                if let Some(token) = &cancellation {
                    token.cancel();
                    app.set_scan_message("Cancelling scan...");
                }
            }
            Action::Quit => {
                if let Some(token) = &cancellation {
                    token.cancel();
                }
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn start_scan(
    repository: Arc<SqliteScanRepository>,
    sender: mpsc::Sender<ScanEvent>,
) -> CancellationToken {
    let cancellation = CancellationToken::new();
    let scan_cancellation = cancellation.clone();

    thread::spawn(move || {
        let outcome = match run_scan(repository, scan_cancellation.clone()) {
            Ok(()) if scan_cancellation.is_cancelled() => ScanEvent::Cancelled,
            Ok(()) => ScanEvent::Completed,
            Err(error) => ScanEvent::Failed(error.to_string()),
        };
        let _ = sender.send(outcome);
    });

    cancellation
}

fn run_scan(
    repository: Arc<SqliteScanRepository>,
    cancellation: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let inventory = InventoryService::with_default_providers();
    let result = runtime.block_on(inventory.scan(SCAN_TIMEOUT, cancellation.clone()));

    if cancellation.is_cancelled() {
        return Ok(());
    }

    repository.save_scan(&result)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_cleanup_only_leaves_an_entered_alternate_screen() {
        let mut current_screen = Vec::new();
        restore_terminal_output(&mut current_screen, false).expect("restore cursor");
        assert_eq!(current_screen, b"\x1b[?25h");

        let mut alternate_screen = Vec::new();
        restore_terminal_output(&mut alternate_screen, true).expect("restore terminal");
        assert_eq!(alternate_screen, b"\x1b[?1049l\x1b[?25h");
    }
}
