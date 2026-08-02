//! Persistence layer: a rebuildable SQLite cache of scan results, never
//! the source of truth (`docs/DECISIONS.md` ADR-0006). See
//! `docs/ARCHITECTURE.md` §2.5 and §9.

pub mod db;
mod error;
mod repository;
mod schema;

pub use error::PersistenceError;
pub use repository::{ScanDiff, ScanRepository, SqliteScanRepository, VersionChange};
