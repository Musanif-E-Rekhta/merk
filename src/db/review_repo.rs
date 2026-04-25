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

// ── BookReview ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct BookReview {
    pub id: Option<RecordId>,
    pub r#in: Option<RecordId>,
    pub out: Option<RecordId>,
    pub rating: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub contains_spoiler: bool,
    pub reading_status: String,
    pub verified_reader: bool,
    pub helpful_count: i64,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct BookReviewResponse {
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
    #[schemars(with = "Option<String>")]
    pub created_at: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<BookReview> for BookReviewResponse {
    fn from(r: BookReview) -> Self {
        BookReviewResponse {
            id: r.id.map(|x| key_to_string(x.key)).unwrap_or_default(),
            user_id: r.r#in.map(|x| key_to_string(x.key)).unwrap_or_default(),
            book_id: r.out.map(|x| key_to_string(x.key)).unwrap_or_default(),
            rating: r.rating,
            title: r.title,
            body: r.body,
            contains_spoiler: r.contains_spoiler,
            reading_status: r.reading_status,
            verified_reader: r.verified_reader,
            helpful_count: r.helpful_count,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct CreateBookReviewDto {
    pub book_slug: String,
    pub rating: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub contains_spoiler: Option<bool>,
    pub reading_status: String,
}

pub struct UpdateBookReviewDto {
    pub rating: Option<i64>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub contains_spoiler: Option<bool>,
}

// ── ChapterReview ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct ChapterReview {
    pub id: Option<RecordId>,
    pub r#in: Option<RecordId>,
    pub out: Option<RecordId>,
    pub rating: i64,
    pub body: Option<String>,
    pub contains_spoiler: bool,
    pub helpful_count: i64,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ChapterReviewResponse {
    pub id: String,
    pub user_id: String,
    pub chapter_id: String,
    pub rating: i64,
    pub body: Option<String>,
    pub contains_spoiler: bool,
    pub helpful_count: i64,
    pub status: String,
    #[schemars(with = "Option<String>")]
    pub created_at: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<ChapterReview> for ChapterReviewResponse {
    fn from(r: ChapterReview) -> Self {
        ChapterReviewResponse {
            id: r.id.map(|x| key_to_string(x.key)).unwrap_or_default(),
            user_id: r.r#in.map(|x| key_to_string(x.key)).unwrap_or_default(),
            chapter_id: r.out.map(|x| key_to_string(x.key)).unwrap_or_default(),
            rating: r.rating,
            body: r.body,
            contains_spoiler: r.contains_spoiler,
            helpful_count: r.helpful_count,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct CreateChapterReviewDto {
    pub book_slug: String,
    pub chapter_slug: String,
    pub rating: i64,
    pub body: Option<String>,
    pub contains_spoiler: Option<bool>,
}

pub struct ReviewListFilters {
    pub spoilers: Option<bool>,
    pub rating: Option<i64>,
    pub status: Option<String>,
}

// ── ReviewRepo ────────────────────────────────────────────────────────────────

pub struct ReviewRepo {
    pub db: Db,
}

impl ReviewRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn list_book_reviews(
        &self,
        book_slug: &str,
        filters: &ReviewListFilters,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BookReviewResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM book_review \
                 WHERE out = (SELECT id FROM book WHERE slug = $slug)[0] \
                 AND ($spoilers = NONE OR contains_spoiler = $spoilers) \
                 AND ($rating = NONE OR rating = $rating) \
                 AND ($status = NONE OR status = $status) \
                 ORDER BY created_at DESC LIMIT $l START $o",
            )
            .bind(("slug", book_slug.to_string()))
            .bind(("spoilers", filters.spoilers))
            .bind(("rating", filters.rating))
            .bind(("status", filters.status.clone()))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;

        let reviews: Vec<BookReview> = resp.take(0)?;
        Ok(reviews.into_iter().map(BookReviewResponse::from).collect())
    }

    /// Get a single book review by its ID.
    pub async fn get_book_review(
        &self,
        review_id: &str,
    ) -> Result<Option<BookReviewResponse>, Error> {
        let mut resp = self
            .db
            .query("SELECT * FROM type::record('book_review', $id)")
            .bind(("id", review_id.to_string()))
            .await?;

        let review: Option<BookReview> = resp.take(0)?;
        Ok(review.map(BookReviewResponse::from))
    }

    /// Create a book review (RELATE user->book_review->book).
    pub async fn create_book_review(
        &self,
        user_id: &str,
        dto: CreateBookReviewDto,
    ) -> Result<BookReviewResponse, Error> {
        // Check if user has a bookmark to determine verified_reader
        let mut bm_resp = self
            .db
            .query(
                "SELECT id FROM bookmark \
                 WHERE in = type::record('user', $user_id) \
                 AND out = (SELECT id FROM book WHERE slug = $slug)[0] \
                 LIMIT 1",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("slug", dto.book_slug.clone()))
            .await?;
        let bm: Option<RecordId> = bm_resp.take("id")?;
        let verified_reader = bm.is_some();

        let data = json!({
            "rating": dto.rating,
            "title": dto.title,
            "body": dto.body,
            "contains_spoiler": dto.contains_spoiler.unwrap_or(false),
            "reading_status": dto.reading_status,
            "verified_reader": verified_reader,
            "helpful_count": 0,
            "status": "published",
        });

        let mut resp = self
            .db
            .query(
                "RELATE type::record('user', $user_id) \
                 ->book_review \
                 ->(SELECT id FROM book WHERE slug = $slug)[0] \
                 CONTENT $data \
                 RETURN AFTER",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("slug", dto.book_slug.clone()))
            .bind(("data", data))
            .await?;

        let created: Vec<BookReview> = resp.take(0)?;
        let review = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("review_repo", "insert failed"))?;

        Ok(BookReviewResponse::from(review))
    }

    /// Partial-update a book review (only the author may update).
    pub async fn update_book_review(
        &self,
        review_id: &str,
        user_id: &str,
        dto: UpdateBookReviewDto,
    ) -> Result<Option<BookReviewResponse>, Error> {
        let mut patch = serde_json::Map::new();
        if let Some(v) = dto.rating {
            patch.insert("rating".to_string(), json!(v));
        }
        if let Some(v) = dto.title {
            patch.insert("title".to_string(), json!(v));
        }
        if let Some(v) = dto.body {
            patch.insert("body".to_string(), json!(v));
        }
        if let Some(v) = dto.contains_spoiler {
            patch.insert("contains_spoiler".to_string(), json!(v));
        }
        patch.insert("updated_at".to_string(), json!(Utc::now()));

        let mut resp = self
            .db
            .query(
                "UPDATE type::record('book_review', $review_id) \
                 MERGE $data \
                 WHERE in = type::record('user', $user_id) \
                 RETURN AFTER",
            )
            .bind(("review_id", review_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .bind(("data", serde_json::Value::Object(patch)))
            .await?;

        let review: Option<BookReview> = resp.take(0)?;
        Ok(review.map(BookReviewResponse::from))
    }

    /// Soft-delete a book review by setting status = "removed".
    pub async fn delete_book_review(&self, review_id: &str, user_id: &str) -> Result<bool, Error> {
        self.db
            .query(
                "UPDATE type::record('book_review', $review_id) \
             SET status = 'removed', updated_at = time::now() \
             WHERE in = type::record('user', $user_id)",
            )
            .bind(("review_id", review_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .await?;

        Ok(true)
    }

    /// Vote on a book review (+1 helpful / -1 not helpful).
    pub async fn vote_book_review(
        &self,
        user_id: &str,
        review_id: &str,
        value: i64,
    ) -> Result<(), Error> {
        if value != 1 && value != -1 {
            return Err(Error::bad_request(
                "invalid_vote",
                "Vote value must be 1 or -1",
            ));
        }

        // Try UPDATE existing vote; if nothing found, RELATE
        let mut update_resp = self
            .db
            .query(
                "UPDATE book_review_vote \
                 SET value = $value, updated_at = time::now() \
                 WHERE in = type::record('user', $user_id) \
                 AND out = type::record('book_review', $review_id) \
                 RETURN AFTER",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("review_id", review_id.to_string()))
            .bind(("value", value))
            .await?;

        let updated: Vec<serde_json::Value> = update_resp.take(0)?;
        if updated.is_empty() {
            self.db
                .query(
                    "RELATE type::record('user', $user_id) \
                 ->book_review_vote \
                 ->type::record('book_review', $review_id) \
                 CONTENT { value: $value }",
                )
                .bind(("user_id", user_id.to_string()))
                .bind(("review_id", review_id.to_string()))
                .bind(("value", value))
                .await?;
        }

        Ok(())
    }

    /// List chapter reviews for a given book and chapter slug.
    pub async fn list_chapter_reviews(
        &self,
        book_slug: &str,
        chapter_slug: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChapterReviewResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM chapter_review \
                 WHERE out = (SELECT id FROM chapter \
                              WHERE slug = $cs \
                              AND book = (SELECT id FROM book WHERE slug = $bs)[0])[0] \
                 ORDER BY created_at DESC LIMIT $l START $o",
            )
            .bind(("bs", book_slug.to_string()))
            .bind(("cs", chapter_slug.to_string()))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;

        let reviews: Vec<ChapterReview> = resp.take(0)?;
        Ok(reviews
            .into_iter()
            .map(ChapterReviewResponse::from)
            .collect())
    }

    /// Create a chapter review (RELATE user->chapter_review->chapter).
    pub async fn create_chapter_review(
        &self,
        user_id: &str,
        dto: CreateChapterReviewDto,
    ) -> Result<ChapterReviewResponse, Error> {
        let data = json!({
            "rating": dto.rating,
            "body": dto.body,
            "contains_spoiler": dto.contains_spoiler.unwrap_or(false),
            "helpful_count": 0,
            "status": "published",
        });

        let mut resp = self
            .db
            .query(
                "RELATE type::record('user', $user_id) \
                 ->chapter_review \
                 ->(SELECT id FROM chapter \
                    WHERE slug = $cs \
                    AND book = (SELECT id FROM book WHERE slug = $bs)[0])[0] \
                 CONTENT $data \
                 RETURN AFTER",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("bs", dto.book_slug.clone()))
            .bind(("cs", dto.chapter_slug.clone()))
            .bind(("data", data))
            .await?;

        let created: Vec<ChapterReview> = resp.take(0)?;
        let review = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("review_repo", "chapter review insert failed"))?;

        Ok(ChapterReviewResponse::from(review))
    }

    /// Vote on a chapter review (+1 / -1).
    pub async fn vote_chapter_review(
        &self,
        user_id: &str,
        review_id: &str,
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
                "UPDATE chapter_review_vote \
                 SET value = $value, updated_at = time::now() \
                 WHERE in = type::record('user', $user_id) \
                 AND out = type::record('chapter_review', $review_id) \
                 RETURN AFTER",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("review_id", review_id.to_string()))
            .bind(("value", value))
            .await?;

        let updated: Vec<serde_json::Value> = update_resp.take(0)?;
        if updated.is_empty() {
            self.db
                .query(
                    "RELATE type::record('user', $user_id) \
                 ->chapter_review_vote \
                 ->type::record('chapter_review', $review_id) \
                 CONTENT { value: $value }",
                )
                .bind(("user_id", user_id.to_string()))
                .bind(("review_id", review_id.to_string()))
                .bind(("value", value))
                .await?;
        }

        Ok(())
    }

    /// Flag a review for moderation.
    pub async fn flag_review(
        &self,
        user_id: &str,
        review_id: &str,
        reason: &str,
        note: Option<String>,
    ) -> Result<(), Error> {
        let data = json!({
            "flagged_by": format!("user:{user_id}"),
            "review": format!("book_review:{review_id}"),
            "reason": reason,
            "note": note,
            "created_at": Utc::now(),
        });

        self.db
            .query("CREATE review_flag CONTENT $data")
            .bind(("data", data))
            .await?;

        Ok(())
    }
}
