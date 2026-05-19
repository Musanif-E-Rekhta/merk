use async_graphql::{ComplexObject, Context, InputObject, Object, Result, SimpleObject};

use crate::api::middleware::Claims;
use crate::db::book_repo::{
    AuthorResponse, BookAuthorEdgeResponse, BookFilters, BookResponse, CategoryResponse,
    CreateAuthorDto, CreateBookDto, FeaturedBookResponse, TagResponse, UpdateBookDto,
};
use crate::state::AppState;

// ── GQL output types ──────────────────────────────────────────────────────────

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct BookGql {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub isbn: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub page_count: Option<i64>,
    pub language: String,
    pub avg_rating: Option<f64>,
    pub review_count: i64,
    pub chapter_count: i64,
    pub is_published: bool,
}

#[ComplexObject]
impl BookGql {
    /// Authors that `wrote` this book, each with the role recorded on the
    /// edge. Empty when the join has no rows (e.g. seed data without a
    /// `wrote` link) — the client renders an "Unknown Author" fallback.
    async fn authors(&self, ctx: &Context<'_>) -> Result<Vec<BookAuthorEdgeGql>> {
        let state = ctx.data::<AppState>()?;
        let edges = state
            .services
            .book_repo
            .list_authors_for_book(&self.id)
            .await?;
        Ok(edges.into_iter().map(BookAuthorEdgeGql::from).collect())
    }
}

#[derive(SimpleObject)]
pub struct BookAuthorEdgeGql {
    pub author: BookAuthorGql,
    pub role: String,
}

impl From<BookAuthorEdgeResponse> for BookAuthorEdgeGql {
    fn from(r: BookAuthorEdgeResponse) -> Self {
        BookAuthorEdgeGql {
            author: BookAuthorGql::from(r.author),
            role: r.role,
        }
    }
}

impl From<BookResponse> for BookGql {
    fn from(r: BookResponse) -> Self {
        BookGql {
            id: r.id,
            title: r.title,
            slug: r.slug,
            isbn: r.isbn,
            summary: r.summary,
            description: r.description,
            cover_url: r.cover_url,
            page_count: r.page_count,
            language: r.language,
            avg_rating: r.avg_rating,
            review_count: r.review_count,
            chapter_count: r.chapter_count,
            is_published: r.is_published,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct BookAuthorGql {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
}

#[ComplexObject]
impl BookAuthorGql {
    /// Whether the requesting user follows this author. Returns `false` for
    /// unauthenticated callers — never errors so the field can ride along on
    /// any author payload.
    async fn is_following(&self, ctx: &Context<'_>) -> Result<bool> {
        let Some(claims) = ctx.data_opt::<Claims>() else {
            return Ok(false);
        };
        let state = ctx.data::<AppState>()?;
        Ok(state
            .services
            .book_repo
            .is_following_author(&claims.sub, &self.slug)
            .await
            .unwrap_or(false))
    }
}

impl From<AuthorResponse> for BookAuthorGql {
    fn from(r: AuthorResponse) -> Self {
        BookAuthorGql {
            id: r.id,
            name: r.name,
            slug: r.slug,
            bio: r.bio,
            avatar_url: r.avatar_url,
            website: r.website,
        }
    }
}

#[derive(SimpleObject)]
pub struct FeaturedBookGql {
    pub book: BookGql,
    pub featured_until: Option<String>,
    pub eyebrow: Option<String>,
    pub headline: Option<String>,
    pub blurb: Option<String>,
}

impl From<FeaturedBookResponse> for FeaturedBookGql {
    fn from(r: FeaturedBookResponse) -> Self {
        FeaturedBookGql {
            book: r.book.into(),
            featured_until: r.featured_until.map(|d| d.to_rfc3339()),
            eyebrow: r.eyebrow,
            headline: r.headline,
            blurb: r.blurb,
        }
    }
}

#[derive(SimpleObject)]
pub struct CategoryGql {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
}

impl From<CategoryResponse> for CategoryGql {
    fn from(r: CategoryResponse) -> Self {
        CategoryGql {
            id: r.id,
            name: r.name,
            slug: r.slug,
            description: r.description,
        }
    }
}

#[derive(SimpleObject)]
pub struct TagGql {
    pub id: String,
    pub name: String,
    pub slug: String,
}

impl From<TagResponse> for TagGql {
    fn from(r: TagResponse) -> Self {
        TagGql {
            id: r.id,
            name: r.name,
            slug: r.slug,
        }
    }
}

// ── GQL input types ───────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct CreateBookInput {
    pub title: String,
    pub slug: String,
    pub isbn: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
}

#[derive(InputObject)]
pub struct UpdateBookInput {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub is_published: Option<bool>,
}

#[derive(InputObject)]
pub struct CreateAuthorInput {
    pub name: String,
    pub slug: String,
    pub bio: Option<String>,
}

// ── Query ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct BookQuery;

#[Object]
impl BookQuery {
    async fn books(
        &self,
        ctx: &Context<'_>,
        q: Option<String>,
        lang: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<BookGql>> {
        let state = ctx.data::<AppState>()?;
        let f = BookFilters {
            q,
            lang,
            is_published: Some(true),
        };
        let books = state
            .services
            .book_repo
            .list_books(&f, limit.unwrap_or(20), offset.unwrap_or(0))
            .await?;
        Ok(books.into_iter().map(Into::into).collect())
    }

    async fn book(&self, ctx: &Context<'_>, slug: String) -> Result<Option<BookGql>> {
        let state = ctx.data::<AppState>()?;
        let book = state.services.book_repo.get_book_by_slug(&slug).await?;
        Ok(book.map(Into::into))
    }

    async fn featured(&self, ctx: &Context<'_>) -> Result<Option<FeaturedBookGql>> {
        let state = ctx.data::<AppState>()?;
        let pick = state.services.book_repo.get_featured().await?;
        Ok(pick.map(Into::into))
    }

    async fn authors(
        &self,
        ctx: &Context<'_>,
        q: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<BookAuthorGql>> {
        let state = ctx.data::<AppState>()?;
        let authors = state
            .services
            .book_repo
            .list_authors(q.as_deref(), limit.unwrap_or(20), offset.unwrap_or(0))
            .await?;
        Ok(authors.into_iter().map(Into::into).collect())
    }

    async fn author(&self, ctx: &Context<'_>, slug: String) -> Result<Option<BookAuthorGql>> {
        let state = ctx.data::<AppState>()?;
        let author = state.services.book_repo.get_author_by_slug(&slug).await?;
        Ok(author.map(Into::into))
    }

    async fn categories(&self, ctx: &Context<'_>) -> Result<Vec<CategoryGql>> {
        let state = ctx.data::<AppState>()?;
        let cats = state.services.book_repo.list_categories().await?;
        Ok(cats.into_iter().map(Into::into).collect())
    }

    async fn books_by_author(
        &self,
        ctx: &Context<'_>,
        author_slug: String,
    ) -> Result<Vec<BookGql>> {
        let state = ctx.data::<AppState>()?;
        let books = state
            .services
            .book_repo
            .get_books_by_author(&author_slug)
            .await?;
        Ok(books.into_iter().map(Into::into).collect())
    }

    async fn books_by_category(
        &self,
        ctx: &Context<'_>,
        slug: String,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<BookGql>> {
        let state = ctx.data::<AppState>()?;
        let books = state
            .services
            .book_repo
            .get_books_by_category(&slug, limit.unwrap_or(20), offset.unwrap_or(0))
            .await?;
        Ok(books.into_iter().map(Into::into).collect())
    }

    async fn books_by_tag(
        &self,
        ctx: &Context<'_>,
        slug: String,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<BookGql>> {
        let state = ctx.data::<AppState>()?;
        let books = state
            .services
            .book_repo
            .get_books_by_tag(&slug, limit.unwrap_or(20), offset.unwrap_or(0))
            .await?;
        Ok(books.into_iter().map(Into::into).collect())
    }
}

// ── Mutation ──────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct BookMutation;

#[Object]
impl BookMutation {
    async fn create_book(&self, ctx: &Context<'_>, input: CreateBookInput) -> Result<BookGql> {
        let _claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = CreateBookDto {
            title: input.title,
            slug: input.slug,
            isbn: input.isbn,
            summary: input.summary,
            description: input.description,
            cover_url: None,
            page_count: None,
            language: input.language,
            publisher_slug: None,
        };

        let book = state.services.book_repo.create_book(dto).await?;
        Ok(book.into())
    }

    async fn update_book(
        &self,
        ctx: &Context<'_>,
        slug: String,
        input: UpdateBookInput,
    ) -> Result<Option<BookGql>> {
        let _claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = UpdateBookDto {
            title: input.title,
            summary: input.summary,
            description: input.description,
            cover_url: None,
            page_count: None,
            is_published: input.is_published,
        };

        let book = state.services.book_repo.update_book(&slug, dto).await?;
        Ok(book.map(Into::into))
    }

    async fn create_author(
        &self,
        ctx: &Context<'_>,
        input: CreateAuthorInput,
    ) -> Result<BookAuthorGql> {
        let _claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = CreateAuthorDto {
            name: input.name,
            slug: input.slug,
            bio: input.bio,
            avatar_url: None,
            website: None,
        };

        let author = state.services.book_repo.create_author(dto).await?;
        Ok(author.into())
    }

    async fn follow_author(&self, ctx: &Context<'_>, slug: String) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .book_repo
            .follow_author(&claims.sub, &slug)
            .await?;
        Ok(true)
    }

    async fn unfollow_author(&self, ctx: &Context<'_>, slug: String) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .book_repo
            .unfollow_author(&claims.sub, &slug)
            .await?;
        Ok(true)
    }
}
