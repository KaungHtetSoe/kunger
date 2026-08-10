// `unwrap`/`expect` are worth flagging in production provider/parsing code
// (see docs/SECURITY.md — untrusted input must never panic the app), but
// idiomatic in test assertions, so the two lints are relaxed under `cfg(test)`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod classification;
pub mod domain;
pub mod inventory;
pub mod persistence;
pub mod process;
pub mod providers;
pub mod tui;

#[cfg(feature = "desktop")]
pub mod commands;

#[cfg(feature = "desktop")]
use std::sync::Arc;

#[cfg(feature = "desktop")]
use tauri::Manager;

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            log::info!("Kunger starting up");

            let app_data_dir = app.path().app_data_dir()?;
            let db_path = app_data_dir.join("kunger.db");
            let conn = persistence::db::open(&db_path)?;
            let repository: Arc<dyn persistence::ScanRepository> =
                Arc::new(persistence::SqliteScanRepository::new(conn));
            let service = inventory::InventoryService::with_default_providers();
            let app_state = Arc::new(commands::AppState::new(service, repository));
            app.manage(app_state);

            Ok(())
        })
        // Custom command permissions (capabilities/default.json) are
        // wired up once the frontend actually starts invoking these
        // (M4.5+) — verifying the exact ACL entries needed is easiest
        // done against a real `invoke()` call rather than guessed here.
        .invoke_handler(tauri::generate_handler![
            commands::provider::get_provider_status,
            commands::scan::start_inventory_scan,
            commands::scan::get_scan_status,
            commands::scan::cancel_inventory_scan,
            commands::inventory_commands::get_inventory_summary,
            commands::inventory_commands::list_software_items,
            commands::inventory_commands::get_software_item,
            commands::inventory_commands::list_duplicate_groups,
            commands::inventory_commands::list_provider_warnings,
            commands::export::export_inventory,
            commands::inventory_commands::rebuild_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
