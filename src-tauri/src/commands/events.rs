//! Scan lifecycle event emission, abstracted behind a trait so command
//! logic is testable without a running Tauri app (`tauri::AppHandle`
//! can't be constructed outside one).

use crate::domain::InventorySummary;

pub trait ScanEventEmitter: Send + Sync {
    fn emit_started(&self);
    fn emit_completed(&self, summary: &InventorySummary);
    fn emit_failed(&self, message: &str);
    fn emit_cancelled(&self);
}

/// Real implementation, emitting Tauri events the frontend subscribes to:
/// `scan-started`, `scan-completed` (payload: [`InventorySummary`]),
/// `scan-failed` (payload: error message), `scan-cancelled`.
pub struct TauriScanEventEmitter {
    app_handle: tauri::AppHandle,
}

impl TauriScanEventEmitter {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl ScanEventEmitter for TauriScanEventEmitter {
    fn emit_started(&self) {
        use tauri::Emitter;
        let _ = self.app_handle.emit("scan-started", ());
    }

    fn emit_completed(&self, summary: &InventorySummary) {
        use tauri::Emitter;
        let _ = self.app_handle.emit("scan-completed", summary);
    }

    fn emit_failed(&self, message: &str) {
        use tauri::Emitter;
        let _ = self.app_handle.emit("scan-failed", message);
    }

    fn emit_cancelled(&self) {
        use tauri::Emitter;
        let _ = self.app_handle.emit("scan-cancelled", ());
    }
}

/// Test/no-op implementation.
#[derive(Default)]
pub struct NoopScanEventEmitter;

impl ScanEventEmitter for NoopScanEventEmitter {
    fn emit_started(&self) {}
    fn emit_completed(&self, _summary: &InventorySummary) {}
    fn emit_failed(&self, _message: &str) {}
    fn emit_cancelled(&self) {}
}
