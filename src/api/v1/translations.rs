use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
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
use crate::db::translation_repo::{CreateTranslationDto, WordTranslationResponse};
use crate::error::Error;
use crate::state::AppState;

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/books/{book_slug}/chapters/{chapter_slug}/translations",
            get_with(get_chapter_translations, |op| {
                op.description("Get translations scoped to a chapter")
                    .tag("translations")
            }),
        )
        .api_route(
            "/books/{book_slug}/translations",
            get_with(get_book_translations, |op| {
                op.description("Get translations scoped to a book")
                    .tag("translations")
            }),
        )
        .api_route(
            "/translations",
            get_with(get_global_translations, |op| {
                op.description("Get global word translations")
                    .tag("translations")
            })
            .post_with(submit_translation, |op| {
                op.description("Submit a new word translation")
                    .tag("translations")
            }),
        )
        .api_route(
            "/translations/{id}/vote",
            post_with(vote_translation, |op| {
                op.description("Vote on a translation").tag("translations")
            }),
        )
        .with_state(state)
}

// ── Query / Request types ─────────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct TranslationQuery {
    pub word: String,
    pub lang: String,
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct CreateTranslationRequest {
    #[validate(length(min = 1))]
    pub word: String,
    #[validate(length(min = 1))]
    pub translation: String,
    #[validate(length(min = 2))]
    pub source_lang: String,
    #[validate(length(min = 2))]
    pub target_lang: String,
    pub scope: Option<String>,
    pub book_slug: Option<String>,
    pub chapter_slug: Option<String>,
    pub context_note: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct VoteRequest {
    pub value: i64,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn get_chapter_translations(
    State(state): State<AppState>,
    Path((book_slug, chapter_slug)): Path<(String, String)>,
    Query(q): Query<TranslationQuery>,
) -> Result<Json<Vec<WordTranslationResponse>>, Error> {
    let translations = state
        .services
        .translation_repo
        .get_word_translations(&q.word, &q.lang, Some(&book_slug), Some(&chapter_slug))
        .await?;
    Ok(Json(translations))
}

async fn get_book_translations(
    State(state): State<AppState>,
    Path(book_slug): Path<String>,
    Query(q): Query<TranslationQuery>,
) -> Result<Json<Vec<WordTranslationResponse>>, Error> {
    let translations = state
        .services
        .translation_repo
        .get_word_translations(&q.word, &q.lang, Some(&book_slug), None)
        .await?;
    Ok(Json(translations))
}

async fn get_global_translations(
    State(state): State<AppState>,
    Query(q): Query<TranslationQuery>,
) -> Result<Json<Vec<WordTranslationResponse>>, Error> {
    let translations = state
        .services
        .translation_repo
        .get_word_translations(&q.word, &q.lang, None, None)
        .await?;
    Ok(Json(translations))
}

async fn submit_translation(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<CreateTranslationRequest>,
) -> Result<(StatusCode, Json<WordTranslationResponse>), Error> {
    body.validate()?;
    let scope = body.scope.unwrap_or_else(|| {
        if body.chapter_slug.is_some() {
            "chapter".to_string()
        } else if body.book_slug.is_some() {
            "book".to_string()
        } else {
            "global".to_string()
        }
    });
    let dto = CreateTranslationDto {
        word: body.word,
        translation: body.translation,
        source_lang: body.source_lang,
        target_lang: body.target_lang,
        scope,
        book_slug: body.book_slug,
        chapter_slug: body.chapter_slug,
        context_note: body.context_note,
    };
    let translation = state
        .services
        .translation_repo
        .create_translation(&claims.sub, dto)
        .await?;
    Ok((StatusCode::CREATED, Json(translation)))
}

async fn vote_translation(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<String>,
    Json(body): Json<VoteRequest>,
) -> Result<StatusCode, Error> {
    state
        .services
        .translation_repo
        .vote_translation(&claims.sub, &id, body.value)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
