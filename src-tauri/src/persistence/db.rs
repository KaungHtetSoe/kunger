//! Opens (and, if necessary, recovers) the SQLite cache database. See
//! `docs/DECISIONS.md` ADR-0006 — this database is a rebuildable cache,
//! never the source of truth, so a corrupted file is never a fatal error:
//! it's moved aside and a fresh one is created in its place.

use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::Connection;

use super::error::PersistenceError;
use super::schema;

/// Resolves the default database path:
/// `<user data dir>/kunger/kunger.db`. No privileged or shared location —
/// see `docs/SECURITY.md`.
pub fn default_database_path() -> Result<PathBuf, PersistenceError> {
    let data_dir = dirs::data_dir().ok_or(PersistenceError::NoDataDirectory)?;
    Ok(data_dir.join("kunger").join("kunger.db"))
}

/// Opens the database at `path`, creating and migrating it if it doesn't
/// exist, and transparently recovering from a corrupted file by moving it
/// aside and starting fresh. Creates parent directories as needed.
pub fn open(path: &Path) -> Result<Connection, PersistenceError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match try_open_and_migrate(path) {
        Ok(conn) => Ok(conn),
        Err(_original_error) => {
            let backup_path = corrupt_backup_path(path);
            // Best-effort: if this fails (e.g. the file never existed),
            // the fresh open below still gives us a usable database.
            let _ = std::fs::rename(path, &backup_path);
            try_open_and_migrate(path)
        }
    }
}

fn try_open_and_migrate(path: &Path) -> Result<Connection, PersistenceError> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    schema::apply_migrations(&mut conn)?;
    Ok(conn)
}

fn corrupt_backup_path(path: &Path) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("kunger.db");
    path.with_file_name(format!("{file_name}.corrupt-{timestamp}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_and_migrates_a_fresh_database_at_a_new_path() {
        let dir = std::env::temp_dir().join(format!("kunger-db-test-fresh-{}", std::process::id()));
        let path = dir.join("kunger.db");

        let conn = open(&path).expect("should open and migrate cleanly");
        let version: i64 = conn
            .query_row("SELECT version FROM schema_meta", [], |row| row.get(0))
            .expect("version row");
        assert_eq!(version, 1);

        drop(conn);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reopening_an_existing_database_does_not_lose_data() {
        let dir =
            std::env::temp_dir().join(format!("kunger-db-test-reopen-{}", std::process::id()));
        let path = dir.join("kunger.db");

        {
            let conn = open(&path).expect("first open");
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('theme', 'dark')",
                [],
            )
            .expect("insert");
        }

        let conn = open(&path).expect("second open");
        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme'",
                [],
                |row| row.get(0),
            )
            .expect("select");
        assert_eq!(value, "dark");

        drop(conn);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_corrupted_file_is_moved_aside_and_a_fresh_database_is_created() {
        let dir =
            std::env::temp_dir().join(format!("kunger-db-test-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("kunger.db");
        std::fs::write(&path, b"this is not a sqlite database file").expect("write garbage");

        let conn = open(&path).expect("should recover instead of failing");
        let version: i64 = conn
            .query_row("SELECT version FROM schema_meta", [], |row| row.get(0))
            .expect("version row");
        assert_eq!(version, 1);

        drop(conn);

        let backups: Vec<_> = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1, "expected exactly one corrupt backup file");

        std::fs::remove_dir_all(&dir).ok();
    }
}
