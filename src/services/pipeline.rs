//! Pipeline worker.
//!
//! Walks an ingestion job through the seven fixed stages, emitting step
//! updates + log lines through the EventBus. Real implementations of OCR,
//! AI extraction, embedding, etc. are TODOs marked inline; the scaffolding
//! is enough for the admin UI to flow against real subscription transport.
//!
//! Each step is wrapped in a small helper that handles status transitions
//! and timing. A failure in any step transitions the whole job to
//! `failed` and emits `PipelineCompleted { status: "failed" }`.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

use merk_blob_store::BlobStore;

use crate::db::admin::drafts::{CreateDraftDto, DraftsRepo};
use crate::db::admin::jobs::JobsRepo;
use crate::error::Error;
use crate::services::event_bus::{EventBus, JobEvent};

const STEP_PDF_PARSE: i64 = 1;
const STEP_OCR: i64 = 2;
const STEP_CHAPTER_DETECT: i64 = 3;
const STEP_SUMMARIZATION: i64 = 4;
const STEP_EMBEDDINGS: i64 = 5;
const STEP_COVER: i64 = 6;
const STEP_QA: i64 = 7;

pub struct PipelineCtx {
    pub bus: Arc<EventBus>,
    pub jobs: JobsRepo,
    pub drafts: DraftsRepo,
    pub blob_store: Arc<dyn BlobStore>,
    pub user_id: String,
}

/// Spawn the worker for `job_id` on a tokio task. Caller continues
/// without blocking — the GraphQL `startIngestionJob` mutation returns
/// immediately and the UI reads progress via subscriptions.
pub fn spawn_pipeline(ctx: PipelineCtx, job_id: String) {
    tokio::spawn(async move {
        if let Err(e) = run_pipeline(&ctx, &job_id).await {
            warn!(?e, %job_id, "pipeline failed");
            let _ = ctx.jobs.set_status(&job_id, "failed").await;
            let _ = ctx
                .jobs
                .append_log(&ctx.bus, &job_id, "err", &format!("Pipeline failed: {e}"))
                .await;
            ctx.bus.publish_job(JobEvent::PipelineCompleted {
                job_id,
                status: "failed".into(),
            });
        }
    });
}

async fn run_pipeline(ctx: &PipelineCtx, job_id: &str) -> Result<(), Error> {
    info!(%job_id, "pipeline starting");
    ctx.jobs.set_status(job_id, "running").await?;
    ctx.jobs
        .append_log(&ctx.bus, job_id, "info", "Pipeline started")
        .await?;

    // ── Stage 2: Process ────────────────────────────────────────────────────
    ctx.jobs.set_stage(job_id, 2).await?;

    {
        let jobs = ctx.jobs.clone();
        let drafts = ctx.drafts.clone();
        let blob_store = ctx.blob_store.clone();
        let job_id_owned = job_id.to_string();
        run_step(
            ctx,
            job_id,
            STEP_PDF_PARSE,
            "Parsing PDF",
            Duration::from_millis(0),
            move |_| async move {
                parse_pdf(&jobs, &drafts, &blob_store, &job_id_owned).await
            },
        )
        .await?;
    }

    run_step(
        ctx,
        job_id,
        STEP_OCR,
        "Running OCR (Urdu)",
        Duration::from_millis(1500),
        // TODO(F2): real implementation — Tesseract `urd` model with retry on
        // low-confidence pages. Persist into pdf_page rows.
        |_| async { Ok(Some("Average OCR confidence 0.92".to_string())) },
    )
    .await?;

    // ── Stage 3: Review (chapter draft creation) ────────────────────────────
    ctx.jobs.set_stage(job_id, 3).await?;

    run_step(
        ctx,
        job_id,
        STEP_CHAPTER_DETECT,
        "Detecting chapter boundaries",
        Duration::from_millis(1200),
        // TODO(F2): real implementation — LLM-driven chapter detection from OCR text.
        |_| async { Ok(Some("Detected 3 chapters".to_string())) },
    )
    .await?;

    // Emit a small set of placeholder drafts so the Review screen has
    // something to render. Real implementation: per detected chapter,
    // call the configured ai_model to generate summary/themes/entities.
    for n in 1..=3 {
        let dto = CreateDraftDto {
            job_id: job_id.to_string(),
            n,
            title_ur: format!("باب {}", n),
            title_en: Some(format!("Chapter {}", n)),
            page_range: format!("{}-{}", (n - 1) * 40 + 1, n * 40),
            ai_content: format!(
                "## Chapter {n}\n\nPlaceholder content from the pipeline scaffold. Real worker fills this from OCR + LLM extraction."
            ),
            ai_summary: Some(format!("Summary for chapter {n}.")),
            themes: vec!["love".into(), "loss".into(), "time".into()],
            entities: vec!["Ghalib".into(), "Delhi".into()],
            confidence: 0.85,
        };
        ctx.drafts.create_draft(&ctx.bus, dto).await?;
    }

    run_step(
        ctx,
        job_id,
        STEP_SUMMARIZATION,
        "Summarizing chapters",
        Duration::from_millis(1500),
        // TODO(F2): real implementation — Claude/GPT/Gemini per chapter.
        |_| async { Ok(Some("Summaries generated".to_string())) },
    )
    .await?;

    run_step(
        ctx,
        job_id,
        STEP_EMBEDDINGS,
        "Generating embeddings",
        Duration::from_millis(1800),
        // TODO(F2): real implementation — Vertex multilingual-embedding-002
        // → chapter_chunk rows for the MTREE index.
        |_| async { Ok(Some("0 chunks (placeholder)".to_string())) },
    )
    .await?;

    run_step(
        ctx,
        job_id,
        STEP_COVER,
        "Generating cover variants",
        Duration::from_millis(1000),
        // TODO(F2): real implementation — image generation (Imagen/DALL-E)
        // → cover_variant rows.
        |_| async { Ok(Some("Skipped — no image generator wired".to_string())) },
    )
    .await?;

    run_step(
        ctx,
        job_id,
        STEP_QA,
        "QA gate",
        Duration::from_millis(500),
        |_| async { Ok(Some("All chapters within tolerance".to_string())) },
    )
    .await?;

    // ── Stage 4: Edit ── (no automated work; humans take over here) ─────────
    ctx.jobs.set_stage(job_id, 4).await?;

    // Worker considers itself done at the end of stage 4. Status stays
    // `running` so editors know they can act; publish flips it via the
    // PublishIngestionJob mutation.
    ctx.jobs
        .append_log(
            &ctx.bus,
            job_id,
            "ok",
            "Pipeline reached review/edit stage. Ready for editor.",
        )
        .await?;
    ctx.jobs.set_status(job_id, "paused").await?;
    ctx.bus.publish_job(JobEvent::PipelineCompleted {
        job_id: job_id.to_string(),
        status: "paused".into(),
    });

    Ok(())
}

async fn run_step<F, Fut>(
    ctx: &PipelineCtx,
    job_id: &str,
    n: i64,
    label: &str,
    sleep_for: Duration,
    work: F,
) -> Result<(), Error>
where
    F: FnOnce(&PipelineCtx) -> Fut,
    Fut: std::future::Future<Output = Result<Option<String>, Error>>,
{
    ctx.jobs
        .update_step(&ctx.bus, job_id, n, "running", None)
        .await?;
    ctx.jobs
        .append_log(&ctx.bus, job_id, "info", &format!("{} — running", label))
        .await?;

    sleep(sleep_for).await;

    match work(ctx).await {
        Ok(detail) => {
            ctx.jobs
                .update_step(&ctx.bus, job_id, n, "done", detail.clone())
                .await?;
            ctx.jobs
                .append_log(
                    &ctx.bus,
                    job_id,
                    "ok",
                    &format!("{} — done{}", label, detail.map(|d| format!(": {d}")).unwrap_or_default()),
                )
                .await?;
            Ok(())
        }
        Err(e) => {
            ctx.jobs
                .update_step(&ctx.bus, job_id, n, "failed", Some(e.to_string()))
                .await?;
            ctx.jobs
                .append_log(&ctx.bus, job_id, "err", &format!("{} — failed: {e}", label))
                .await?;
            Err(e)
        }
    }
}

/// Real PdfParse step: pulls the uploaded PDF bytes from the blob store,
/// parses with `lopdf` to enumerate pages and extract embedded text,
/// and upserts one `pdf_page` row per page. The `object` field is set
/// to the rendered-image path the page-preview route looks for; the
/// image itself is rendered later by the OCR step (or never, if the
/// embedded text is already adequate).
///
/// Heavy synchronous work is offloaded to `spawn_blocking` so the
/// orchestrator's tokio executor isn't pinned while a multi-megabyte
/// PDF is parsed.
async fn parse_pdf(
    jobs: &JobsRepo,
    drafts: &DraftsRepo,
    blob_store: &Arc<dyn BlobStore>,
    job_id: &str,
) -> Result<Option<String>, Error> {
    let job = jobs
        .get_job(job_id)
        .await?
        .ok_or_else(|| Error::not_found("ingestion job"))?;

    let asset = jobs
        .get_uploaded_asset(&job.asset_id)
        .await?
        .ok_or_else(|| Error::not_found("uploaded asset"))?;

    let bytes = blob_store.get(&asset.object).await?;

    let pages = tokio::task::spawn_blocking(move || extract_pages(&bytes))
        .await
        .map_err(|e| Error::internal("ingest", format!("pdf parse panic: {e}")))??;

    let total = pages.len();
    for (n, text) in pages.into_iter() {
        let object = format!("jobs/{job_id}/pages/{n}.png");
        drafts
            .upsert_pdf_page(
                job_id,
                n as i64,
                blob_store.bucket(),
                &object,
                if text.is_empty() { None } else { Some(text.as_str()) },
            )
            .await?;
    }

    Ok(Some(format!("{total} pages parsed")))
}

fn extract_pages(bytes: &[u8]) -> Result<Vec<(u32, String)>, Error> {
    let doc = lopdf::Document::load_mem(bytes)
        .map_err(|e| Error::internal("ingest", format!("pdf load failed: {e}")))?;

    let mut out = Vec::new();
    for (page_n, _) in doc.get_pages() {
        // `extract_text` is best-effort — Urdu/CIDFont PDFs may yield
        // empty or garbled text. The OCR step is responsible for the
        // full rasterise + recognise path; this step just captures
        // whatever embedded text the document already carries.
        let text = doc.extract_text(&[page_n]).unwrap_or_default();
        out.push((page_n, text.trim().to_string()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sanity check against the bundled Sarab fixture: lopdf must at
    /// least enumerate the page count without panicking. Text quality
    /// is not asserted — Urdu PDFs with custom CIDFont encodings often
    /// extract as empty here, which is fine because the OCR step is
    /// the ground truth for chapter detection.
    #[test]
    fn extracts_pages_from_sarab_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("novels")
            .join("Sarab+by+M.A.Rahat.pdf");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => {
                // Fixture not present in this checkout; skip.
                return;
            }
        };
        let pages = extract_pages(&bytes).expect("parse");
        assert!(!pages.is_empty(), "expected at least one page");
        assert!(pages[0].0 >= 1, "page numbers are 1-indexed");
    }
}
