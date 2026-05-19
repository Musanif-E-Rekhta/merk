use crate::config::AppConfig;
use crate::error::Error;
use surrealdb::Surreal;
use surrealdb::engine::any::{Any, connect};
use surrealdb::types::RecordIdKey;
use tracing::info;

pub mod admin;
pub mod book_repo;
pub mod bookmark_repo;
pub mod chapter_repo;
pub mod collection_repo;
pub mod comment_repo;
pub mod highlight_repo;
pub mod profile_repo;
pub mod rbac_repo;
pub mod refresh_token_repo;
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

/// Recursively drop keys whose value is JSON `null` from any object reached
/// by `v`. Use this on payloads built via `json!({...})` immediately before
/// binding to `CONTENT $data` or `MERGE $data`.
///
/// Why: the SurrealDB Rust SDK turns `Option::<T>::None` into `Value::None`
/// for direct binds, but `serde_json::Value::Null` (what `json!()` emits for
/// a `None` field) becomes `Value::Null`. SurrealDB treats `option<T>` as
/// `NONE | T` — assigning `NULL` to such a field fails coercion, and the
/// stored value poisons every subsequent UPDATE on that record. Dropping
/// null keys instead lets the field fall back to its DEFAULT (`NONE` for
/// `option<T>`), which is the intended behaviour.
pub fn strip_nulls(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k, strip_nulls(v)))
                .collect(),
        ),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(strip_nulls).collect())
        }
        other => other,
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
