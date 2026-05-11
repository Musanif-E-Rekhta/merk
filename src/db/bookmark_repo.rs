use chrono::{DateTime, Datelike, Utc};
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
    #[serde(default)]
    pub last_chapter: Option<RecordId>,
    #[serde(default)]
    pub last_offset: Option<i64>,
    #[serde(default)]
    pub last_read_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub progress_pct: Option<f64>,
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
    pub last_chapter_id: Option<String>,
    pub last_offset: Option<i64>,
    #[schemars(with = "Option<String>")]
    pub last_read_at: Option<DateTime<Utc>>,
    pub progress_pct: Option<f64>,
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
            last_chapter_id: b.last_chapter.map(|r| key_to_string(r.key)),
            last_offset: b.last_offset,
            last_read_at: b.last_read_at,
            progress_pct: b.progress_pct,
        }
    }
}

// ── Continue Reading rail ─────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ContinueItemResponse {
    pub book_id: String,
    pub book_slug: String,
    pub book_title: String,
    pub cover_url: Option<String>,
    pub last_chapter_id: Option<String>,
    pub last_chapter_slug: Option<String>,
    pub last_chapter_title: Option<String>,
    pub last_chapter_number: Option<i64>,
    pub progress_pct: Option<f64>,
    pub time_left_mins: Option<i64>,
    #[schemars(with = "Option<String>")]
    pub last_read_at: Option<DateTime<Utc>>,
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
    pub on_track: bool,
    pub pace_hint: Option<String>,
}

impl From<ReadingGoal> for ReadingGoalResponse {
    fn from(g: ReadingGoal) -> Self {
        let progress_pct =
            if g.target > 0 { (g.completed as f64 / g.target as f64) * 100.0 } else { 0.0 };
        let (on_track, pace_hint) = compute_pace(g.year, g.target, g.completed);
        ReadingGoalResponse {
            id: g.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            year: g.year,
            target: g.target,
            completed: g.completed,
            progress_pct,
            on_track,
            pace_hint,
        }
    }
}

/// Compute on-track flag and a server-rendered pace hint for a reading goal.
/// Mid-year a target of 24 wants ~12 done; if the user is at 14, they're
/// "ahead by 2"; at 9, "behind by 3"; at 24+, "Goal reached".
fn compute_pace(year: i64, target: i64, completed: i64) -> (bool, Option<String>) {
    if target <= 0 {
        return (true, None);
    }
    if completed >= target {
        return (true, Some("Goal reached!".into()));
    }

    let now = Utc::now();
    let current_year = now.year() as i64;

    // For past years, just report behind/ahead vs the final target.
    if year < current_year {
        let behind = target - completed;
        return (false, Some(format!("Ended {} short of target", behind)));
    }
    // For future years there's no expected pace yet.
    if year > current_year {
        return (true, Some(format!("0 of {} so far", target)));
    }

    // Current year: prorate by day-of-year against 365/366.
    let days_in_year = if (year as i32).rem_euclid(4) == 0
        && ((year as i32).rem_euclid(100) != 0 || (year as i32).rem_euclid(400) == 0)
    {
        366.0
    } else {
        365.0
    };
    let day_of_year = now.ordinal() as f64;
    let expected = (target as f64) * (day_of_year / days_in_year);
    let diff = (completed as f64) - expected;

    if diff >= 0.0 {
        let ahead = diff.floor() as i64;
        if ahead == 0 {
            (true, Some(format!("On pace · {} of {}", completed, target)))
        } else {
            (
                true,
                Some(format!("Ahead by {} · {} of {}", ahead, completed, target)),
            )
        }
    } else {
        let behind = (-diff).ceil() as i64;
        (
            false,
            Some(format!(
                "Behind by {} · {} of {}",
                behind, completed, target
            )),
        )
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

    /// List all bookmarks for a user, optionally filtered by status and
    /// ordered by `updated_at_desc` (default), `completed_at_desc`,
    /// `last_read_at_desc`, or `created_asc`.
    pub async fn list_user_bookmarks(
        &self,
        user_id: &str,
        status: Option<&str>,
        order: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BookmarkResponse>, Error> {
        // Whitelist sort keys so we can interpolate without query-injection risk.
        let order_clause = match order.unwrap_or("updated_at_desc") {
            "completed_at_desc" => "ORDER BY completed_at DESC",
            "last_read_at_desc" => "ORDER BY last_read_at DESC",
            "created_asc" => "ORDER BY created_at ASC",
            "created_desc" => "ORDER BY created_at DESC",
            _ => "ORDER BY updated_at DESC",
        };

        let sql = format!(
            "SELECT * FROM bookmark \
             WHERE in = type::record('user', $user_id) \
             AND ($status = NONE OR status = $status) \
             {} LIMIT $l START $o",
            order_clause
        );

        let mut resp = self
            .db
            .query(sql)
            .bind(("user_id", user_id.to_string()))
            .bind(("status", status.map(str::to_string)))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;

        let bookmarks: Vec<Bookmark> = resp.take(0)?;
        Ok(bookmarks.into_iter().map(BookmarkResponse::from).collect())
    }

    /// Continue Reading rail — last-N in-progress reads, joined with book +
    /// last-chapter info so the client gets one composed payload.
    pub async fn list_continue(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<ContinueItemResponse>, Error> {
        // Bookmark + book are joined via `out`; last_chapter is a record link.
        // FETCH expands those links inline so we can pluck nested fields.
        let mut resp = self
            .db
            .query(
                "SELECT id, progress_pct, last_read_at, \
                        out.id          AS book_id, \
                        out.slug        AS book_slug, \
                        out.title       AS book_title, \
                        out.cover_url   AS cover_url, \
                        last_chapter.id                AS last_chapter_id, \
                        last_chapter.slug              AS last_chapter_slug, \
                        last_chapter.title             AS last_chapter_title, \
                        last_chapter.number            AS last_chapter_number, \
                        last_chapter.reading_time_mins AS last_chapter_time_mins \
                 FROM bookmark \
                 WHERE in = type::record('user', $user_id) \
                   AND status = 'reading' \
                 ORDER BY last_read_at DESC, updated_at DESC \
                 LIMIT $l",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("l", limit))
            .await?;

        #[derive(Debug, Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct Row {
            progress_pct: Option<f64>,
            last_read_at: Option<DateTime<Utc>>,
            book_id: Option<RecordId>,
            book_slug: Option<String>,
            book_title: Option<String>,
            cover_url: Option<String>,
            last_chapter_id: Option<RecordId>,
            last_chapter_slug: Option<String>,
            last_chapter_title: Option<String>,
            last_chapter_number: Option<i64>,
            last_chapter_time_mins: Option<i64>,
        }

        let rows: Vec<Row> = resp.take(0)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let book_id = r.book_id.map(|rid| key_to_string(rid.key))?;
                let book_slug = r.book_slug?;
                let book_title = r.book_title?;
                // time_left = chapter total - (chapter total * progress_pct/100) — rough.
                let time_left_mins = match (r.last_chapter_time_mins, r.progress_pct) {
                    (Some(total), Some(pct)) => {
                        let remaining = (total as f64) * (1.0 - (pct / 100.0)).max(0.0);
                        Some(remaining.ceil() as i64)
                    }
                    (Some(total), None) => Some(total),
                    _ => None,
                };
                Some(ContinueItemResponse {
                    book_id,
                    book_slug,
                    book_title,
                    cover_url: r.cover_url,
                    last_chapter_id: r.last_chapter_id.map(|rid| key_to_string(rid.key)),
                    last_chapter_slug: r.last_chapter_slug,
                    last_chapter_title: r.last_chapter_title,
                    last_chapter_number: r.last_chapter_number,
                    progress_pct: r.progress_pct,
                    time_left_mins,
                    last_read_at: r.last_read_at,
                })
            })
            .collect())
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
