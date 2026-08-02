//! Repository interface and SQLite implementation for scan data. See
//! `docs/ARCHITECTURE.md` §2.5 and §9.

use std::collections::HashMap;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::domain::{DuplicateGroup, InventorySummary, ProviderInventory, SoftwareItem};
use crate::inventory::ScanResult;

use super::error::PersistenceError;

/// The set of items that changed between the two most recent scans.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanDiff {
    pub new_items: Vec<SoftwareItem>,
    pub removed_items: Vec<SoftwareItem>,
    pub version_changed: Vec<VersionChange>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionChange {
    pub id: String,
    pub display_name: String,
    pub previous_version: Option<String>,
    pub current_version: Option<String>,
}

/// Persistence interface for scan data. A trait (rather than exposing
/// [`SqliteScanRepository`] directly) so the command layer (Prompt 08) can
/// depend on this instead of a concrete SQLite type, and so tests can
/// substitute an alternate implementation if needed.
pub trait ScanRepository: Send + Sync {
    /// Persists a completed scan (summary, items, duplicate groups, and
    /// raw provider results) in one transaction. Returns the new scan id.
    fn save_scan(&self, result: &ScanResult) -> Result<i64, PersistenceError>;

    fn latest_scan_summary(&self) -> Result<Option<InventorySummary>, PersistenceError>;
    fn previous_scan_summary(&self) -> Result<Option<InventorySummary>, PersistenceError>;

    /// The id of the most recently saved scan, if any. Lets callers that
    /// need scan-id-scoped data (duplicate groups, provider results) find
    /// the latest one without a separate "latest" variant of every method.
    fn latest_scan_id(&self) -> Result<Option<i64>, PersistenceError>;

    fn list_items(&self, scan_id: i64) -> Result<Vec<SoftwareItem>, PersistenceError>;
    fn latest_items(&self) -> Result<Vec<SoftwareItem>, PersistenceError>;

    fn list_duplicate_groups(&self, scan_id: i64) -> Result<Vec<DuplicateGroup>, PersistenceError>;
    fn list_provider_results(
        &self,
        scan_id: i64,
    ) -> Result<Vec<ProviderInventory>, PersistenceError>;

    /// Diffs the two most recent scans. Returns `None` if fewer than two
    /// scans have been recorded yet.
    fn diff_latest_two_scans(&self) -> Result<Option<ScanDiff>, PersistenceError>;

    /// Wipes all cached scan data. Safe at any time — the cache is always
    /// rebuildable from a fresh scan (`docs/DECISIONS.md` ADR-0006).
    fn rebuild_cache(&self) -> Result<(), PersistenceError>;

    fn get_setting(&self, key: &str) -> Result<Option<String>, PersistenceError>;
    fn set_setting(&self, key: &str, value: &str) -> Result<(), PersistenceError>;
}

pub struct SqliteScanRepository {
    conn: Mutex<Connection>,
}

impl SqliteScanRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl ScanRepository for SqliteScanRepository {
    fn save_scan(&self, result: &ScanResult) -> Result<i64, PersistenceError> {
        let mut conn = self.lock();
        let tx = conn.transaction()?;

        let summary_json = to_json(&result.summary)?;
        let started_at = result
            .summary
            .last_scan_started_at
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339();
        let completed_at = result
            .summary
            .last_scan_completed_at
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339();
        let status = enum_to_column(&result.summary.status)?;

        tx.execute(
            "INSERT INTO scan_sessions (started_at, completed_at, status, duration_ms, summary_json) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                started_at,
                completed_at,
                status,
                result.summary.scan_duration_ms.map(|ms| ms as i64),
                summary_json
            ],
        )?;
        let scan_id = tx.last_insert_rowid();

        for item in &result.items {
            let data_json = to_json(item)?;
            tx.execute(
                "INSERT INTO software_items (
                    scan_id, item_id, package_name, display_name, category, package_manager,
                    scope, installation_reason, classification_confidence, version, update_available, data_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    scan_id,
                    item.id,
                    item.package_name,
                    item.display_name,
                    enum_to_column(&item.category)?,
                    enum_to_column(&item.package_manager)?,
                    enum_to_column(&item.scope)?,
                    enum_to_column(&item.installation_reason)?,
                    enum_to_column(&item.classification_confidence)?,
                    item.version,
                    item.update_available as i64,
                    data_json,
                ],
            )?;
        }

        for group in &result.duplicate_groups {
            let data_json = to_json(group)?;
            tx.execute(
                "INSERT INTO duplicate_groups (scan_id, group_id, data_json) VALUES (?1, ?2, ?3)",
                params![scan_id, group.id, data_json],
            )?;
        }

        for provider_result in &result.provider_results {
            let data_json = to_json(provider_result)?;
            tx.execute(
                "INSERT INTO provider_results (scan_id, provider_id, status, data_json) VALUES (?1, ?2, ?3, ?4)",
                params![scan_id, provider_result.provider_id, enum_to_column(&provider_result.status)?, data_json],
            )?;
        }

        tx.commit()?;
        Ok(scan_id)
    }

    fn latest_scan_summary(&self) -> Result<Option<InventorySummary>, PersistenceError> {
        let conn = self.lock();
        let summary_json: Option<String> = conn
            .query_row(
                "SELECT summary_json FROM scan_sessions ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        summary_json.map(|json| from_json(&json)).transpose()
    }

    fn previous_scan_summary(&self) -> Result<Option<InventorySummary>, PersistenceError> {
        let conn = self.lock();
        let summary_json: Option<String> = conn
            .query_row(
                "SELECT summary_json FROM scan_sessions ORDER BY id DESC LIMIT 1 OFFSET 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        summary_json.map(|json| from_json(&json)).transpose()
    }

    fn latest_scan_id(&self) -> Result<Option<i64>, PersistenceError> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id FROM scan_sessions ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(PersistenceError::from)
    }

    fn list_items(&self, scan_id: i64) -> Result<Vec<SoftwareItem>, PersistenceError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT data_json FROM software_items WHERE scan_id = ?1 ORDER BY display_name",
        )?;
        let rows = stmt.query_map([scan_id], |row| row.get::<_, String>(0))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(from_json(&row?)?);
        }
        Ok(items)
    }

    fn latest_items(&self) -> Result<Vec<SoftwareItem>, PersistenceError> {
        match self.latest_scan_id()? {
            Some(scan_id) => self.list_items(scan_id),
            None => Ok(Vec::new()),
        }
    }

    fn list_duplicate_groups(&self, scan_id: i64) -> Result<Vec<DuplicateGroup>, PersistenceError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT data_json FROM duplicate_groups WHERE scan_id = ?1 ORDER BY group_id",
        )?;
        let rows = stmt.query_map([scan_id], |row| row.get::<_, String>(0))?;

        let mut groups = Vec::new();
        for row in rows {
            groups.push(from_json(&row?)?);
        }
        Ok(groups)
    }

    fn list_provider_results(
        &self,
        scan_id: i64,
    ) -> Result<Vec<ProviderInventory>, PersistenceError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT data_json FROM provider_results WHERE scan_id = ?1 ORDER BY provider_id",
        )?;
        let rows = stmt.query_map([scan_id], |row| row.get::<_, String>(0))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(from_json(&row?)?);
        }
        Ok(results)
    }

    fn diff_latest_two_scans(&self) -> Result<Option<ScanDiff>, PersistenceError> {
        let (latest_id, previous_id) = {
            let conn = self.lock();
            let mut stmt = conn.prepare("SELECT id FROM scan_sessions ORDER BY id DESC LIMIT 2")?;
            let ids: Vec<i64> = stmt
                .query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            if ids.len() < 2 {
                return Ok(None);
            }
            (ids[0], ids[1])
        };

        let latest_items = self.list_items(latest_id)?;
        let previous_items = self.list_items(previous_id)?;

        let previous_by_id: HashMap<&str, &SoftwareItem> = previous_items
            .iter()
            .map(|item| (item.id.as_str(), item))
            .collect();
        let latest_by_id: HashMap<&str, &SoftwareItem> = latest_items
            .iter()
            .map(|item| (item.id.as_str(), item))
            .collect();

        let new_items: Vec<SoftwareItem> = latest_items
            .iter()
            .filter(|item| !previous_by_id.contains_key(item.id.as_str()))
            .cloned()
            .collect();
        let removed_items: Vec<SoftwareItem> = previous_items
            .iter()
            .filter(|item| !latest_by_id.contains_key(item.id.as_str()))
            .cloned()
            .collect();

        let mut version_changed = Vec::new();
        for item in &latest_items {
            if let Some(previous) = previous_by_id.get(item.id.as_str()) {
                if previous.version != item.version {
                    version_changed.push(VersionChange {
                        id: item.id.clone(),
                        display_name: item.display_name.clone(),
                        previous_version: previous.version.clone(),
                        current_version: item.version.clone(),
                    });
                }
            }
        }

        Ok(Some(ScanDiff {
            new_items,
            removed_items,
            version_changed,
        }))
    }

    fn rebuild_cache(&self) -> Result<(), PersistenceError> {
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        tx.execute_batch(
            "DELETE FROM software_items; \
             DELETE FROM duplicate_groups; \
             DELETE FROM provider_results; \
             DELETE FROM scan_sessions;",
        )?;
        tx.commit()?;
        Ok(())
    }

    fn get_setting(&self, key: &str) -> Result<Option<String>, PersistenceError> {
        let conn = self.lock();
        conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(PersistenceError::from)
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<(), PersistenceError> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

fn to_json<T: Serialize>(value: &T) -> Result<String, PersistenceError> {
    serde_json::to_string(value).map_err(PersistenceError::Serialize)
}

fn from_json<T: DeserializeOwned>(json: &str) -> Result<T, PersistenceError> {
    serde_json::from_str(json).map_err(PersistenceError::Deserialize)
}

/// Converts a `#[serde(rename_all = "camelCase")]` fieldless enum to its
/// plain string form (e.g. `SoftwareCategory::CommandLineTool` ->
/// `"commandLineTool"`) for use as an indexed SQL column value, reusing
/// serde's own naming rather than hand-written match arms per enum.
fn enum_to_column<T: Serialize>(value: &T) -> Result<String, PersistenceError> {
    let json = serde_json::to_value(value).map_err(PersistenceError::Serialize)?;
    Ok(json.as_str().unwrap_or_default().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ClassificationConfidence, InventoryStatus, PackageManager, ProviderStatus, SoftwareCategory,
    };
    use crate::persistence::db;

    fn repo() -> SqliteScanRepository {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        let mut conn = conn;
        crate::persistence::schema::apply_migrations(&mut conn).expect("apply migrations");
        SqliteScanRepository::new(conn)
    }

    fn sample_item(id: &str, display_name: &str, version: Option<&str>) -> SoftwareItem {
        let mut item = SoftwareItem::new(id, id, display_name, PackageManager::Apt);
        item.version = version.map(str::to_string);
        item.category = SoftwareCategory::Application;
        item.classification_confidence = ClassificationConfidence::High;
        item
    }

    fn sample_scan_result(items: Vec<SoftwareItem>) -> ScanResult {
        let now = chrono::Utc::now();
        let mut items_by_category = HashMap::new();
        items_by_category.insert(SoftwareCategory::Application, items.len());

        ScanResult {
            summary: InventorySummary {
                status: InventoryStatus::Completed,
                total_items: items.len(),
                items_by_category,
                items_by_package_manager: HashMap::new(),
                providers_with_warnings: Vec::new(),
                providers_with_errors: Vec::new(),
                duplicate_group_count: 0,
                last_scan_started_at: Some(now),
                last_scan_completed_at: Some(now),
                scan_duration_ms: Some(42),
            },
            items,
            duplicate_groups: vec![DuplicateGroup {
                id: "dup:example".to_string(),
                item_ids: vec!["apt:a".to_string(), "apt:b".to_string()],
                reason: "example".to_string(),
                confidence: ClassificationConfidence::Medium,
            }],
            provider_results: vec![{
                let mut inventory = ProviderInventory::started("apt", now);
                inventory.status = ProviderStatus::Success;
                inventory
            }],
        }
    }

    #[test]
    fn enum_to_column_uses_serdes_camel_case_naming() {
        assert_eq!(
            enum_to_column(&SoftwareCategory::CommandLineTool).expect("ok"),
            "commandLineTool"
        );
        assert_eq!(
            enum_to_column(&PackageManager::AppImage).expect("ok"),
            "appImage"
        );
    }

    #[test]
    fn save_scan_round_trips_items_summary_and_duplicate_groups() {
        let repository = repo();
        let scan_result = sample_scan_result(vec![sample_item("apt:git", "git", Some("1.0"))]);

        let scan_id = repository.save_scan(&scan_result).expect("save scan");

        let items = repository.list_items(scan_id).expect("list items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "apt:git");

        let summary = repository
            .latest_scan_summary()
            .expect("latest summary")
            .expect("summary present");
        assert_eq!(summary.total_items, 1);

        let groups = repository
            .list_duplicate_groups(scan_id)
            .expect("list groups");
        assert_eq!(groups.len(), 1);

        let provider_results = repository
            .list_provider_results(scan_id)
            .expect("list provider results");
        assert_eq!(provider_results.len(), 1);
    }

    #[test]
    fn latest_and_previous_summary_reflect_scan_order() {
        let repository = repo();
        repository
            .save_scan(&sample_scan_result(vec![sample_item(
                "apt:a",
                "a",
                Some("1.0"),
            )]))
            .expect("first save");
        repository
            .save_scan(&sample_scan_result(vec![
                sample_item("apt:a", "a", Some("1.0")),
                sample_item("apt:b", "b", None),
            ]))
            .expect("second save");

        let latest = repository
            .latest_scan_summary()
            .expect("latest")
            .expect("present");
        let previous = repository
            .previous_scan_summary()
            .expect("previous")
            .expect("present");

        assert_eq!(latest.total_items, 2);
        assert_eq!(previous.total_items, 1);
    }

    #[test]
    fn diff_returns_none_with_fewer_than_two_scans() {
        let repository = repo();
        repository
            .save_scan(&sample_scan_result(vec![sample_item(
                "apt:a",
                "a",
                Some("1.0"),
            )]))
            .expect("save");

        assert_eq!(repository.diff_latest_two_scans().expect("diff"), None);
    }

    #[test]
    fn diff_detects_new_removed_and_version_changed_items() {
        let repository = repo();
        repository
            .save_scan(&sample_scan_result(vec![
                sample_item("apt:stays", "stays", Some("1.0")),
                sample_item("apt:removed", "removed", Some("1.0")),
            ]))
            .expect("first save");
        repository
            .save_scan(&sample_scan_result(vec![
                sample_item("apt:stays", "stays", Some("2.0")),
                sample_item("apt:new", "new", Some("1.0")),
            ]))
            .expect("second save");

        let diff = repository
            .diff_latest_two_scans()
            .expect("diff")
            .expect("some diff");

        assert_eq!(diff.new_items.len(), 1);
        assert_eq!(diff.new_items[0].id, "apt:new");
        assert_eq!(diff.removed_items.len(), 1);
        assert_eq!(diff.removed_items[0].id, "apt:removed");
        assert_eq!(diff.version_changed.len(), 1);
        assert_eq!(diff.version_changed[0].id, "apt:stays");
        assert_eq!(
            diff.version_changed[0].previous_version.as_deref(),
            Some("1.0")
        );
        assert_eq!(
            diff.version_changed[0].current_version.as_deref(),
            Some("2.0")
        );
    }

    #[test]
    fn rebuild_cache_removes_all_scan_data_but_leaves_the_schema_usable() {
        let repository = repo();
        repository
            .save_scan(&sample_scan_result(vec![sample_item(
                "apt:a",
                "a",
                Some("1.0"),
            )]))
            .expect("save");

        repository.rebuild_cache().expect("rebuild");

        assert!(repository.latest_scan_summary().expect("query").is_none());
        assert!(repository.latest_items().expect("query").is_empty());

        // The schema itself must still be usable afterwards -- a fresh
        // scan can be saved immediately.
        let scan_id = repository
            .save_scan(&sample_scan_result(vec![sample_item("apt:b", "b", None)]))
            .expect("save after rebuild");
        assert_eq!(repository.list_items(scan_id).expect("list").len(), 1);
    }

    #[test]
    fn settings_round_trip_and_upsert_on_conflict() {
        let repository = repo();
        assert_eq!(repository.get_setting("theme").expect("get"), None);

        repository.set_setting("theme", "dark").expect("set");
        assert_eq!(
            repository.get_setting("theme").expect("get"),
            Some("dark".to_string())
        );

        repository.set_setting("theme", "light").expect("update");
        assert_eq!(
            repository.get_setting("theme").expect("get"),
            Some("light".to_string())
        );
    }

    #[test]
    fn latest_items_is_empty_when_no_scans_have_been_saved() {
        let repository = repo();
        assert!(repository.latest_items().expect("query").is_empty());
    }

    #[test]
    fn opening_via_db_module_and_saving_through_the_repository_works_end_to_end() {
        let dir = std::env::temp_dir().join(format!("kunger-repo-e2e-{}", std::process::id()));
        let path = dir.join("kunger.db");
        let conn = db::open(&path).expect("open via db module");
        let repository = SqliteScanRepository::new(conn);

        let scan_id = repository
            .save_scan(&sample_scan_result(vec![sample_item(
                "apt:a",
                "a",
                Some("1.0"),
            )]))
            .expect("save");
        assert_eq!(repository.list_items(scan_id).expect("list").len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Isolates the SQLite-read + JSON-deserialize cost of `latest_items()`
    /// from the in-memory filter/sort work `list_software_items_impl`
    /// layers on top (measured separately in
    /// `commands::inventory_commands::tests::performance`) -- against a
    /// real file-based database, not `Connection::open_in_memory()`, to
    /// match what the packaged app actually does. Numbers feed
    /// `docs/PERFORMANCE.md`.
    #[test]
    fn latest_items_read_cost_at_thousands_of_items() {
        let dir = std::env::temp_dir().join(format!("kunger-repo-perf-{}", std::process::id()));
        let path = dir.join("kunger.db");
        let conn = db::open(&path).expect("open via db module");
        let repository = SqliteScanRepository::new(conn);

        const COUNT: usize = 5000;
        let items: Vec<SoftwareItem> = (0..COUNT)
            .map(|i| {
                sample_item(
                    &format!("apt:pkg-{i}"),
                    &format!("Package {i}"),
                    Some("1.0"),
                )
            })
            .collect();

        let save_started = std::time::Instant::now();
        repository
            .save_scan(&sample_scan_result(items))
            .expect("save");
        println!(
            "save_scan over {COUNT} items (file-backed sqlite): {:?}",
            save_started.elapsed()
        );

        let read_started = std::time::Instant::now();
        let read_back = repository.latest_items().expect("latest_items");
        let read_elapsed = read_started.elapsed();
        println!("latest_items() over {COUNT} items (file-backed sqlite): {read_elapsed:?}");

        assert_eq!(read_back.len(), COUNT);
        assert!(
            read_elapsed.as_millis() < 500,
            "latest_items() took {read_elapsed:?}, expected well under 500ms for {COUNT} items"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
