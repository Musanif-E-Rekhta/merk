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

// ── Bookmark ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Bookmark {
    pub id: Option<RecordId>,
    pub r#in: Option<RecordId>,
    pub out: Option<RecordId>,
    pub status: String,
    pub progress: Option<i64>,
    pub notes: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct BookmarkResponse {
    pub id: String,
    pub book_id: String,
    pub status: String,
    pub progress: Option<i64>,
    pub notes: Option<String>,
    #[schemars(with = "Option<String>")]
    pub started_at: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub completed_at: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Bookmark> for BookmarkResponse {
    fn from(b: Bookmark) -> Self {
        BookmarkResponse {
            id: b.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            book_id: b.out.map(|r| key_to_string(r.key)).unwrap_or_default(),
            status: b.status,
            progress: b.progress,
            notes: b.notes,
            started_at: b.started_at,
            completed_at: b.completed_at,
            updated_at: b.updated_at,
        }
    }
}

pub struct UpsertBookmarkDto {
    pub status: String,
    pub progress: Option<i64>,
    pub notes: Option<String>,
}

// ── ReadingGoal ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct ReadingGoal {
    pub id: Option<RecordId>,
    pub user: RecordId,
    pub year: i64,
    pub target: i64,
    pub completed: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ReadingGoalResponse {
    pub id: String,
    pub year: i64,
    pub target: i64,
    pub completed: i64,
    pub progress_pct: f64,
}

impl From<ReadingGoal> for ReadingGoalResponse {
    fn from(g: ReadingGoal) -> Self {
        let progress_pct =
            if g.target > 0 { (g.completed as f64 / g.target as f64) * 100.0 } else { 0.0 };
        ReadingGoalResponse {
            id: g.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            year: g.year,
            target: g.target,
            completed: g.completed,
            progress_pct,
        }
    }
}

// ── BookmarkRepo ──────────────────────────────────────────────────────────────

pub struct BookmarkRepo {
    pub db: Db,
}

impl BookmarkRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Create or update a bookmark (RELATE edge) for a user on a book.
    pub async fn upsert_bookmark(
        &self,
        user_id: &str,
        book_slug: &str,
        dto: UpsertBookmarkDto,
    ) -> Result<BookmarkResponse, Error> {
        // Try UPDATE first; if nothing returned, RELATE
        let mut update_resp = self.db
            .query(
                "UPDATE bookmark \
                 SET status = $status, progress = $progress, notes = $notes, updated_at = time::now() \
                 WHERE in = type::record('user', $user_id) \
                 AND out = (SELECT id FROM book WHERE slug = $slug)[0] \
                 RETURN AFTER",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("slug", book_slug.to_string()))
            .bind(("status", dto.status.clone()))
            .bind(("progress", dto.progress))
            .bind(("notes", dto.notes.clone()))
            .await?;

        let updated: Vec<Bookmark> = update_resp.take(0)?;
        if let Some(b) = updated.into_iter().next() {
            return Ok(BookmarkResponse::from(b));
        }

        // No existing bookmark — create via RELATE
        let data = json!({
            "status": dto.status,
            "progress": dto.progress,
            "notes": dto.notes,
            "started_at": Utc::now(),
        });

        let mut relate_resp = self
            .db
            .query(
                "RELATE type::record('user', $user_id) \
                 ->bookmark \
                 ->(SELECT id FROM book WHERE slug = $slug)[0] \
                 CONTENT $data \
                 RETURN AFTER",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("slug", book_slug.to_string()))
            .bind(("data", data))
            .await?;

        let created: Vec<Bookmark> = relate_resp.take(0)?;
        let bookmark = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("bookmark_repo", "insert failed"))?;

        Ok(BookmarkResponse::from(bookmark))
    }

    /// Remove a bookmark edge between a user and a book (by slug).
    pub async fn remove_bookmark(&self, user_id: &str, book_slug: &str) -> Result<bool, Error> {
        self.db
            .query(
                "DELETE bookmark \
             WHERE in = type::record('user', $user_id) \
             AND out = (SELECT id FROM book WHERE slug = $slug)[0]",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("slug", book_slug.to_string()))
            .await?;

        Ok(true)
    }

    /// List all bookmarks for a user, optionally filtered by status.
    pub async fn list_user_bookmarks(
        &self,
        user_id: &str,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BookmarkResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT *, out as out FROM bookmark \
                 WHERE in = type::record('user', $user_id) \
                 AND ($status = NONE OR status = $status) \
                 ORDER BY updated_at DESC LIMIT $l START $o",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("status", status.map(str::to_string)))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;

        let bookmarks: Vec<Bookmark> = resp.take(0)?;
        Ok(bookmarks.into_iter().map(BookmarkResponse::from).collect())
    }

    /// Get a single bookmark for a user and book slug.
    pub async fn get_bookmark(
        &self,
        user_id: &str,
        book_slug: &str,
    ) -> Result<Option<BookmarkResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM bookmark \
                 WHERE in = type::record('user', $user_id) \
                 AND out = (SELECT id FROM book WHERE slug = $slug)[0] \
                 LIMIT 1",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("slug", book_slug.to_string()))
            .await?;

        let bookmark: Option<Bookmark> = resp.take(0)?;
        Ok(bookmark.map(BookmarkResponse::from))
    }

    /// Get a user's reading goal for a given year.
    pub async fn get_reading_goal(
        &self,
        user_id: &str,
        year: i64,
    ) -> Result<Option<ReadingGoalResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM reading_goal \
                 WHERE user = type::record('user', $user_id) AND year = $year \
                 LIMIT 1",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("year", year))
            .await?;

        let goal: Option<ReadingGoal> = resp.take(0)?;
        Ok(goal.map(ReadingGoalResponse::from))
    }

    /// Create or update a reading goal for a user and year.
    pub async fn upsert_reading_goal(
        &self,
        user_id: &str,
        year: i64,
        target: i64,
    ) -> Result<ReadingGoalResponse, Error> {
        // Try UPDATE first
        let mut update_resp = self
            .db
            .query(
                "UPDATE reading_goal \
                 SET target = $target, updated_at = time::now() \
                 WHERE user = type::record('user', $user_id) AND year = $year \
                 RETURN AFTER",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("year", year))
            .bind(("target", target))
            .await?;

        let updated: Vec<ReadingGoal> = update_resp.take(0)?;
        if let Some(g) = updated.into_iter().next() {
            return Ok(ReadingGoalResponse::from(g));
        }

        // Not found — create new
        let data = json!({
            "user": format!("user:{user_id}"),
            "year": year,
            "target": target,
            "completed": 0,
        });

        let mut create_resp = self
            .db
            .query("CREATE reading_goal CONTENT $data")
            .bind(("data", data))
            .await?;

        let created: Vec<ReadingGoal> = create_resp.take(0)?;
        let goal = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("bookmark_repo", "reading goal insert failed"))?;

        Ok(ReadingGoalResponse::from(goal))
    }
}
