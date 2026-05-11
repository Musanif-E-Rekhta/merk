//! GraphQL Subscription root.
//!
//! Subscribes to the in-process `EventBus` (broadcast channels) and
//! filters by `job_id` per client. Auth is checked once at subscription
//! start; admin gating mirrors the queries/mutations.
//!
//! WebSocket transport is wired in `mod.rs` via async-graphql's
//! `graphql-transport-ws` sub-protocol.

use async_graphql::{Context, Result, Subscription};
use chrono::{DateTime, Utc};
use futures_util::{Stream, StreamExt};
use tokio_stream::wrappers::BroadcastStream;

use crate::api::middleware::Claims;
use crate::state::AppState;
use merk_events::{DraftEvent, JobEvent};

/// Wrapper struct so we can implement `From<JobEvent>` cleanly into
/// individual GraphQL output objects via `flatten`.
#[derive(async_graphql::SimpleObject, Clone)]
pub struct JobEventGql {
    pub kind: String,           // "step_update" | "log_entry" | "chapter_draft_added" | "pipeline_completed"
    pub job_id: String,
    // Step update
    pub n: Option<i64>,
    pub label: Option<String>,
    pub status: Option<String>,
    pub detail: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    // Log entry
    pub t: Option<DateTime<Utc>>,
    pub log_kind: Option<String>,
    pub message: Option<String>,
    // Chapter draft added
    pub draft_id: Option<String>,
}

impl From<JobEvent> for JobEventGql {
    fn from(e: JobEvent) -> Self {
        let mut out = Self {
            kind: String::new(),
            job_id: String::new(),
            n: None,
            label: None,
            status: None,
            detail: None,
            started_at: None,
            finished_at: None,
            t: None,
            log_kind: None,
            message: None,
            draft_id: None,
        };
        match e {
            JobEvent::StepUpdate {
                job_id,
                n,
                label,
                status,
                detail,
                started_at,
                finished_at,
            } => {
                out.kind = "step_update".into();
                out.job_id = job_id;
                out.n = Some(n);
                out.label = Some(label);
                out.status = Some(status);
                out.detail = detail;
                out.started_at = started_at;
                out.finished_at = finished_at;
            }
            JobEvent::LogEntry {
                job_id,
                t,
                level,
                message,
            } => {
                out.kind = "log_entry".into();
                out.job_id = job_id;
                out.t = Some(t);
                out.log_kind = Some(level);
                out.message = Some(message);
            }
            JobEvent::ChapterDraftAdded { job_id, draft_id, n } => {
                out.kind = "chapter_draft_added".into();
                out.job_id = job_id;
                out.n = Some(n);
                out.draft_id = Some(draft_id);
            }
            JobEvent::PipelineCompleted { job_id, status } => {
                out.kind = "pipeline_completed".into();
                out.job_id = job_id;
                out.status = Some(status);
            }
        }
        out
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct DraftEventGql {
    pub kind: String, // "draft_updated" | "draft_approved" | "draft_flagged"
    pub job_id: String,
    pub draft_id: String,
    pub by_user: Option<String>,
    pub reason: Option<String>,
}

impl From<DraftEvent> for DraftEventGql {
    fn from(e: DraftEvent) -> Self {
        match e {
            DraftEvent::DraftUpdated { job_id, draft_id } => Self {
                kind: "draft_updated".into(),
                job_id,
                draft_id,
                by_user: None,
                reason: None,
            },
            DraftEvent::DraftApproved {
                job_id,
                draft_id,
                by_user,
            } => Self {
                kind: "draft_approved".into(),
                job_id,
                draft_id,
                by_user: Some(by_user),
                reason: None,
            },
            DraftEvent::DraftFlagged {
                job_id,
                draft_id,
                reason,
            } => Self {
                kind: "draft_flagged".into(),
                job_id,
                draft_id,
                by_user: None,
                reason: Some(reason),
            },
        }
    }
}

#[derive(Default)]
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Live job events (steps + log + draft-added + completed) for a
    /// single ingestion job. Caller must have admin/editor role.
    async fn job_events(
        &self,
        ctx: &Context<'_>,
        job: String,
    ) -> Result<impl Stream<Item = JobEventGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state.services.rbac.require_admin(&claims.sub).await?;

        let rx = state.services.event_bus.jobs.subscribe();
        Ok(BroadcastStream::new(rx).filter_map(move |r| {
            let job = job.clone();
            async move {
                match r {
                    Ok(ev) => {
                        let gql: JobEventGql = ev.into();
                        if gql.job_id == job {
                            Some(gql)
                        } else {
                            None
                        }
                    }
                    // Lagged messages are silently dropped — clients that
                    // care can refetch the underlying query.
                    Err(_) => None,
                }
            }
        }))
    }

    /// Draft-state events (updated / approved / flagged) for a job.
    async fn draft_events(
        &self,
        ctx: &Context<'_>,
        job: String,
    ) -> Result<impl Stream<Item = DraftEventGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state.services.rbac.require_admin(&claims.sub).await?;

        let rx = state.services.event_bus.drafts.subscribe();
        Ok(BroadcastStream::new(rx).filter_map(move |r| {
            let job = job.clone();
            async move {
                match r {
                    Ok(ev) => {
                        let gql: DraftEventGql = ev.into();
                        if gql.job_id == job {
                            Some(gql)
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            }
        }))
    }
}
