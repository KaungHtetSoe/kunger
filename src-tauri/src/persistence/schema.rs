//! Schema migrations. Forward-only: each entry in [`MIGRATIONS`] takes the
//! database from version N to N+1. Applied inside a single transaction per
//! migration on every [`crate::persistence::db::open`] call, so a fresh or
//! up-to-date database is a no-op.

use rusqlite::Connection;

use super::error::PersistenceError;

/// Migration 1: initial schema. See `docs/ARCHITECTURE.md` §9.
///
/// `software_items` and the other per-scan tables carry a handful of
/// indexed columns for filtering/sorting alongside a full `data_json`
/// column (the complete serialized domain type) — this keeps the schema
/// small while still supporting the query patterns `list_software_items`
/// (Prompt 08) needs, without a full field-per-column normalization that
/// would need a migration every time a domain field is added.
const MIGRATION_0001: &str = r#"
CREATE TABLE scan_sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at      TEXT NOT NULL,
    completed_at    TEXT NOT NULL,
    status          TEXT NOT NULL,
    duration_ms     INTEGER,
    summary_json    TEXT NOT NULL
);

CREATE TABLE software_items (
    scan_id                     INTEGER NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
    item_id                     TEXT NOT NULL,
    package_name                TEXT NOT NULL,
    display_name                TEXT NOT NULL,
    category                    TEXT NOT NULL,
    package_manager             TEXT NOT NULL,
    scope                       TEXT NOT NULL,
    installation_reason         TEXT NOT NULL,
    classification_confidence   TEXT NOT NULL,
    version                     TEXT,
    update_available            INTEGER NOT NULL,
    data_json                   TEXT NOT NULL,
    PRIMARY KEY (scan_id, item_id)
);

CREATE INDEX idx_software_items_scan_category ON software_items(scan_id, category);
CREATE INDEX idx_software_items_scan_manager ON software_items(scan_id, package_manager);
CREATE INDEX idx_software_items_scan_display_name ON software_items(scan_id, display_name);

CREATE TABLE duplicate_groups (
    scan_id     INTEGER NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
    group_id    TEXT NOT NULL,
    data_json   TEXT NOT NULL,
    PRIMARY KEY (scan_id, group_id)
);

CREATE TABLE provider_results (
    scan_id       INTEGER NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
    provider_id   TEXT NOT NULL,
    status        TEXT NOT NULL,
    data_json     TEXT NOT NULL,
    PRIMARY KEY (scan_id, provider_id)
);

CREATE TABLE settings (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);
"#;

const MIGRATIONS: &[&str] = &[MIGRATION_0001];

pub fn apply_migrations(conn: &mut Connection) -> Result<(), PersistenceError> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_meta (version INTEGER NOT NULL)")?;

    let current_version: i64 = conn
        .query_row("SELECT version FROM schema_meta LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    if (current_version as usize) >= MIGRATIONS.len() {
        return Ok(());
    }

    for (index, migration) in MIGRATIONS.iter().enumerate().skip(current_version as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(migration)?;
        tx.execute("DELETE FROM schema_meta", [])?;
        tx.execute(
            "INSERT INTO schema_meta (version) VALUES (?1)",
            [(index + 1) as i64],
        )?;
        tx.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_cleanly_to_a_fresh_in_memory_database() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        apply_migrations(&mut conn).expect("migrations should apply");

        let version: i64 = conn
            .query_row("SELECT version FROM schema_meta", [], |row| row.get(0))
            .expect("version row");
        assert_eq!(version, 1);
    }

    #[test]
    fn is_idempotent_when_run_twice() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        apply_migrations(&mut conn).expect("first run");
        apply_migrations(&mut conn).expect("second run should be a no-op, not an error");

        let version: i64 = conn
            .query_row("SELECT version FROM schema_meta", [], |row| row.get(0))
            .expect("version row");
        assert_eq!(version, 1);
    }

    #[test]
    fn creates_every_expected_table() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        apply_migrations(&mut conn).expect("migrations should apply");

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .expect("prepare");
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("query")
            .filter_map(Result::ok)
            .collect();

        for expected in [
            "scan_sessions",
            "software_items",
            "duplicate_groups",
            "provider_results",
            "settings",
            "schema_meta",
        ] {
            assert!(
                names.contains(&expected.to_string()),
                "missing table: {expected}"
            );
        }
    }
}
