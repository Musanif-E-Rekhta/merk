use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, put_with},
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
use crate::db::bookmark_repo::{BookmarkResponse, ReadingGoalResponse, UpsertBookmarkDto};
use crate::db::collection_repo::{
    AddBookDto, CollectionBookResponse, CollectionResponse, CreateCollectionDto,
    UpdateCollectionDto,
};
use crate::error::Error;
use crate::state::AppState;

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        // Collections
        .api_route(
            "/me/collections",
            get_with(list_my_collections, |op| {
                op.description("List current user's collections")
                    .tag("collections")
            })
            .post_with(create_collection, |op| {
                op.description("Create a new collection").tag("collections")
            }),
        )
        .api_route(
            "/me/collections/{id}",
            get_with(get_collection, |op| {
                op.description("Get a collection by ID").tag("collections")
            })
            .put_with(update_collection, |op| {
                op.description("Update a collection").tag("collections")
            })
            .delete_with(delete_collection, |op| {
                op.description("Delete a collection").tag("collections")
            }),
        )
        .api_route(
            "/me/collections/{id}/books",
            get_with(list_collection_books, |op| {
                op.description("List books in a collection")
                    .tag("collections")
            })
            .post_with(add_book, |op| {
                op.description("Add a book to a collection")
                    .tag("collections")
            }),
        )
        .api_route(
            "/me/collections/{id}/books/{slug}",
            delete_with(remove_book, |op| {
                op.description("Remove a book from a collection")
                    .tag("collections")
            }),
        )
        // Bookmarks
        .api_route(
            "/me/bookmarks",
            get_with(list_bookmarks, |op| {
                op.description("List current user's bookmarks")
                    .tag("bookmarks")
            }),
        )
        .api_route(
            "/books/{slug}/bookmark",
            put_with(upsert_bookmark, |op| {
                op.description("Create or update a bookmark on a book")
                    .tag("bookmarks")
            })
            .delete_with(remove_bookmark, |op| {
                op.description("Remove a bookmark from a book")
                    .tag("bookmarks")
            }),
        )
        // Reading goal
        .api_route(
            "/me/reading-goal",
            get_with(get_reading_goal, |op| {
                op.description("Get current user's reading goal")
                    .tag("reading-goal")
            })
            .put_with(upsert_reading_goal, |op| {
                op.description("Create or update a reading goal")
                    .tag("reading-goal")
            }),
        )
        .with_state(state)
}

// ── Query / Request types ─────────────────────────────────────────────────────

use crate::api::v1::Pagination;

#[derive(Deserialize, JsonSchema)]
pub struct BookmarkListQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl BookmarkListQuery {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).min(100)
    }
    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0)
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct ReadingGoalQuery {
    pub year: Option<i64>,
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct CreateCollectionRequest {
    #[validate(length(min = 1))]
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct AddBookRequest {
    #[validate(length(min = 1))]
    pub book_slug: String,
    pub position: Option<i64>,
    pub note: Option<String>,
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct UpsertBookmarkRequest {
    #[validate(length(min = 1))]
    pub status: String,
    pub progress: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct UpsertReadingGoalRequest {
    pub year: Option<i64>,
    #[validate(range(min = 1))]
    pub target: i64,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_my_collections(
    State(state): State<AppState>,
    claims: Claims,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<CollectionResponse>>, Error> {
    let collections = state
        .services
        .collection_repo
        .list_user_collections(&claims.sub, pagination.limit(), pagination.offset())
        .await?;
    Ok(Json(collections))
}

async fn create_collection(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<CreateCollectionRequest>,
) -> Result<(StatusCode, Json<CollectionResponse>), Error> {
    body.validate()?;
    let dto = CreateCollectionDto {
        name: body.name,
        description: body.description,
        is_public: body.is_public,
    };
    let collection = state
        .services
        .collection_repo
        .create_collection(&claims.sub, dto)
        .await?;
    Ok((StatusCode::CREATED, Json(collection)))
}

async fn get_collection(
    State(state): State<AppState>,
    _claims: Claims,
    Path(id): Path<String>,
) -> Result<Json<CollectionResponse>, Error> {
    state
        .services
        .collection_repo
        .get_collection(&id)
        .await?
        .ok_or_else(|| Error::not_found("Collection not found"))
        .map(Json)
}

async fn update_collection(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<String>,
    Json(body): Json<UpdateCollectionRequest>,
) -> Result<Json<CollectionResponse>, Error> {
    let dto = UpdateCollectionDto {
        name: body.name,
        description: body.description,
        is_public: body.is_public,
    };
    state
        .services
        .collection_repo
        .update_collection(&id, &claims.sub, dto)
        .await?
        .ok_or_else(|| Error::not_found("Collection not found"))
        .map(Json)
}

async fn delete_collection(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<String>,
) -> Result<StatusCode, Error> {
    state
        .services
        .collection_repo
        .delete_collection(&id, &claims.sub)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_collection_books(
    State(state): State<AppState>,
    _claims: Claims,
    Path(id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<CollectionBookResponse>>, Error> {
    let books = state
        .services
        .collection_repo
        .list_collection_books(&id, pagination.limit(), pagination.offset())
        .await?;
    Ok(Json(books))
}

async fn add_book(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<String>,
    Json(body): Json<AddBookRequest>,
) -> Result<(StatusCode, Json<CollectionBookResponse>), Error> {
    body.validate()?;
    let dto = AddBookDto {
        position: body.position,
        note: body.note,
    };
    let entry = state
        .services
        .collection_repo
        .add_book(&id, &body.book_slug, &claims.sub, dto)
        .await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

async fn remove_book(
    State(state): State<AppState>,
    claims: Claims,
    Path((id, slug)): Path<(String, String)>,
) -> Result<StatusCode, Error> {
    state
        .services
        .collection_repo
        .remove_book(&id, &slug, &claims.sub)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_bookmarks(
    State(state): State<AppState>,
    claims: Claims,
    Query(q): Query<BookmarkListQuery>,
) -> Result<Json<Vec<BookmarkResponse>>, Error> {
    let bookmarks = state
        .services
        .bookmark_repo
        .list_user_bookmarks(&claims.sub, q.status.as_deref(), q.limit(), q.offset())
        .await?;
    Ok(Json(bookmarks))
}

async fn upsert_bookmark(
    State(state): State<AppState>,
    claims: Claims,
    Path(slug): Path<String>,
    Json(body): Json<UpsertBookmarkRequest>,
) -> Result<Json<BookmarkResponse>, Error> {
    body.validate()?;
    let dto = UpsertBookmarkDto {
        status: body.status,
        progress: body.progress,
        notes: body.notes,
    };
    let bookmark = state
        .services
        .bookmark_repo
        .upsert_bookmark(&claims.sub, &slug, dto)
        .await?;
    Ok(Json(bookmark))
}

async fn remove_bookmark(
    State(state): State<AppState>,
    claims: Claims,
    Path(slug): Path<String>,
) -> Result<StatusCode, Error> {
    state
        .services
        .bookmark_repo
        .remove_bookmark(&claims.sub, &slug)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_reading_goal(
    State(state): State<AppState>,
    claims: Claims,
    Query(q): Query<ReadingGoalQuery>,
) -> Result<Json<ReadingGoalResponse>, Error> {
    use chrono::Datelike;
    let year = q.year.unwrap_or_else(|| chrono::Utc::now().year() as i64);
    state
        .services
        .bookmark_repo
        .get_reading_goal(&claims.sub, year)
        .await?
        .ok_or_else(|| Error::not_found("Reading goal not found"))
        .map(Json)
}

async fn upsert_reading_goal(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<UpsertReadingGoalRequest>,
) -> Result<Json<ReadingGoalResponse>, Error> {
    body.validate()?;
    use chrono::Datelike;
    let year = body
        .year
        .unwrap_or_else(|| chrono::Utc::now().year() as i64);
    let goal = state
        .services
        .bookmark_repo
        .upsert_reading_goal(&claims.sub, year, body.target)
        .await?;
    Ok(Json(goal))
}
