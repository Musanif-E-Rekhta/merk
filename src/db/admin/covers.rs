//! Cover variants — AI-generated candidate book covers per ingestion job.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::{Db, record_id_key_to_string};
use crate::error::Error;

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct CoverVariant {
    pub id: Option<RecordId>,
    pub job: RecordId,
    pub bucket: String,
    pub object: String,
    pub palette: serde_json::Value,
    pub model: Option<RecordId>,
    pub prompt: Option<String>,
    pub generated_at: Option<DateTime<Utc>>,
    pub is_selected: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct CoverVariantResponse {
    pub id: String,
    pub job_id: String,
    pub bucket: String,
    pub object: String,
    pub palette: serde_json::Value,
    pub model_id: Option<String>,
    pub prompt: Option<String>,
    pub is_selected: bool,
}

impl From<CoverVariant> for CoverVariantResponse {
    fn from(c: CoverVariant) -> Self {
        Self {
            id: c
                .id
                .map(|r| record_id_key_to_string(&r.key))
                .unwrap_or_default(),
            job_id: record_id_key_to_string(&c.job.key),
            bucket: c.bucket,
            object: c.object,
            palette: c.palette,
            model_id: c.model.map(|r| record_id_key_to_string(&r.key)),
            prompt: c.prompt,
            is_selected: c.is_selected,
        }
    }
}

pub struct CoversRepo {
    pub db: Db,
}

impl CoversRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn list_variants(
        &self,
        job_id: &str,
    ) -> Result<Vec<CoverVariantResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM cover_variant \
                 WHERE job = type::record('ingestion_job', $jid) ORDER BY generated_at DESC",
            )
            .bind(("jid", job_id.to_string()))
            .await?;
        let rows: Vec<CoverVariant> = resp.take(0)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn create_variant(
        &self,
        job_id: &str,
        bucket: &str,
        object: &str,
        prompt: Option<&str>,
    ) -> Result<CoverVariantResponse, Error> {
        let mut resp = self
            .db
            .query(
                "CREATE cover_variant SET \
                   job = type::record('ingestion_job', $jid), \
                   bucket = $bucket, object = $object, prompt = $prompt \
                 RETURN AFTER",
            )
            .bind(("jid", job_id.to_string()))
            .bind(("bucket", bucket.to_string()))
            .bind(("object", object.to_string()))
            .bind(("prompt", prompt.map(str::to_string)))
            .await?;
        let created: Vec<CoverVariant> = resp.take(0)?;
        Ok(created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("admin", "cover insert failed"))?
            .into())
    }

    /// Mark `variant_id` as selected and clear `is_selected` on all other
    /// variants for the same job. Atomic-ish via two queries.
    pub async fn select_variant(&self, job_id: &str, variant_id: &str) -> Result<bool, Error> {
        self.db
            .query(
                "UPDATE cover_variant SET is_selected = false \
                 WHERE job = type::record('ingestion_job', $jid)",
            )
            .bind(("jid", job_id.to_string()))
            .await?;
        let mut resp = self
            .db
            .query(
                "UPDATE type::record('cover_variant', $id) SET is_selected = true RETURN AFTER",
            )
            .bind(("id", variant_id.to_string()))
            .await?;
        let rows: Vec<CoverVariant> = resp.take(0)?;
        Ok(!rows.is_empty())
    }
}
