//! Admin GraphQL surface for the crawler — plan §9.
//!
//! Every operation is gated through `services::rbac::RbacService::require_admin`.
//! The crawler itself runs in a background `tokio::spawn`; this module
//! only triggers it. Progress flows back through the existing
//! `JobEvents` subscription once the crawler's handoff creates an
//! `ingestion_job` — there is no separate `CrawlEvents` subscription.

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::api::middleware::Claims;
use crate::db::admin::crawl::CrawlRunSummaryRow;
use crate::state::AppState;
use merk_crawl::store::{CrawlSourceRow, CrawlTargetRow};
use merk_crawl::{Crawler, HttpClient, Query};

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct CrawlSourceGql {
    pub id: String,
    pub kind: String,
    pub base_url: String,
    pub enabled: bool,
    pub paused: bool,
}

impl From<CrawlSourceRow> for CrawlSourceGql {
    fn from(r: CrawlSourceRow) -> Self {
        Self {
            id: r.id,
            kind: r.kind,
            base_url: r.base_url,
            enabled: r.enabled,
            paused: r.paused,
        }
    }
}

#[derive(SimpleObject)]
pub struct CrawlRunGql {
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

impl From<CrawlRunSummaryRow> for CrawlRunGql {
    fn from(r: CrawlRunSummaryRow) -> Self {
        Self {
            id: r.id,
            source_id: r.source_id,
            source_kind: r.source_kind,
            started_at: r.started_at,
            finished_at: r.finished_at,
            status: r.status,
            candidates_seen: r.candidates_seen,
            candidates_skipped: r.candidates_skipped,
            targets_downloaded: r.targets_downloaded,
            targets_handed_off: r.targets_handed_off,
        }
    }
}

#[derive(SimpleObject)]
pub struct CrawlTargetGql {
    pub id: String,
    pub source_id: String,
    pub source_url: String,
    pub pdf_url: Option<String>,
    pub pdf_sha256: Option<String>,
    pub title_raw: Option<String>,
    pub author_name_raw: Option<String>,
    pub status: String,
    pub skip_reason: Option<String>,
    pub ingest_job_id: Option<String>,
    pub book_id: Option<String>,
    pub takedown_at: Option<DateTime<Utc>>,
    pub attempts: i64,
}

impl From<CrawlTargetRow> for CrawlTargetGql {
    fn from(r: CrawlTargetRow) -> Self {
        Self {
            id: r.id,
            source_id: r.source_id,
            source_url: r.source_url,
            pdf_url: r.pdf_url,
            pdf_sha256: r.pdf_sha256,
            title_raw: r.title_raw,
            author_name_raw: r.author_name_raw,
            status: r.status,
            skip_reason: r.skip_reason,
            ingest_job_id: r.ingest_job,
            book_id: r.book,
            takedown_at: r.takedown_at,
            attempts: r.attempts,
        }
    }
}

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct UpsertCrawlSourceInput {
    pub kind: String,
    pub base_url: String,
    pub enabled: Option<bool>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn require_admin(ctx: &Context<'_>) -> Result<(Claims, AppState)> {
    let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?.clone();
    let state = ctx.data::<AppState>()?.clone();
    state.services.rbac.require_admin(&claims.sub).await?;
    Ok((claims, state))
}

/// Build a concrete `Source` from a `crawl_source.kind` string.
///
/// `base_url` is read off the matching `crawl_source` row so callers
/// can override `https://www.rekhta.org` per-environment (CDN preview,
/// staging mirror, etc.).
fn build_source(
    kind: &str,
    base_url: &str,
    http: &Arc<HttpClient>,
) -> Result<Arc<dyn merk_crawl::Source>> {
    use merk_crawl::sources::rekhta::{PdAllowList, RekhtaSource};

    match kind {
        "rekhta" => {
            // Phase-0 gate: the default allow-list is conservative
            // classic Urdu PD authors. Override via DB config (TODO:
            // when an `allow_list` column lands on `crawl_source`).
            let src = RekhtaSource::new(http.clone())
                .with_base_url(base_url.to_string())
                .with_allow_list(PdAllowList::classic_urdu_default());
            Ok(Arc::new(src))
        }
        "fixture" => Err(
            "fixture source is for tests/CLI with a manifest path — not launchable from GraphQL"
                .into(),
        ),
        other => Err(format!("unknown crawl source kind: {other}").into()),
    }
}

// ── Query ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct AdminCrawlQuery;

#[Object]
impl AdminCrawlQuery {
    async fn crawl_sources(&self, ctx: &Context<'_>) -> Result<Vec<CrawlSourceGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .crawl_store
            .list_sources()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn crawl_runs(
        &self,
        ctx: &Context<'_>,
        source_kind: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<CrawlRunGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .crawl_store
            .list_runs(source_kind.as_deref(), limit.unwrap_or(20), offset.unwrap_or(0))
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn crawl_targets(
        &self,
        ctx: &Context<'_>,
        source_kind: Option<String>,
        status: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<CrawlTargetGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state
            .services
            .crawl_store
            .list_targets(
                source_kind.as_deref(),
                status.as_deref(),
                limit.unwrap_or(50),
                offset.unwrap_or(0),
            )
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn crawl_target(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<CrawlTargetGql>> {
        let (_c, state) = require_admin(ctx).await?;
        Ok(state.services.crawl_store.get_target(&id).await?.map(Into::into))
    }
}

// ── Mutation ──────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct AdminCrawlMutation;

#[Object]
impl AdminCrawlMutation {
    async fn upsert_crawl_source(
        &self,
        ctx: &Context<'_>,
        input: UpsertCrawlSourceInput,
    ) -> Result<CrawlSourceGql> {
        let (_c, state) = require_admin(ctx).await?;
        let row = state
            .services
            .crawl_store
            .upsert_source(&input.kind, &input.base_url, input.enabled.unwrap_or(true))
            .await?;
        Ok(row.into())
    }

    async fn set_crawl_source_paused(
        &self,
        ctx: &Context<'_>,
        kind: String,
        paused: bool,
    ) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        state.services.crawl_store.set_paused(&kind, paused).await?;
        Ok(true)
    }

    /// Launch a crawl run. Returns the new `crawl_run` id immediately;
    /// the walk happens in the background and progress is visible
    /// through `crawlRun(id)` polling and the existing `JobEvents`
    /// subscription once a target is handed off.
    ///
    /// `query` is JSON-encoded. Pass `"{}"` for defaults. The schema
    /// is `merk_crawl::Query` (q, author_allow, category, max, cursor).
    async fn start_crawl_run(
        &self,
        ctx: &Context<'_>,
        kind: String,
        query: Option<String>,
    ) -> Result<String> {
        let (claims, state) = require_admin(ctx).await?;
        let parsed: Query = match query.as_deref() {
            Some(s) if !s.trim().is_empty() => serde_json::from_str(s)
                .map_err(|e| async_graphql::Error::new(format!("invalid query json: {e}")))?,
            _ => Query::default(),
        };

        // The source row's base_url is authoritative — admin upserts
        // it to point at production or a staging mirror as needed.
        let source_row = state
            .services
            .crawl_store
            .list_sources()
            .await?
            .into_iter()
            .find(|s| s.kind == kind)
            .ok_or_else(|| {
                async_graphql::Error::new(format!(
                    "no `crawl_source` row for kind={kind}; upsert one first"
                ))
            })?;

        let http = Arc::new(
            HttpClient::new().map_err(|e| async_graphql::Error::new(e.to_string()))?,
        );
        let source = build_source(&kind, &source_row.base_url, &http)?;
        let handoff = Arc::new(state.services.crawl_handoff.with_user(claims.sub.clone()));
        let store = Arc::new(state.services.crawl_store.clone());

        let crawler = Crawler::new(source, store, http, handoff);
        let run_id = crawler
            .launch(parsed)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(run_id)
    }

    async fn cancel_crawl_run(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        state.services.crawl_store.cancel_run(&id).await?;
        Ok(true)
    }

    async fn requeue_crawl_target(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        state.services.crawl_store.requeue_target(&id).await?;
        Ok(true)
    }

    async fn blacklist_crawl_target(
        &self,
        ctx: &Context<'_>,
        id: String,
        reason: String,
    ) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        state
            .services
            .crawl_store
            .blacklist_target(&id, &reason)
            .await?;
        Ok(true)
    }

    async fn record_takedown(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let (_c, state) = require_admin(ctx).await?;
        state.services.crawl_store.record_takedown(&id).await?;
        Ok(true)
    }
}
