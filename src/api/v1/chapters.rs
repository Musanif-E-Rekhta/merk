use aide::axum::{ApiRouter, routing::get_with};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::api::middleware::Claims;
use crate::db::chapter_repo::{
    ChapterListItem, ChapterResponse, CreateChapterDto, UpdateChapterDto,
};
use crate::error::Error;
use crate::state::AppState;

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/books/{book_slug}/chapters",
            get_with(list_chapters, |op| {
                op.description("List chapters of a book").tag("chapters")
            })
            .post_with(create_chapter, |op| {
                op.description("Create a chapter in a book").tag("chapters")
            }),
        )
        .api_route(
            "/books/{book_slug}/chapters/{chapter_slug}",
            get_with(get_chapter, |op| {
                op.description("Get a chapter by slug").tag("chapters")
            })
            .put_with(update_chapter, |op| {
                op.description("Update a chapter").tag("chapters")
            }),
        )
        .api_route(
            "/books/{book_slug}/chapters/by-number/{number}",
            get_with(get_chapter_by_number, |op| {
                op.description("Resolve chapter number to slug")
                    .tag("chapters")
            }),
        )
        .with_state(state)
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema, Validate)]
pub struct CreateChapterRequest {
    pub number: i64,
    pub title: Option<String>,
    #[validate(length(min = 1))]
    pub slug: String,
    #[validate(length(min = 1))]
    pub content: String,
    pub content_format: Option<String>,
    pub summary: Option<String>,
    pub meta_description: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateChapterRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub meta_description: Option<String>,
    pub is_published: Option<bool>,
}

#[derive(Serialize, JsonSchema)]
pub struct ChapterSlugResponse {
    pub slug: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_chapters(
    State(state): State<AppState>,
    Path(book_slug): Path<String>,
) -> Result<Json<Vec<ChapterListItem>>, Error> {
    let chapters = state
        .services
        .chapter_repo
        .list_chapters(&book_slug)
        .await?;
    Ok(Json(chapters))
}

async fn get_chapter(
    State(state): State<AppState>,
    Path((book_slug, chapter_slug)): Path<(String, String)>,
) -> Result<Json<ChapterResponse>, Error> {
    state
        .services
        .chapter_repo
        .get_chapter_by_slug(&book_slug, &chapter_slug)
        .await?
        .ok_or_else(|| Error::not_found("Chapter not found"))
        .map(Json)
}

async fn get_chapter_by_number(
    State(state): State<AppState>,
    Path((book_slug, number)): Path<(String, i64)>,
) -> Result<Json<ChapterSlugResponse>, Error> {
    let slug = state
        .services
        .chapter_repo
        .get_chapter_slug_by_number(&book_slug, number)
        .await?
        .ok_or_else(|| Error::not_found("Chapter not found"))?;
    Ok(Json(ChapterSlugResponse { slug }))
}

async fn create_chapter(
    State(state): State<AppState>,
    _claims: Claims,
    Path(book_slug): Path<String>,
    Json(body): Json<CreateChapterRequest>,
) -> Result<(StatusCode, Json<ChapterResponse>), Error> {
    body.validate()?;
    let dto = CreateChapterDto {
        number: body.number,
        title: body.title,
        slug: body.slug,
        content: body.content,
        content_format: body.content_format,
        summary: body.summary,
        meta_description: body.meta_description,
    };
    let chapter = state
        .services
        .chapter_repo
        .create_chapter(&book_slug, dto)
        .await?;
    Ok((StatusCode::CREATED, Json(chapter)))
}

async fn update_chapter(
    State(state): State<AppState>,
    _claims: Claims,
    Path((book_slug, chapter_slug)): Path<(String, String)>,
    Json(body): Json<UpdateChapterRequest>,
) -> Result<Json<ChapterResponse>, Error> {
    let dto = UpdateChapterDto {
        title: body.title,
        content: body.content,
        summary: body.summary,
        meta_description: body.meta_description,
        is_published: body.is_published,
    };
    state
        .services
        .chapter_repo
        .update_chapter(&book_slug, &chapter_slug, dto)
        .await?
        .ok_or_else(|| Error::not_found("Chapter not found"))
        .map(Json)
}
