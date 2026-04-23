use crate::config::AppConfig;
use crate::error::Error;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::{Any, connect};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use tracing::info;

pub mod profile_repo;
pub mod rbac_repo;
pub mod user_repo;

#[derive(RustEmbed)]
#[folder = "src/db/migrations/"]
struct Migrations;

pub type Db = Surreal<Any>;

pub fn record_id_key_to_string(key: &RecordIdKey) -> String {
    match key {
        RecordIdKey::String(s) => s.clone(),
        RecordIdKey::Number(n) => n.to_string(),
        RecordIdKey::Uuid(u) => u.to_string(),
        other => format!("{other:?}"),
    }
}

#[derive(Debug, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct MigrationRecord {
    pub id: RecordId,
    pub name: String,
}

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

    run_migrations(&db).await?;

    Ok(db)
}

async fn run_migrations(db: &Surreal<Any>) -> Result<(), Error> {
    info!("Running database migrations...");

    db.query("DEFINE TABLE IF NOT EXISTS _migrations SCHEMALESS")
        .await
        .map_err(|e| {
            Error::upstream(
                "surrealdb",
                format!("Failed to create _migrations table: {}", e),
            )
        })?;

    let mut available_migrations: Vec<String> =
        Migrations::iter().map(|entry| entry.into_owned()).collect();

    available_migrations.sort();

    for file_name in available_migrations {
        let mut response = db
            .query("SELECT * FROM _migrations WHERE name = $name")
            .bind(("name", file_name.clone()))
            .await?;

        let applied: Option<MigrationRecord> = response.take(0)?;

        if applied.is_none() {
            info!("Applying migration: {}", file_name);

            let file = Migrations::get(&file_name).ok_or_else(|| {
                Error::internal(
                    "migration",
                    format!("Migration file not found: {}", file_name),
                )
            })?;
            let query_str = std::str::from_utf8(file.data.as_ref())?;

            match db.query(query_str).await {
                Ok(_) => {
                    let _record: Option<MigrationRecord> = db
                        .query("CREATE _migrations SET name = $name, applied_at = time::now()")
                        .bind(("name", file_name.clone()))
                        .await?
                        .take(0)?;

                    info!("Successfully applied migration: {}", file_name);
                }
                Err(e) => {
                    return Err(Error::internal(
                        "migration",
                        format!("Failed to apply migration {}: {}", file_name, e),
                    ));
                }
            }
        }
    }

    info!("Database schema is fully migrated and up to date.");
    Ok(())
}
