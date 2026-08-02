//! `get_provider_status` command.

use super::{AppState, ProviderStatusResponse};

pub async fn get_provider_status_impl(state: &AppState) -> Vec<ProviderStatusResponse> {
    state
        .inventory
        .provider_status_details()
        .await
        .into_iter()
        .map(|(metadata, available)| ProviderStatusResponse {
            id: metadata.id.as_str().to_string(),
            display_name: metadata.display_name.to_string(),
            description: metadata.description.to_string(),
            available,
        })
        .collect()
}

#[tauri::command]
pub async fn get_provider_status(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ProviderStatusResponse>, super::CommandError> {
    Ok(get_provider_status_impl(&state).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::test_state;
    use crate::providers::mock::MockInventoryProvider;

    #[tokio::test]
    async fn reports_availability_and_metadata_for_every_provider() {
        let state = test_state(vec![
            Box::new(MockInventoryProvider::new("apt")),
            Box::new(MockInventoryProvider::new("flatpak").unavailable()),
        ]);

        let statuses = get_provider_status_impl(&state).await;

        assert_eq!(statuses.len(), 2);
        assert!(statuses.iter().any(|s| s.id == "apt" && s.available));
        assert!(statuses.iter().any(|s| s.id == "flatpak" && !s.available));
    }
}
