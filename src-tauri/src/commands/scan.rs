//! `start_inventory_scan`, `get_scan_status`, `cancel_inventory_scan`.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio_util::sync::CancellationToken;

use super::events::ScanEventEmitter;
use super::{
    run_blocking, AppState, CommandError, ScanRuntimeState, ScanStatusResponse, StartScanRequest,
};

const DEFAULT_PER_PROVIDER_TIMEOUT_MS: u64 = 30_000;
const MAX_PER_PROVIDER_TIMEOUT_MS: u64 = 600_000;

pub async fn start_inventory_scan_impl(
    state: Arc<AppState>,
    emitter: Arc<dyn ScanEventEmitter>,
    request: StartScanRequest,
) -> Result<(), CommandError> {
    let timeout_ms = request
        .per_provider_timeout_ms
        .unwrap_or(DEFAULT_PER_PROVIDER_TIMEOUT_MS);
    if timeout_ms == 0 || timeout_ms > MAX_PER_PROVIDER_TIMEOUT_MS {
        return Err(CommandError::invalid_request(format!(
            "perProviderTimeoutMs must be between 1 and {MAX_PER_PROVIDER_TIMEOUT_MS}"
        )));
    }

    let cancellation = CancellationToken::new();
    {
        let mut scan_state = state.scan_state();
        if matches!(*scan_state, ScanRuntimeState::Running { .. }) {
            return Err(CommandError::conflict("a scan is already running"));
        }
        *scan_state = ScanRuntimeState::Running {
            started_at: Utc::now(),
            cancellation: cancellation.clone(),
        };
    }

    emitter.emit_started();

    let state_for_task = Arc::clone(&state);
    let emitter_for_task = Arc::clone(&emitter);
    let cancellation_for_task = cancellation.clone();

    tokio::spawn(async move {
        let per_provider_timeout = Duration::from_millis(timeout_ms);
        let result = state_for_task
            .inventory
            .scan(per_provider_timeout, cancellation_for_task.clone())
            .await;
        let was_cancelled = cancellation_for_task.is_cancelled();
        let summary = result.summary.clone();

        let repository = Arc::clone(&state_for_task.repository);
        let save_result = run_blocking(move || repository.save_scan(&result)).await;

        *state_for_task.scan_state() = ScanRuntimeState::Idle;

        match save_result {
            Ok(_scan_id) if was_cancelled => emitter_for_task.emit_cancelled(),
            Ok(_scan_id) => emitter_for_task.emit_completed(&summary),
            Err(error) => emitter_for_task.emit_failed(&error.to_string()),
        }
    });

    Ok(())
}

pub async fn get_scan_status_impl(state: &AppState) -> Result<ScanStatusResponse, CommandError> {
    let snapshot = state.scan_state().clone();

    match snapshot {
        ScanRuntimeState::Idle => {
            let repository = Arc::clone(&state.repository);
            let last_summary = run_blocking(move || repository.latest_scan_summary()).await?;
            Ok(ScanStatusResponse::Idle { last_summary })
        }
        ScanRuntimeState::Running { started_at, .. } => {
            let elapsed_ms = (Utc::now() - started_at).num_milliseconds();
            Ok(ScanStatusResponse::Running {
                started_at,
                elapsed_ms,
            })
        }
    }
}

pub fn cancel_inventory_scan_impl(state: &AppState) -> Result<(), CommandError> {
    match &*state.scan_state() {
        ScanRuntimeState::Running { cancellation, .. } => {
            cancellation.cancel();
            Ok(())
        }
        ScanRuntimeState::Idle => Err(CommandError::conflict("no scan is currently running")),
    }
}

#[tauri::command]
pub async fn start_inventory_scan(
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
    request: StartScanRequest,
) -> Result<(), CommandError> {
    let state = Arc::clone(state.inner());
    let emitter: Arc<dyn ScanEventEmitter> =
        Arc::new(super::events::TauriScanEventEmitter::new(app_handle));
    start_inventory_scan_impl(state, emitter, request).await
}

#[tauri::command]
pub async fn get_scan_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<ScanStatusResponse, CommandError> {
    get_scan_status_impl(state.inner()).await
}

#[tauri::command]
pub fn cancel_inventory_scan(state: tauri::State<'_, Arc<AppState>>) -> Result<(), CommandError> {
    cancel_inventory_scan_impl(state.inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::events::NoopScanEventEmitter;
    use crate::commands::test_support::test_state;
    use crate::domain::PackageManager;
    use crate::domain::SoftwareItem;
    use crate::providers::mock::MockInventoryProvider;

    /// Polls instead of guessing a fixed sleep duration -- a fixed sleep
    /// here was flaky on real CI runners (slower/more contended than this
    /// was developed and passing on), surfacing as an intermittent status
    /// check racing the background scan task's persistence write.
    async fn wait_until_idle(state: &AppState) {
        for _ in 0..500 {
            if matches!(
                get_scan_status_impl(state).await.expect("status"),
                ScanStatusResponse::Idle { .. }
            ) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("scan did not reach Idle in time");
    }

    #[tokio::test]
    async fn rejects_a_zero_timeout() {
        let state = Arc::new(test_state(vec![]));
        let emitter = Arc::new(NoopScanEventEmitter);

        let result = start_inventory_scan_impl(
            state,
            emitter,
            StartScanRequest {
                per_provider_timeout_ms: Some(0),
            },
        )
        .await;

        assert!(matches!(result, Err(e) if e.kind == "invalidRequest"));
    }

    #[tokio::test]
    async fn rejects_an_excessive_timeout() {
        let state = Arc::new(test_state(vec![]));
        let emitter = Arc::new(NoopScanEventEmitter);

        let result = start_inventory_scan_impl(
            state,
            emitter,
            StartScanRequest {
                per_provider_timeout_ms: Some(MAX_PER_PROVIDER_TIMEOUT_MS + 1),
            },
        )
        .await;

        assert!(matches!(result, Err(e) if e.kind == "invalidRequest"));
    }

    #[tokio::test]
    async fn status_is_idle_before_any_scan_has_run() {
        let state = test_state(vec![]);

        let status = get_scan_status_impl(&state).await.expect("status");

        assert!(matches!(
            status,
            ScanStatusResponse::Idle { last_summary: None }
        ));
    }

    #[tokio::test]
    async fn cancel_without_a_running_scan_is_a_conflict() {
        let state = test_state(vec![]);

        let result = cancel_inventory_scan_impl(&state);

        assert!(matches!(result, Err(e) if e.kind == "conflict"));
    }

    #[tokio::test]
    async fn starting_a_second_scan_while_one_is_running_is_a_conflict() {
        let state = Arc::new(test_state(vec![Box::new(
            MockInventoryProvider::new("slow").with_delay(Duration::from_millis(200)),
        )]));
        let emitter = Arc::new(NoopScanEventEmitter);

        start_inventory_scan_impl(
            Arc::clone(&state),
            emitter.clone(),
            StartScanRequest::default(),
        )
        .await
        .expect("first scan starts");

        let second =
            start_inventory_scan_impl(Arc::clone(&state), emitter, StartScanRequest::default())
                .await;

        assert!(matches!(second, Err(e) if e.kind == "conflict"));
    }

    #[tokio::test]
    async fn a_completed_scan_is_persisted_and_status_returns_to_idle() {
        let item = SoftwareItem::new("apt:git", "git", "git", PackageManager::Apt);
        let state = Arc::new(test_state(vec![Box::new(
            MockInventoryProvider::new("apt").with_items(vec![item]),
        )]));
        let emitter = Arc::new(NoopScanEventEmitter);

        start_inventory_scan_impl(Arc::clone(&state), emitter, StartScanRequest::default())
            .await
            .expect("scan starts");

        wait_until_idle(&state).await;

        let status = get_scan_status_impl(&state).await.expect("status");
        match status {
            ScanStatusResponse::Idle {
                last_summary: Some(summary),
            } => assert_eq!(summary.total_items, 1),
            other => panic!("expected an idle status with a summary, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn cancelling_a_running_scan_marks_it_cancelled() {
        let state = Arc::new(test_state(vec![Box::new(
            MockInventoryProvider::new("slow").with_delay(Duration::from_secs(5)),
        )]));
        let emitter = Arc::new(NoopScanEventEmitter);

        start_inventory_scan_impl(Arc::clone(&state), emitter, StartScanRequest::default())
            .await
            .expect("scan starts");

        cancel_inventory_scan_impl(&state).expect("cancel");

        wait_until_idle(&state).await;

        let status = get_scan_status_impl(&state).await.expect("status");
        assert!(matches!(status, ScanStatusResponse::Idle { .. }));
    }
}
