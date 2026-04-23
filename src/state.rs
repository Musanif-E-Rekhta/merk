use crate::config::AppConfig;
use crate::db::Db;
use std::sync::Arc;

/// Shared application state injected into every Axum handler via [`axum::extract::State`].
///
/// Cheap to clone — `config` is behind an `Arc` and `Db` is an internal Arc-backed handle.
#[derive(Clone)]
pub struct AppState {
    /// Parsed, immutable application configuration.
    pub config: Arc<AppConfig>,
    /// SurrealDB client handle (connection pool managed internally by the driver).
    pub db: Db,
}

impl AppState {
    /// Wrap `config` in an `Arc` and pair it with the live database handle.
    pub fn new(config: AppConfig, db: Db) -> Self {
        Self {
            config: Arc::new(config),
            db,
        }
    }
}
