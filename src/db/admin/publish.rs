//! Publish flow + admin library + admin author resolution.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::book_repo::{BookResponse, AuthorResponse};
use crate::db::{Db, record_id_key_to_string};
use crate::error::Error;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct PublishCheck {
    pub ok: bool,
    pub gate: Option<String>, // "warn" | "err" | None
    pub label: String,
    pub detail: Option<String>,
}

pub struct PublishDto {
    pub visibility: String,            // "public" | "unlisted" | "draft"
    pub schedule_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct PublishResult {
    pub book: BookResponse,
}

#[derive(Default)]
pub struct AdminBookFilters {
    pub visibility: Option<String>,
    pub is_published: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct AdminBookListItem {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub visibility: Option<String>,
    pub is_published: bool,
    pub chapter_count: i64,
    pub avg_rating: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct AuthorMatch {
    pub author: AuthorResponse,
    pub confidence: f64,
}

pub struct PublishRepo {
    pub db: Db,
}

impl PublishRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Hard-coded server-authoritative checklist. Each gate either passes,
    /// warns, or hard-blocks publish.
    pub async fn publish_checks(&self, job_id: &str) -> Result<Vec<PublishCheck>, Error> {
        // Aggregate counts the checks need.
        #[derive(Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct Counts {
            total: i64,
            approved: i64,
            flagged: i64,
            covers: i64,
            selected_cover: i64,
        }
        let mut resp = self
            .db
            .query(
                "RETURN { \
                   total:    (SELECT count() FROM chapter_draft WHERE job = type::record('ingestion_job', $jid) GROUP ALL)[0].count ?? 0, \
                   approved: (SELECT count() FROM chapter_draft WHERE job = type::record('ingestion_job', $jid) AND status = 'approved' GROUP ALL)[0].count ?? 0, \
                   flagged:  (SELECT count() FROM chapter_draft WHERE job = type::record('ingestion_job', $jid) AND status = 'flag' GROUP ALL)[0].count ?? 0, \
                   covers:   (SELECT count() FROM cover_variant WHERE job = type::record('ingestion_job', $jid) GROUP ALL)[0].count ?? 0, \
                   selected_cover: (SELECT count() FROM cover_variant WHERE job = type::record('ingestion_job', $jid) AND is_selected = true GROUP ALL)[0].count ?? 0 \
                 }",
            )
            .bind(("jid", job_id.to_string()))
            .await?;
        let c: Option<Counts> = resp.take(0)?;
        let c = c.unwrap_or(Counts {
            total: 0,
            approved: 0,
            flagged: 0,
            covers: 0,
            selected_cover: 0,
        });

        let mut out = Vec::new();
        // Hard requirements.
        out.push(PublishCheck {
            ok: c.total > 0,
            gate: if c.total > 0 { None } else { Some("err".into()) },
            label: "Chapters extracted".into(),
            detail: Some(format!("{} draft chapters", c.total)),
        });
        out.push(PublishCheck {
            ok: c.flagged == 0,
            gate: if c.flagged == 0 { None } else { Some("warn".into()) },
            label: "No flagged chapters".into(),
            detail: if c.flagged == 0 {
                None
            } else {
                Some(format!("{} chapters need review", c.flagged))
            },
        });
        out.push(PublishCheck {
            ok: c.approved == c.total && c.total > 0,
            gate: if c.approved == c.total && c.total > 0 {
                None
            } else {
                Some("err".into())
            },
            label: "All chapters approved".into(),
            detail: Some(format!("{}/{} approved", c.approved, c.total)),
        });
        out.push(PublishCheck {
            ok: c.selected_cover > 0,
            gate: if c.selected_cover > 0 {
                None
            } else {
                Some("warn".into())
            },
            label: "Cover selected".into(),
            detail: Some(format!("{} variants generated", c.covers)),
        });

        Ok(out)
    }

    /// Materialize a `book` + `chapter` rows from the approved drafts. The
    /// selected cover variant's `bucket/object` becomes the book's
    /// `cover_url`. Idempotent: re-publishing the same job updates rather
    /// than duplicates.
    pub async fn publish(&self, job_id: &str, dto: PublishDto) -> Result<PublishResult, Error> {
        // Resolve job + selected cover.
        #[derive(Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct JobInfo {
            hint_title: Option<String>,
            hint_title_ur: Option<String>,
            book: Option<RecordId>,
        }

        let mut resp = self
            .db
            .query("SELECT hint_title, hint_title_ur, book FROM type::record('ingestion_job', $jid)")
            .bind(("jid", job_id.to_string()))
            .await?;
        let info: Option<JobInfo> = resp.take(0)?;
        let info = info.ok_or_else(|| Error::not_found("Ingestion job not found"))?;

        let title = info
            .hint_title
            .or(info.hint_title_ur)
            .unwrap_or_else(|| "Untitled".to_string());
        let slug = slugify(&title);

        // Cover URL: from the selected variant if any.
        let mut crresp = self
            .db
            .query(
                "SELECT bucket, object FROM cover_variant \
                 WHERE job = type::record('ingestion_job', $jid) AND is_selected = true LIMIT 1",
            )
            .bind(("jid", job_id.to_string()))
            .await?;
        #[derive(Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct CoverPath {
            bucket: String,
            object: String,
        }
        let cover_path: Option<CoverPath> = crresp.take(0)?;
        let cover_url = cover_path
            .map(|c| format!("/api/v1/cover/{}/{}", c.bucket, c.object));

        // Create or update the book.
        let book_id = if let Some(existing) = info.book.as_ref() {
            record_id_key_to_string(&existing.key)
        } else {
            let mut bresp = self
                .db
                .query(
                    "CREATE book SET \
                       title = $title, slug = $slug, \
                       language = 'ur', \
                       cover_url = $cover_url, \
                       visibility = $vis, \
                       scheduled_publish_at = $sched, \
                       ingestion_job = type::record('ingestion_job', $jid), \
                       is_published = $published \
                     RETURN AFTER",
                )
                .bind(("title", title.clone()))
                .bind(("slug", slug.clone()))
                .bind(("cover_url", cover_url.clone()))
                .bind(("vis", dto.visibility.clone()))
                .bind(("sched", dto.schedule_at))
                .bind(("jid", job_id.to_string()))
                .bind(("published", dto.visibility == "public"))
                .await?;
            #[derive(Deserialize, SurrealValue)]
            #[surreal(crate = "surrealdb::types")]
            struct BookRow {
                id: Option<RecordId>,
            }
            let row: Option<BookRow> = bresp.take(0)?;
            let id = row
                .and_then(|r| r.id)
                .ok_or_else(|| Error::internal("admin", "book insert failed"))?;
            let id_str = record_id_key_to_string(&id.key);

            // Link the book back onto the ingestion_job.
            self.db
                .query(
                    "UPDATE type::record('ingestion_job', $jid) SET \
                       book = type::record('book', $bid), \
                       status = 'completed', completed_at = time::now()",
                )
                .bind(("jid", job_id.to_string()))
                .bind(("bid", id_str.clone()))
                .await?;

            id_str
        };

        // Insert chapters from approved drafts. Slug is derived per draft.
        self.db
            .query(
                "FOR $d IN (SELECT id, n, title_ur, title_en, page_range, \
                                  human_content ?? ai_content AS body, \
                                  ai_content_format, confidence \
                            FROM chapter_draft \
                            WHERE job = type::record('ingestion_job', $jid) \
                              AND status = 'approved' \
                            ORDER BY n) { \
                   CREATE chapter SET \
                     book = type::record('book', $bid), \
                     number = $d.n, \
                     title = $d.title_en ?? $d.title_ur, \
                     slug = string::concat('ch-', string::concat('', $d.n)), \
                     content = $d.body, \
                     content_format = $d.ai_content_format, \
                     confidence = $d.confidence, \
                     is_published = true, \
                     published_at = time::now(); \
                 }",
            )
            .bind(("jid", job_id.to_string()))
            .bind(("bid", book_id.clone()))
            .await?;

        // Recount + return.
        let mut resp = self
            .db
            .query(
                "SELECT count() AS c FROM chapter \
                 WHERE book = type::record('book', $bid) GROUP ALL",
            )
            .bind(("bid", book_id.clone()))
            .await?;
        #[derive(Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct C {
            c: i64,
        }
        let chapter_count: Option<C> = resp.take(0)?;
        if let Some(c) = chapter_count {
            self.db
                .query("UPDATE type::record('book', $bid) SET chapter_count = $c")
                .bind(("bid", book_id.clone()))
                .bind(("c", c.c))
                .await?;
        }

        let mut bresp = self
            .db
            .query("SELECT * FROM type::record('book', $bid)")
            .bind(("bid", book_id))
            .await?;
        let book: Option<crate::db::book_repo::Book> = bresp.take(0)?;
        let book = book.ok_or_else(|| Error::internal("admin", "book gone after publish"))?;
        Ok(PublishResult { book: book.into() })
    }

    // ── Admin library ───────────────────────────────────────────────────────

    pub async fn list_books(
        &self,
        f: &AdminBookFilters,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AdminBookListItem>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT id, title, slug, visibility, is_published, chapter_count, avg_rating \
                 FROM book \
                 WHERE ($vis = NONE OR visibility = $vis) \
                   AND ($published = NONE OR is_published = $published) \
                 ORDER BY created_at DESC LIMIT $l START $o",
            )
            .bind(("vis", f.visibility.clone()))
            .bind(("published", f.is_published))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;
        #[derive(Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct Row {
            id: Option<RecordId>,
            title: String,
            slug: String,
            visibility: Option<String>,
            is_published: bool,
            chapter_count: i64,
            avg_rating: Option<f64>,
        }
        let rows: Vec<Row> = resp.take(0)?;
        Ok(rows
            .into_iter()
            .map(|r| AdminBookListItem {
                id: r
                    .id
                    .map(|rid| record_id_key_to_string(&rid.key))
                    .unwrap_or_default(),
                title: r.title,
                slug: r.slug,
                visibility: r.visibility,
                is_published: r.is_published,
                chapter_count: r.chapter_count,
                avg_rating: r.avg_rating,
            })
            .collect())
    }

    pub async fn update_book_visibility(
        &self,
        slug: &str,
        visibility: &str,
    ) -> Result<bool, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE book SET visibility = $vis, is_published = $pub \
                 WHERE slug = $slug RETURN AFTER",
            )
            .bind(("slug", slug.to_string()))
            .bind(("vis", visibility.to_string()))
            .bind(("pub", visibility == "public"))
            .await?;
        let rows: Vec<crate::db::book_repo::Book> = resp.take(0)?;
        Ok(!rows.is_empty())
    }

    pub async fn unpublish_book(&self, slug: &str) -> Result<bool, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE book SET visibility = 'draft', is_published = false \
                 WHERE slug = $slug RETURN AFTER",
            )
            .bind(("slug", slug.to_string()))
            .await?;
        let rows: Vec<crate::db::book_repo::Book> = resp.take(0)?;
        Ok(!rows.is_empty())
    }

    // ── Admin author resolution ─────────────────────────────────────────────

    pub async fn match_authors(&self, name: &str) -> Result<Vec<AuthorMatch>, Error> {
        // Crude fuzzy match: substring + prefix, both directions.
        let needle = name.to_lowercase();
        let mut resp = self
            .db
            .query(
                "SELECT * FROM author \
                 WHERE string::lowercase(name) ~ $needle \
                    OR string::starts_with(string::lowercase(name), $needle) \
                 LIMIT 10",
            )
            .bind(("needle", needle.clone()))
            .await?;
        let rows: Vec<crate::db::book_repo::Author> = resp.take(0)?;
        Ok(rows
            .into_iter()
            .map(|a| {
                // Confidence: 1.0 for exact, 0.8 for prefix, 0.5 otherwise.
                let lname = a.name.to_lowercase();
                let confidence = if lname == needle {
                    1.0
                } else if lname.starts_with(&needle) {
                    0.8
                } else {
                    0.5
                };
                AuthorMatch {
                    author: a.into(),
                    confidence,
                }
            })
            .collect())
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
