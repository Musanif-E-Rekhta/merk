use crate::db::Db;
use crate::error::Error;
use crate::services::auth::hash_password;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};

fn key_to_string(key: RecordIdKey) -> String {
    match key {
        RecordIdKey::String(s) => s,
        RecordIdKey::Number(n) => n.to_string(),
        RecordIdKey::Uuid(u) => u.to_string(),
        other => format!("{other:?}"),
    }
}

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
            id: r.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
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

pub struct UserRepo;

impl UserRepo {
    pub async fn create_user(db: &Db, dto: CreateUserDto) -> Result<User, Error> {
        let hashed_password = hash_password(&dto.raw_password);

        let mut response = db
            .query("CREATE user SET username = $username, email = $email, password_hash = $password_hash")
            .bind(("username", dto.username))
            .bind(("email", dto.email))
            .bind(("password_hash", hashed_password))
            .await?;

        let user: Option<User> = response.take(0)?;
        user.ok_or_else(|| Error::internal("user_repo", "Failed to create user"))
    }

    pub async fn get_user_by_email(db: &Db, email: &str) -> Result<Option<User>, Error> {
        let mut response = db
            .query("SELECT * FROM user WHERE email = $email LIMIT 1")
            .bind(("email", email.to_string()))
            .await?;

        let user: Option<User> = response.take(0)?;
        Ok(user)
    }

    pub async fn get_user_by_id(db: &Db, id: &str) -> Result<Option<User>, Error> {
        let mut response = db
            .query("SELECT * FROM type::record('user', $id)")
            .bind(("id", id.to_string()))
            .await?;

        let user: Option<User> = response.take(0)?;
        Ok(user)
    }

    pub async fn update_last_login(db: &Db, id: &str) -> Result<(), Error> {
        db.query("UPDATE type::record('user', $id) SET last_login = time::now()")
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn deactivate_user(db: &Db, id: &str) -> Result<(), Error> {
        db.query("UPDATE type::record('user', $id) SET is_active = false")
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn reset_password(db: &Db, id: &str, new_password: &str) -> Result<(), Error> {
        let hashed = hash_password(new_password);
        db.query("UPDATE type::record('user', $id) SET password_hash = $hash")
            .bind(("id", id.to_string()))
            .bind(("hash", hashed))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateUserDto, UserRepo};
    use crate::services::auth::verify_password;
    use rstest::{fixture, rstest};
    use surrealdb::Surreal;
    use surrealdb::engine::any::{Any, connect};
    use surrealdb::types::RecordIdKey;

    fn record_id_key_to_string(key: RecordIdKey) -> String {
        match key {
            RecordIdKey::String(s) => s,
            RecordIdKey::Number(n) => n.to_string(),
            RecordIdKey::Uuid(u) => u.to_string(),
            other => format!("{other:?}"),
        }
    }

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
        let user = UserRepo::create_user(&db, dto("alice", "alice@example.com"))
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
        let user = UserRepo::create_user(&db, dto("bob", "bob@example.com"))
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
        UserRepo::create_user(&db, dto("carol", "carol@example.com"))
            .await
            .unwrap();
        let found = UserRepo::get_user_by_email(&db, "carol@example.com")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "carol");
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_user_by_email_not_found(#[future] db: Surreal<Any>) {
        let db = db.await;
        let found = UserRepo::get_user_by_email(&db, "ghost@example.com")
            .await
            .unwrap();
        assert!(found.is_none());
    }

    // ── get by id ────────────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_get_user_by_id_found(#[future] db: Surreal<Any>) {
        let db = db.await;
        let created = UserRepo::create_user(&db, dto("dave", "dave@example.com"))
            .await
            .unwrap();
        let id = record_id_key_to_string(created.id.unwrap().key);
        let found = UserRepo::get_user_by_id(&db, &id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "dave@example.com");
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_user_by_id_not_found(#[future] db: Surreal<Any>) {
        let db = db.await;
        let found = UserRepo::get_user_by_id(&db, "nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    // ── update_last_login ────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_update_last_login(#[future] db: Surreal<Any>) {
        let db = db.await;
        let created = UserRepo::create_user(&db, dto("eve", "eve@example.com"))
            .await
            .unwrap();
        let id = record_id_key_to_string(created.id.unwrap().key);
        assert!(created.last_login.is_none());
        UserRepo::update_last_login(&db, &id).await.unwrap();
        let updated = UserRepo::get_user_by_id(&db, &id).await.unwrap().unwrap();
        assert!(updated.last_login.is_some());
    }

    // ── deactivate ───────────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_deactivate_user(#[future] db: Surreal<Any>) {
        let db = db.await;
        let created = UserRepo::create_user(&db, dto("frank", "frank@example.com"))
            .await
            .unwrap();
        let id = record_id_key_to_string(created.id.unwrap().key);
        assert!(created.is_active);
        UserRepo::deactivate_user(&db, &id).await.unwrap();
        let deactivated = UserRepo::get_user_by_id(&db, &id).await.unwrap().unwrap();
        assert!(!deactivated.is_active);
    }

    // ── reset_password ───────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_reset_password(#[future] db: Surreal<Any>) {
        let db = db.await;
        let created = UserRepo::create_user(&db, dto("grace", "grace@example.com"))
            .await
            .unwrap();
        let id = record_id_key_to_string(created.id.unwrap().key);
        let old_hash = created.password_hash.clone();
        UserRepo::reset_password(&db, &id, "newpassword456")
            .await
            .unwrap();
        let updated = UserRepo::get_user_by_id(&db, &id).await.unwrap().unwrap();
        assert_ne!(updated.password_hash, old_hash);
        assert!(verify_password("newpassword456", &updated.password_hash));
        assert!(!verify_password("secret123", &updated.password_hash));
    }
}
