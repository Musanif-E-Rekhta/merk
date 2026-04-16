use crate::config::AppConfig;
use crate::db::Db;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: Db,
}

impl AppState {
    pub fn new(config: AppConfig, db: Db) -> Self {
        Self {
            config: Arc::new(config),
            db,
        }
    }
}
