// `unwrap`/`expect` are worth flagging in production provider/parsing code
// (see docs/SECURITY.md — untrusted input must never panic the app), but
// idiomatic in test assertions, so the two lints are relaxed under `cfg(test)`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod classification;
pub mod domain;
pub mod inventory;
pub mod process;
pub mod providers;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|_app| {
            log::info!("Kunger starting up");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
