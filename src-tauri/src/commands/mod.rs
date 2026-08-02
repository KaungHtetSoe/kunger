//! Tauri IPC command layer: the trust boundary between the frontend and
//! the inventory/persistence layers. Every command validates its input
//! before touching business logic — see `docs/ARCHITECTURE.md` §2.6 and
//! §10.
//!
//! Command functions themselves (`#[tauri::command]`) are kept thin,
//! extracting Tauri-specific parameters and delegating to plain `_impl`
//! functions that take [`AppState`] directly — those are what the unit
//! tests in each submodule exercise, since constructing a real
//! `tauri::State`/`AppHandle` outside a running app is impractical.

pub mod events;
pub mod export;
pub mod inventory_commands;
pub mod provider;
pub mod scan;
mod types;

// Note: the actual `#[tauri::command]` functions are referenced by their
// full submodule path (e.g. `commands::scan::start_inventory_scan`) in
// `lib.rs`'s `tauri::generate_handler!` call, not through a flat
// re-export here — the command macro generates hidden sibling items
// alongside each function that only resolve correctly from their
// original module path.
pub use events::{NoopScanEventEmitter, ScanEventEmitter, TauriScanEventEmitter};
pub use types::*;

use std::sync::Mutex as StdMutex;
use std::sync::{Arc, MutexGuard};

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

use crate::inventory::InventoryService;
use crate::persistence::ScanRepository;

/// Runtime (in-memory, not persisted) state of the currently running scan,
/// if any.
#[derive(Debug, Clone)]
pub enum ScanRuntimeState {
    Idle,
    Running {
        started_at: DateTime<Utc>,
        cancellation: CancellationToken,
    },
}

/// Shared application state managed by Tauri (`app.manage(state)`), and
/// constructed directly (no Tauri machinery needed) in tests.
pub struct AppState {
    pub inventory: InventoryService,
    pub repository: Arc<dyn ScanRepository>,
    scan_state: StdMutex<ScanRuntimeState>,
}

impl AppState {
    pub fn new(inventory: InventoryService, repository: Arc<dyn ScanRepository>) -> Self {
        Self {
            inventory,
            repository,
            scan_state: StdMutex::new(ScanRuntimeState::Idle),
        }
    }

    fn scan_state(&self) -> MutexGuard<'_, ScanRuntimeState> {
        self.scan_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// A structured, serializable error returned to the frontend. `kind` is
/// machine-readable (for the frontend to branch on without string
/// matching, mirroring [`crate::domain::ProviderError`]'s design —
/// ADR-0009); `message` is human-readable.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub kind: String,
    pub message: String,
}

impl CommandError {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            kind: "invalidRequest".to_string(),
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: "notFound".to_string(),
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: "conflict".to_string(),
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: "internal".to_string(),
            message: message.into(),
        }
    }
}

impl From<crate::persistence::PersistenceError> for CommandError {
    fn from(error: crate::persistence::PersistenceError) -> Self {
        CommandError::internal(error.to_string())
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for CommandError {}

/// Runs a blocking [`ScanRepository`] call off the async executor thread,
/// per ADR-0014 (`rusqlite` is synchronous). Every command that touches
/// the repository goes through this rather than calling it inline.
pub(crate) async fn run_blocking<F, T>(f: F) -> Result<T, CommandError>
where
    F: FnOnce() -> Result<T, crate::persistence::PersistenceError> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(result) => result.map_err(CommandError::from),
        Err(join_error) => Err(CommandError::internal(join_error.to_string())),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::AppState;
    use crate::inventory::InventoryService;
    use crate::persistence::{db, SqliteScanRepository};
    use crate::providers::InventoryProvider;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A fresh `AppState` backed by a throwaway temp-file SQLite database
    /// (never shared between tests) and the given mock/test providers.
    pub fn test_state(providers: Vec<Box<dyn InventoryProvider>>) -> AppState {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "kunger-cmd-test-{}-{}-{unique}.db",
            std::process::id(),
            fastrand_ish()
        ));
        let conn = db::open(&path).expect("open test db");
        let repository = Arc::new(SqliteScanRepository::new(conn));
        AppState::new(InventoryService::new(providers), repository)
    }

    fn fastrand_ish() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}
