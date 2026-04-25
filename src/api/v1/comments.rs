use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with, put_with},
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
use crate::db::comment_repo::{CommentResponse, CreateCommentDto, UpdateCommentDto};
use crate::error::Error;
use crate::state::AppState;

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/books/{book_slug}/chapters/{chapter_slug}/comments",
            get_with(list_comments, |op| {
                op.description("List comments on a chapter").tag("comments")
            })
            .post_with(create_comment, |op| {
                op.description("Post a comment on a chapter")
                    .tag("comments")
            }),
        )
        .api_route(
            "/comments/{comment_id}",
            put_with(update_comment, |op| {
                op.description("Update a comment").tag("comments")
            })
            .delete_with(delete_comment, |op| {
                op.description("Delete a comment").tag("comments")
            }),
        )
        .api_route(
            "/comments/{comment_id}/vote",
            post_with(vote_comment, |op| {
                op.description("Vote on a comment").tag("comments")
            }),
        )
        .api_route(
            "/highlights/{highlight_id}/comments",
            get_with(list_highlight_comments, |op| {
                op.description("List comments on a highlight")
                    .tag("comments")
            }),
        )
        .with_state(state)
}

// ── Query / Request types ─────────────────────────────────────────────────────

use crate::api::v1::Pagination;

#[derive(Deserialize, JsonSchema, Validate)]
pub struct CreateCommentRequest {
    #[validate(length(min = 1))]
    pub body: String,
    pub highlight_id: Option<String>,
    pub parent_id: Option<String>,
    pub is_spoiler: Option<bool>,
    pub offset_start: Option<i64>,
    pub offset_end: Option<i64>,
    pub text_snapshot: Option<String>,
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct UpdateCommentRequest {
    #[validate(length(min = 1))]
    pub body: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct VoteRequest {
    pub value: i64,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_comments(
    State(state): State<AppState>,
    Path((book_slug, chapter_slug)): Path<(String, String)>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<CommentResponse>>, Error> {
    let comments = state
        .services
        .comment_repo
        .list_chapter_comments(
            &book_slug,
            &chapter_slug,
            pagination.limit(),
            pagination.offset(),
        )
        .await?;
    Ok(Json(comments))
}

async fn create_comment(
    State(state): State<AppState>,
    claims: Claims,
    Path((book_slug, chapter_slug)): Path<(String, String)>,
    Json(body): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<CommentResponse>), Error> {
    body.validate()?;
    let dto = CreateCommentDto {
        book_slug,
        chapter_slug,
        highlight_id: body.highlight_id,
        parent_id: body.parent_id,
        body: body.body,
        is_spoiler: body.is_spoiler,
        offset_start: body.offset_start,
        offset_end: body.offset_end,
        text_snapshot: body.text_snapshot,
    };
    let comment = state
        .services
        .comment_repo
        .create_comment(&claims.sub, dto)
        .await?;
    Ok((StatusCode::CREATED, Json(comment)))
}

async fn update_comment(
    State(state): State<AppState>,
    claims: Claims,
    Path(comment_id): Path<String>,
    Json(body): Json<UpdateCommentRequest>,
) -> Result<Json<CommentResponse>, Error> {
    body.validate()?;
    let dto = UpdateCommentDto { body: body.body };
    state
        .services
        .comment_repo
        .update_comment(&comment_id, &claims.sub, dto)
        .await?
        .ok_or_else(|| Error::not_found("Comment not found"))
        .map(Json)
}

async fn delete_comment(
    State(state): State<AppState>,
    claims: Claims,
    Path(comment_id): Path<String>,
) -> Result<StatusCode, Error> {
    state
        .services
        .comment_repo
        .delete_comment(&comment_id, &claims.sub)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn vote_comment(
    State(state): State<AppState>,
    claims: Claims,
    Path(comment_id): Path<String>,
    Json(body): Json<VoteRequest>,
) -> Result<StatusCode, Error> {
    state
        .services
        .comment_repo
        .vote_comment(&claims.sub, &comment_id, body.value)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_highlight_comments(
    State(state): State<AppState>,
    Path(highlight_id): Path<String>,
) -> Result<Json<Vec<CommentResponse>>, Error> {
    let comments = state
        .services
        .comment_repo
        .list_highlight_comments(&highlight_id)
        .await?;
    Ok(Json(comments))
}
