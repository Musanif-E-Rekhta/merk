//! SurrealDB-backed `RbacStore` impl, wrapped in a `merk_rbac::Authorizer`.
//!
//! The roles/permissions schema (migration 0002) stores a graph
//! `user -> assigned_role -> role -> has_permission -> permission`. This
//! file walks that graph to satisfy the trait; everything else (caching,
//! permission DSL, gate helpers) lives in `merk_rbac`.

use async_trait::async_trait;
use merk_rbac::{Authorizer, RbacError, RbacStore};
use surrealdb::types::SurrealValue;

use crate::db::Db;
use crate::error::Error;

#[derive(Clone)]
pub struct SurrealRbacStore {
    db: Db,
}

impl SurrealRbacStore {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RbacStore for SurrealRbacStore {
    async fn user_roles(&self, user_id: &str) -> Result<Vec<String>, RbacError> {
        #[derive(serde::Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct R {
            name: String,
        }
        let mut resp = self
            .db
            .query("SELECT name FROM type::record('user', $u)->assigned_role->role")
            .bind(("u", user_id.to_string()))
            .await
            .map_err(|e| RbacError::Storage(e.to_string()))?;
        let rows: Vec<R> = resp
            .take(0)
            .map_err(|e| RbacError::Storage(e.to_string()))?;
        Ok(rows.into_iter().map(|r| r.name).collect())
    }

    async fn user_permissions(&self, user_id: &str) -> Result<Vec<String>, RbacError> {
        #[derive(serde::Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct P {
            name: String,
        }
        let mut resp = self
            .db
            .query(
                "SELECT name FROM \
                 type::record('user', $u)->assigned_role->role->has_permission->permission",
            )
            .bind(("u", user_id.to_string()))
            .await
            .map_err(|e| RbacError::Storage(e.to_string()))?;
        let rows: Vec<P> = resp
            .take(0)
            .map_err(|e| RbacError::Storage(e.to_string()))?;
        Ok(rows.into_iter().map(|r| r.name).collect())
    }
}

/// Domain-specific facade: pre-wires the `SurrealRbacStore` into a
/// `merk_rbac::Authorizer` and maps `RbacError` → merk `Error`.
pub struct RbacService {
    inner: Authorizer<SurrealRbacStore>,
}

impl RbacService {
    pub fn new(db: Db) -> Self {
        Self {
            inner: Authorizer::new(SurrealRbacStore::new(db)),
        }
    }

    pub async fn user_roles(&self, user_id: &str) -> Result<Vec<String>, Error> {
        Ok(self.inner.user_roles(user_id).await?)
    }

    pub async fn require_admin(&self, user_id: &str) -> Result<(), Error> {
        Ok(self.inner.require_admin(user_id).await?)
    }
}
