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

// ── Highlight ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Highlight {
    pub id: Option<RecordId>,
    pub user: RecordId,
    pub book: RecordId,
    pub chapter: RecordId,
    pub offset_start: i64,
    pub offset_end: i64,
    pub paragraph: i64,
    pub text_snapshot: String,
    pub color: String,
    pub note: Option<String>,
    pub is_public: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct HighlightResponse {
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
    #[schemars(with = "Option<String>")]
    pub created_at: Option<DateTime<Utc>>,
}

impl From<Highlight> for HighlightResponse {
    fn from(h: Highlight) -> Self {
        HighlightResponse {
            id: h.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            user_id: key_to_string(h.user.key),
            book_id: key_to_string(h.book.key),
            chapter_id: key_to_string(h.chapter.key),
            offset_start: h.offset_start,
            offset_end: h.offset_end,
            paragraph: h.paragraph,
            text_snapshot: h.text_snapshot,
            color: h.color,
            note: h.note,
            is_public: h.is_public,
            created_at: h.created_at,
        }
    }
}

pub struct CreateHighlightDto {
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

pub struct UpdateHighlightDto {
    pub color: Option<String>,
    pub note: Option<String>,
    pub is_public: Option<bool>,
}

// ── HighlightRepo ─────────────────────────────────────────────────────────────

pub struct HighlightRepo {
    pub db: Db,
}

impl HighlightRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn list_chapter_highlights(
        &self,
        book_slug: &str,
        chapter_slug: &str,
        public_only: bool,
    ) -> Result<Vec<HighlightResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM highlight \
                 WHERE chapter = (SELECT id FROM chapter \
                                  WHERE slug = $cs \
                                  AND book = (SELECT id FROM book WHERE slug = $bs)[0])[0] \
                 AND ($pub = false OR is_public = true) \
                 ORDER BY offset_start ASC",
            )
            .bind(("bs", book_slug.to_string()))
            .bind(("cs", chapter_slug.to_string()))
            .bind(("pub", public_only))
            .await?;

        let highlights: Vec<Highlight> = resp.take(0)?;
        Ok(highlights
            .into_iter()
            .map(HighlightResponse::from)
            .collect())
    }

    /// List all highlights created by a user.
    pub async fn list_user_highlights(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<HighlightResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM highlight \
                 WHERE user = type::record('user', $user_id) \
                 ORDER BY created_at DESC LIMIT $l START $o",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;

        let highlights: Vec<Highlight> = resp.take(0)?;
        Ok(highlights
            .into_iter()
            .map(HighlightResponse::from)
            .collect())
    }

    /// Create a new highlight after resolving book/chapter slugs to RecordIds.
    pub async fn create_highlight(
        &self,
        user_id: &str,
        dto: CreateHighlightDto,
    ) -> Result<HighlightResponse, Error> {
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

        let data = json!({
            "user": format!("user:{user_id}"),
            "book": book_id,
            "chapter": chapter_id,
            "offset_start": dto.offset_start,
            "offset_end": dto.offset_end,
            "paragraph": dto.paragraph,
            "text_snapshot": dto.text_snapshot,
            "color": dto.color.unwrap_or_else(|| "yellow".to_string()),
            "note": dto.note,
            "is_public": dto.is_public.unwrap_or(false),
        });

        let mut resp = self
            .db
            .query("CREATE highlight CONTENT $data")
            .bind(("data", data))
            .await?;

        let created: Vec<Highlight> = resp.take(0)?;
        let highlight = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("highlight_repo", "insert failed"))?;

        Ok(HighlightResponse::from(highlight))
    }

    /// Update color, note, or visibility of a highlight (owner only).
    pub async fn update_highlight(
        &self,
        highlight_id: &str,
        user_id: &str,
        dto: UpdateHighlightDto,
    ) -> Result<Option<HighlightResponse>, Error> {
        let mut patch = serde_json::Map::new();
        if let Some(v) = dto.color {
            patch.insert("color".to_string(), json!(v));
        }
        if let Some(v) = dto.note {
            patch.insert("note".to_string(), json!(v));
        }
        if let Some(v) = dto.is_public {
            patch.insert("is_public".to_string(), json!(v));
        }
        patch.insert("updated_at".to_string(), json!(Utc::now()));

        let mut resp = self
            .db
            .query(
                "UPDATE type::record('highlight', $highlight_id) \
                 MERGE $data \
                 WHERE user = type::record('user', $user_id) \
                 RETURN AFTER",
            )
            .bind(("highlight_id", highlight_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .bind(("data", serde_json::Value::Object(patch)))
            .await?;

        let highlight: Option<Highlight> = resp.take(0)?;
        Ok(highlight.map(HighlightResponse::from))
    }

    /// Delete a highlight (owner only).
    pub async fn delete_highlight(&self, highlight_id: &str, user_id: &str) -> Result<bool, Error> {
        self.db
            .query(
                "DELETE type::record('highlight', $highlight_id) \
             WHERE user = type::record('user', $user_id)",
            )
            .bind(("highlight_id", highlight_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .await?;

        Ok(true)
    }
}
