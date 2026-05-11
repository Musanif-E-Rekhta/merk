//! Persistence for refresh-token sessions.
//!
//! Plaintext tokens are never stored. Callers hash with SHA-256 before
//! every read or write. Hash collisions are infeasible at 256 bits, so
//! the unique index over `token_hash` is the lookup key.

use crate::db::Db;
use crate::db::record_id_key_to_string;
use crate::error::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct RefreshToken {
    pub id: Option<RecordId>,
    pub user: RecordId,
    pub token_hash: String,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl RefreshToken {
    /// Active means: not revoked AND not yet expired.
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at > Utc::now()
    }

    /// Returns the user record ID as a plain string ("user:abc"-style key only).
    pub fn user_id_str(&self) -> String {
        record_id_key_to_string(&self.user.key)
    }

    /// Returns the refresh-token record ID as a plain string key.
    pub fn id_str(&self) -> Option<String> {
        self.id.as_ref().map(|r| record_id_key_to_string(&r.key))
    }
}

pub struct RefreshTokenRepo {
    db: Db,
}

impl RefreshTokenRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Insert a new refresh-token row and return its record id.
    pub async fn create(
        &self,
        user_id: &str,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<String, Error> {
        let mut resp = self
            .db
            .query(
                "CREATE refresh_token SET \
                 user = type::record('user', $user_id), \
                 token_hash = $token_hash, \
                 expires_at = $expires_at",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("token_hash", token_hash.to_string()))
            .bind(("expires_at", expires_at))
            .await?;
        let row: Option<RefreshToken> = resp.take(0)?;
        let row = row.ok_or_else(|| {
            Error::internal("refresh_token_repo", "CREATE returned no row")
        })?;
        row.id_str().ok_or_else(|| {
            Error::internal("refresh_token_repo", "created row missing id")
        })
    }

    pub async fn find_active_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM refresh_token \
                 WHERE token_hash = $token_hash \
                 AND revoked_at = NONE \
                 AND expires_at > time::now() \
                 LIMIT 1",
            )
            .bind(("token_hash", token_hash.to_string()))
            .await?;
        let row: Option<RefreshToken> = resp.take(0)?;
        Ok(row)
    }

    pub async fn mark_used(&self, id: &str) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('refresh_token', $id) SET last_used_at = time::now()",
            )
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn revoke(&self, id: &str) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('refresh_token', $id) SET revoked_at = time::now()",
            )
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    /// Idempotent: returns `true` when a row was actually revoked.
    pub async fn revoke_by_hash(&self, token_hash: &str) -> Result<bool, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE refresh_token SET revoked_at = time::now() \
                 WHERE token_hash = $token_hash AND revoked_at = NONE",
            )
            .bind(("token_hash", token_hash.to_string()))
            .await?;
        let updated: Vec<RefreshToken> = resp.take(0)?;
        Ok(!updated.is_empty())
    }
}
