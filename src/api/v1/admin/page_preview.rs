//! `GET /api/v1/admin/ingestion-jobs/{job_id}/pages/{n}` — binary PNG/WEBP.
//!
//! REST exception per §3.5 of the plan: GraphQL would force base64
//! round-tripping for image bytes, which is wasteful. The Review pane's
//! PDF-side `<img>` hits this endpoint directly.

use crate::api::middleware::Claims;
use crate::error::Error;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use axum::routing::get;
use axum::Router;

pub fn routes(state: AppState) -> aide::axum::ApiRouter {
    aide::axum::ApiRouter::from(
        Router::<AppState>::new()
            .route(
                "/admin/ingestion-jobs/{job_id}/pages/{n}",
                get(serve_page),
            )
            .with_state(state),
    )
}

async fn serve_page(
    claims: Claims,
    State(state): State<AppState>,
    Path((job_id, n)): Path<(String, i64)>,
) -> Result<Response, Error> {
    state.services.rbac.require_admin(&claims.sub).await?;

    let page = state
        .services
        .admin_drafts
        .get_pdf_page(&job_id, n)
        .await?
        .ok_or_else(|| Error::not_found("Page not yet rendered"))?;

    let bytes = state.services.blob_store.get(&page.object).await?;

    // Best-effort content type from object extension; default WEBP since
    // that's what the plan example uses.
    let content_type = if page.object.ends_with(".png") {
        "image/png"
    } else {
        "image/webp"
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=300"),
    );

    Ok((StatusCode::OK, headers, Body::from(bytes)).into_response_legacy())
}

trait IntoResponseLegacy {
    fn into_response_legacy(self) -> Response;
}

impl IntoResponseLegacy for (StatusCode, HeaderMap, Body) {
    fn into_response_legacy(self) -> Response {
        let mut resp = Response::new(self.2);
        *resp.status_mut() = self.0;
        *resp.headers_mut() = self.1;
        resp
    }
}
