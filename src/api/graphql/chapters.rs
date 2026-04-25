use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::api::middleware::Claims;
use crate::db::chapter_repo::{
    ChapterListItem, ChapterNav, ChapterResponse, CreateChapterDto, UpdateChapterDto,
};
use crate::state::AppState;

// ── GQL output types ──────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct ChapterNavGql {
    pub number: i64,
    pub title: Option<String>,
    pub slug: String,
}

impl From<ChapterNav> for ChapterNavGql {
    fn from(r: ChapterNav) -> Self {
        ChapterNavGql {
            number: r.number,
            title: r.title,
            slug: r.slug,
        }
    }
}

#[derive(SimpleObject)]
pub struct ChapterGql {
    pub id: String,
    pub book_id: String,
    pub number: i64,
    pub title: Option<String>,
    pub slug: String,
    pub content: String,
    pub content_format: String,
    pub summary: Option<String>,
    pub meta_description: Option<String>,
    pub word_count: Option<i64>,
    pub reading_time_mins: Option<i64>,
    pub avg_rating: Option<f64>,
    pub review_count: i64,
    pub is_published: bool,
    pub prev_chapter: Option<ChapterNavGql>,
    pub next_chapter: Option<ChapterNavGql>,
}

impl From<ChapterResponse> for ChapterGql {
    fn from(r: ChapterResponse) -> Self {
        ChapterGql {
            id: r.id,
            book_id: r.book_id,
            number: r.number,
            title: r.title,
            slug: r.slug,
            content: r.content,
            content_format: r.content_format,
            summary: r.summary,
            meta_description: r.meta_description,
            word_count: r.word_count,
            reading_time_mins: r.reading_time_mins,
            avg_rating: r.avg_rating,
            review_count: r.review_count,
            is_published: r.is_published,
            prev_chapter: r.prev_chapter.map(Into::into),
            next_chapter: r.next_chapter.map(Into::into),
        }
    }
}

#[derive(SimpleObject)]
pub struct ChapterListItemGql {
    pub id: String,
    pub number: i64,
    pub title: Option<String>,
    pub slug: String,
    pub summary: Option<String>,
    pub reading_time_mins: Option<i64>,
    pub avg_rating: Option<f64>,
}

impl From<ChapterListItem> for ChapterListItemGql {
    fn from(r: ChapterListItem) -> Self {
        ChapterListItemGql {
            id: r.id,
            number: r.number,
            title: r.title,
            slug: r.slug,
            summary: r.summary,
            reading_time_mins: r.reading_time_mins,
            avg_rating: r.avg_rating,
        }
    }
}

// ── GQL input types ───────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct CreateChapterInput {
    pub number: i64,
    pub title: Option<String>,
    pub slug: String,
    pub content: String,
    pub content_format: Option<String>,
    pub summary: Option<String>,
    pub meta_description: Option<String>,
}

#[derive(InputObject)]
pub struct UpdateChapterInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub meta_description: Option<String>,
    pub is_published: Option<bool>,
}

// ── Query ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ChapterQuery;

#[Object]
impl ChapterQuery {
    async fn chapters(
        &self,
        ctx: &Context<'_>,
        book_slug: String,
    ) -> Result<Vec<ChapterListItemGql>> {
        let state = ctx.data::<AppState>()?;
        let chapters = state
            .services
            .chapter_repo
            .list_chapters(&book_slug)
            .await?;
        Ok(chapters.into_iter().map(Into::into).collect())
    }

    async fn chapter(
        &self,
        ctx: &Context<'_>,
        book_slug: String,
        chapter_slug: String,
    ) -> Result<Option<ChapterGql>> {
        let state = ctx.data::<AppState>()?;
        let chapter = state
            .services
            .chapter_repo
            .get_chapter_by_slug(&book_slug, &chapter_slug)
            .await?;
        Ok(chapter.map(Into::into))
    }
}

// ── Mutation ──────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ChapterMutation;

#[Object]
impl ChapterMutation {
    async fn create_chapter(
        &self,
        ctx: &Context<'_>,
        book_slug: String,
        input: CreateChapterInput,
    ) -> Result<ChapterGql> {
        let _claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = CreateChapterDto {
            number: input.number,
            title: input.title,
            slug: input.slug,
            content: input.content,
            content_format: input.content_format,
            summary: input.summary,
            meta_description: input.meta_description,
        };

        let chapter = state
            .services
            .chapter_repo
            .create_chapter(&book_slug, dto)
            .await?;
        Ok(chapter.into())
    }

    async fn update_chapter(
        &self,
        ctx: &Context<'_>,
        book_slug: String,
        chapter_slug: String,
        input: UpdateChapterInput,
    ) -> Result<Option<ChapterGql>> {
        let _claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let dto = UpdateChapterDto {
            title: input.title,
            content: input.content,
            summary: input.summary,
            meta_description: input.meta_description,
            is_published: input.is_published,
        };

        let chapter = state
            .services
            .chapter_repo
            .update_chapter(&book_slug, &chapter_slug, dto)
            .await?;
        Ok(chapter.map(Into::into))
    }
}
