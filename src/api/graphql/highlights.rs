use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::api::middleware::Claims;
use crate::db::highlight_repo::{CreateHighlightDto, HighlightResponse, UpdateHighlightDto};
use crate::state::AppState;

#[derive(SimpleObject)]
pub struct HighlightGql {
    pub id: String,
    pub user_id: String,
    pub book_id: String,
    pub chapter_id: String,
    pub offset_start: i64,
    pub offset_end: i64,
    pub paragraph: i64,
    pub text_snapshot: String,
    pub color: String,
    pub note: Option<String>,
    pub is_public: bool,
}

impl From<HighlightResponse> for HighlightGql {
    fn from(r: HighlightResponse) -> Self {
        HighlightGql {
            id: r.id,
            user_id: r.user_id,
            book_id: r.book_id,
            chapter_id: r.chapter_id,
            offset_start: r.offset_start,
            offset_end: r.offset_end,
            paragraph: r.paragraph,
            text_snapshot: r.text_snapshot,
            color: r.color,
            note: r.note,
            is_public: r.is_public,
        }
    }
}

#[derive(InputObject)]
pub struct CreateHighlightInput {
    pub book_slug: String,
    pub chapter_slug: String,
    pub offset_start: i64,
    pub offset_end: i64,
    pub paragraph: i64,
    pub text_snapshot: String,
    pub color: Option<String>,
    pub note: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(InputObject)]
pub struct UpdateHighlightInput {
    pub color: Option<String>,
    pub note: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Default)]
pub struct HighlightQuery;

#[Object]
impl HighlightQuery {
    async fn chapter_highlights(
        &self,
        ctx: &Context<'_>,
        book_slug: String,
        chapter_slug: String,
        public: Option<bool>,
    ) -> Result<Vec<HighlightGql>> {
        let state = ctx.data::<AppState>()?;
        let highlights = state
            .services
            .highlight_repo
            .list_chapter_highlights(&book_slug, &chapter_slug, public.unwrap_or(false))
            .await?;
        Ok(highlights.into_iter().map(Into::into).collect())
    }

    async fn my_highlights(
        &self,
        ctx: &Context<'_>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<HighlightGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let highlights = state
            .services
            .highlight_repo
            .list_user_highlights(&claims.sub, limit.unwrap_or(20), offset.unwrap_or(0))
            .await?;
        Ok(highlights.into_iter().map(Into::into).collect())
    }
}

#[derive(Default)]
pub struct HighlightMutation;

#[Object]
impl HighlightMutation {
    async fn create_highlight(
        &self,
        ctx: &Context<'_>,
        input: CreateHighlightInput,
    ) -> Result<HighlightGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let dto = CreateHighlightDto {
            book_slug: input.book_slug,
            chapter_slug: input.chapter_slug,
            offset_start: input.offset_start,
            offset_end: input.offset_end,
            paragraph: input.paragraph,
            text_snapshot: input.text_snapshot,
            color: input.color,
            note: input.note,
            is_public: input.is_public,
        };
        let highlight = state
            .services
            .highlight_repo
            .create_highlight(&claims.sub, dto)
            .await?;
        Ok(highlight.into())
    }

    async fn update_highlight(
        &self,
        ctx: &Context<'_>,
        highlight_id: String,
        input: UpdateHighlightInput,
    ) -> Result<Option<HighlightGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let dto = UpdateHighlightDto {
            color: input.color,
            note: input.note,
            is_public: input.is_public,
        };
        let highlight = state
            .services
            .highlight_repo
            .update_highlight(&highlight_id, &claims.sub, dto)
            .await?;
        Ok(highlight.map(Into::into))
    }

    async fn delete_highlight(&self, ctx: &Context<'_>, highlight_id: String) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .highlight_repo
            .delete_highlight(&highlight_id, &claims.sub)
            .await?;
        Ok(true)
    }
}
