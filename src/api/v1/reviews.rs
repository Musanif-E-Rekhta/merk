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
use crate::db::review_repo::{
    BookReviewResponse, ChapterReviewResponse, CreateBookReviewDto, CreateChapterReviewDto,
    ReviewListFilters, UpdateBookReviewDto,
};
use crate::error::Error;
use crate::state::AppState;

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        // Book reviews
        .api_route(
            "/books/{book_slug}/reviews",
            get_with(list_book_reviews, |op| {
                op.description("List reviews for a book").tag("reviews")
            })
            .post_with(create_book_review, |op| {
                op.description("Create a book review").tag("reviews")
            }),
        )
        .api_route(
            "/books/{book_slug}/reviews/{review_id}",
            put_with(update_book_review, |op| {
                op.description("Update a book review").tag("reviews")
            })
            .delete_with(delete_book_review, |op| {
                op.description("Delete a book review").tag("reviews")
            }),
        )
        .api_route(
            "/books/{book_slug}/reviews/{review_id}/vote",
            post_with(vote_book_review, |op| {
                op.description("Vote on a book review").tag("reviews")
            }),
        )
        // Chapter reviews
        .api_route(
            "/books/{book_slug}/chapters/{chapter_slug}/reviews",
            get_with(list_chapter_reviews, |op| {
                op.description("List reviews for a chapter").tag("reviews")
            })
            .post_with(create_chapter_review, |op| {
                op.description("Create a chapter review").tag("reviews")
            }),
        )
        // Flags
        .api_route(
            "/reviews/{review_id}/flag",
            post_with(flag_review, |op| {
                op.description("Flag a review for moderation")
                    .tag("reviews")
            }),
        )
        .with_state(state)
}

// ── Query / Request types ─────────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct ReviewListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub spoilers: Option<bool>,
    pub rating: Option<i64>,
}

impl ReviewListQuery {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).min(100)
    }
    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0)
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct ChapterReviewListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl ChapterReviewListQuery {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).min(100)
    }
    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0)
    }
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct CreateBookReviewRequest {
    #[validate(range(min = 1, max = 5))]
    pub rating: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub contains_spoiler: Option<bool>,
    pub reading_status: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateBookReviewRequest {
    pub rating: Option<i64>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub contains_spoiler: Option<bool>,
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct CreateChapterReviewRequest {
    #[validate(range(min = 1, max = 5))]
    pub rating: i64,
    pub body: Option<String>,
    pub contains_spoiler: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct VoteRequest {
    pub value: i64,
}

#[derive(Deserialize, JsonSchema)]
pub struct FlagRequest {
    pub reason: String,
    pub note: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_book_reviews(
    State(state): State<AppState>,
    Path(book_slug): Path<String>,
    Query(q): Query<ReviewListQuery>,
) -> Result<Json<Vec<BookReviewResponse>>, Error> {
    let filters = ReviewListFilters {
        spoilers: q.spoilers,
        rating: q.rating,
        status: Some("published".to_string()),
    };
    let reviews = state
        .services
        .review_repo
        .list_book_reviews(&book_slug, &filters, q.limit(), q.offset())
        .await?;
    Ok(Json(reviews))
}

async fn create_book_review(
    State(state): State<AppState>,
    claims: Claims,
    Path(book_slug): Path<String>,
    Json(body): Json<CreateBookReviewRequest>,
) -> Result<(StatusCode, Json<BookReviewResponse>), Error> {
    body.validate()?;
    let dto = CreateBookReviewDto {
        book_slug,
        rating: body.rating,
        title: body.title,
        body: body.body,
        contains_spoiler: body.contains_spoiler,
        reading_status: body.reading_status,
    };
    let review = state
        .services
        .review_repo
        .create_book_review(&claims.sub, dto)
        .await?;
    Ok((StatusCode::CREATED, Json(review)))
}

async fn update_book_review(
    State(state): State<AppState>,
    claims: Claims,
    Path((_book_slug, review_id)): Path<(String, String)>,
    Json(body): Json<UpdateBookReviewRequest>,
) -> Result<Json<BookReviewResponse>, Error> {
    let dto = UpdateBookReviewDto {
        rating: body.rating,
        title: body.title,
        body: body.body,
        contains_spoiler: body.contains_spoiler,
    };
    state
        .services
        .review_repo
        .update_book_review(&review_id, &claims.sub, dto)
        .await?
        .ok_or_else(|| Error::not_found("Review not found"))
        .map(Json)
}

async fn delete_book_review(
    State(state): State<AppState>,
    claims: Claims,
    Path((_book_slug, review_id)): Path<(String, String)>,
) -> Result<StatusCode, Error> {
    state
        .services
        .review_repo
        .delete_book_review(&review_id, &claims.sub)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn vote_book_review(
    State(state): State<AppState>,
    claims: Claims,
    Path((_book_slug, review_id)): Path<(String, String)>,
    Json(body): Json<VoteRequest>,
) -> Result<StatusCode, Error> {
    state
        .services
        .review_repo
        .vote_book_review(&claims.sub, &review_id, body.value)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_chapter_reviews(
    State(state): State<AppState>,
    Path((book_slug, chapter_slug)): Path<(String, String)>,
    Query(q): Query<ChapterReviewListQuery>,
) -> Result<Json<Vec<ChapterReviewResponse>>, Error> {
    let reviews = state
        .services
        .review_repo
        .list_chapter_reviews(&book_slug, &chapter_slug, q.limit(), q.offset())
        .await?;
    Ok(Json(reviews))
}

async fn create_chapter_review(
    State(state): State<AppState>,
    claims: Claims,
    Path((book_slug, chapter_slug)): Path<(String, String)>,
    Json(body): Json<CreateChapterReviewRequest>,
) -> Result<(StatusCode, Json<ChapterReviewResponse>), Error> {
    body.validate()?;
    let dto = CreateChapterReviewDto {
        book_slug,
        chapter_slug,
        rating: body.rating,
        body: body.body,
        contains_spoiler: body.contains_spoiler,
    };
    let review = state
        .services
        .review_repo
        .create_chapter_review(&claims.sub, dto)
        .await?;
    Ok((StatusCode::CREATED, Json(review)))
}

async fn flag_review(
    State(state): State<AppState>,
    claims: Claims,
    Path(review_id): Path<String>,
    Json(body): Json<FlagRequest>,
) -> Result<StatusCode, Error> {
    state
        .services
        .review_repo
        .flag_review(&claims.sub, &review_id, &body.reason, body.note)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
