use crate::db::Db;
use crate::db::book_repo::{Author, AuthorResponse};
use crate::db::record_id_key_to_string;
use crate::error::Error;
use crate::services::auth::hash_password;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct User {
    pub id: Option<RecordId>,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_active: bool,
    pub is_verified: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    pub is_verified: bool,
}

impl From<User> for UserResponse {
    fn from(r: User) -> Self {
        UserResponse {
            id: r
                .id
                .map(|r| record_id_key_to_string(&r.key))
                .unwrap_or_default(),
            username: r.username,
            email: r.email,
            is_active: r.is_active,
            is_verified: r.is_verified,
        }
    }
}

pub struct CreateUserDto {
    pub username: String,
    pub email: String,
    pub raw_password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct UserStats {
    pub books_reading: i64,
    pub books_completed: i64,
    pub books_read_later: i64,
    pub books_dropped: i64,
    pub highlights_count: i64,
    pub reviews_count: i64,
    pub reading_sessions_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ReadingSessionResponse {
    pub id: String,
    pub book_id: String,
    pub chapter_id: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_mins: Option<i64>,
    pub page_start: i64,
    pub page_end: Option<i64>,
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct ReadingSessionRaw {
    id: Option<RecordId>,
    book: RecordId,
    chapter: Option<RecordId>,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    duration_mins: Option<i64>,
    page_start: i64,
    page_end: Option<i64>,
    device: Option<String>,
}

impl From<ReadingSessionRaw> for ReadingSessionResponse {
    fn from(r: ReadingSessionRaw) -> Self {
        ReadingSessionResponse {
            id: r
                .id
                .map(|rid| record_id_key_to_string(&rid.key))
                .unwrap_or_default(),
            book_id: record_id_key_to_string(&r.book.key),
            chapter_id: r.chapter.map(|c| record_id_key_to_string(&c.key)),
            started_at: r.started_at.map(|dt| dt.to_rfc3339()),
            ended_at: r.ended_at.map(|dt| dt.to_rfc3339()),
            duration_mins: r.duration_mins,
            page_start: r.page_start,
            page_end: r.page_end,
            device: r.device,
        }
    }
}

pub struct UserRepo {
    pub db: Db,
}

impl UserRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn create_user(&self, dto: CreateUserDto) -> Result<User, Error> {
        let hashed_password = hash_password(&dto.raw_password);

        let mut response = self.db
            .query("CREATE user SET username = $username, email = $email, password_hash = $password_hash")
            .bind(("username", dto.username))
            .bind(("email", dto.email))
            .bind(("password_hash", hashed_password))
            .await?;

        let user: Option<User> = response.take(0)?;
        user.ok_or_else(|| Error::internal("user_repo", "Failed to create user"))
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, Error> {
        let mut response = self
            .db
            .query("SELECT * FROM user WHERE email = $email LIMIT 1")
            .bind(("email", email.to_string()))
            .await?;

        let user: Option<User> = response.take(0)?;
        Ok(user)
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, Error> {
        let mut response = self
            .db
            .query("SELECT * FROM type::record('user', $id)")
            .bind(("id", id.to_string()))
            .await?;

        let user: Option<User> = response.take(0)?;
        Ok(user)
    }

    pub async fn update_last_login(&self, id: &str) -> Result<(), Error> {
        self.db
            .query("UPDATE type::record('user', $id) SET last_login = time::now()")
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn deactivate_user(&self, id: &str) -> Result<(), Error> {
        self.db
            .query("UPDATE type::record('user', $id) SET is_active = false")
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn reset_password(&self, id: &str, new_password: &str) -> Result<(), Error> {
        let hashed = hash_password(new_password);
        self.db
            .query("UPDATE type::record('user', $id) SET password_hash = $hash")
            .bind(("id", id.to_string()))
            .bind(("hash", hashed))
            .await?;
        Ok(())
    }

    pub async fn set_reset_token(
        &self,
        email: &str,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE user SET \
                 password_reset_token = $token, \
                 password_reset_expires_at = $expires_at \
                 WHERE email = $email",
            )
            .bind(("email", email.to_string()))
            .bind(("token", token.to_string()))
            .bind(("expires_at", expires_at))
            .await?;
        let updated: Vec<User> = resp.take(0)?;
        Ok(!updated.is_empty())
    }

    pub async fn get_user_by_reset_token(&self, token: &str) -> Result<Option<User>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM user \
                 WHERE password_reset_token = $token \
                 AND password_reset_expires_at > time::now() \
                 LIMIT 1",
            )
            .bind(("token", token.to_string()))
            .await?;
        let user: Option<User> = resp.take(0)?;
        Ok(user)
    }

    pub async fn clear_reset_token(&self, id: &str) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('user', $id) SET \
             password_reset_token = NONE, \
             password_reset_expires_at = NONE",
            )
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn get_user_stats(&self, user_id: &str) -> Result<UserStats, Error> {
        let mut resp = self.db
            .query(
                "RETURN { \
                   books_reading:          (SELECT count() FROM bookmark WHERE user = type::record('user', $uid) AND status = 'reading'   GROUP ALL)[0].count ?? 0, \
                   books_completed:        (SELECT count() FROM bookmark WHERE user = type::record('user', $uid) AND status = 'completed' GROUP ALL)[0].count ?? 0, \
                   books_read_later:       (SELECT count() FROM bookmark WHERE user = type::record('user', $uid) AND status = 'readlater' GROUP ALL)[0].count ?? 0, \
                   books_dropped:          (SELECT count() FROM bookmark WHERE user = type::record('user', $uid) AND status = 'dropped'   GROUP ALL)[0].count ?? 0, \
                   highlights_count:       (SELECT count() FROM highlight        WHERE user = type::record('user', $uid) GROUP ALL)[0].count ?? 0, \
                   reviews_count:          (SELECT count() FROM book_review      WHERE user = type::record('user', $uid) GROUP ALL)[0].count ?? 0, \
                   reading_sessions_count: (SELECT count() FROM reading_session  WHERE user = type::record('user', $uid) GROUP ALL)[0].count ?? 0 \
                 }",
            )
            .bind(("uid", user_id.to_string()))
            .await?;
        let stats: Option<UserStats> = resp.take(0)?;
        stats.ok_or_else(|| Error::internal("user_repo", "Failed to compute user stats"))
    }

    pub async fn get_reading_sessions(
        &self,
        user_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ReadingSessionResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM reading_session \
                 WHERE user = type::record('user', $user_id) \
                 ORDER BY started_at DESC \
                 LIMIT $limit START $offset",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;
        let rows: Vec<ReadingSessionRaw> = resp.take(0)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_following_authors(
        &self,
        user_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuthorResponse>, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT * FROM type::record('user', $user_id)->follows->author \
                 LIMIT $limit START $offset",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;
        let authors: Vec<Author> = resp.take(0)?;
        Ok(authors.into_iter().map(AuthorResponse::from).collect())
    }

    pub async fn reset_password_and_clear_token(
        &self,
        id: &str,
        new_password: &str,
    ) -> Result<(), Error> {
        let hashed = hash_password(new_password);
        self.db
            .query(
                "UPDATE type::record('user', $id) SET \
                 password_hash = $hash, \
                 password_reset_token = NONE, \
                 password_reset_expires_at = NONE",
            )
            .bind(("id", id.to_string()))
            .bind(("hash", hashed))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateUserDto, UserRepo};
    use crate::db::record_id_key_to_string;
    use crate::services::auth::verify_password;
    use rstest::{fixture, rstest};
    use surrealdb::Surreal;
    use surrealdb::engine::any::{Any, connect};

    #[fixture]
    async fn db() -> Surreal<Any> {
        let db = connect("mem://").await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();
        db.query(
            r#"
            DEFINE TABLE user SCHEMAFULL;
            DEFINE FIELD username      ON user TYPE string;
            DEFINE FIELD email         ON user TYPE string ASSERT string::is_email($value);
            DEFINE FIELD password_hash ON user TYPE string;
            DEFINE FIELD is_active     ON user TYPE bool DEFAULT true;
            DEFINE FIELD is_verified   ON user TYPE bool DEFAULT false;
            DEFINE FIELD created_at    ON user TYPE datetime DEFAULT time::now() READONLY;
            DEFINE FIELD updated_at    ON user TYPE datetime DEFAULT time::now() VALUE time::now();
            DEFINE FIELD last_login    ON user TYPE option<datetime>;
            DEFINE FIELD metadata      ON user TYPE object DEFAULT {};
            DEFINE INDEX user_username_idx ON user COLUMNS username UNIQUE;
            DEFINE INDEX user_email_idx    ON user COLUMNS email    UNIQUE;
        "#,
        )
        .await
        .unwrap();
        db
    }

    fn dto(username: &str, email: &str) -> CreateUserDto {
        CreateUserDto {
            username: username.to_string(),
            email: email.to_string(),
            raw_password: "secret123".to_string(),
        }
    }

    // ── create ──────────────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_create_user_sets_fields(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = UserRepo::new(db);
        let user = repo
            .create_user(dto("alice", "alice@example.com"))
            .await
            .unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.email, "alice@example.com");
        assert!(user.is_active);
        assert!(!user.is_verified);
        assert!(user.id.is_some());
        assert!(user.last_login.is_none());
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_user_hashes_password(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = UserRepo::new(db);
        let user = repo
            .create_user(dto("bob", "bob@example.com"))
            .await
            .unwrap();
        assert_ne!(user.password_hash, "secret123");
        assert!(verify_password("secret123", &user.password_hash));
    }

    // ── get by email ─────────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_get_user_by_email_found(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = UserRepo::new(db);
        repo.create_user(dto("carol", "carol@example.com"))
            .await
            .unwrap();
        let found = repo.get_user_by_email("carol@example.com").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "carol");
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_user_by_email_not_found(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = UserRepo::new(db);
        let found = repo.get_user_by_email("ghost@example.com").await.unwrap();
        assert!(found.is_none());
    }

    // ── get by id ────────────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_get_user_by_id_found(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = UserRepo::new(db);
        let created = repo
            .create_user(dto("dave", "dave@example.com"))
            .await
            .unwrap();
        let id = record_id_key_to_string(&created.id.unwrap().key);
        let found = repo.get_user_by_id(&id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "dave@example.com");
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_user_by_id_not_found(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = UserRepo::new(db);
        let found = repo.get_user_by_id("nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    // ── update_last_login ────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_update_last_login(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = UserRepo::new(db);
        let created = repo
            .create_user(dto("eve", "eve@example.com"))
            .await
            .unwrap();
        let id = record_id_key_to_string(&created.id.unwrap().key);
        assert!(created.last_login.is_none());
        repo.update_last_login(&id).await.unwrap();
        let updated = repo.get_user_by_id(&id).await.unwrap().unwrap();
        assert!(updated.last_login.is_some());
    }

    // ── deactivate ───────────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_deactivate_user(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = UserRepo::new(db);
        let created = repo
            .create_user(dto("frank", "frank@example.com"))
            .await
            .unwrap();
        let id = record_id_key_to_string(&created.id.unwrap().key);
        assert!(created.is_active);
        repo.deactivate_user(&id).await.unwrap();
        let deactivated = repo.get_user_by_id(&id).await.unwrap().unwrap();
        assert!(!deactivated.is_active);
    }

    // ── reset_password ───────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_reset_password(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = UserRepo::new(db);
        let created = repo
            .create_user(dto("grace", "grace@example.com"))
            .await
            .unwrap();
        let id = record_id_key_to_string(&created.id.unwrap().key);
        let old_hash = created.password_hash.clone();
        repo.reset_password(&id, "newpassword456").await.unwrap();
        let updated = repo.get_user_by_id(&id).await.unwrap().unwrap();
        assert_ne!(updated.password_hash, old_hash);
        assert!(verify_password("newpassword456", &updated.password_hash));
        assert!(!verify_password("secret123", &updated.password_hash));
    }
}
