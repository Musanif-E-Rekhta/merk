//! SurrealDB-backed `merk_crawl::CrawlStore` + a `Handoff` impl that
//! threads downloaded PDFs into the existing `uploaded_asset` →
//! `ingestion_job` flow.
//!
//! Mutations here do not emit `JobEvent`s directly — the ingestion
//! events fire once the pipeline picks up the new job, exactly as
//! with a human upload.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use merk_blob_store::BlobStore;
use merk_crawl::handoff::{Handoff, HandoffAccept, HandoffError, JobId};
use merk_crawl::store::{
    CrawlRunId, CrawlSourceRow, CrawlStore, CrawlStoreError, CrawlTargetId, CrawlTargetRow,
    RecordDownloadedDto, RecordSkippedDto,
};
use merk_crawl::{CrawlRunSummary, Query};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use surrealdb::types::{RecordId, SurrealValue};
use tracing::{info, warn};

use crate::db::admin::jobs::{CreateJobDto, CreateUploadedAssetDto, JobsRepo};
use crate::db::{Db, record_id_key_to_string};
use crate::error::Error;

// ── Rows ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct CrawlSourceRecord {
    id: Option<RecordId>,
    kind: String,
    base_url: String,
    enabled: bool,
    paused: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct CrawlRunRecord {
    id: Option<RecordId>,
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct CrawlTargetRecord {
    id: Option<RecordId>,
    source: RecordId,
    source_url: String,
    pdf_url: Option<String>,
    pdf_sha256: Option<String>,
    title_raw: Option<String>,
    author_name_raw: Option<String>,
    status: String,
    skip_reason: Option<String>,
    ingest_job: Option<RecordId>,
    book: Option<RecordId>,
    takedown_at: Option<DateTime<Utc>>,
    attempts: i64,
}

impl From<CrawlTargetRecord> for CrawlTargetRow {
    fn from(r: CrawlTargetRecord) -> Self {
        Self {
            id: r
                .id
                .as_ref()
                .map(|i| record_id_key_to_string(&i.key))
                .unwrap_or_default(),
            source_id: record_id_key_to_string(&r.source.key),
            source_url: r.source_url,
            pdf_url: r.pdf_url,
            pdf_sha256: r.pdf_sha256,
            title_raw: r.title_raw,
            author_name_raw: r.author_name_raw,
            status: r.status,
            skip_reason: r.skip_reason,
            ingest_job: r.ingest_job.map(|i| record_id_key_to_string(&i.key)),
            book: r.book.map(|i| record_id_key_to_string(&i.key)),
            takedown_at: r.takedown_at,
            attempts: r.attempts,
        }
    }
}

impl From<CrawlSourceRecord> for CrawlSourceRow {
    fn from(r: CrawlSourceRecord) -> Self {
        Self {
            id: r
                .id
                .as_ref()
                .map(|i| record_id_key_to_string(&i.key))
                .unwrap_or_default(),
            kind: r.kind,
            base_url: r.base_url,
            enabled: r.enabled,
            paused: r.paused,
        }
    }
}

// ── CrawlStore impl ──────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SurrealCrawlStore {
    pub db: Db,
}

impl SurrealCrawlStore {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

fn map_err(e: surrealdb::Error) -> CrawlStoreError {
    CrawlStoreError::Storage(e.to_string())
}

#[async_trait]
impl CrawlStore for SurrealCrawlStore {
    async fn find_source_by_kind(
        &self,
        kind: &str,
    ) -> Result<Option<CrawlSourceRow>, CrawlStoreError> {
        let mut resp = self
            .db
            .query("SELECT * FROM crawl_source WHERE kind = $k LIMIT 1")
            .bind(("k", kind.to_string()))
            .await
            .map_err(map_err)?;
        let rows: Vec<CrawlSourceRecord> = resp.take(0).map_err(map_err)?;
        Ok(rows.into_iter().next().map(Into::into))
    }

    async fn start_run(
        &self,
        source_id: &str,
        query: &Query,
    ) -> Result<CrawlRunId, CrawlStoreError> {
        let query_json = serde_json::to_value(query)
            .map_err(|e| CrawlStoreError::Storage(e.to_string()))?;
        let mut resp = self
            .db
            .query(
                "(CREATE crawl_run SET \
                   source = type::record('crawl_source', $sid), \
                   query = $q, \
                   status = 'running')[0]",
            )
            .bind(("sid", source_id.to_string()))
            .bind(("q", query_json))
            .await
            .map_err(map_err)?;
        let row: Option<CrawlRunRecord> = resp.take(0).map_err(map_err)?;
        let id = row
            .and_then(|r| r.id)
            .ok_or_else(|| CrawlStoreError::Storage("crawl_run insert failed".into()))?;
        Ok(record_id_key_to_string(&id.key))
    }

    async fn finish_run(
        &self,
        run_id: &str,
        status: &str,
    ) -> Result<DateTime<Utc>, CrawlStoreError> {
        let now = Utc::now();
        self.db
            .query(
                "UPDATE type::record('crawl_run', $id) SET \
                   status = $s, finished_at = $now",
            )
            .bind(("id", run_id.to_string()))
            .bind(("s", status.to_string()))
            .bind(("now", now))
            .await
            .map_err(map_err)?;
        Ok(now)
    }

    async fn checkpoint_run(
        &self,
        run_id: &str,
        summary: &CrawlRunSummary,
        last_source_url: &str,
    ) -> Result<(), CrawlStoreError> {
        let ck = serde_json::json!({
            "candidates_seen": summary.candidates_seen,
            "candidates_skipped": summary.candidates_skipped,
            "targets_downloaded": summary.targets_downloaded,
            "targets_handed_off": summary.targets_handed_off,
            "last_source_url": last_source_url,
        });
        self.db
            .query(
                "UPDATE type::record('crawl_run', $id) SET \
                   candidates_seen = $seen, \
                   candidates_skipped = $skipped, \
                   targets_downloaded = $dl, \
                   targets_handed_off = $ho, \
                   last_checkpoint = $ck",
            )
            .bind(("id", run_id.to_string()))
            .bind(("seen", summary.candidates_seen as i64))
            .bind(("skipped", summary.candidates_skipped as i64))
            .bind(("dl", summary.targets_downloaded as i64))
            .bind(("ho", summary.targets_handed_off as i64))
            .bind(("ck", ck))
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn find_target_by_source_url(
        &self,
        source_id: &str,
        source_url: &str,
    ) -> Result<Option<CrawlTargetRow>, CrawlStoreError> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM crawl_target \
                 WHERE source = type::record('crawl_source', $sid) AND source_url = $u \
                 LIMIT 1",
            )
            .bind(("sid", source_id.to_string()))
            .bind(("u", source_url.to_string()))
            .await
            .map_err(map_err)?;
        let rows: Vec<CrawlTargetRecord> = resp.take(0).map_err(map_err)?;
        Ok(rows.into_iter().next().map(Into::into))
    }

    async fn find_target_by_sha256(
        &self,
        sha: &str,
    ) -> Result<Option<CrawlTargetRow>, CrawlStoreError> {
        let mut resp = self
            .db
            .query("SELECT * FROM crawl_target WHERE pdf_sha256 = $s LIMIT 1")
            .bind(("s", sha.to_string()))
            .await
            .map_err(map_err)?;
        let rows: Vec<CrawlTargetRecord> = resp.take(0).map_err(map_err)?;
        Ok(rows.into_iter().next().map(Into::into))
    }

    async fn record_skipped(
        &self,
        dto: RecordSkippedDto,
    ) -> Result<CrawlTargetId, CrawlStoreError> {
        let mut resp = self
            .db
            .query(
                "(CREATE crawl_target SET \
                   source = type::record('crawl_source', $sid), \
                   source_url = $url, \
                   title_raw = $title, author_name_raw = $author, \
                   status = 'skipped', skip_reason = $reason)[0]",
            )
            .bind(("sid", dto.source_id))
            .bind(("url", dto.candidate.source_url))
            .bind(("title", dto.candidate.title_raw))
            .bind(("author", dto.candidate.author_name_raw))
            .bind(("reason", dto.reason.as_str().to_string()))
            .await
            .map_err(map_err)?;
        let row: Option<CrawlTargetRecord> = resp.take(0).map_err(map_err)?;
        let id = row
            .and_then(|r| r.id)
            .ok_or_else(|| CrawlStoreError::Storage("crawl_target insert failed".into()))?;
        Ok(record_id_key_to_string(&id.key))
    }

    async fn record_downloaded(
        &self,
        dto: RecordDownloadedDto,
    ) -> Result<CrawlTargetId, CrawlStoreError> {
        let mut resp = self
            .db
            .query(
                "(CREATE crawl_target SET \
                   source = type::record('crawl_source', $sid), \
                   source_url = $url, \
                   pdf_url = $pdf, pdf_sha256 = $sha, pdf_etag = $etag, \
                   pdf_last_modified = $lm, size_bytes = $sz, \
                   title_raw = $title, author_name_raw = $author, \
                   language_hint = $lang, year_hint = $year, \
                   cover_url = $cover, hints_json = $hints, \
                   status = 'downloaded')[0]",
            )
            .bind(("sid", dto.source_id))
            .bind(("url", dto.resolved.source_url))
            .bind(("pdf", dto.resolved.pdf_url))
            .bind(("sha", dto.sha256))
            .bind(("etag", dto.etag))
            .bind(("lm", dto.last_modified))
            .bind(("sz", dto.size_bytes))
            .bind(("title", dto.resolved.title_raw))
            .bind(("author", dto.resolved.author_name_raw))
            .bind(("lang", dto.resolved.language_hint))
            .bind(("year", dto.resolved.year_hint))
            .bind(("cover", dto.resolved.cover_url))
            .bind(("hints", dto.resolved.hints_json))
            .await
            .map_err(map_err)?;
        let row: Option<CrawlTargetRecord> = resp.take(0).map_err(map_err)?;
        let id = row
            .and_then(|r| r.id)
            .ok_or_else(|| CrawlStoreError::Storage("crawl_target insert failed".into()))?;
        Ok(record_id_key_to_string(&id.key))
    }

    async fn record_handed_off(
        &self,
        target_id: &str,
        job_id: &str,
    ) -> Result<(), CrawlStoreError> {
        self.db
            .query(
                "UPDATE type::record('crawl_target', $id) SET \
                   status = 'handed_off', \
                   ingest_job = type::record('ingestion_job', $jid)",
            )
            .bind(("id", target_id.to_string()))
            .bind(("jid", job_id.to_string()))
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

// ── Higher-level admin operations ────────────────────────────────────────────
//
// These wrap admin GraphQL mutations from the plan §9 (upsertCrawlSource,
// setCrawlSourcePaused, startCrawlRun, etc.). They're plain methods on the
// repo so both the GraphQL layer and the CLI binary can call them.

impl SurrealCrawlStore {
    pub async fn upsert_source(
        &self,
        kind: &str,
        base_url: &str,
        enabled: bool,
    ) -> Result<CrawlSourceRow, Error> {
        let mut resp = self
            .db
            .query(
                "UPSERT crawl_source CONTENT { \
                   kind: $k, base_url: $u, enabled: $e \
                 } WHERE kind = $k; \
                 SELECT * FROM crawl_source WHERE kind = $k LIMIT 1",
            )
            .bind(("k", kind.to_string()))
            .bind(("u", base_url.to_string()))
            .bind(("e", enabled))
            .await?;
        // Two statements; take(1) is the SELECT.
        let rows: Vec<CrawlSourceRecord> = resp.take(1)?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("admin", "crawl_source upsert failed"))?;
        Ok(row.into())
    }

    pub async fn set_paused(&self, kind: &str, paused: bool) -> Result<(), Error> {
        self.db
            .query("UPDATE crawl_source SET paused = $p WHERE kind = $k")
            .bind(("p", paused))
            .bind(("k", kind.to_string()))
            .await?;
        Ok(())
    }

    pub async fn list_sources(&self) -> Result<Vec<CrawlSourceRow>, Error> {
        let mut resp = self
            .db
            .query("SELECT * FROM crawl_source ORDER BY kind ASC")
            .await?;
        let rows: Vec<CrawlSourceRecord> = resp.take(0)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_targets(
        &self,
        source_kind: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CrawlTargetRow>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM crawl_target \
                 WHERE ($sk = NONE OR source.kind = $sk) \
                   AND ($st = NONE OR status = $st) \
                 ORDER BY created_at DESC LIMIT $l START $o",
            )
            .bind(("sk", source_kind.map(str::to_string)))
            .bind(("st", status.map(str::to_string)))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;
        let rows: Vec<CrawlTargetRecord> = resp.take(0)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn requeue_target(&self, target_id: &str) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('crawl_target', $id) SET \
                   status = 'new', \
                   pdf_sha256 = NONE, \
                   attempts = attempts + 1",
            )
            .bind(("id", target_id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn record_takedown(&self, target_id: &str) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('crawl_target', $id) SET \
                   status = 'takedown', \
                   takedown_at = time::now()",
            )
            .bind(("id", target_id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn blacklist_target(
        &self,
        target_id: &str,
        reason: &str,
    ) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('crawl_target', $id) SET \
                   status = 'skipped', skip_reason = $r",
            )
            .bind(("id", target_id.to_string()))
            .bind(("r", reason.to_string()))
            .await?;
        Ok(())
    }

    pub async fn get_target(&self, target_id: &str) -> Result<Option<CrawlTargetRow>, Error> {
        let mut resp = self
            .db
            .query("SELECT * FROM type::record('crawl_target', $id)")
            .bind(("id", target_id.to_string()))
            .await?;
        let rows: Vec<CrawlTargetRecord> = resp.take(0)?;
        Ok(rows.into_iter().next().map(Into::into))
    }

    pub async fn cancel_run(&self, run_id: &str) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('crawl_run', $id) SET \
                   status = 'cancelled', finished_at = time::now()",
            )
            .bind(("id", run_id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn list_runs(
        &self,
        source_kind: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CrawlRunSummaryRow>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT \
                   meta::id(id) AS id, \
                   meta::id(source) AS source_id, \
                   source.kind AS source_kind, \
                   started_at, finished_at, status, \
                   candidates_seen, candidates_skipped, \
                   targets_downloaded, targets_handed_off \
                 FROM crawl_run \
                 WHERE ($sk = NONE OR source.kind = $sk) \
                 ORDER BY started_at DESC LIMIT $l START $o",
            )
            .bind(("sk", source_kind.map(str::to_string)))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;
        let rows: Vec<CrawlRunSummaryRow> = resp.take(0)?;
        Ok(rows)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct CrawlRunSummaryRow {
    pub id: String,
    pub source_id: String,
    pub source_kind: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: String,
    pub candidates_seen: i64,
    pub candidates_skipped: i64,
    pub targets_downloaded: i64,
    pub targets_handed_off: i64,
}

// ── Handoff impl ─────────────────────────────────────────────────────────────

/// Bridges a finished crawl-target into the existing ingestion pipeline.
/// The bytes land in the configured blob store; an `uploaded_asset`
/// row records the storage; a new `ingestion_job` runs the same
/// 5-stage flow a human upload would.
#[derive(Clone)]
pub struct IngestHandoff {
    pub jobs: JobsRepo,
    pub blob_store: Arc<dyn BlobStore>,
    /// User id rows are created under. In Phase 2 the admin GraphQL
    /// layer rebuilds this with the caller's `claims.sub`; the
    /// Services-level default is empty and the call fails clearly.
    pub user_id: String,
}

impl IngestHandoff {
    pub fn new(jobs: JobsRepo, blob_store: Arc<dyn BlobStore>, user_id: String) -> Self {
        Self {
            jobs,
            blob_store,
            user_id,
        }
    }

    pub fn with_user(&self, user_id: String) -> Self {
        Self {
            jobs: self.jobs.clone(),
            blob_store: self.blob_store.clone(),
            user_id,
        }
    }
}

#[async_trait]
impl Handoff for IngestHandoff {
    async fn accept(&self, dto: HandoffAccept) -> Result<JobId, HandoffError> {
        if self.user_id.is_empty() {
            return Err(HandoffError::Rejected(
                "crawler user_id not configured".into(),
            ));
        }
        let object = format!("crawl/{}.pdf", dto.sha256);
        self.blob_store
            .put(&object, &dto.pdf_bytes)
            .await
            .map_err(|e| HandoffError::Blob(e.to_string()))?;

        let asset = self
            .jobs
            .create_uploaded_asset(
                &self.user_id,
                CreateUploadedAssetDto {
                    filename: dto.filename,
                    mime: "application/pdf".into(),
                    size_bytes: dto.pdf_bytes.len() as i64,
                    bucket: self.blob_store.bucket().to_string(),
                    object,
                },
            )
            .await
            .map_err(|e| HandoffError::Storage(e.to_string()))?;

        let job = self
            .jobs
            .create_job(
                &self.user_id,
                CreateJobDto {
                    asset_id: asset.id,
                    hint_title: dto.title_hint,
                    hint_author: dto.author_hint,
                },
            )
            .await
            .map_err(|e| HandoffError::Storage(e.to_string()))?;

        info!(target_id = %dto.target_id, job_id = %job.id, "crawled PDF handed off to ingest");
        Ok(job.id)
    }
}

// `warn` is used in the GraphQL layer; keep the import linked here so a
// future refactor can route per-call failures through a single helper.
#[allow(dead_code)]
fn _silence_warn(s: &str) {
    warn!("{s}");
}
