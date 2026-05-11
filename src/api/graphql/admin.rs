//! Admin GraphQL surface — F1-back §3.
//!
//! Every operation is gated through `services::rbac::RbacService::require_admin`,
//! which checks the caller has the `editor` or `admin` role via the 0002
//! RBAC graph. There are no REST equivalents except for the two §3.2 / §3.5
//! exceptions (multipart upload + binary page render).

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};

use crate::api::middleware::Claims;
use crate::db::admin::ai::{AiModelResponse, UsageByModel, UsageOverview};
use crate::db::admin::covers::CoverVariantResponse;
use crate::db::admin::drafts::{ChapterDraftResponse, UpdateDraftDto};
use crate::db::admin::jobs::{
    CreateJobDto, IngestionJobResponse, JobListFilters, JobLogEntryResponse, PipelineStepResponse,
    UpdateJobConfigDto,
};
use crate::db::admin::publish::{
    AdminBookFilters, AdminBookListItem, AuthorMatch, PublishCheck, PublishDto, PublishResult,
};
use crate::services::pipeline::{PipelineCtx, spawn_pipeline};
use crate::state::AppState;

// ── Output GQL types ──────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct IngestionJobGql {
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

impl From<IngestionJobResponse> for IngestionJobGql {
    fn from(j: IngestionJobResponse) -> Self {
        Self {
            id: j.id,
            asset_id: j.asset_id,
            book_id: j.book_id,
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

#[derive(SimpleObject)]
pub struct PipelineStepGql {
    pub id: String,
    pub n: i64,
    pub label: String,
    pub status: String,
    pub detail: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl From<PipelineStepResponse> for PipelineStepGql {
    fn from(s: PipelineStepResponse) -> Self {
        Self {
            id: s.id,
            n: s.n,
            label: s.label,
            status: s.status,
            detail: s.detail,
            started_at: s.started_at,
            finished_at: s.finished_at,
        }
    }
}

#[derive(SimpleObject)]
pub struct JobLogEntryGql {
    pub id: String,
    pub t: Option<DateTime<Utc>>,
    pub kind: String,
    pub message: String,
}

impl From<JobLogEntryResponse> for JobLogEntryGql {
    fn from(e: JobLogEntryResponse) -> Self {
        Self {
            id: e.id,
            t: e.t,
            kind: e.kind,
            message: e.message,
        }
    }
}

#[derive(SimpleObject)]
pub struct ChapterDraftGql {
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
    pub approved_at: Option<DateTime<Utc>>,
    pub pages_re_ocr_count: i64,
}

impl From<ChapterDraftResponse> for ChapterDraftGql {
    fn from(d: ChapterDraftResponse) -> Self {
        Self {
            id: d.id,
            job_id: d.job_id,
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
            approved_by_id: d.approved_by_id,
            approved_at: d.approved_at,
            pages_re_ocr_count: d.pages_re_ocr_count,
        }
    }
}

#[derive(SimpleObject)]
pub struct AiModelGql {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub label: String,
    pub note: Option<String>,
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
    pub is_active: bool,
}

impl From<AiModelResponse> for AiModelGql {
    fn from(m: AiModelResponse) -> Self {
        Self {
            id: m.id,
            provider: m.provider,
            name: m.name,
            label: m.label,
            note: m.note,
            input_cost_per_million: m.input_cost_per_million,
            output_cost_per_million: m.output_cost_per_million,
            is_active: m.is_active,
        }
    }
}

#[derive(SimpleObject)]
pub struct UsageByModelGql {
    pub model_id: String,
    pub model_label: String,
    pub tokens_used: i64,
    pub cost_usd: f64,
}

impl From<UsageByModel> for UsageByModelGql {
    fn from(u: UsageByModel) -> Self {
        Self {
            model_id: u.model_id,
            model_label: u.model_label,
            tokens_used: u.tokens_used,
            cost_usd: u.cost_usd,
        }
    }
}

#[derive(SimpleObject)]
pub struct UsageOverviewGql {
    pub period: String,
    pub tokens_used: i64,
    pub est_cost_usd: f64,
    pub monthly_budget_usd: f64,
    pub budget_used_pct: f64,
    pub by_model: Vec<UsageByModelGql>,
}

impl From<UsageOverview> for UsageOverviewGql {
    fn from(u: UsageOverview) -> Self {
        Self {
            period: u.period,
            tokens_used: u.tokens_used,
            est_cost_usd: u.est_cost_usd,
            monthly_budget_usd: u.monthly_budget_usd,
            budget_used_pct: u.budget_used_pct,
            by_model: u.by_model.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(SimpleObject)]
pub struct CoverVariantGql {
    pub id: String,
    pub job_id: String,
    pub bucket: String,
    pub object: String,
    pub model_id: Option<String>,
    pub prompt: Option<String>,
    pub is_selected: bool,
}

impl From<CoverVariantResponse> for CoverVariantGql {
    fn from(c: CoverVariantResponse) -> Self {
        Self {
            id: c.id,
            job_id: c.job_id,
            bucket: c.bucket,
            object: c.object,
            model_id: c.model_id,
            prompt: c.prompt,
            is_selected: c.is_selected,
        }
    }
}

#[derive(SimpleObject)]
pub struct PublishCheckGql {
    pub ok: bool,
    pub gate: Option<String>,
    pub label: String,
    pub detail: Option<String>,
}

impl From<PublishCheck> for PublishCheckGql {
    fn from(c: PublishCheck) -> Self {
        Self {
            ok: c.ok,
            gate: c.gate,
            label: c.label,
            detail: c.detail,
        }
    }
}

#[derive(SimpleObject)]
pub struct AdminBookGql {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub visibility: Option<String>,
    pub is_published: bool,
    pub chapter_count: i64,
    pub avg_rating: Option<f64>,
}

impl From<AdminBookListItem> for AdminBookGql {
    fn from(b: AdminBookListItem) -> Self {
        Self {
            id: b.id,
            title: b.title,
            slug: b.slug,
            visibility: b.visibility,
            is_published: b.is_published,
            chapter_count: b.chapter_count,
            avg_rating: b.avg_rating,
        }
    }
}

#[derive(SimpleObject)]
pub struct AuthorMatchGql {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub bio: Option<String>,
    pub confidence: f64,
}

impl From<AuthorMatch> for AuthorMatchGql {
    fn from(m: AuthorMatch) -> Self {
        Self {
            id: m.author.id,
            name: m.author.name,
            slug: m.author.slug,
            bio: m.author.bio,
            confidence: m.confidence,
        }
    }
}

// ── Input GQL types ───────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct CreateIngestionJobInput {
    pub asset_id: String,
    pub hint_title: Option<String>,
    pub hint_author: Option<String>,
}

#[derive(InputObject)]
pub struct UpdateJobConfigInput {
    pub ai_provider: Option<String>,
    pub ai_model: Option<String>,
}

#[derive(InputObject)]
pub struct UpdateChapterDraftInput {
    pub human_content: Option<String>,
    pub title_ur: Option<String>,
    pub title_en: Option<String>,
    pub page_range: Option<String>,
}

#[derive(InputObject)]
pub struct PublishInput {
    pub visibility: String,
    pub schedule_at: Option<DateTime<Utc>>,
}

// ── Query ─────────────────────────────────────────────────────────────────────

async fn require_admin(ctx: &Context<'_>) -> Result<(Claims, AppState)> {
    let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?.clone();
    let state = ctx.data::<AppState>()?.clone();
    state.services.rbac.require_admin(&claims.sub).await?;
    Ok((claims, state))
}

#[derive(Default)]
pub struct AdminQuery;

#[Object]
impl AdminQuery {
    /// List ingestion jobs, newest first.
    async fn ingestion_jobs(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
        stage: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<IngestionJobGql>> {
        let (_c, state) = require_admin(ctx).await?;
        let jobs = state
            .services
            .admin_jobs
            .list_jobs(
                &JobListFilters { status, stage },
                limit.unwrap_or(20),
                offset.unwrap_or(0),
            )
            .await?;
        Ok(jobs.into_iter().map(Into::into).collect())
    }

    async fn ingestion_job(&self, ctx: &Context<'_>, id: String) -> Result<Option<IngestionJobGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_jobs
            .get_job(&id)
            .await?
            .map(Into::into))
    }

    async fn job_steps(&self, ctx: &Context<'_>, job: String) -> Result<Vec<PipelineStepGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_jobs
            .list_steps(&job)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn job_log(
        &self,
        ctx: &Context<'_>,
        job: String,
        since: Option<String>,
        limit: Option<i64>,
    ) -> Result<Vec<JobLogEntryGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_jobs
            .list_log(&job, since.as_deref(), limit.unwrap_or(200))
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn chapter_drafts(
        &self,
        ctx: &Context<'_>,
        job: String,
    ) -> Result<Vec<ChapterDraftGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_drafts
            .list_drafts(&job)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn chapter_draft(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<ChapterDraftGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_drafts
            .get_draft(&id)
            .await?
            .map(Into::into))
    }

    async fn ai_models(&self, ctx: &Context<'_>) -> Result<Vec<AiModelGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_ai
            .list_models()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn admin_usage(
        &self,
        ctx: &Context<'_>,
        period: Option<String>,
    ) -> Result<UsageOverviewGql> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_ai
            .usage_overview(period.as_deref().unwrap_or("month"))
            .await?
            .into())
    }

    async fn cover_variants(
        &self,
        ctx: &Context<'_>,
        job: String,
    ) -> Result<Vec<CoverVariantGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_covers
            .list_variants(&job)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn publish_checks(
        &self,
        ctx: &Context<'_>,
        job: String,
    ) -> Result<Vec<PublishCheckGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_publish
            .publish_checks(&job)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn admin_books(
        &self,
        ctx: &Context<'_>,
        visibility: Option<String>,
        is_published: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<AdminBookGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_publish
            .list_books(
                &AdminBookFilters {
                    visibility,
                    is_published,
                },
                limit.unwrap_or(20),
                offset.unwrap_or(0),
            )
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn match_authors(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> Result<Vec<AuthorMatchGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_publish
            .match_authors(&name)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}

// ── Mutation ──────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct AdminMutation;

#[Object]
impl AdminMutation {
    async fn create_ingestion_job(
        &self,
        ctx: &Context<'_>,
        input: CreateIngestionJobInput,
    ) -> Result<IngestionJobGql> {
        let (claims, state) = require_admin(ctx).await?;
        let job = state
            .services
            .admin_jobs
            .create_job(
                &claims.sub,
                CreateJobDto {
                    asset_id: input.asset_id,
                    hint_title: input.hint_title,
                    hint_author: input.hint_author,
                },
            )
            .await?;
        Ok(job.into())
    }

    async fn start_ingestion_job(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let (claims, state) = require_admin(ctx).await?;
        spawn_pipeline(
            PipelineCtx {
                bus: state.services.event_bus.clone(),
                jobs: state.services.admin_jobs.clone(),
                drafts: state.services.admin_drafts.clone(),
                blob_store: state.services.blob_store.clone(),
                user_id: claims.sub.clone(),
            },
            id,
        );
        Ok(true)
    }

    async fn pause_ingestion_job(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        state.services.admin_jobs.set_status(&id, "paused").await?;
        Ok(true)
    }

    async fn resume_ingestion_job(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        state.services.admin_jobs.set_status(&id, "running").await?;
        Ok(true)
    }

    async fn cancel_ingestion_job(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        state.services.admin_jobs.set_status(&id, "failed").await?;
        Ok(true)
    }

    async fn update_ingestion_job_config(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateJobConfigInput,
    ) -> Result<Option<IngestionJobGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_jobs
            .update_config(
                &id,
                UpdateJobConfigDto {
                    ai_provider: input.ai_provider,
                    ai_model: input.ai_model,
                },
            )
            .await?
            .map(Into::into))
    }

    async fn update_chapter_draft(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateChapterDraftInput,
    ) -> Result<Option<ChapterDraftGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_drafts
            .update_draft(
                &state.services.event_bus,
                &id,
                UpdateDraftDto {
                    human_content: input.human_content,
                    title_ur: input.title_ur,
                    title_en: input.title_en,
                    page_range: input.page_range,
                },
            )
            .await?
            .map(Into::into))
    }

    async fn approve_chapter_draft(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let (claims, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_drafts
            .approve_draft(&state.services.event_bus, &id, &claims.sub)
            .await?)
    }

    async fn flag_chapter_draft(
        &self,
        ctx: &Context<'_>,
        id: String,
        reason: String,
    ) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_drafts
            .flag_draft(&state.services.event_bus, &id, &reason)
            .await?)
    }

    async fn reject_chapter_draft(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state.services.admin_drafts.reject_draft(&id).await?)
    }

    async fn re_ocr_chapter_pages(
        &self,
        ctx: &Context<'_>,
        id: String,
        pages: String,
    ) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state.services.admin_drafts.re_ocr_request(&id, &pages).await?)
    }

    async fn select_cover_variant(
        &self,
        ctx: &Context<'_>,
        job: String,
        variant: String,
    ) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state.services.admin_covers.select_variant(&job, &variant).await?)
    }

    /// Stub cover generator — creates a placeholder `cover_variant` row
    /// with no real image. Replace in F2 with an Imagen / DALL-E call.
    async fn generate_cover_variants(
        &self,
        ctx: &Context<'_>,
        job: String,
        prompt: Option<String>,
    ) -> Result<Vec<CoverVariantGql>> {
        let (_c, state) = require_admin(ctx).await?;
        let bucket = state.services.blob_store.bucket().to_string();
        let mut out = Vec::new();
        for i in 1..=3 {
            let object = format!("covers/{job}/{}.png", i);
            let v = state
                .services
                .admin_covers
                .create_variant(&job, &bucket, &object, prompt.as_deref())
                .await?;
            out.push(v.into());
        }
        Ok(out)
    }

    async fn publish_ingestion_job(
        &self,
        ctx: &Context<'_>,
        job: String,
        input: PublishInput,
    ) -> Result<PublishedBookGql> {
        let (_c, state) = require_admin(ctx).await?;
        let PublishResult { book } = state
            .services
            .admin_publish
            .publish(
                &job,
                PublishDto {
                    visibility: input.visibility,
                    schedule_at: input.schedule_at,
                },
            )
            .await?;
        Ok(PublishedBookGql {
            id: book.id,
            title: book.title,
            slug: book.slug,
            cover_url: book.cover_url,
            chapter_count: book.chapter_count,
        })
    }

    async fn update_admin_book(
        &self,
        ctx: &Context<'_>,
        slug: String,
        visibility: String,
    ) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .admin_publish
            .update_book_visibility(&slug, &visibility)
            .await?)
    }

    async fn unpublish_book(&self, ctx: &Context<'_>, slug: String) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state.services.admin_publish.unpublish_book(&slug).await?)
    }
}

#[derive(SimpleObject)]
pub struct PublishedBookGql {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub cover_url: Option<String>,
    pub chapter_count: i64,
}
