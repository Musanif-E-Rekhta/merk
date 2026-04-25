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

// ── Collection ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Collection {
    pub id: Option<RecordId>,
    pub user: RecordId,
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub is_public: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct CollectionResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub is_public: bool,
    #[schemars(with = "Option<String>")]
    pub created_at: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Collection> for CollectionResponse {
    fn from(c: Collection) -> Self {
        CollectionResponse {
            id: c.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            user_id: key_to_string(c.user.key),
            name: c.name,
            description: c.description,
            cover_url: c.cover_url,
            is_public: c.is_public,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

// ── CollectionBook edge ───────────────────────────────────────────────────────

/// Raw graph-edge row returned by SurrealDB for collection_book relations.
#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct CollectionBook {
    /// `out` holds the book RecordId populated automatically by RELATE.
    pub out: Option<RecordId>,
    pub position: Option<i64>,
    pub note: Option<String>,
    pub added_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct CollectionBookResponse {
    pub book_id: String,
    pub position: Option<i64>,
    pub note: Option<String>,
    #[schemars(with = "Option<String>")]
    pub added_at: Option<DateTime<Utc>>,
}

impl From<CollectionBook> for CollectionBookResponse {
    fn from(cb: CollectionBook) -> Self {
        CollectionBookResponse {
            book_id: cb.out.map(|r| key_to_string(r.key)).unwrap_or_default(),
            position: cb.position,
            note: cb.note,
            added_at: cb.added_at,
        }
    }
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

pub struct CreateCollectionDto {
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

pub struct UpdateCollectionDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

pub struct AddBookDto {
    pub position: Option<i64>,
    pub note: Option<String>,
}

// ── CollectionRepo ────────────────────────────────────────────────────────────

pub struct CollectionRepo {
    pub db: Db,
}

impl CollectionRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// List all collections owned by a user.
    pub async fn list_user_collections(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CollectionResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM collection \
                 WHERE user = type::record('user', $user_id) \
                 ORDER BY created_at DESC LIMIT $l START $o",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;

        let collections: Vec<Collection> = resp.take(0)?;
        Ok(collections
            .into_iter()
            .map(CollectionResponse::from)
            .collect())
    }

    /// Get a single collection by its ID.
    pub async fn get_collection(
        &self,
        collection_id: &str,
    ) -> Result<Option<CollectionResponse>, Error> {
        let mut resp = self
            .db
            .query("SELECT * FROM type::record('collection', $collection_id)")
            .bind(("collection_id", collection_id.to_string()))
            .await?;

        let collection: Option<Collection> = resp.take(0)?;
        Ok(collection.map(CollectionResponse::from))
    }

    /// Create a new collection owned by the user.
    pub async fn create_collection(
        &self,
        user_id: &str,
        dto: CreateCollectionDto,
    ) -> Result<CollectionResponse, Error> {
        let data = json!({
            "user": format!("user:{user_id}"),
            "name": dto.name,
            "description": dto.description,
            "cover_url": null,
            "is_public": dto.is_public.unwrap_or(false),
        });

        let mut resp = self
            .db
            .query("CREATE collection CONTENT $data")
            .bind(("data", data))
            .await?;

        let created: Vec<Collection> = resp.take(0)?;
        let collection = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("collection_repo", "insert failed"))?;

        Ok(CollectionResponse::from(collection))
    }

    /// Partially update a collection (owner only).
    pub async fn update_collection(
        &self,
        collection_id: &str,
        user_id: &str,
        dto: UpdateCollectionDto,
    ) -> Result<Option<CollectionResponse>, Error> {
        let mut patch = serde_json::Map::new();
        if let Some(v) = dto.name {
            patch.insert("name".to_string(), json!(v));
        }
        if let Some(v) = dto.description {
            patch.insert("description".to_string(), json!(v));
        }
        if let Some(v) = dto.is_public {
            patch.insert("is_public".to_string(), json!(v));
        }
        patch.insert("updated_at".to_string(), json!(Utc::now()));

        let mut resp = self
            .db
            .query(
                "UPDATE type::record('collection', $collection_id) \
                 MERGE $data \
                 WHERE user = type::record('user', $user_id) \
                 RETURN AFTER",
            )
            .bind(("collection_id", collection_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .bind(("data", serde_json::Value::Object(patch)))
            .await?;

        let collection: Option<Collection> = resp.take(0)?;
        Ok(collection.map(CollectionResponse::from))
    }

    /// Delete a collection (owner only).
    pub async fn delete_collection(
        &self,
        collection_id: &str,
        user_id: &str,
    ) -> Result<bool, Error> {
        self.db
            .query(
                "DELETE type::record('collection', $collection_id) \
             WHERE user = type::record('user', $user_id)",
            )
            .bind(("collection_id", collection_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .await?;

        Ok(true)
    }

    /// Add a book to a collection (owner only), creating a RELATE edge.
    pub async fn add_book(
        &self,
        collection_id: &str,
        book_slug: &str,
        user_id: &str,
        dto: AddBookDto,
    ) -> Result<CollectionBookResponse, Error> {
        // Verify the collection belongs to this user
        let mut own_resp = self
            .db
            .query(
                "SELECT id FROM type::record('collection', $collection_id) \
                 WHERE user = type::record('user', $user_id) \
                 LIMIT 1",
            )
            .bind(("collection_id", collection_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .await?;
        let owned: Option<RecordId> = own_resp.take("id")?;
        if owned.is_none() {
            return Err(Error::not_found("Collection not found or access denied"));
        }

        let data = json!({
            "position": dto.position,
            "note": dto.note,
            "added_at": Utc::now(),
        });

        let mut resp = self
            .db
            .query(
                "RELATE type::record('collection', $collection_id) \
                 ->collection_book \
                 ->(SELECT id FROM book WHERE slug = $slug)[0] \
                 CONTENT $data \
                 RETURN AFTER",
            )
            .bind(("collection_id", collection_id.to_string()))
            .bind(("slug", book_slug.to_string()))
            .bind(("data", data))
            .await?;

        let created: Vec<CollectionBook> = resp.take(0)?;
        let entry = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("collection_repo", "add_book insert failed"))?;

        Ok(CollectionBookResponse::from(entry))
    }

    /// Remove a book from a collection (owner only).
    pub async fn remove_book(
        &self,
        collection_id: &str,
        book_slug: &str,
        user_id: &str,
    ) -> Result<bool, Error> {
        // Verify ownership
        let mut own_resp = self
            .db
            .query(
                "SELECT id FROM type::record('collection', $collection_id) \
                 WHERE user = type::record('user', $user_id) \
                 LIMIT 1",
            )
            .bind(("collection_id", collection_id.to_string()))
            .bind(("user_id", user_id.to_string()))
            .await?;
        let owned: Option<RecordId> = own_resp.take("id")?;
        if owned.is_none() {
            return Err(Error::not_found("Collection not found or access denied"));
        }

        self.db
            .query(
                "DELETE collection_book \
             WHERE in = type::record('collection', $collection_id) \
             AND out = (SELECT id FROM book WHERE slug = $slug)[0]",
            )
            .bind(("collection_id", collection_id.to_string()))
            .bind(("slug", book_slug.to_string()))
            .await?;

        Ok(true)
    }

    /// List books in a collection ordered by position.
    pub async fn list_collection_books(
        &self,
        collection_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CollectionBookResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT *, out as out FROM collection_book \
                 WHERE in = type::record('collection', $collection_id) \
                 ORDER BY position ASC LIMIT $l START $o",
            )
            .bind(("collection_id", collection_id.to_string()))
            .bind(("l", limit))
            .bind(("o", offset))
            .await?;

        let entries: Vec<CollectionBook> = resp.take(0)?;
        Ok(entries
            .into_iter()
            .map(CollectionBookResponse::from)
            .collect())
    }
}
