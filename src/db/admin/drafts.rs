//! Chapter drafts + PDF page cache (review/edit data).

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::{Db, record_id_key_to_string};
use crate::error::Error;
use crate::services::event_bus::{DraftEvent, EventBus, JobEvent};

// ── Chapter draft ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct ChapterDraft {
    pub id: Option<RecordId>,
    pub job: RecordId,
    pub n: i64,
    pub title_ur: String,
    pub title_en: Option<String>,
    pub page_range: String,
    pub ai_content: String,
    pub ai_content_format: String,
    pub human_content: Option<String>,
    pub ai_summary: Option<String>,
    pub themes: Vec<String>,
    pub entities: Vec<String>,
    pub confidence: f64,
    pub status: String,
    pub flag_reason: Option<String>,
    pub approved_by: Option<RecordId>,
    pub approved_at: Option<DateTime<Utc>>,
    pub pages_re_ocr_count: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ChapterDraftResponse {
    pub id: String,
    pub job_id: String,
    pub n: i64,
    pub title_ur: String,
    pub title_en: Option<String>,
    pub page_range: String,
    pub ai_content: String,
    pub ai_content_format: String,
    pub human_content: Option<String>,
    pub ai_summary: Option<String>,
    pub themes: Vec<String>,
    pub entities: Vec<String>,
    pub confidence: f64,
    pub status: String,
    pub flag_reason: Option<String>,
    pub approved_by_id: Option<String>,
    #[schemars(with = "Option<String>")]
    pub approved_at: Option<DateTime<Utc>>,
    pub pages_re_ocr_count: i64,
}

impl From<ChapterDraft> for ChapterDraftResponse {
    fn from(d: ChapterDraft) -> Self {
        Self {
            id: d
                .id
                .map(|r| record_id_key_to_string(&r.key))
                .unwrap_or_default(),
            job_id: record_id_key_to_string(&d.job.key),
            n: d.n,
            title_ur: d.title_ur,
            title_en: d.title_en,
            page_range: d.page_range,
            ai_content: d.ai_content,
            ai_content_format: d.ai_content_format,
            human_content: d.human_content,
            ai_summary: d.ai_summary,
            themes: d.themes,
            entities: d.entities,
            confidence: d.confidence,
            status: d.status,
            flag_reason: d.flag_reason,
            approved_by_id: d.approved_by.map(|r| record_id_key_to_string(&r.key)),
            approved_at: d.approved_at,
            pages_re_ocr_count: d.pages_re_ocr_count,
        }
    }
}

pub struct CreateDraftDto {
    pub job_id: String,
    pub n: i64,
    pub title_ur: String,
    pub title_en: Option<String>,
    pub page_range: String,
    pub ai_content: String,
    pub ai_summary: Option<String>,
    pub themes: Vec<String>,
    pub entities: Vec<String>,
    pub confidence: f64,
}

pub struct UpdateDraftDto {
    pub human_content: Option<String>,
    pub title_ur: Option<String>,
    pub title_en: Option<String>,
    pub page_range: Option<String>,
}

// ── PDF page ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct PdfPage {
    pub id: Option<RecordId>,
    pub job: RecordId,
    pub n: i64,
    pub bucket: String,
    pub object: String,
    pub ocr_text: Option<String>,
    pub ocr_confidence: Option<f64>,
    pub retried_count: i64,
    pub rendered_at: Option<DateTime<Utc>>,
}

// ── Repo ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DraftsRepo {
    pub db: Db,
}

impl DraftsRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn list_drafts(
        &self,
        job_id: &str,
    ) -> Result<Vec<ChapterDraftResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM chapter_draft \
                 WHERE job = type::record('ingestion_job', $jid) ORDER BY n",
            )
            .bind(("jid", job_id.to_string()))
            .await?;
        let rows: Vec<ChapterDraft> = resp.take(0)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_draft(&self, draft_id: &str) -> Result<Option<ChapterDraftResponse>, Error> {
        let mut resp = self
            .db
            .query("SELECT * FROM type::record('chapter_draft', $id)")
            .bind(("id", draft_id.to_string()))
            .await?;
        let d: Option<ChapterDraft> = resp.take(0)?;
        Ok(d.map(Into::into))
    }

    pub async fn create_draft(
        &self,
        bus: &EventBus,
        dto: CreateDraftDto,
    ) -> Result<ChapterDraftResponse, Error> {
        let initial_status = if dto.confidence < 0.7 { "flag" } else { "ok" };
        let mut resp = self
            .db
            .query(
                "CREATE chapter_draft SET \
                   job = type::record('ingestion_job', $jid), \
                   n = $n, title_ur = $title_ur, title_en = $title_en, \
                   page_range = $page_range, ai_content = $ai_content, \
                   ai_content_format = 'markdown', \
                   ai_summary = $ai_summary, themes = $themes, entities = $entities, \
                   confidence = $confidence, status = $status \
                 RETURN AFTER",
            )
            .bind(("jid", dto.job_id.clone()))
            .bind(("n", dto.n))
            .bind(("title_ur", dto.title_ur))
            .bind(("title_en", dto.title_en))
            .bind(("page_range", dto.page_range))
            .bind(("ai_content", dto.ai_content))
            .bind(("ai_summary", dto.ai_summary))
            .bind(("themes", dto.themes))
            .bind(("entities", dto.entities))
            .bind(("confidence", dto.confidence))
            .bind(("status", initial_status.to_string()))
            .await?;
        let created: Vec<ChapterDraft> = resp.take(0)?;
        let draft = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("admin", "draft insert failed"))?;
        let resp: ChapterDraftResponse = draft.into();

        bus.publish_job(JobEvent::ChapterDraftAdded {
            job_id: dto.job_id,
            draft_id: resp.id.clone(),
            n: resp.n,
        });
        Ok(resp)
    }

    pub async fn update_draft(
        &self,
        bus: &EventBus,
        draft_id: &str,
        dto: UpdateDraftDto,
    ) -> Result<Option<ChapterDraftResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE type::record('chapter_draft', $id) SET \
                   human_content = $human_content ?? human_content, \
                   title_ur      = $title_ur      ?? title_ur, \
                   title_en      = $title_en      ?? title_en, \
                   page_range    = $page_range    ?? page_range \
                 RETURN AFTER",
            )
            .bind(("id", draft_id.to_string()))
            .bind(("human_content", dto.human_content))
            .bind(("title_ur", dto.title_ur))
            .bind(("title_en", dto.title_en))
            .bind(("page_range", dto.page_range))
            .await?;
        let rows: Vec<ChapterDraft> = resp.take(0)?;
        let updated = rows.into_iter().next().map(ChapterDraftResponse::from);

        if let Some(ref r) = updated {
            bus.publish_draft(DraftEvent::DraftUpdated {
                job_id: r.job_id.clone(),
                draft_id: r.id.clone(),
            });
        }
        Ok(updated)
    }

    pub async fn approve_draft(
        &self,
        bus: &EventBus,
        draft_id: &str,
        user_id: &str,
    ) -> Result<bool, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE type::record('chapter_draft', $id) SET \
                   status = 'approved', \
                   approved_by = type::record('user', $uid), \
                   approved_at = time::now() \
                 RETURN AFTER",
            )
            .bind(("id", draft_id.to_string()))
            .bind(("uid", user_id.to_string()))
            .await?;
        let rows: Vec<ChapterDraft> = resp.take(0)?;
        if let Some(d) = rows.into_iter().next() {
            bus.publish_draft(DraftEvent::DraftApproved {
                job_id: record_id_key_to_string(&d.job.key),
                draft_id: draft_id.to_string(),
                by_user: user_id.to_string(),
            });
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn flag_draft(
        &self,
        bus: &EventBus,
        draft_id: &str,
        reason: &str,
    ) -> Result<bool, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE type::record('chapter_draft', $id) SET \
                   status = 'flag', flag_reason = $reason \
                 RETURN AFTER",
            )
            .bind(("id", draft_id.to_string()))
            .bind(("reason", reason.to_string()))
            .await?;
        let rows: Vec<ChapterDraft> = resp.take(0)?;
        if let Some(d) = rows.into_iter().next() {
            bus.publish_draft(DraftEvent::DraftFlagged {
                job_id: record_id_key_to_string(&d.job.key),
                draft_id: draft_id.to_string(),
                reason: reason.to_string(),
            });
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn reject_draft(&self, draft_id: &str) -> Result<bool, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE type::record('chapter_draft', $id) SET status = 'rejected' RETURN AFTER",
            )
            .bind(("id", draft_id.to_string()))
            .await?;
        let rows: Vec<ChapterDraft> = resp.take(0)?;
        Ok(!rows.is_empty())
    }

    /// Bump pages_re_ocr_count and reset status to ok so the worker
    /// re-evaluates. The actual re-OCR is queued; this just marks intent.
    pub async fn re_ocr_request(
        &self,
        draft_id: &str,
        _pages: &str,
    ) -> Result<bool, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE type::record('chapter_draft', $id) SET \
                   pages_re_ocr_count = pages_re_ocr_count + 1, \
                   status = 'ok' \
                 RETURN AFTER",
            )
            .bind(("id", draft_id.to_string()))
            .await?;
        let rows: Vec<ChapterDraft> = resp.take(0)?;
        Ok(!rows.is_empty())
    }

    // ── PDF page ────────────────────────────────────────────────────────────

    pub async fn get_pdf_page(&self, job_id: &str, n: i64) -> Result<Option<PdfPage>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM pdf_page \
                 WHERE job = type::record('ingestion_job', $jid) AND n = $n LIMIT 1",
            )
            .bind(("jid", job_id.to_string()))
            .bind(("n", n))
            .await?;
        Ok(resp.take(0)?)
    }

    /// Idempotent upsert: re-running the parser overwrites the prior
    /// row instead of failing on the unique `(job, n)` index.
    pub async fn upsert_pdf_page(
        &self,
        job_id: &str,
        n: i64,
        bucket: &str,
        object: &str,
        ocr_text: Option<&str>,
    ) -> Result<(), Error> {
        self.db
            .query(
                "LET $job = type::record('ingestion_job', $jid); \
                 LET $existing = (SELECT id FROM pdf_page \
                   WHERE job = $job AND n = $n LIMIT 1)[0].id; \
                 IF $existing IS NONE { \
                   CREATE pdf_page SET job = $job, n = $n, \
                     bucket = $bucket, object = $object, ocr_text = $ocr_text \
                 } ELSE { \
                   UPDATE $existing SET \
                     bucket = $bucket, object = $object, ocr_text = $ocr_text \
                 }",
            )
            .bind(("jid", job_id.to_string()))
            .bind(("n", n))
            .bind(("bucket", bucket.to_string()))
            .bind(("object", object.to_string()))
            .bind(("ocr_text", ocr_text.map(str::to_string)))
            .await?;
        Ok(())
    }
}
