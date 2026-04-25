use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::api::middleware::Claims;
use crate::db::review_repo::{
    BookReviewResponse, ChapterReviewResponse, CreateBookReviewDto, CreateChapterReviewDto,
    ReviewListFilters, UpdateBookReviewDto,
};
use crate::state::AppState;

// ── GQL output types ──────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct BookReviewGql {
    pub id: String,
    pub user_id: String,
    pub book_id: String,
    pub rating: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub contains_spoiler: bool,
    pub reading_status: String,
    pub verified_reader: bool,
    pub helpful_count: i64,
    pub status: String,
}

impl From<BookReviewResponse> for BookReviewGql {
    fn from(r: BookReviewResponse) -> Self {
        BookReviewGql {
            id: r.id,
            user_id: r.user_id,
            book_id: r.book_id,
            rating: r.rating,
            title: r.title,
            body: r.body,
            contains_spoiler: r.contains_spoiler,
            reading_status: r.reading_status,
            verified_reader: r.verified_reader,
            helpful_count: r.helpful_count,
            status: r.status,
        }
    }
}

#[derive(SimpleObject)]
pub struct ChapterReviewGql {
    pub id: String,
    pub user_id: String,
    pub chapter_id: String,
    pub rating: i64,
    pub body: Option<String>,
    pub contains_spoiler: bool,
    pub helpful_count: i64,
    pub status: String,
}

impl From<ChapterReviewResponse> for ChapterReviewGql {
    fn from(r: ChapterReviewResponse) -> Self {
        ChapterReviewGql {
            id: r.id,
            user_id: r.user_id,
            chapter_id: r.chapter_id,
            rating: r.rating,
            body: r.body,
            contains_spoiler: r.contains_spoiler,
            helpful_count: r.helpful_count,
            status: r.status,
        }
    }
}

// ── GQL input types ───────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct CreateBookReviewInput {
    pub book_slug: String,
    pub rating: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub contains_spoiler: Option<bool>,
    pub reading_status: String,
}

#[derive(InputObject)]
pub struct UpdateBookReviewInput {
    pub rating: Option<i64>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub contains_spoiler: Option<bool>,
}

#[derive(InputObject)]
pub struct CreateChapterReviewInput {
    pub book_slug: String,
    pub chapter_slug: String,
    pub rating: i64,
    pub body: Option<String>,
    pub contains_spoiler: Option<bool>,
}

// ── Query ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ReviewQuery;

#[Object]
impl ReviewQuery {
    async fn book_reviews(
        &self,
        ctx: &Context<'_>,
        book_slug: String,
        limit: Option<i64>,
        offset: Option<i64>,
        spoilers: Option<bool>,
    ) -> Result<Vec<BookReviewGql>> {
        let state = ctx.data::<AppState>()?;
        let filters = ReviewListFilters {
            spoilers,
            rating: None,
            status: Some("published".to_string()),
        };
        let reviews = state
            .services
            .review_repo
            .list_book_reviews(
                &book_slug,
                &filters,
                limit.unwrap_or(20),
                offset.unwrap_or(0),
            )
            .await?;
        Ok(reviews.into_iter().map(Into::into).collect())
    }

    async fn chapter_reviews(
        &self,
        ctx: &Context<'_>,
        book_slug: String,
        chapter_slug: String,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ChapterReviewGql>> {
        let state = ctx.data::<AppState>()?;
        let reviews = state
            .services
            .review_repo
            .list_chapter_reviews(
                &book_slug,
                &chapter_slug,
                limit.unwrap_or(20),
                offset.unwrap_or(0),
            )
            .await?;
        Ok(reviews.into_iter().map(Into::into).collect())
    }
}

// ── Mutation ──────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ReviewMutation;

#[Object]
impl ReviewMutation {
    async fn create_book_review(
        &self,
        ctx: &Context<'_>,
        input: CreateBookReviewInput,
    ) -> Result<BookReviewGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = CreateBookReviewDto {
            book_slug: input.book_slug,
            rating: input.rating,
            title: input.title,
            body: input.body,
            contains_spoiler: input.contains_spoiler,
            reading_status: input.reading_status,
        };

        let review = state
            .services
            .review_repo
            .create_book_review(&claims.sub, dto)
            .await?;
        Ok(review.into())
    }

    async fn update_book_review(
        &self,
        ctx: &Context<'_>,
        review_id: String,
        input: UpdateBookReviewInput,
    ) -> Result<Option<BookReviewGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = UpdateBookReviewDto {
            rating: input.rating,
            title: input.title,
            body: input.body,
            contains_spoiler: input.contains_spoiler,
        };

        let review = state
            .services
            .review_repo
            .update_book_review(&review_id, &claims.sub, dto)
            .await?;
        Ok(review.map(Into::into))
    }

    async fn delete_book_review(&self, ctx: &Context<'_>, review_id: String) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .review_repo
            .delete_book_review(&review_id, &claims.sub)
            .await?;
        Ok(true)
    }

    async fn vote_book_review(
        &self,
        ctx: &Context<'_>,
        review_id: String,
        value: i64,
    ) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .review_repo
            .vote_book_review(&claims.sub, &review_id, value)
            .await?;
        Ok(true)
    }

    async fn create_chapter_review(
        &self,
        ctx: &Context<'_>,
        input: CreateChapterReviewInput,
    ) -> Result<ChapterReviewGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = CreateChapterReviewDto {
            book_slug: input.book_slug,
            chapter_slug: input.chapter_slug,
            rating: input.rating,
            body: input.body,
            contains_spoiler: input.contains_spoiler,
        };

        let review = state
            .services
            .review_repo
            .create_chapter_review(&claims.sub, dto)
            .await?;
        Ok(review.into())
    }

    async fn vote_chapter_review(
        &self,
        ctx: &Context<'_>,
        review_id: String,
        value: i64,
    ) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .review_repo
            .vote_chapter_review(&claims.sub, &review_id, value)
            .await?;
        Ok(true)
    }

    async fn flag_review(
        &self,
        ctx: &Context<'_>,
        review_id: String,
        reason: String,
        note: Option<String>,
    ) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .review_repo
            .flag_review(&claims.sub, &review_id, &reason, note)
            .await?;
        Ok(true)
    }
}
