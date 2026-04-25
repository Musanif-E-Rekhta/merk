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
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::api::middleware::Claims;
use crate::db::book_repo::{
    AuthorResponse, BookFilters, BookResponse, CategoryResponse, CreateAuthorDto, CreateBookDto,
    UpdateBookDto,
};
use crate::error::Error;
use crate::state::AppState;

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        // Books
        .api_route(
            "/books",
            get_with(list_books, |op| {
                op.description("List all books").tag("books")
            })
            .post_with(create_book, |op| {
                op.description("Create a new book (admin)").tag("books")
            }),
        )
        .api_route(
            "/books/{slug}",
            get_with(get_book, |op| {
                op.description("Get book by slug").tag("books")
            })
            .put_with(update_book, |op| {
                op.description("Update a book").tag("books")
            }),
        )
        .api_route(
            "/books/{slug}/authors",
            get_with(list_book_authors, |op| {
                op.description("List authors of a book").tag("books")
            }),
        )
        // Authors
        .api_route(
            "/authors",
            get_with(list_authors, |op| {
                op.description("List all authors").tag("authors")
            })
            .post_with(create_author, |op| {
                op.description("Create a new author").tag("authors")
            }),
        )
        .api_route(
            "/authors/{slug}",
            get_with(get_author, |op| {
                op.description("Get author by slug").tag("authors")
            }),
        )
        .api_route(
            "/authors/{slug}/follow",
            post_with(follow_author, |op| {
                op.description("Follow an author").tag("authors")
            })
            .delete_with(unfollow_author, |op| {
                op.description("Unfollow an author").tag("authors")
            }),
        )
        // Categories
        .api_route(
            "/categories",
            get_with(list_categories, |op| {
                op.description("List all categories").tag("categories")
            }),
        )
        .api_route(
            "/categories/{slug}/books",
            get_with(get_books_by_category, |op| {
                op.description("List books in a category").tag("categories")
            }),
        )
        // Tags
        .api_route(
            "/tags/{slug}/books",
            get_with(get_books_by_tag, |op| {
                op.description("List books with a tag").tag("tags")
            }),
        )
        .with_state(state)
}

// ── Query / Request types ─────────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct BookListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
    pub lang: Option<String>,
}

impl BookListQuery {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).min(100)
    }
    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0)
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct AuthorListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
}

impl AuthorListQuery {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).min(100)
    }
    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0)
    }
}

use crate::api::v1::Pagination;

#[derive(Deserialize, JsonSchema, Validate)]
pub struct CreateBookRequest {
    #[validate(length(min = 1))]
    pub title: String,
    #[validate(length(min = 1))]
    pub slug: String,
    pub isbn: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub page_count: Option<i64>,
    pub language: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateBookRequest {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub page_count: Option<i64>,
    pub is_published: Option<bool>,
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct CreateAuthorRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(length(min = 1))]
    pub slug: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize, JsonSchema)]
pub struct EmptyResponse {}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_books(
    State(state): State<AppState>,
    Query(q): Query<BookListQuery>,
) -> Result<Json<Vec<BookResponse>>, Error> {
    let filters = BookFilters {
        q: q.q.clone(),
        lang: q.lang.clone(),
        is_published: Some(true),
    };
    let books = state
        .services
        .book_repo
        .list_books(&filters, q.limit(), q.offset())
        .await?;
    Ok(Json(books))
}

async fn get_book(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<BookResponse>, Error> {
    state
        .services
        .book_repo
        .get_book_by_slug(&slug)
        .await?
        .ok_or_else(|| Error::not_found("Book not found"))
        .map(Json)
}

async fn create_book(
    State(state): State<AppState>,
    _claims: Claims,
    Json(body): Json<CreateBookRequest>,
) -> Result<(StatusCode, Json<BookResponse>), Error> {
    body.validate()?;
    let dto = CreateBookDto {
        title: body.title,
        slug: body.slug,
        isbn: body.isbn,
        summary: body.summary,
        description: body.description,
        cover_url: body.cover_url,
        page_count: body.page_count,
        language: body.language,
        publisher_slug: None,
    };
    let book = state.services.book_repo.create_book(dto).await?;
    Ok((StatusCode::CREATED, Json(book)))
}

async fn update_book(
    State(state): State<AppState>,
    _claims: Claims,
    Path(slug): Path<String>,
    Json(body): Json<UpdateBookRequest>,
) -> Result<Json<BookResponse>, Error> {
    let dto = UpdateBookDto {
        title: body.title,
        summary: body.summary,
        description: body.description,
        cover_url: body.cover_url,
        page_count: body.page_count,
        is_published: body.is_published,
    };
    state
        .services
        .book_repo
        .update_book(&slug, dto)
        .await?
        .ok_or_else(|| Error::not_found("Book not found"))
        .map(Json)
}

async fn list_book_authors(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<AuthorResponse>>, Error> {
    // Reuse get_books_by_author logic via a dedicated traversal — for now
    // we return authors related to the book via the `wrote` edge.
    let mut resp = state
        .db
        .query("SELECT <-wrote<-author.* FROM book WHERE slug = $slug")
        .bind(("slug", slug))
        .await?;

    use crate::db::book_repo::Author;
    let authors: Vec<Author> = resp.take(0)?;
    Ok(Json(
        authors.into_iter().map(AuthorResponse::from).collect(),
    ))
}

async fn list_authors(
    State(state): State<AppState>,
    Query(q): Query<AuthorListQuery>,
) -> Result<Json<Vec<AuthorResponse>>, Error> {
    let authors = state
        .services
        .book_repo
        .list_authors(q.q.as_deref(), q.limit(), q.offset())
        .await?;
    Ok(Json(authors))
}

async fn get_author(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<AuthorResponse>, Error> {
    state
        .services
        .book_repo
        .get_author_by_slug(&slug)
        .await?
        .ok_or_else(|| Error::not_found("Author not found"))
        .map(Json)
}

async fn create_author(
    State(state): State<AppState>,
    _claims: Claims,
    Json(body): Json<CreateAuthorRequest>,
) -> Result<(StatusCode, Json<AuthorResponse>), Error> {
    body.validate()?;
    let dto = CreateAuthorDto {
        name: body.name,
        slug: body.slug,
        bio: body.bio,
        avatar_url: body.avatar_url,
        website: body.website,
    };
    let author = state.services.book_repo.create_author(dto).await?;
    Ok((StatusCode::CREATED, Json(author)))
}

async fn follow_author(
    State(state): State<AppState>,
    claims: Claims,
    Path(slug): Path<String>,
) -> Result<StatusCode, Error> {
    state
        .services
        .book_repo
        .follow_author(&claims.sub, &slug)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unfollow_author(
    State(state): State<AppState>,
    claims: Claims,
    Path(slug): Path<String>,
) -> Result<StatusCode, Error> {
    state
        .services
        .book_repo
        .unfollow_author(&claims.sub, &slug)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<CategoryResponse>>, Error> {
    let categories = state.services.book_repo.list_categories().await?;
    Ok(Json(categories))
}

async fn get_books_by_category(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<BookResponse>>, Error> {
    let books = state
        .services
        .book_repo
        .get_books_by_category(&slug, pagination.limit(), pagination.offset())
        .await?;
    Ok(Json(books))
}

async fn get_books_by_tag(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<BookResponse>>, Error> {
    let books = state
        .services
        .book_repo
        .get_books_by_tag(&slug, pagination.limit(), pagination.offset())
        .await?;
    Ok(Json(books))
}
