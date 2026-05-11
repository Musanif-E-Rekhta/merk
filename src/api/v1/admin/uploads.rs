//! Admin upload endpoints.
//!
//! Two flows, picked by the client:
//! - **Direct** — `POST /api/v1/admin/uploads` with raw body, metadata in
//!   headers (`X-Filename`, `Content-Type`). Server streams to the
//!   configured `BlobStore`. Simpler; backend memory pressure under load.
//! - **Pre-signed** (only meaningful with GCS) — `POST /api/v1/admin/uploads/sign`
//!   returns a short-lived signed URL. Client uploads directly to the
//!   bucket, then `POST /api/v1/admin/uploads/{asset_id}/finalize` registers it.
//!
//! Both flows produce an `uploaded_asset` row with `status = "ready"`.

use crate::api::middleware::Claims;
use crate::db::admin::jobs::{CreateUploadedAssetDto, UploadedAssetResponse};
use crate::error::Error;
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const MAX_BYTES: usize = 200 * 1024 * 1024; // 200 MB per plan §3.2

pub fn routes(state: AppState) -> aide::axum::ApiRouter {
    aide::axum::ApiRouter::from(
        Router::<AppState>::new()
            .route("/admin/uploads", post(direct_upload))
            .route("/admin/uploads/sign", post(sign_upload))
            .route("/admin/uploads/{asset_id}/finalize", post(finalize_upload))
            .with_state(state),
    )
}

/// Direct multipart-style upload — body is the raw file bytes.
async fn direct_upload(
    claims: Claims,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<UploadedAssetResponse>, Error> {
    state.services.rbac.require_admin(&claims.sub).await?;

    if body.len() > MAX_BYTES {
        return Err(Error::bad_request(
            "file_too_large",
            format!("Files must be ≤ {} bytes", MAX_BYTES),
        ));
    }

    let filename = headers
        .get("x-filename")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("upload.bin")
        .to_string();
    let mime = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let object = format!("uploads/{}/{}", claims.sub, filename);
    state.services.blob_store.put(&object, &body).await?;

    let asset = state
        .services
        .admin_jobs
        .create_uploaded_asset(
            &claims.sub,
            CreateUploadedAssetDto {
                filename,
                mime,
                size_bytes: body.len() as i64,
                bucket: state.services.blob_store.bucket().to_string(),
                object,
            },
        )
        .await?;
    Ok(Json(asset))
}

#[derive(Deserialize, JsonSchema)]
pub struct SignUploadRequest {
    pub filename: String,
    pub content_type: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct SignUploadResponse {
    /// `Some(_)` when the configured blob store supports signed PUT URLs
    /// (i.e. GCS). For `LocalBlobStore`, `None` — the client must use the
    /// direct upload endpoint instead.
    pub upload_url: Option<String>,
    pub bucket: String,
    pub object: String,
    /// Headers the client must include on the PUT (e.g. `Content-Type`).
    pub headers: serde_json::Value,
}

async fn sign_upload(
    claims: Claims,
    State(state): State<AppState>,
    Json(req): Json<SignUploadRequest>,
) -> Result<Json<SignUploadResponse>, Error> {
    state.services.rbac.require_admin(&claims.sub).await?;

    let object = format!("uploads/{}/{}", claims.sub, req.filename);
    let upload_url = state
        .services
        .blob_store
        .signed_put_url(&object, 600)
        .await?;

    let mut headers = serde_json::Map::new();
    if let Some(ct) = req.content_type {
        headers.insert("Content-Type".into(), serde_json::Value::String(ct));
    }

    Ok(Json(SignUploadResponse {
        upload_url,
        bucket: state.services.blob_store.bucket().to_string(),
        object,
        headers: serde_json::Value::Object(headers),
    }))
}

#[derive(Deserialize, JsonSchema)]
pub struct FinalizeUploadRequest {
    pub filename: String,
    pub mime: String,
    pub size_bytes: i64,
    pub object: String,
}

async fn finalize_upload(
    claims: Claims,
    State(state): State<AppState>,
    Path(_asset_id): Path<String>,
    Json(req): Json<FinalizeUploadRequest>,
) -> Result<Json<UploadedAssetResponse>, Error> {
    state.services.rbac.require_admin(&claims.sub).await?;

    // For GCS we'd verify the object exists. For LocalBlobStore the file
    // was already saved by direct_upload; finalize is essentially the
    // CreateUploadedAsset call with the metadata the client recorded.
    let asset = state
        .services
        .admin_jobs
        .create_uploaded_asset(
            &claims.sub,
            CreateUploadedAssetDto {
                filename: req.filename,
                mime: req.mime,
                size_bytes: req.size_bytes,
                bucket: state.services.blob_store.bucket().to_string(),
                object: req.object,
            },
        )
        .await?;
    Ok(Json(asset))
}
