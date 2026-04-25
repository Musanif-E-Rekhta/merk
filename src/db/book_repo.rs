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

// ── Publisher ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Publisher {
    pub id: Option<RecordId>,
    pub name: String,
    pub website: Option<String>,
    pub country: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct PublisherResponse {
    pub id: String,
    pub name: String,
    pub website: Option<String>,
    pub country: Option<String>,
}

impl From<Publisher> for PublisherResponse {
    fn from(p: Publisher) -> Self {
        PublisherResponse {
            id: p.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            name: p.name,
            website: p.website,
            country: p.country,
        }
    }
}

pub struct CreatePublisherDto {
    pub name: String,
    pub website: Option<String>,
    pub country: Option<String>,
}

// ── Category ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Category {
    pub id: Option<RecordId>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub children: Vec<CategoryResponse>,
}

impl From<Category> for CategoryResponse {
    fn from(c: Category) -> Self {
        CategoryResponse {
            id: c.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            name: c.name,
            slug: c.slug,
            description: c.description,
            children: vec![],
        }
    }
}

pub struct CreateCategoryDto {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_slug: Option<String>,
}

// ── Tag ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Tag {
    pub id: Option<RecordId>,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct TagResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
}

impl From<Tag> for TagResponse {
    fn from(t: Tag) -> Self {
        TagResponse {
            id: t.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            name: t.name,
            slug: t.slug,
        }
    }
}

pub struct CreateTagDto {
    pub name: String,
    pub slug: String,
}

// ── Author ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Author {
    pub id: Option<RecordId>,
    pub name: String,
    pub slug: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct AuthorResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
}

impl From<Author> for AuthorResponse {
    fn from(a: Author) -> Self {
        AuthorResponse {
            id: a.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            name: a.name,
            slug: a.slug,
            bio: a.bio,
            avatar_url: a.avatar_url,
            website: a.website,
        }
    }
}

pub struct CreateAuthorDto {
    pub name: String,
    pub slug: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
}

pub struct UpdateAuthorDto {
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
}

// ── Book ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Book {
    pub id: Option<RecordId>,
    pub title: String,
    pub slug: String,
    pub isbn: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub page_count: Option<i64>,
    pub language: String,
    pub avg_rating: Option<f64>,
    pub review_count: i64,
    pub chapter_count: i64,
    pub is_published: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct BookResponse {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub isbn: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub page_count: Option<i64>,
    pub language: String,
    pub avg_rating: Option<f64>,
    pub review_count: i64,
    pub chapter_count: i64,
    pub is_published: bool,
    #[schemars(with = "Option<String>")]
    pub created_at: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Book> for BookResponse {
    fn from(b: Book) -> Self {
        BookResponse {
            id: b.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            title: b.title,
            slug: b.slug,
            isbn: b.isbn,
            summary: b.summary,
            description: b.description,
            cover_url: b.cover_url,
            page_count: b.page_count,
            language: b.language,
            avg_rating: b.avg_rating,
            review_count: b.review_count,
            chapter_count: b.chapter_count,
            is_published: b.is_published,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }
    }
}

pub struct CreateBookDto {
    pub title: String,
    pub slug: String,
    pub isbn: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub page_count: Option<i64>,
    pub language: Option<String>,
    pub publisher_slug: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateBookDto {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub page_count: Option<i64>,
    pub is_published: Option<bool>,
}

pub struct BookFilters {
    pub q: Option<String>,
    pub lang: Option<String>,
    pub is_published: Option<bool>,
}

// ── BookRepo ─────────────────────────────────────────────────────────────────

pub struct BookRepo {
    pub db: Db,
}

impl BookRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    // ── Books ────────────────────────────────────────────────────────────────

    pub async fn list_books(
        &self,
        filters: &BookFilters,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BookResponse>, Error> {
        let mut response = self
            .db
            .query(
                "SELECT * FROM book \
                 WHERE ($q = NONE OR title @@ $q) \
                 AND ($lang = NONE OR language = $lang) \
                 AND ($is_published = NONE OR is_published = $is_published) \
                 ORDER BY created_at DESC LIMIT $limit START $offset",
            )
            .bind(("q", filters.q.clone()))
            .bind(("lang", filters.lang.clone()))
            .bind(("is_published", filters.is_published))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;

        let books: Vec<Book> = response.take(0)?;
        Ok(books.into_iter().map(BookResponse::from).collect())
    }

    pub async fn get_book_by_slug(&self, slug: &str) -> Result<Option<BookResponse>, Error> {
        let mut response = self
            .db
            .query("SELECT * FROM book WHERE slug = $slug LIMIT 1")
            .bind(("slug", slug.to_string()))
            .await?;

        let book: Option<Book> = response.take(0)?;
        Ok(book.map(BookResponse::from))
    }

    pub async fn create_book(&self, dto: CreateBookDto) -> Result<BookResponse, Error> {
        let language = dto.language.unwrap_or_else(|| "en".to_string());

        let data = json!({
            "title": dto.title,
            "slug": dto.slug,
            "isbn": dto.isbn,
            "summary": dto.summary,
            "description": dto.description,
            "cover_url": dto.cover_url,
            "page_count": dto.page_count,
            "language": language,
            "avg_rating": null,
            "review_count": 0,
            "chapter_count": 0,
            "is_published": false,
        });

        let mut response = self
            .db
            .query("CREATE book CONTENT $data")
            .bind(("data", data))
            .await?;

        let created: Vec<Book> = response.take(0)?;
        let book = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("book_repo", "insert failed"))?;

        // If a publisher_slug was provided, relate the book to the publisher
        if let Some(publisher_slug) = dto.publisher_slug {
            let book_id = book
                .id
                .clone()
                .ok_or_else(|| Error::internal("book_repo", "missing book id"))?;
            self.db
                .query(
                    "RELATE (SELECT id FROM publisher WHERE slug = $pub_slug)[0]\
                 ->published->$book_id",
                )
                .bind(("pub_slug", publisher_slug))
                .bind(("book_id", book_id))
                .await?;
        }

        Ok(BookResponse::from(book))
    }

    pub async fn update_book(
        &self,
        slug: &str,
        dto: UpdateBookDto,
    ) -> Result<Option<BookResponse>, Error> {
        let mut updates = serde_json::Map::new();
        if let Some(v) = dto.title {
            updates.insert("title".to_string(), json!(v));
        }
        if let Some(v) = dto.summary {
            updates.insert("summary".to_string(), json!(v));
        }
        if let Some(v) = dto.description {
            updates.insert("description".to_string(), json!(v));
        }
        if let Some(v) = dto.cover_url {
            updates.insert("cover_url".to_string(), json!(v));
        }
        if let Some(v) = dto.page_count {
            updates.insert("page_count".to_string(), json!(v));
        }
        if let Some(v) = dto.is_published {
            updates.insert("is_published".to_string(), json!(v));
        }
        updates.insert("updated_at".to_string(), json!(Utc::now()));

        let mut response = self
            .db
            .query("UPDATE book MERGE $data WHERE slug = $slug")
            .bind(("slug", slug.to_string()))
            .bind(("data", serde_json::Value::Object(updates)))
            .await?;

        let book: Option<Book> = response.take(0)?;
        Ok(book.map(BookResponse::from))
    }

    // ── Authors ──────────────────────────────────────────────────────────────

    pub async fn list_authors(
        &self,
        q: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuthorResponse>, Error> {
        let mut response = self
            .db
            .query(
                "SELECT * FROM author \
                 WHERE ($q = NONE OR name @@ $q) \
                 ORDER BY created_at DESC LIMIT $limit START $offset",
            )
            .bind(("q", q.map(str::to_string)))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;

        let authors: Vec<Author> = response.take(0)?;
        Ok(authors.into_iter().map(AuthorResponse::from).collect())
    }

    pub async fn get_author_by_slug(&self, slug: &str) -> Result<Option<AuthorResponse>, Error> {
        let mut response = self
            .db
            .query("SELECT * FROM author WHERE slug = $slug LIMIT 1")
            .bind(("slug", slug.to_string()))
            .await?;

        let author: Option<Author> = response.take(0)?;
        Ok(author.map(AuthorResponse::from))
    }

    pub async fn create_author(&self, dto: CreateAuthorDto) -> Result<AuthorResponse, Error> {
        let data = json!({
            "name": dto.name,
            "slug": dto.slug,
            "bio": dto.bio,
            "avatar_url": dto.avatar_url,
            "website": dto.website,
        });

        let mut response = self
            .db
            .query("CREATE author CONTENT $data")
            .bind(("data", data))
            .await?;

        let created: Vec<Author> = response.take(0)?;
        let author = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("book_repo", "insert failed"))?;

        Ok(AuthorResponse::from(author))
    }

    pub async fn get_books_by_author(&self, author_slug: &str) -> Result<Vec<BookResponse>, Error> {
        let mut response = self
            .db
            .query("SELECT ->wrote->book.* FROM author WHERE slug = $slug")
            .bind(("slug", author_slug.to_string()))
            .await?;

        let books: Vec<Book> = response.take(0)?;
        Ok(books.into_iter().map(BookResponse::from).collect())
    }

    pub async fn relate_author_book(
        &self,
        author_slug: &str,
        book_slug: &str,
        role: &str,
    ) -> Result<(), Error> {
        self.db
            .query(
                "RELATE (SELECT id FROM author WHERE slug = $author_slug)[0]\
             ->wrote->(SELECT id FROM book WHERE slug = $book_slug)[0] \
             SET role = $role",
            )
            .bind(("author_slug", author_slug.to_string()))
            .bind(("book_slug", book_slug.to_string()))
            .bind(("role", role.to_string()))
            .await?;
        Ok(())
    }

    pub async fn follow_author(&self, user_id: &str, author_slug: &str) -> Result<(), Error> {
        self.db.query(
            "IF NOT EXISTS (SELECT id FROM follows WHERE in = type::record('user', $user_id) AND out = (SELECT id FROM author WHERE slug = $author_slug)[0]) THEN \
             RELATE type::record('user', $user_id)->follows->(SELECT id FROM author WHERE slug = $author_slug)[0] \
             END",
        )
        .bind(("user_id", user_id.to_string()))
        .bind(("author_slug", author_slug.to_string()))
        .await?;
        Ok(())
    }

    pub async fn unfollow_author(&self, user_id: &str, author_slug: &str) -> Result<(), Error> {
        self.db
            .query(
                "DELETE follows WHERE in = type::record('user', $user_id) \
             AND out = (SELECT id FROM author WHERE slug = $author_slug)[0]",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("author_slug", author_slug.to_string()))
            .await?;
        Ok(())
    }

    // ── Categories ───────────────────────────────────────────────────────────

    pub async fn list_categories(&self) -> Result<Vec<CategoryResponse>, Error> {
        let mut response = self
            .db
            .query("SELECT * FROM category ORDER BY name ASC")
            .await?;

        let categories: Vec<Category> = response.take(0)?;
        Ok(categories.into_iter().map(CategoryResponse::from).collect())
    }

    pub async fn get_category_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<CategoryResponse>, Error> {
        let mut response = self
            .db
            .query("SELECT * FROM category WHERE slug = $slug LIMIT 1")
            .bind(("slug", slug.to_string()))
            .await?;

        let category: Option<Category> = response.take(0)?;
        Ok(category.map(CategoryResponse::from))
    }

    pub async fn get_books_by_category(
        &self,
        slug: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BookResponse>, Error> {
        let mut response = self
            .db
            .query(
                "SELECT * FROM book \
                 WHERE categories CONTAINS (SELECT id FROM category WHERE slug = $slug)[0] \
                 ORDER BY created_at DESC LIMIT $limit START $offset",
            )
            .bind(("slug", slug.to_string()))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;

        let books: Vec<Book> = response.take(0)?;
        Ok(books.into_iter().map(BookResponse::from).collect())
    }

    // ── Tags ─────────────────────────────────────────────────────────────────

    pub async fn list_tags(&self, limit: i64, offset: i64) -> Result<Vec<TagResponse>, Error> {
        let mut response = self
            .db
            .query("SELECT * FROM tag ORDER BY name ASC LIMIT $limit START $offset")
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;

        let tags: Vec<Tag> = response.take(0)?;
        Ok(tags.into_iter().map(TagResponse::from).collect())
    }

    pub async fn get_books_by_tag(
        &self,
        slug: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BookResponse>, Error> {
        let mut response = self
            .db
            .query(
                "SELECT * FROM book \
                 WHERE tags CONTAINS (SELECT id FROM tag WHERE slug = $slug)[0] \
                 ORDER BY created_at DESC LIMIT $limit START $offset",
            )
            .bind(("slug", slug.to_string()))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;

        let books: Vec<Book> = response.take(0)?;
        Ok(books.into_iter().map(BookResponse::from).collect())
    }
}
