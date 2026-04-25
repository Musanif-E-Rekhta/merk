use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};

use crate::db::Db;
use crate::error::Error;

fn key_to_string(key: RecordIdKey) -> String {
    match key {
        RecordIdKey::String(s) => s,
        RecordIdKey::Number(n) => n.to_string(),
        RecordIdKey::Uuid(u) => u.to_string(),
        other => format!("{other:?}"),
    }
}

// ── Comment ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Comment {
    pub id: Option<RecordId>,
    pub user: RecordId,
    pub book: RecordId,
    pub chapter: RecordId,
    pub highlight: Option<RecordId>,
    pub parent: Option<RecordId>,
    pub body: String,
    pub is_spoiler: bool,
    pub offset_start: Option<i64>,
    pub offset_end: Option<i64>,
    pub text_snapshot: Option<String>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct CommentResponse {
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
    #[schemars(with = "Option<String>")]
    pub created_at: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Comment> for CommentResponse {
    fn from(c: Comment) -> Self {
        let is_deleted = c.deleted_at.is_some();
        let body = if is_deleted { "[deleted]".to_string() } else { c.body };
        CommentResponse {
            id: c.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            user_id: key_to_string(c.user.key),
            chapter_id: key_to_string(c.chapter.key),
            highlight_id: c.highlight.map(|r| key_to_string(r.key)),
            parent_id: c.parent.map(|r| key_to_string(r.key)),
            body,
            is_spoiler: c.is_spoiler,
            is_deleted,
            offset_start: c.offset_start,
            offset_end: c.offset_end,
            text_snapshot: c.text_snapshot,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

pub struct CreateCommentDto {
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

pub struct UpdateCommentDto {
    pub body: String,
}

// ── CommentRepo ───────────────────────────────────────────────────────────────

pub struct CommentRepo {
    pub db: Db,
}

impl CommentRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn list_chapter_comments(
        &self,
        book_slug: &str,
        chapter_slug: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CommentResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM comment \
                 WHERE chapter = (SELECT id FROM chapter \
                                  WHERE slug = $cs \
                                  AND book = (SELECT id FROM book WHERE slug = $bs)[0])[0] \
                 AND parent = NONE \
                 ORDER BY created_at ASC LIMIT $l START $o",
            )
            .bind(("bs", book_slug.to_string()))
            .bind(("cs", chapter_slug.to_string()))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;

        let comments: Vec<Comment> = resp.take(0)?;
        Ok(comments.into_iter().map(CommentResponse::from).collect())
    }

    /// List all comments attached to a highlight.
    pub async fn list_highlight_comments(
        &self,
        highlight_id: &str,
    ) -> Result<Vec<CommentResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM comment \
                 WHERE highlight = type::record('highlight', $highlight_id) \
                 ORDER BY created_at ASC",
            )
            .bind(("highlight_id", highlight_id.to_string()))
            .await?;

        let comments: Vec<Comment> = resp.take(0)?;
        Ok(comments.into_iter().map(CommentResponse::from).collect())
    }

    /// List direct replies to a parent comment.
    pub async fn list_replies(&self, parent_id: &str) -> Result<Vec<CommentResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM comment \
                 WHERE parent = type::record('comment', $parent_id) \
                 ORDER BY created_at ASC",
            )
            .bind(("parent_id", parent_id.to_string()))
            .await?;

        let comments: Vec<Comment> = resp.take(0)?;
        Ok(comments.into_iter().map(CommentResponse::from).collect())
    }

    /// Create a comment, resolving book/chapter slugs and optional highlight/parent refs.
    pub async fn create_comment(
        &self,
        user_id: &str,
        dto: CreateCommentDto,
    ) -> Result<CommentResponse, Error> {
        // Resolve book RecordId
        let mut book_resp = self
            .db
            .query("SELECT id FROM book WHERE slug = $slug LIMIT 1")
            .bind(("slug", dto.book_slug.clone()))
            .await?;
        let book_id: Option<RecordId> = book_resp.take("id")?;
        let book_id = book_id.ok_or_else(|| Error::not_found("Book not found"))?;

        // Resolve chapter RecordId
        let mut ch_resp = self
            .db
            .query("SELECT id FROM chapter WHERE slug = $cs AND book = $book LIMIT 1")
            .bind(("cs", dto.chapter_slug.clone()))
            .bind(("book", book_id.clone()))
            .await?;
        let chapter_id: Option<RecordId> = ch_resp.take("id")?;
        let chapter_id = chapter_id.ok_or_else(|| Error::not_found("Chapter not found"))?;

        // Build optional highlight / parent RecordIds as SurrealDB thing strings
        let highlight_val: serde_json::Value = dto
            .highlight_id
            .as_deref()
            .map(|id| serde_json::Value::String(format!("highlight:{id}")))
            .unwrap_or(serde_json::Value::Null);

        let parent_val: serde_json::Value = dto
            .parent_id
            .as_deref()
            .map(|id| serde_json::Value::String(format!("comment:{id}")))
            .unwrap_or(serde_json::Value::Null);

        let data = json!({
            "user": format!("user:{user_id}"),
            "book": book_id,
            "chapter": chapter_id,
            "highlight": highlight_val,
            "parent": parent_val,
            "body": dto.body,
            "is_spoiler": dto.is_spoiler.unwrap_or(false),
            "offset_start": dto.offset_start,
            "offset_end": dto.offset_end,
            "text_snapshot": dto.text_snapshot,
        });

        let mut resp = self
            .db
            .query("CREATE comment CONTENT $data")
            .bind(("data", data))
            .await?;

        let created: Vec<Comment> = resp.take(0)?;
        let comment = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("comment_repo", "insert failed"))?;

        Ok(CommentResponse::from(comment))
    }

    /// Update the body of a non-deleted comment (owner only).
    pub async fn update_comment(
        &self,
        comment_id: &str,
        user_id: &str,
        dto: UpdateCommentDto,
    ) -> Result<Option<CommentResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE type::record('comment', $comment_id) \
                 SET body = $body, updated_at = time::now() \
                 WHERE user = type::record('user', $user_id) \
                 AND deleted_at = NONE \
                 RETURN AFTER",
            )
            .bind(("comment_id", comment_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .bind(("body", dto.body))
            .await?;

        let comment: Option<Comment> = resp.take(0)?;
        Ok(comment.map(CommentResponse::from))
    }

    /// Soft-delete a comment by setting deleted_at (owner only).
    pub async fn delete_comment(&self, comment_id: &str, user_id: &str) -> Result<bool, Error> {
        self.db
            .query(
                "UPDATE type::record('comment', $comment_id) \
             SET deleted_at = time::now(), updated_at = time::now() \
             WHERE user = type::record('user', $user_id)",
            )
            .bind(("comment_id", comment_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .await?;

        Ok(true)
    }

    /// Vote on a comment (+1 or -1), creating or updating the vote edge.
    pub async fn vote_comment(
        &self,
        user_id: &str,
        comment_id: &str,
        value: i64,
    ) -> Result<(), Error> {
        if value != 1 && value != -1 {
            return Err(Error::bad_request(
                "invalid_vote",
                "Vote value must be 1 or -1",
            ));
        }

        let mut update_resp = self
            .db
            .query(
                "UPDATE comment_vote \
                 SET value = $value, updated_at = time::now() \
                 WHERE in = type::record('user', $user_id) \
                 AND out = type::record('comment', $comment_id) \
                 RETURN AFTER",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("comment_id", comment_id.to_string()))
            .bind(("value", value))
            .await?;

        let updated: Vec<serde_json::Value> = update_resp.take(0)?;
        if updated.is_empty() {
            self.db
                .query(
                    "RELATE type::record('user', $user_id) \
                 ->comment_vote \
                 ->type::record('comment', $comment_id) \
                 CONTENT { value: $value }",
                )
                .bind(("user_id", user_id.to_string()))
                .bind(("comment_id", comment_id.to_string()))
                .bind(("value", value))
                .await?;
        }

        Ok(())
    }
}
