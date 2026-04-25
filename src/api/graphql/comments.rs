use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::api::middleware::Claims;
use crate::db::comment_repo::{CommentResponse, CreateCommentDto, UpdateCommentDto};
use crate::state::AppState;

#[derive(SimpleObject)]
pub struct CommentGql {
    pub id: String,
    pub user_id: String,
    pub chapter_id: String,
    pub highlight_id: Option<String>,
    pub parent_id: Option<String>,
    pub body: String,
    pub is_spoiler: bool,
    pub is_deleted: bool,
    pub offset_start: Option<i64>,
    pub offset_end: Option<i64>,
    pub text_snapshot: Option<String>,
}

impl From<CommentResponse> for CommentGql {
    fn from(r: CommentResponse) -> Self {
        CommentGql {
            id: r.id,
            user_id: r.user_id,
            chapter_id: r.chapter_id,
            highlight_id: r.highlight_id,
            parent_id: r.parent_id,
            body: r.body,
            is_spoiler: r.is_spoiler,
            is_deleted: r.is_deleted,
            offset_start: r.offset_start,
            offset_end: r.offset_end,
            text_snapshot: r.text_snapshot,
        }
    }
}

#[derive(InputObject)]
pub struct CreateCommentInput {
    pub book_slug: String,
    pub chapter_slug: String,
    pub highlight_id: Option<String>,
    pub parent_id: Option<String>,
    pub body: String,
    pub is_spoiler: Option<bool>,
    pub offset_start: Option<i64>,
    pub offset_end: Option<i64>,
    pub text_snapshot: Option<String>,
}

#[derive(Default)]
pub struct CommentQuery;

#[Object]
impl CommentQuery {
    async fn chapter_comments(
        &self,
        ctx: &Context<'_>,
        book_slug: String,
        chapter_slug: String,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<CommentGql>> {
        let state = ctx.data::<AppState>()?;
        let comments = state
            .services
            .comment_repo
            .list_chapter_comments(
                &book_slug,
                &chapter_slug,
                limit.unwrap_or(20),
                offset.unwrap_or(0),
            )
            .await?;
        Ok(comments.into_iter().map(Into::into).collect())
    }

    async fn highlight_comments(
        &self,
        ctx: &Context<'_>,
        highlight_id: String,
    ) -> Result<Vec<CommentGql>> {
        let state = ctx.data::<AppState>()?;
        let comments = state
            .services
            .comment_repo
            .list_highlight_comments(&highlight_id)
            .await?;
        Ok(comments.into_iter().map(Into::into).collect())
    }

    async fn comment_replies(
        &self,
        ctx: &Context<'_>,
        parent_id: String,
    ) -> Result<Vec<CommentGql>> {
        let state = ctx.data::<AppState>()?;
        let replies = state.services.comment_repo.list_replies(&parent_id).await?;
        Ok(replies.into_iter().map(Into::into).collect())
    }
}

#[derive(Default)]
pub struct CommentMutation;

#[Object]
impl CommentMutation {
    async fn create_comment(
        &self,
        ctx: &Context<'_>,
        input: CreateCommentInput,
    ) -> Result<CommentGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let dto = CreateCommentDto {
            book_slug: input.book_slug,
            chapter_slug: input.chapter_slug,
            highlight_id: input.highlight_id,
            parent_id: input.parent_id,
            body: input.body,
            is_spoiler: input.is_spoiler,
            offset_start: input.offset_start,
            offset_end: input.offset_end,
            text_snapshot: input.text_snapshot,
        };
        let comment = state
            .services
            .comment_repo
            .create_comment(&claims.sub, dto)
            .await?;
        Ok(comment.into())
    }

    async fn update_comment(
        &self,
        ctx: &Context<'_>,
        comment_id: String,
        body: String,
    ) -> Result<Option<CommentGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let dto = UpdateCommentDto { body };
        let comment = state
            .services
            .comment_repo
            .update_comment(&comment_id, &claims.sub, dto)
            .await?;
        Ok(comment.map(Into::into))
    }

    async fn delete_comment(&self, ctx: &Context<'_>, comment_id: String) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .comment_repo
            .delete_comment(&comment_id, &claims.sub)
            .await?;
        Ok(true)
    }

    async fn vote_comment(
        &self,
        ctx: &Context<'_>,
        comment_id: String,
        value: i64,
    ) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .comment_repo
            .vote_comment(&claims.sub, &comment_id, value)
            .await?;
        Ok(true)
    }
}
