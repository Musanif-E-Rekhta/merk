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

// ── WordTranslation ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct WordTranslation {
    pub id: Option<RecordId>,
    pub word: String,
    pub translation: String,
    pub source_lang: String,
    pub target_lang: String,
    pub submitted_by: RecordId,
    pub scope: String,
    pub book: Option<RecordId>,
    pub chapter: Option<RecordId>,
    pub context_note: Option<String>,
    pub upvotes: i64,
    pub downvotes: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct WordTranslationResponse {
    pub id: String,
    pub word: String,
    pub translation: String,
    pub source_lang: String,
    pub target_lang: String,
    pub submitted_by: String,
    pub scope: String,
    pub book_id: Option<String>,
    pub chapter_id: Option<String>,
    pub context_note: Option<String>,
    pub upvotes: i64,
    pub downvotes: i64,
    pub score: i64,
    #[schemars(with = "Option<String>")]
    pub created_at: Option<DateTime<Utc>>,
}

impl From<WordTranslation> for WordTranslationResponse {
    fn from(t: WordTranslation) -> Self {
        let score = t.upvotes - t.downvotes;
        WordTranslationResponse {
            id: t.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            word: t.word,
            translation: t.translation,
            source_lang: t.source_lang,
            target_lang: t.target_lang,
            submitted_by: key_to_string(t.submitted_by.key),
            scope: t.scope,
            book_id: t.book.map(|r| key_to_string(r.key)),
            chapter_id: t.chapter.map(|r| key_to_string(r.key)),
            context_note: t.context_note,
            upvotes: t.upvotes,
            downvotes: t.downvotes,
            score,
            created_at: t.created_at,
        }
    }
}

pub struct CreateTranslationDto {
    pub word: String,
    pub translation: String,
    pub source_lang: String,
    pub target_lang: String,
    pub scope: String,
    pub book_slug: Option<String>,
    pub chapter_slug: Option<String>,
    pub context_note: Option<String>,
}

// ── TranslationRepo ───────────────────────────────────────────────────────────

pub struct TranslationRepo {
    pub db: Db,
}

impl TranslationRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Get translations for a word, priority-ordered by scope then by score.
    /// Scope conditions are only included when the corresponding slugs are provided.
    pub async fn get_word_translations(
        &self,
        word: &str,
        target_lang: &str,
        book_slug: Option<&str>,
        chapter_slug: Option<&str>,
    ) -> Result<Vec<WordTranslationResponse>, Error> {
        let scope_clause = match (book_slug, chapter_slug) {
            (Some(_), Some(_)) => {
                "AND (\
                   (scope = 'chapter' AND chapter = (SELECT id FROM chapter WHERE slug = $cs)[0]) \
                   OR (scope = 'book' AND book = (SELECT id FROM book WHERE slug = $bs)[0]) \
                   OR scope = 'global' \
                 )"
            }
            (Some(_), None) => {
                "AND (\
                   (scope = 'book' AND book = (SELECT id FROM book WHERE slug = $bs)[0]) \
                   OR scope = 'global' \
                 )"
            }
            _ => "AND scope = 'global'",
        };

        let query = format!(
            "SELECT * FROM word_translation \
             WHERE word = $word AND target_lang = $lang \
             {scope_clause} \
             ORDER BY (scope = 'chapter') DESC, (scope = 'book') DESC, \
             (upvotes - downvotes) DESC"
        );

        let mut q = self
            .db
            .query(query)
            .bind(("word", word.to_string()))
            .bind(("lang", target_lang.to_string()));

        if let Some(bs) = book_slug {
            q = q.bind(("bs", bs.to_string()));
        }
        if let Some(cs) = chapter_slug {
            q = q.bind(("cs", cs.to_string()));
        }

        let mut resp = q.await?;
        let translations: Vec<WordTranslation> = resp.take(0)?;
        Ok(translations
            .into_iter()
            .map(WordTranslationResponse::from)
            .collect())
    }

    /// Submit a new translation, resolving optional book/chapter slugs to RecordIds.
    pub async fn create_translation(
        &self,
        user_id: &str,
        dto: CreateTranslationDto,
    ) -> Result<WordTranslationResponse, Error> {
        // Resolve book RecordId if provided
        let book_id: Option<RecordId> = if let Some(ref bs) = dto.book_slug {
            let mut br = self
                .db
                .query("SELECT id FROM book WHERE slug = $slug LIMIT 1")
                .bind(("slug", bs.clone()))
                .await?;
            let id: Option<RecordId> = br.take("id")?;
            Some(id.ok_or_else(|| Error::not_found("Book not found"))?)
        } else {
            None
        };

        // Resolve chapter RecordId if provided
        let chapter_id: Option<RecordId> = if let Some(ref cs) = dto.chapter_slug {
            let book_rid = book_id.clone().ok_or_else(|| {
                Error::bad_request(
                    "missing_book",
                    "book_slug required when chapter_slug is set",
                )
            })?;
            let mut cr = self
                .db
                .query("SELECT id FROM chapter WHERE slug = $cs AND book = $book LIMIT 1")
                .bind(("cs", cs.clone()))
                .bind(("book", book_rid))
                .await?;
            let id: Option<RecordId> = cr.take("id")?;
            Some(id.ok_or_else(|| Error::not_found("Chapter not found"))?)
        } else {
            None
        };

        let data = json!({
            "word": dto.word,
            "translation": dto.translation,
            "source_lang": dto.source_lang,
            "target_lang": dto.target_lang,
            "submitted_by": format!("user:{user_id}"),
            "scope": dto.scope,
            "book": book_id,
            "chapter": chapter_id,
            "context_note": dto.context_note,
            "upvotes": 0,
            "downvotes": 0,
        });

        let mut resp = self
            .db
            .query("CREATE word_translation CONTENT $data")
            .bind(("data", data))
            .await?;

        let created: Vec<WordTranslation> = resp.take(0)?;
        let translation = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("translation_repo", "insert failed"))?;

        Ok(WordTranslationResponse::from(translation))
    }

    /// Vote on a translation (+1 or -1), creating or updating the vote edge.
    pub async fn vote_translation(
        &self,
        user_id: &str,
        translation_id: &str,
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
                "UPDATE translation_vote \
                 SET value = $value, updated_at = time::now() \
                 WHERE in = type::record('user', $user_id) \
                 AND out = type::record('word_translation', $trans_id) \
                 RETURN AFTER",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("trans_id", translation_id.to_string()))
            .bind(("value", value))
            .await?;

        let updated: Vec<serde_json::Value> = update_resp.take(0)?;
        if updated.is_empty() {
            self.db
                .query(
                    "RELATE type::record('user', $user_id) \
                 ->translation_vote \
                 ->type::record('word_translation', $trans_id) \
                 CONTENT { value: $value }",
                )
                .bind(("user_id", user_id.to_string()))
                .bind(("trans_id", translation_id.to_string()))
                .bind(("value", value))
                .await?;
        }

        Ok(())
    }
}
