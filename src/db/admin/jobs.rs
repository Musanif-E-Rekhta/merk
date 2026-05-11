//! Ingestion-job lifecycle: uploaded_asset → ingestion_job → pipeline_step
//! → job_log_entry. The pipeline worker (services::pipeline) drives state
//! transitions; this repo is the persistence layer.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::{Db, record_id_key_to_string};
use crate::error::Error;
use crate::services::event_bus::{EventBus, JobEvent};

// ── Uploaded asset ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct UploadedAsset {
    pub id: Option<RecordId>,
    pub user: RecordId,
    pub filename: String,
    pub mime: String,
    pub size_bytes: i64,
    pub bucket: String,
    pub object: String,
    pub status: String,
    pub page_count: Option<i64>,
    pub sha256: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub finalized_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct UploadedAssetResponse {
    pub id: String,
    pub filename: String,
    pub mime: String,
    pub size_bytes: i64,
    pub bucket: String,
    pub object: String,
    pub status: String,
    pub page_count: Option<i64>,
}

impl From<UploadedAsset> for UploadedAssetResponse {
    fn from(a: UploadedAsset) -> Self {
        Self {
            id: a
                .id
                .map(|r| record_id_key_to_string(&r.key))
                .unwrap_or_default(),
            filename: a.filename,
            mime: a.mime,
            size_bytes: a.size_bytes,
            bucket: a.bucket,
            object: a.object,
            status: a.status,
            page_count: a.page_count,
        }
    }
}

pub struct CreateUploadedAssetDto {
    pub filename: String,
    pub mime: String,
    pub size_bytes: i64,
    pub bucket: String,
    pub object: String,
}

// ── Ingestion job ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct IngestionJob {
    pub id: Option<RecordId>,
    pub asset: RecordId,
    pub book: Option<RecordId>,
    pub hint_title: Option<String>,
    pub hint_title_ur: Option<String>,
    pub hint_author: Option<String>,
    pub pages: Option<i64>,
    pub size_bytes: Option<i64>,
    pub stage: i64,
    pub status: String,
    pub ai_provider: Option<String>,
    pub ai_model: Option<String>,
    pub overall_confidence: Option<f64>,
    pub chapters_total: Option<i64>,
    pub chapters_flagged: Option<i64>,
    pub tokens_used: i64,
    pub est_cost_usd: f64,
    pub cover_color: Option<String>,
    pub cover_glyph: Option<String>,
    pub created_by: RecordId,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct IngestionJobResponse {
    pub id: String,
    pub asset_id: String,
    pub book_id: Option<String>,
    pub hint_title: Option<String>,
    pub hint_title_ur: Option<String>,
    pub hint_author: Option<String>,
    pub pages: Option<i64>,
    pub size_bytes: Option<i64>,
    pub stage: i64,
    pub status: String,
    pub ai_provider: Option<String>,
    pub ai_model: Option<String>,
    pub overall_confidence: Option<f64>,
    pub chapters_total: Option<i64>,
    pub chapters_flagged: Option<i64>,
    pub tokens_used: i64,
    pub est_cost_usd: f64,
    pub cover_color: Option<String>,
    pub cover_glyph: Option<String>,
}

impl From<IngestionJob> for IngestionJobResponse {
    fn from(j: IngestionJob) -> Self {
        Self {
            id: j
                .id
                .map(|r| record_id_key_to_string(&r.key))
                .unwrap_or_default(),
            asset_id: record_id_key_to_string(&j.asset.key),
            book_id: j.book.map(|r| record_id_key_to_string(&r.key)),
            hint_title: j.hint_title,
            hint_title_ur: j.hint_title_ur,
            hint_author: j.hint_author,
            pages: j.pages,
            size_bytes: j.size_bytes,
            stage: j.stage,
            status: j.status,
            ai_provider: j.ai_provider,
            ai_model: j.ai_model,
            overall_confidence: j.overall_confidence,
            chapters_total: j.chapters_total,
            chapters_flagged: j.chapters_flagged,
            tokens_used: j.tokens_used,
            est_cost_usd: j.est_cost_usd,
            cover_color: j.cover_color,
            cover_glyph: j.cover_glyph,
        }
    }
}

pub struct CreateJobDto {
    pub asset_id: String,
    pub hint_title: Option<String>,
    pub hint_author: Option<String>,
}

pub struct UpdateJobConfigDto {
    pub ai_provider: Option<String>,
    pub ai_model: Option<String>,
}

#[derive(Default)]
pub struct JobListFilters {
    pub status: Option<String>,
    pub stage: Option<i64>,
}

// ── Pipeline step + log ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct PipelineStep {
    pub id: Option<RecordId>,
    pub job: RecordId,
    pub n: i64,
    pub label: String,
    pub status: String,
    pub detail: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct PipelineStepResponse {
    pub id: String,
    pub n: i64,
    pub label: String,
    pub status: String,
    pub detail: Option<String>,
    #[schemars(with = "Option<String>")]
    pub started_at: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub finished_at: Option<DateTime<Utc>>,
}

impl From<PipelineStep> for PipelineStepResponse {
    fn from(s: PipelineStep) -> Self {
        Self {
            id: s
                .id
                .map(|r| record_id_key_to_string(&r.key))
                .unwrap_or_default(),
            n: s.n,
            label: s.label,
            status: s.status,
            detail: s.detail,
            started_at: s.started_at,
            finished_at: s.finished_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct JobLogEntry {
    pub id: Option<RecordId>,
    pub job: RecordId,
    pub t: Option<DateTime<Utc>>,
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct JobLogEntryResponse {
    pub id: String,
    #[schemars(with = "Option<String>")]
    pub t: Option<DateTime<Utc>>,
    pub kind: String,
    pub message: String,
}

impl From<JobLogEntry> for JobLogEntryResponse {
    fn from(e: JobLogEntry) -> Self {
        Self {
            id: e
                .id
                .map(|r| record_id_key_to_string(&r.key))
                .unwrap_or_default(),
            t: e.t,
            kind: e.kind,
            message: e.message,
        }
    }
}

// ── Repo ──────────────────────────────────────────────────────────────────────

const STEP_LABELS: &[&str] = &[
    "PDF parse",
    "OCR",
    "Chapter detection",
    "Summarization",
    "Embeddings",
    "Cover generation",
    "QA gate",
];

#[derive(Clone)]
pub struct JobsRepo {
    pub db: Db,
}

impl JobsRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    // ── Uploaded asset ──────────────────────────────────────────────────────

    pub async fn create_uploaded_asset(
        &self,
        user_id: &str,
        dto: CreateUploadedAssetDto,
    ) -> Result<UploadedAssetResponse, Error> {
        let mut resp = self
            .db
            .query(
                "CREATE uploaded_asset SET \
                   user = type::record('user', $uid), \
                   filename = $filename, mime = $mime, size_bytes = $size, \
                   bucket = $bucket, object = $object, \
                   status = 'ready', finalized_at = time::now()",
            )
            .bind(("uid", user_id.to_string()))
            .bind(("filename", dto.filename))
            .bind(("mime", dto.mime))
            .bind(("size", dto.size_bytes))
            .bind(("bucket", dto.bucket))
            .bind(("object", dto.object))
            .await?;
        let created: Vec<UploadedAsset> = resp.take(0)?;
        Ok(created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("admin", "uploaded_asset insert failed"))?
            .into())
    }

    pub async fn get_uploaded_asset(
        &self,
        asset_id: &str,
    ) -> Result<Option<UploadedAsset>, Error> {
        let mut resp = self
            .db
            .query("SELECT * FROM type::record('uploaded_asset', $id)")
            .bind(("id", asset_id.to_string()))
            .await?;
        Ok(resp.take(0)?)
    }

    // ── Ingestion job ───────────────────────────────────────────────────────

    pub async fn create_job(
        &self,
        user_id: &str,
        dto: CreateJobDto,
    ) -> Result<IngestionJobResponse, Error> {
        let mut resp = self
            .db
            .query(
                "LET $asset = type::record('uploaded_asset', $aid); \
                 LET $job = (CREATE ingestion_job SET \
                   asset = $asset, \
                   hint_title = $title, hint_author = $author, \
                   pages = $asset.page_count, size_bytes = $asset.size_bytes, \
                   stage = 1, status = 'queued', \
                   ai_provider = 'claude', ai_model = 'claude-sonnet-4-6', \
                   created_by = type::record('user', $uid))[0]; \
                 RETURN $job",
            )
            .bind(("uid", user_id.to_string()))
            .bind(("aid", dto.asset_id))
            .bind(("title", dto.hint_title))
            .bind(("author", dto.hint_author))
            .await?;

        let job: Option<IngestionJob> = resp.take(2)?;
        let job = job.ok_or_else(|| Error::internal("admin", "ingestion_job insert failed"))?;
        let job_id = record_id_key_to_string(
            &job.id
                .as_ref()
                .ok_or_else(|| Error::internal("admin", "missing job id"))?
                .key,
        );

        // Seed the seven fixed pipeline_step rows so step queries are
        // always populated, even before the worker starts.
        for (i, label) in STEP_LABELS.iter().enumerate() {
            self.db
                .query(
                    "CREATE pipeline_step SET \
                       job = type::record('ingestion_job', $jid), \
                       n = $n, label = $label, status = 'pending'",
                )
                .bind(("jid", job_id.clone()))
                .bind(("n", (i as i64) + 1))
                .bind(("label", label.to_string()))
                .await?;
        }

        Ok(job.into())
    }

    pub async fn get_job(&self, job_id: &str) -> Result<Option<IngestionJobResponse>, Error> {
        let mut resp = self
            .db
            .query("SELECT * FROM type::record('ingestion_job', $id)")
            .bind(("id", job_id.to_string()))
            .await?;
        let job: Option<IngestionJob> = resp.take(0)?;
        Ok(job.map(Into::into))
    }

    pub async fn list_jobs(
        &self,
        f: &JobListFilters,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<IngestionJobResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM ingestion_job \
                 WHERE ($status = NONE OR status = $status) \
                   AND ($stage  = NONE OR stage  = $stage) \
                 ORDER BY created_at DESC LIMIT $l START $o",
            )
            .bind(("status", f.status.clone()))
            .bind(("stage", f.stage))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;
        let rows: Vec<IngestionJob> = resp.take(0)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn set_status(&self, job_id: &str, status: &str) -> Result<(), Error> {
        // Track completed_at when we land in a terminal state.
        let completed = matches!(status, "completed" | "failed");
        self.db
            .query(if completed {
                "UPDATE type::record('ingestion_job', $id) SET status = $s, completed_at = time::now()"
            } else {
                "UPDATE type::record('ingestion_job', $id) SET status = $s"
            })
            .bind(("id", job_id.to_string()))
            .bind(("s", status.to_string()))
            .await?;
        Ok(())
    }

    pub async fn set_stage(&self, job_id: &str, stage: i64) -> Result<(), Error> {
        self.db
            .query("UPDATE type::record('ingestion_job', $id) SET stage = $stage")
            .bind(("id", job_id.to_string()))
            .bind(("stage", stage))
            .await?;
        Ok(())
    }

    pub async fn update_config(
        &self,
        job_id: &str,
        dto: UpdateJobConfigDto,
    ) -> Result<Option<IngestionJobResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE type::record('ingestion_job', $id) SET \
                   ai_provider = $provider, ai_model = $model \
                 RETURN AFTER",
            )
            .bind(("id", job_id.to_string()))
            .bind(("provider", dto.ai_provider))
            .bind(("model", dto.ai_model))
            .await?;
        let updated: Vec<IngestionJob> = resp.take(0)?;
        Ok(updated.into_iter().next().map(Into::into))
    }

    // ── Pipeline steps ──────────────────────────────────────────────────────

    pub async fn list_steps(&self, job_id: &str) -> Result<Vec<PipelineStepResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM pipeline_step \
                 WHERE job = type::record('ingestion_job', $id) ORDER BY n",
            )
            .bind(("id", job_id.to_string()))
            .await?;
        let rows: Vec<PipelineStep> = resp.take(0)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Update a step's status, then publish a `StepUpdate` event so live
    /// subscribers see the change without polling.
    pub async fn update_step(
        &self,
        bus: &EventBus,
        job_id: &str,
        n: i64,
        status: &str,
        detail: Option<String>,
    ) -> Result<(), Error> {
        let now_running = status == "running";
        let now_terminal = matches!(status, "done" | "failed");
        let mut resp = self
            .db
            .query(
                "UPDATE pipeline_step SET \
                   status = $status, detail = $detail, \
                   started_at = (IF $running AND started_at = NONE THEN time::now() ELSE started_at END), \
                   finished_at = (IF $terminal THEN time::now() ELSE finished_at END) \
                 WHERE job = type::record('ingestion_job', $id) AND n = $n \
                 RETURN AFTER",
            )
            .bind(("id", job_id.to_string()))
            .bind(("n", n))
            .bind(("status", status.to_string()))
            .bind(("detail", detail))
            .bind(("running", now_running))
            .bind(("terminal", now_terminal))
            .await?;

        let updated: Vec<PipelineStep> = resp.take(0)?;
        if let Some(s) = updated.into_iter().next() {
            bus.publish_job(JobEvent::StepUpdate {
                job_id: job_id.to_string(),
                n: s.n,
                label: s.label,
                status: s.status,
                detail: s.detail,
                started_at: s.started_at,
                finished_at: s.finished_at,
            });
        }
        Ok(())
    }

    // ── Job log ─────────────────────────────────────────────────────────────

    pub async fn append_log(
        &self,
        bus: &EventBus,
        job_id: &str,
        kind: &str,
        message: &str,
    ) -> Result<(), Error> {
        let mut resp = self
            .db
            .query(
                "CREATE job_log_entry SET \
                   job = type::record('ingestion_job', $id), \
                   kind = $kind, message = $msg \
                 RETURN AFTER",
            )
            .bind(("id", job_id.to_string()))
            .bind(("kind", kind.to_string()))
            .bind(("msg", message.to_string()))
            .await?;
        let created: Vec<JobLogEntry> = resp.take(0)?;
        if let Some(e) = created.into_iter().next() {
            bus.publish_job(JobEvent::LogEntry {
                job_id: job_id.to_string(),
                t: e.t.unwrap_or_else(chrono::Utc::now),
                level: e.kind,
                message: e.message,
            });
        }
        Ok(())
    }

    pub async fn list_log(
        &self,
        job_id: &str,
        since_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<JobLogEntryResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM job_log_entry \
                 WHERE job = type::record('ingestion_job', $id) \
                   AND ($since = NONE OR id > type::record('job_log_entry', $since)) \
                 ORDER BY t ASC LIMIT $l",
            )
            .bind(("id", job_id.to_string()))
            .bind(("since", since_id.map(str::to_string)))
            .bind(("l", limit))
            .await?;
        let rows: Vec<JobLogEntry> = resp.take(0)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
