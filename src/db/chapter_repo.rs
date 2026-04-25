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

// ── ChapterNav ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ChapterNav {
    pub number: i64,
    pub title: Option<String>,
    pub slug: String,
}

// Raw DB helper for ChapterNav (no JsonSchema, used internally only)
#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct ChapterNavRaw {
    pub number: i64,
    pub title: Option<String>,
    pub slug: String,
}

impl From<ChapterNavRaw> for ChapterNav {
    fn from(r: ChapterNavRaw) -> Self {
        ChapterNav {
            number: r.number,
            title: r.title,
            slug: r.slug,
        }
    }
}

// ── ChapterListItem ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ChapterListItem {
    pub id: String,
    pub number: i64,
    pub title: Option<String>,
    pub slug: String,
    pub summary: Option<String>,
    pub reading_time_mins: Option<i64>,
    pub avg_rating: Option<f64>,
    pub is_published: bool,
}

// Raw DB form with RecordId for the id field
#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct ChapterListItemRaw {
    pub id: RecordId,
    pub number: i64,
    pub title: Option<String>,
    pub slug: String,
    pub summary: Option<String>,
    pub reading_time_mins: Option<i64>,
    pub avg_rating: Option<f64>,
    pub is_published: bool,
}

impl From<ChapterListItemRaw> for ChapterListItem {
    fn from(r: ChapterListItemRaw) -> Self {
        ChapterListItem {
            id: key_to_string(r.id.key),
            number: r.number,
            title: r.title,
            slug: r.slug,
            summary: r.summary,
            reading_time_mins: r.reading_time_mins,
            avg_rating: r.avg_rating,
            is_published: r.is_published,
        }
    }
}

// ── Chapter ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Chapter {
    pub id: Option<RecordId>,
    pub book: RecordId,
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
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// ── ChapterResponse ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ChapterResponse {
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
    #[schemars(with = "Option<String>")]
    pub published_at: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
    pub prev_chapter: Option<ChapterNav>,
    pub next_chapter: Option<ChapterNav>,
}

impl Chapter {
    fn into_response(
        self,
        prev_chapter: Option<ChapterNav>,
        next_chapter: Option<ChapterNav>,
    ) -> ChapterResponse {
        ChapterResponse {
            id: self.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            book_id: key_to_string(self.book.key),
            number: self.number,
            title: self.title,
            slug: self.slug,
            content: self.content,
            content_format: self.content_format,
            summary: self.summary,
            meta_description: self.meta_description,
            word_count: self.word_count,
            reading_time_mins: self.reading_time_mins,
            avg_rating: self.avg_rating,
            review_count: self.review_count,
            is_published: self.is_published,
            published_at: self.published_at,
            updated_at: self.updated_at,
            prev_chapter,
            next_chapter,
        }
    }
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

pub struct CreateChapterDto {
    pub number: i64,
    pub title: Option<String>,
    pub slug: String,
    pub content: String,
    pub content_format: Option<String>,
    pub summary: Option<String>,
    pub meta_description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateChapterDto {
    pub title: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub meta_description: Option<String>,
    pub is_published: Option<bool>,
}

// ── IdOnly helper ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct IdOnly {
    id: RecordId,
}

// ── SlugOnly helper ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct SlugOnly {
    slug: String,
}

// ── ChapterRepo ───────────────────────────────────────────────────────────────

pub struct ChapterRepo {
    pub db: Db,
}

impl ChapterRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn list_chapters(&self, book_slug: &str) -> Result<Vec<ChapterListItem>, Error> {
        let mut response = self.db
            .query(
                "SELECT id, number, title, slug, summary, reading_time_mins, avg_rating, is_published \
                 FROM chapter \
                 WHERE book = (SELECT id FROM book WHERE slug = $slug)[0] \
                 AND is_published = true \
                 ORDER BY number ASC",
            )
            .bind(("slug", book_slug.to_string()))
            .await?;

        let raws: Vec<ChapterListItemRaw> = response.take(0)?;
        Ok(raws.into_iter().map(ChapterListItem::from).collect())
    }

    pub async fn get_chapter_by_slug(
        &self,
        book_slug: &str,
        chapter_slug: &str,
    ) -> Result<Option<ChapterResponse>, Error> {
        let mut response = self
            .db
            .query(
                "SELECT * FROM chapter \
                 WHERE book = (SELECT id FROM book WHERE slug = $book_slug)[0] \
                 AND slug = $chapter_slug \
                 AND is_published = true \
                 LIMIT 1",
            )
            .bind(("book_slug", book_slug.to_string()))
            .bind(("chapter_slug", chapter_slug.to_string()))
            .await?;

        let chapter: Option<Chapter> = response.take(0)?;

        match chapter {
            None => Ok(None),
            Some(chapter) => {
                let book_id = chapter.book.clone();
                let number = chapter.number;

                let mut prev_response = self
                    .db
                    .query(
                        "SELECT number, title, slug FROM chapter \
                         WHERE book = $book_id AND number = $n AND is_published = true \
                         LIMIT 1",
                    )
                    .bind(("book_id", book_id.clone()))
                    .bind(("n", number - 1))
                    .await?;

                let mut next_response = self
                    .db
                    .query(
                        "SELECT number, title, slug FROM chapter \
                         WHERE book = $book_id AND number = $n AND is_published = true \
                         LIMIT 1",
                    )
                    .bind(("book_id", book_id))
                    .bind(("n", number + 1))
                    .await?;

                let prev_raw: Option<ChapterNavRaw> = prev_response.take(0)?;
                let next_raw: Option<ChapterNavRaw> = next_response.take(0)?;

                Ok(Some(chapter.into_response(
                    prev_raw.map(ChapterNav::from),
                    next_raw.map(ChapterNav::from),
                )))
            }
        }
    }

    pub async fn get_chapter_slug_by_number(
        &self,
        book_slug: &str,
        number: i64,
    ) -> Result<Option<String>, Error> {
        let mut response = self
            .db
            .query(
                "SELECT slug FROM chapter \
                 WHERE book = (SELECT id FROM book WHERE slug = $book_slug)[0] \
                 AND number = $number \
                 AND is_published = true \
                 LIMIT 1",
            )
            .bind(("book_slug", book_slug.to_string()))
            .bind(("number", number))
            .await?;

        let result: Option<SlugOnly> = response.take(0)?;
        Ok(result.map(|r| r.slug))
    }

    pub async fn create_chapter(
        &self,
        book_slug: &str,
        dto: CreateChapterDto,
    ) -> Result<ChapterResponse, Error> {
        // Resolve the book's RecordId first
        let mut book_response = self
            .db
            .query("SELECT id FROM book WHERE slug = $slug LIMIT 1")
            .bind(("slug", book_slug.to_string()))
            .await?;

        let book_record: Option<IdOnly> = book_response.take(0)?;
        let book_id = book_record
            .ok_or_else(|| Error::not_found("Book not found"))?
            .id;

        let word_count = dto.content.split_whitespace().count() as i64;
        let reading_time_mins = (word_count as f64 / 238.0).ceil() as i64;
        let content_format = dto.content_format.unwrap_or_else(|| "markdown".to_string());

        let data = json!({
            "book": book_id,
            "number": dto.number,
            "title": dto.title,
            "slug": dto.slug,
            "content": dto.content,
            "content_format": content_format,
            "summary": dto.summary,
            "meta_description": dto.meta_description,
            "word_count": word_count,
            "reading_time_mins": reading_time_mins,
            "avg_rating": null,
            "review_count": 0,
            "is_published": false,
            "published_at": null,
        });

        let mut response = self
            .db
            .query("CREATE chapter CONTENT $data")
            .bind(("data", data))
            .await?;

        let created: Vec<Chapter> = response.take(0)?;
        let chapter = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("chapter_repo", "insert failed"))?;

        Ok(chapter.into_response(None, None))
    }

    pub async fn update_chapter(
        &self,
        book_slug: &str,
        chapter_slug: &str,
        dto: UpdateChapterDto,
    ) -> Result<Option<ChapterResponse>, Error> {
        let mut updates = serde_json::Map::new();

        if let Some(v) = dto.title {
            updates.insert("title".to_string(), json!(v));
        }
        if let Some(ref v) = dto.content {
            let word_count = v.split_whitespace().count() as i64;
            let reading_time_mins = (word_count as f64 / 238.0).ceil() as i64;
            updates.insert("content".to_string(), json!(v));
            updates.insert("word_count".to_string(), json!(word_count));
            updates.insert("reading_time_mins".to_string(), json!(reading_time_mins));
        }
        if let Some(v) = dto.summary {
            updates.insert("summary".to_string(), json!(v));
        }
        if let Some(v) = dto.meta_description {
            updates.insert("meta_description".to_string(), json!(v));
        }
        if let Some(v) = dto.is_published {
            updates.insert("is_published".to_string(), json!(v));
            if v {
                updates.insert("published_at".to_string(), json!(Utc::now()));
            }
        }
        updates.insert("updated_at".to_string(), json!(Utc::now()));

        let mut response = self
            .db
            .query(
                "UPDATE chapter MERGE $data \
                 WHERE book = (SELECT id FROM book WHERE slug = $book_slug)[0] \
                 AND slug = $chapter_slug",
            )
            .bind(("book_slug", book_slug.to_string()))
            .bind(("chapter_slug", chapter_slug.to_string()))
            .bind(("data", serde_json::Value::Object(updates)))
            .await?;

        let chapter: Option<Chapter> = response.take(0)?;

        match chapter {
            None => Ok(None),
            Some(chapter) => {
                let book_id = chapter.book.clone();
                let number = chapter.number;

                let mut prev_response = self
                    .db
                    .query(
                        "SELECT number, title, slug FROM chapter \
                         WHERE book = $book_id AND number = $n AND is_published = true \
                         LIMIT 1",
                    )
                    .bind(("book_id", book_id.clone()))
                    .bind(("n", number - 1))
                    .await?;

                let mut next_response = self
                    .db
                    .query(
                        "SELECT number, title, slug FROM chapter \
                         WHERE book = $book_id AND number = $n AND is_published = true \
                         LIMIT 1",
                    )
                    .bind(("book_id", book_id))
                    .bind(("n", number + 1))
                    .await?;

                let prev_raw: Option<ChapterNavRaw> = prev_response.take(0)?;
                let next_raw: Option<ChapterNavRaw> = next_response.take(0)?;

                Ok(Some(chapter.into_response(
                    prev_raw.map(ChapterNav::from),
                    next_raw.map(ChapterNav::from),
                )))
            }
        }
    }
}
