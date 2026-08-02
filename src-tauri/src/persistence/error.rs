//! Typed errors for the persistence layer.

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not serialize data for storage: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("could not deserialize stored data (schema drift or corruption): {0}")]
    Deserialize(#[source] serde_json::Error),
    #[error("could not determine the application data directory")]
    NoDataDirectory,
}
