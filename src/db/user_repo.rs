use crate::db::Db;
use crate::db::book_repo::{Author, AuthorResponse};
use crate::db::record_id_key_to_string;
use crate::error::Error;
use merk_auth::hash_password;
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
    /// `Option<String>` rather than `String` so deserialization survives rows
    /// written before migration 0006 added the column. Production rows get
    /// `'free'` via the `DEFINE FIELD … DEFAULT` clause.
    pub plan_tier: Option<String>,
    /// AES-GCM ciphertext (base64) of the TOTP secret. `None` until the user
    /// runs `Setup2fa`. Rotated on each setup attempt.
    #[serde(default)]
    pub totp_secret_enc: Option<String>,
    /// `Some(_)` once `Verify2fa` has confirmed the secret. While `None` and
    /// `totp_secret_enc` is `Some`, the user is mid-setup and login still
    /// behaves as if 2FA is disabled.
    #[serde(default)]
    pub totp_enabled_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub totp_last_used_at: Option<DateTime<Utc>>,
    /// Argon2id-hashed recovery codes. Single-use: index removed on consumption.
    #[serde(default)]
    pub totp_recovery_codes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    pub is_verified: bool,
    pub plan_tier: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct UserPlan {
    pub tier: String,
    pub name: String,
    pub features: Vec<String>,
}

impl UserPlan {
    /// Map a `plan_tier` string to a presentable plan block. Server-side so a
    /// future paid tier flip doesn't require a client release.
    pub fn from_tier(tier: &str) -> Self {
        match tier {
            "patron" => UserPlan {
                tier: "patron".into(),
                name: "Musanif Patron".into(),
                features: vec![
                    "Unlimited highlights".into(),
                    "Cross-device sync".into(),
                    "Early access to new books".into(),
                ],
            },
            "reader" => UserPlan {
                tier: "reader".into(),
                name: "Musanif Reader".into(),
                features: vec!["Unlimited highlights".into(), "Cross-device sync".into()],
            },
            _ => UserPlan {
                tier: "free".into(),
                name: "Musanif Free".into(),
                features: vec!["Read public catalogue".into()],
            },
        }
    }
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
            plan_tier: r.plan_tier.unwrap_or_else(|| "free".to_string()),
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
    pub hours_read: f64,
    pub day_streak: i64,
}

pub struct RecordReadingSessionDto {
    pub book_slug: String,
    pub chapter_slug: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_mins: Option<i64>,
    pub page_start: Option<i64>,
    pub page_end: Option<i64>,
    pub device: Option<String>,
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

        // SurrealDB 3.x doesn't always materialise `DEFAULT` values into the
        // response of a `CREATE` (the record is returned before downstream
        // computed fields settle), and serde then fails to deserialise the
        // bool fields as `none`. Set them explicitly so the response has
        // every field the `User` struct expects.
        let mut response = self
            .db
            .query(
                "CREATE user SET \
                    username = $username, \
                    email = $email, \
                    password_hash = $password_hash, \
                    is_active = true, \
                    is_verified = false",
            )
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
        // Note: `$token` is a protected variable name in SurrealDB 3.0
        // (reserved for JWT/auth context), so this binds `$reset_token`.
        let mut resp = self
            .db
            .query(
                "UPDATE user SET \
                 password_reset_token = $reset_token, \
                 password_reset_expires_at = $expires_at \
                 WHERE email = $email",
            )
            .bind(("email", email.to_string()))
            .bind(("reset_token", token.to_string()))
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
                 WHERE password_reset_token = $reset_token \
                 AND password_reset_expires_at > time::now() \
                 LIMIT 1",
            )
            .bind(("reset_token", token.to_string()))
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

    pub async fn set_email_verification_token(
        &self,
        user_id: &str,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('user', $id) SET \
                 email_verification_token = $verify_token, \
                 email_verification_expires_at = $expires_at",
            )
            .bind(("id", user_id.to_string()))
            .bind(("verify_token", token.to_string()))
            .bind(("expires_at", expires_at))
            .await?;
        Ok(())
    }

    /// Mark the user verified and clear the token in a single round trip.
    /// Returns the verified user, or `None` if the token is unknown or expired.
    pub async fn consume_email_verification_token(
        &self,
        token: &str,
    ) -> Result<Option<User>, Error> {
        let mut resp = self
            .db
            .query(
                "UPDATE user SET \
                 is_verified = true, \
                 email_verification_token = NONE, \
                 email_verification_expires_at = NONE \
                 WHERE email_verification_token = $verify_token \
                 AND email_verification_expires_at > time::now()",
            )
            .bind(("verify_token", token.to_string()))
            .await?;
        let updated: Vec<User> = resp.take(0)?;
        Ok(updated.into_iter().next())
    }

    pub async fn get_user_stats(&self, user_id: &str) -> Result<UserStats, Error> {
        // Aggregate counts + total reading minutes in one round trip.
        #[derive(Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct StatsRaw {
            books_reading: i64,
            books_completed: i64,
            books_read_later: i64,
            books_dropped: i64,
            highlights_count: i64,
            reviews_count: i64,
            reading_sessions_count: i64,
            total_mins: i64,
        }

        let mut resp = self.db
            .query(
                "RETURN { \
                   books_reading:          (SELECT count() FROM bookmark WHERE in = type::record('user', $uid) AND status = 'reading'   GROUP ALL)[0].count ?? 0, \
                   books_completed:        (SELECT count() FROM bookmark WHERE in = type::record('user', $uid) AND status = 'completed' GROUP ALL)[0].count ?? 0, \
                   books_read_later:       (SELECT count() FROM bookmark WHERE in = type::record('user', $uid) AND status = 'readlater' GROUP ALL)[0].count ?? 0, \
                   books_dropped:          (SELECT count() FROM bookmark WHERE in = type::record('user', $uid) AND status = 'dropped'   GROUP ALL)[0].count ?? 0, \
                   highlights_count:       (SELECT count() FROM highlight        WHERE user = type::record('user', $uid) GROUP ALL)[0].count ?? 0, \
                   reviews_count:          (SELECT count() FROM book_review      WHERE in   = type::record('user', $uid) GROUP ALL)[0].count ?? 0, \
                   reading_sessions_count: (SELECT count() FROM reading_session  WHERE user = type::record('user', $uid) GROUP ALL)[0].count ?? 0, \
                   total_mins:             (SELECT math::sum(duration_mins) AS s FROM reading_session WHERE user = type::record('user', $uid) AND duration_mins != NONE GROUP ALL)[0].s ?? 0 \
                 }",
            )
            .bind(("uid", user_id.to_string()))
            .await?;
        let raw: Option<StatsRaw> = resp.take(0)?;
        let raw = raw.ok_or_else(|| Error::internal("user_repo", "Failed to compute user stats"))?;

        let day_streak = self.compute_day_streak(user_id).await?;

        Ok(UserStats {
            books_reading: raw.books_reading,
            books_completed: raw.books_completed,
            books_read_later: raw.books_read_later,
            books_dropped: raw.books_dropped,
            highlights_count: raw.highlights_count,
            reviews_count: raw.reviews_count,
            reading_sessions_count: raw.reading_sessions_count,
            hours_read: (raw.total_mins as f64) / 60.0,
            day_streak,
        })
    }

    /// Walk consecutive UTC days backwards from today (or yesterday if today
    /// is empty), counting days that have at least one reading session. Stops
    /// at the first gap.
    async fn compute_day_streak(&self, user_id: &str) -> Result<i64, Error> {
        let mut resp = self
            .db
            .query(
                "SELECT VALUE started_at FROM reading_session \
                 WHERE user = type::record('user', $uid) \
                 ORDER BY started_at DESC LIMIT 1000",
            )
            .bind(("uid", user_id.to_string()))
            .await?;
        let starts: Vec<DateTime<Utc>> = resp.take(0)?;
        if starts.is_empty() {
            return Ok(0);
        }

        use std::collections::BTreeSet;
        let unique: BTreeSet<chrono::NaiveDate> = starts.iter().map(|d| d.date_naive()).collect();

        let today = Utc::now().date_naive();
        let yesterday = today.pred_opt().unwrap_or(today);
        let mut day = if unique.contains(&today) {
            today
        } else if unique.contains(&yesterday) {
            yesterday
        } else {
            return Ok(0);
        };

        let mut streak = 0i64;
        while unique.contains(&day) {
            streak += 1;
            day = match day.pred_opt() {
                Some(d) => d,
                None => break,
            };
        }
        Ok(streak)
    }

    // ── TOTP / 2FA ──────────────────────────────────────────────────────────

    /// Stash the encrypted secret without flipping `totp_enabled_at`. The
    /// user has scanned the QR code but not yet typed a valid code, so 2FA
    /// is *not* yet active for login.
    pub async fn set_totp_pending(
        &self,
        user_id: &str,
        encrypted_secret: &str,
    ) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('user', $id) SET \
                   totp_secret_enc     = $secret, \
                   totp_enabled_at     = NONE, \
                   totp_last_used_at   = NONE, \
                   totp_recovery_codes = NONE",
            )
            .bind(("id", user_id.to_string()))
            .bind(("secret", encrypted_secret.to_string()))
            .await?;
        Ok(())
    }

    /// Flip 2FA on, persisting the hashed recovery codes generated at setup.
    pub async fn enable_totp(
        &self,
        user_id: &str,
        recovery_hashes: Vec<String>,
    ) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('user', $id) SET \
                   totp_enabled_at     = time::now(), \
                   totp_recovery_codes = $codes",
            )
            .bind(("id", user_id.to_string()))
            .bind(("codes", recovery_hashes))
            .await?;
        Ok(())
    }

    /// Clear all four totp fields atomically.
    pub async fn disable_totp(&self, user_id: &str) -> Result<(), Error> {
        self.db
            .query(
                "UPDATE type::record('user', $id) SET \
                   totp_secret_enc     = NONE, \
                   totp_enabled_at     = NONE, \
                   totp_last_used_at   = NONE, \
                   totp_recovery_codes = NONE",
            )
            .bind(("id", user_id.to_string()))
            .await?;
        Ok(())
    }

    /// Stamp `totp_last_used_at` after a successful TOTP code verification,
    /// providing a freshness signal for replay-protection follow-up work.
    pub async fn mark_totp_used(&self, user_id: &str) -> Result<(), Error> {
        self.db
            .query("UPDATE type::record('user', $id) SET totp_last_used_at = time::now()")
            .bind(("id", user_id.to_string()))
            .await?;
        Ok(())
    }

    /// Splice out one recovery hash by index. Used right after a recovery
    /// code is consumed during login.
    pub async fn consume_recovery_code(&self, user_id: &str, index: usize) -> Result<(), Error> {
        self.db
            .query(
                "LET $u = type::record('user', $id); \
                 LET $codes = $u.totp_recovery_codes ?? []; \
                 UPDATE $u SET totp_recovery_codes = array::concat( \
                    array::slice($codes, 0, $idx), \
                    array::slice($codes, $idx + 1, array::len($codes)) \
                 )",
            )
            .bind(("id", user_id.to_string()))
            .bind(("idx", index as i64))
            .await?;
        Ok(())
    }

    /// Append a reading session and refresh the matching bookmark's
    /// last_read_at / last_chapter / last_offset for the Continue rail.
    pub async fn record_reading_session(
        &self,
        user_id: &str,
        dto: RecordReadingSessionDto,
    ) -> Result<ReadingSessionResponse, Error> {
        // Resolve book slug → record id; chapter slug optional.
        let mut resp = self
            .db
            .query(
                "LET $book = (SELECT id FROM book WHERE slug = $book_slug)[0].id; \
                 LET $chapter = IF $chapter_slug = NONE THEN NONE ELSE \
                     (SELECT id FROM chapter WHERE book = $book AND slug = $chapter_slug)[0].id \
                 END; \
                 CREATE reading_session SET \
                    user          = type::record('user', $uid), \
                    book          = $book, \
                    chapter       = $chapter, \
                    started_at    = $started_at, \
                    ended_at      = $ended_at, \
                    duration_mins = $duration_mins, \
                    page_start    = $page_start ?? 0, \
                    page_end      = $page_end, \
                    device        = $device \
                 RETURN AFTER; \
                 UPDATE bookmark \
                    SET last_read_at = $ended_at ?? $started_at, \
                        last_chapter = $chapter \
                    WHERE in = type::record('user', $uid) AND out = $book",
            )
            .bind(("uid", user_id.to_string()))
            .bind(("book_slug", dto.book_slug))
            .bind(("chapter_slug", dto.chapter_slug))
            .bind(("started_at", dto.started_at))
            .bind(("ended_at", dto.ended_at))
            .bind(("duration_mins", dto.duration_mins))
            .bind(("page_start", dto.page_start))
            .bind(("page_end", dto.page_end))
            .bind(("device", dto.device))
            .await?;

        // The CREATE statement is the third in the chain (after two LET).
        let created: Vec<ReadingSessionRaw> = resp.take(2)?;
        let session = created
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("user_repo", "session insert failed"))?;
        Ok(session.into())
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
    use merk_auth::verify_password;
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
