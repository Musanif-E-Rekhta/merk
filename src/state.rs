use crate::config::AppConfig;
use crate::db::Db;
use crate::services::Services;
use std::sync::Arc;

/// Shared application state injected into every Axum handler via [`axum::extract::State`].
///
/// Cheap to clone — `config`, `db`, and `services` are all behind an `Arc` or internal Arc-backed handles.
#[derive(Clone)]
pub struct AppState {
    /// Parsed, immutable application configuration.
    pub config: Arc<AppConfig>,
    /// SurrealDB client handle (connection pool managed internally by the driver).
    pub db: Db,
    /// Business logic services.
    pub services: Arc<Services>,
}

impl AppState {
    /// Initialize state with all its dependencies.
    pub fn new(config: AppConfig, db: Db) -> Self {
        let config = Arc::new(config);
        let services = Arc::new(Services::new(db.clone(), config.clone()));

        Self {
            config,
            db,
            services,
        }
    }
}
