use crate::config::AppConfig;
use crate::error::Error;
use surrealdb::Surreal;
use surrealdb::engine::any::{Any, connect};
use surrealdb::types::RecordIdKey;
use tracing::info;

pub mod book_repo;
pub mod bookmark_repo;
pub mod chapter_repo;
pub mod collection_repo;
pub mod comment_repo;
pub mod highlight_repo;
pub mod profile_repo;
pub mod rbac_repo;
pub mod review_repo;
pub mod translation_repo;
pub mod user_repo;

/// Shared database handle type alias. `Surreal<Any>` supports both WebSocket and HTTP protocols.
pub type Db = Surreal<Any>;

/// Convert a SurrealDB [`RecordIdKey`] to its string representation.
pub fn record_id_key_to_string(key: &RecordIdKey) -> String {
    match key {
        RecordIdKey::String(s) => s.clone(),
        RecordIdKey::Number(n) => n.to_string(),
        RecordIdKey::Uuid(u) => u.to_string(),
        other => format!("{other:?}"),
    }
}

/// Connect to SurrealDB, authenticate, select the configured namespace/database, and run migrations.
pub async fn connect_to_db(config: &AppConfig) -> Result<Surreal<Any>, Error> {
    let db = connect(&config.surrealdb_url)
        .await
        .map_err(|e| Error::upstream("surrealdb", format!("Failed to connect: {}", e)))?;

    if config.surrealdb_url.starts_with("ws") && !config.surrealdb_user.is_empty() {
        db.signin(surrealdb::opt::auth::Root {
            username: config.surrealdb_user.to_string(),
            password: config.surrealdb_pass.to_string(),
        })
        .await
        .map_err(|e| Error::upstream("surrealdb", format!("Failed to sign in: {}", e)))?;
    }

    db.use_ns(&config.surrealdb_ns)
        .use_db(&config.surrealdb_db)
        .await
        .map_err(|e| {
            Error::upstream(
                "surrealdb",
                format!("Failed to select namespace/database: {}", e),
            )
        })?;

    info!(
        "Connected to SurrealDB at ns: '{}' | db: '{}'",
        config.surrealdb_ns, config.surrealdb_db
    );

    merk_migrations::Migrator::up(&db, None)
        .await
        .map_err(|e| Error::internal("migration", e.to_string()))?;

    Ok(db)
}
