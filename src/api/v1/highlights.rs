use aide::axum::{
    ApiRouter,
    routing::{get_with, put_with},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use schemars::JsonSchema;
use serde::Deserialize;
use validator::Validate;

use crate::api::middleware::Claims;
use crate::db::highlight_repo::{CreateHighlightDto, HighlightResponse, UpdateHighlightDto};
use crate::error::Error;
use crate::state::AppState;

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/books/{book_slug}/chapters/{chapter_slug}/highlights",
            get_with(list_highlights, |op| {
                op.description("List highlights for a chapter")
                    .tag("highlights")
            })
            .post_with(create_highlight, |op| {
                op.description("Create a highlight on a chapter")
                    .tag("highlights")
            }),
        )
        .api_route(
            "/me/highlights",
            get_with(list_my_highlights, |op| {
                op.description("List the current user's highlights")
                    .tag("highlights")
            }),
        )
        .api_route(
            "/highlights/{highlight_id}",
            put_with(update_highlight, |op| {
                op.description("Update a highlight").tag("highlights")
            })
            .delete_with(delete_highlight, |op| {
                op.description("Delete a highlight").tag("highlights")
            }),
        )
        .with_state(state)
}

// ── Query / Request types ─────────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct HighlightListQuery {
    pub public: Option<bool>,
}

use crate::api::v1::Pagination;

#[derive(Deserialize, JsonSchema, Validate)]
pub struct CreateHighlightRequest {
    pub offset_start: i64,
    pub offset_end: i64,
    pub paragraph: i64,
    #[validate(length(min = 1))]
    pub text_snapshot: String,
    pub color: Option<String>,
    pub note: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateHighlightRequest {
    pub color: Option<String>,
    pub note: Option<String>,
    pub is_public: Option<bool>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_highlights(
    State(state): State<AppState>,
    Path((book_slug, chapter_slug)): Path<(String, String)>,
    Query(q): Query<HighlightListQuery>,
) -> Result<Json<Vec<HighlightResponse>>, Error> {
    let public_only = q.public.unwrap_or(false);
    let highlights = state
        .services
        .highlight_repo
        .list_chapter_highlights(&book_slug, &chapter_slug, public_only)
        .await?;
    Ok(Json(highlights))
}

async fn list_my_highlights(
    State(state): State<AppState>,
    claims: Claims,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<HighlightResponse>>, Error> {
    let highlights = state
        .services
        .highlight_repo
        .list_user_highlights(&claims.sub, pagination.limit(), pagination.offset())
        .await?;
    Ok(Json(highlights))
}

async fn create_highlight(
    State(state): State<AppState>,
    claims: Claims,
    Path((book_slug, chapter_slug)): Path<(String, String)>,
    Json(body): Json<CreateHighlightRequest>,
) -> Result<(StatusCode, Json<HighlightResponse>), Error> {
    body.validate()?;
    let dto = CreateHighlightDto {
        book_slug,
        chapter_slug,
        offset_start: body.offset_start,
        offset_end: body.offset_end,
        paragraph: body.paragraph,
        text_snapshot: body.text_snapshot,
        color: body.color,
        note: body.note,
        is_public: body.is_public,
    };
    let highlight = state
        .services
        .highlight_repo
        .create_highlight(&claims.sub, dto)
        .await?;
    Ok((StatusCode::CREATED, Json(highlight)))
}

async fn update_highlight(
    State(state): State<AppState>,
    claims: Claims,
    Path(highlight_id): Path<String>,
    Json(body): Json<UpdateHighlightRequest>,
) -> Result<Json<HighlightResponse>, Error> {
    let dto = UpdateHighlightDto {
        color: body.color,
        note: body.note,
        is_public: body.is_public,
    };
    state
        .services
        .highlight_repo
        .update_highlight(&highlight_id, &claims.sub, dto)
        .await?
        .ok_or_else(|| Error::not_found("Highlight not found"))
        .map(Json)
}

async fn delete_highlight(
    State(state): State<AppState>,
    claims: Claims,
    Path(highlight_id): Path<String>,
) -> Result<StatusCode, Error> {
    state
        .services
        .highlight_repo
        .delete_highlight(&highlight_id, &claims.sub)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
