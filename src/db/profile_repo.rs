use crate::db::Db;
use crate::error::Error;
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
pub struct Profile {
    pub id: Option<RecordId>,
    pub user: RecordId,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub language: String,
    pub country: String,
    pub timezone: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ProfileResponse {
    pub id: String,
    pub user_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub language: String,
    pub country: String,
    pub timezone: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
}

impl From<Profile> for ProfileResponse {
    fn from(p: Profile) -> Self {
        ProfileResponse {
            id: p.id.map(|r| key_to_string(r.key)).unwrap_or_default(),
            user_id: key_to_string(p.user.key),
            first_name: p.first_name,
            last_name: p.last_name,
            display_name: p.display_name,
            avatar_url: p.avatar_url,
            bio: p.bio,
            language: p.language,
            country: p.country,
            timezone: p.timezone,
            phone: p.phone,
            website: p.website,
        }
    }
}

pub struct CreateProfileDto {
    pub user_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub language: Option<String>,
    pub country: Option<String>,
}

pub struct UpdateProfileDto {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub language: Option<String>,
    pub country: Option<String>,
    pub timezone: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
}

pub struct ProfileRepo {
    pub db: Db,
}

impl ProfileRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn create_profile(&self, dto: CreateProfileDto) -> Result<Profile, Error> {
        let mut response = self
            .db
            .query(
                "CREATE profile SET \
                 user = type::record('user', $user_id), \
                 first_name = $first_name, \
                 last_name = $last_name, \
                 display_name = $display_name, \
                 language = $language, \
                 country = $country",
            )
            .bind(("user_id", dto.user_id))
            .bind(("first_name", dto.first_name))
            .bind(("last_name", dto.last_name))
            .bind(("display_name", dto.display_name))
            .bind(("language", dto.language.unwrap_or_else(|| "en".to_string())))
            .bind(("country", dto.country.unwrap_or_else(|| "US".to_string())))
            .await?;

        let profile: Option<Profile> = response.take(0)?;
        profile.ok_or_else(|| Error::internal("profile_repo", "Failed to create profile"))
    }

    pub async fn get_profile_by_user_id(&self, user_id: &str) -> Result<Option<Profile>, Error> {
        let mut response = self
            .db
            .query("SELECT * FROM profile WHERE user = type::record('user', $user_id) LIMIT 1")
            .bind(("user_id", user_id.to_string()))
            .await?;

        let profile: Option<Profile> = response.take(0)?;
        Ok(profile)
    }

    pub async fn update_profile(
        &self,
        user_id: &str,
        dto: UpdateProfileDto,
    ) -> Result<Option<Profile>, Error> {
        let mut response = self.db
            .query(
                "UPDATE profile SET \
                 first_name   = IF $first_name   IS NOT NONE THEN $first_name   ELSE first_name END, \
                 last_name    = IF $last_name    IS NOT NONE THEN $last_name    ELSE last_name END, \
                 display_name = IF $display_name IS NOT NONE THEN $display_name ELSE display_name END, \
                 avatar_url   = IF $avatar_url   IS NOT NONE THEN $avatar_url   ELSE avatar_url END, \
                 bio          = IF $bio          IS NOT NONE THEN $bio          ELSE bio END, \
                 language     = IF $language     IS NOT NONE THEN $language     ELSE language END, \
                 country      = IF $country      IS NOT NONE THEN $country      ELSE country END, \
                 timezone     = IF $timezone     IS NOT NONE THEN $timezone     ELSE timezone END, \
                 phone        = IF $phone        IS NOT NONE THEN $phone        ELSE phone END, \
                 website      = IF $website      IS NOT NONE THEN $website      ELSE website END, \
                 updated_at   = time::now() \
                 WHERE user = type::record('user', $user_id)",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("first_name", dto.first_name))
            .bind(("last_name", dto.last_name))
            .bind(("display_name", dto.display_name))
            .bind(("avatar_url", dto.avatar_url))
            .bind(("bio", dto.bio))
            .bind(("language", dto.language))
            .bind(("country", dto.country))
            .bind(("timezone", dto.timezone))
            .bind(("phone", dto.phone))
            .bind(("website", dto.website))
            .await?;

        let profile: Option<Profile> = response.take(0)?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateProfileDto, ProfileRepo, UpdateProfileDto};
    use crate::db::user_repo::{CreateUserDto, UserRepo};
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
        db.query(r#"
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

            DEFINE TABLE profile SCHEMAFULL;
            DEFINE FIELD user         ON profile TYPE record<user>;
            DEFINE FIELD first_name   ON profile TYPE option<string>;
            DEFINE FIELD last_name    ON profile TYPE option<string>;
            DEFINE FIELD display_name ON profile TYPE option<string>;
            DEFINE FIELD avatar_url   ON profile TYPE option<string>;
            DEFINE FIELD bio          ON profile TYPE option<string>;
            DEFINE FIELD language     ON profile TYPE string DEFAULT "en";
            DEFINE FIELD country      ON profile TYPE string DEFAULT "US";
            DEFINE FIELD timezone     ON profile TYPE option<string>;
            DEFINE FIELD phone        ON profile TYPE option<string>;
            DEFINE FIELD website      ON profile TYPE option<string>;
            DEFINE FIELD created_at   ON profile TYPE datetime DEFAULT time::now() READONLY;
            DEFINE FIELD updated_at   ON profile TYPE datetime DEFAULT time::now() VALUE time::now();
            DEFINE INDEX profile_user_idx ON profile COLUMNS user UNIQUE;
        "#)
        .await
        .unwrap();
        db
    }

    async fn seed_user(db: &Surreal<Any>, username: &str, email: &str) -> String {
        let repo = UserRepo::new(db.clone());
        let user = repo
            .create_user(CreateUserDto {
                username: username.to_string(),
                email: email.to_string(),
                raw_password: "pass".to_string(),
            })
            .await
            .unwrap();
        record_id_key_to_string(user.id.unwrap().key)
    }

    fn empty_update() -> UpdateProfileDto {
        UpdateProfileDto {
            first_name: None,
            last_name: None,
            display_name: None,
            avatar_url: None,
            bio: None,
            language: None,
            country: None,
            timezone: None,
            phone: None,
            website: None,
        }
    }

    // ── create ───────────────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_create_profile_applies_defaults(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = ProfileRepo::new(db.clone());
        let user_id = seed_user(&db, "alice", "alice@example.com").await;
        let profile = repo
            .create_profile(CreateProfileDto {
                user_id,
                first_name: None,
                last_name: None,
                display_name: None,
                language: None,
                country: None,
            })
            .await
            .unwrap();
        assert_eq!(profile.language, "en");
        assert_eq!(profile.country, "US");
        assert!(profile.first_name.is_none());
        assert!(profile.id.is_some());
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_profile_with_explicit_fields(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = ProfileRepo::new(db.clone());
        let user_id = seed_user(&db, "bob", "bob@example.com").await;
        let profile = repo
            .create_profile(CreateProfileDto {
                user_id,
                first_name: Some("Bob".to_string()),
                last_name: Some("Smith".to_string()),
                display_name: Some("bobsmith".to_string()),
                language: Some("fr".to_string()),
                country: Some("FR".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(profile.first_name.as_deref(), Some("Bob"));
        assert_eq!(profile.last_name.as_deref(), Some("Smith"));
        assert_eq!(profile.display_name.as_deref(), Some("bobsmith"));
        assert_eq!(profile.language, "fr");
        assert_eq!(profile.country, "FR");
    }

    // ── get by user_id ───────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_get_profile_by_user_id_found(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = ProfileRepo::new(db.clone());
        let user_id = seed_user(&db, "carol", "carol@example.com").await;
        repo.create_profile(CreateProfileDto {
            user_id: user_id.clone(),
            first_name: Some("Carol".to_string()),
            last_name: None,
            display_name: None,
            language: None,
            country: None,
        })
        .await
        .unwrap();
        let found = repo.get_profile_by_user_id(&user_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().first_name.as_deref(), Some("Carol"));
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_profile_by_user_id_not_found(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = ProfileRepo::new(db.clone());
        let found = repo.get_profile_by_user_id("ghost").await.unwrap();
        assert!(found.is_none());
    }

    // ── update ───────────────────────────────────────────────────────────────

    #[rstest]
    #[tokio::test]
    async fn test_update_profile_partial_leaves_other_fields_unchanged(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = ProfileRepo::new(db.clone());
        let user_id = seed_user(&db, "dave", "dave@example.com").await;
        repo.create_profile(CreateProfileDto {
            user_id: user_id.clone(),
            first_name: Some("Dave".to_string()),
            last_name: Some("Jones".to_string()),
            display_name: None,
            language: Some("de".to_string()),
            country: None,
        })
        .await
        .unwrap();
        repo.update_profile(
            &user_id,
            UpdateProfileDto {
                bio: Some("Hello world".to_string()),
                ..empty_update()
            },
        )
        .await
        .unwrap();
        let updated = repo
            .get_profile_by_user_id(&user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.first_name.as_deref(), Some("Dave"));
        assert_eq!(updated.last_name.as_deref(), Some("Jones"));
        assert_eq!(updated.bio.as_deref(), Some("Hello world"));
        assert_eq!(updated.language, "de");
    }

    #[rstest]
    #[tokio::test]
    async fn test_update_profile_all_fields(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = ProfileRepo::new(db.clone());
        let user_id = seed_user(&db, "eve", "eve@example.com").await;
        repo.create_profile(CreateProfileDto {
            user_id: user_id.clone(),
            first_name: None,
            last_name: None,
            display_name: None,
            language: None,
            country: None,
        })
        .await
        .unwrap();
        repo.update_profile(
            &user_id,
            UpdateProfileDto {
                first_name: Some("Eve".to_string()),
                last_name: Some("Turner".to_string()),
                display_name: Some("eve_t".to_string()),
                avatar_url: Some("https://example.com/avatar.png".to_string()),
                bio: Some("Software engineer".to_string()),
                language: Some("es".to_string()),
                country: Some("ES".to_string()),
                timezone: Some("Europe/Madrid".to_string()),
                phone: Some("+34123456789".to_string()),
                website: Some("https://eve.dev".to_string()),
            },
        )
        .await
        .unwrap();
        let updated = repo
            .get_profile_by_user_id(&user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.first_name.as_deref(), Some("Eve"));
        assert_eq!(updated.last_name.as_deref(), Some("Turner"));
        assert_eq!(updated.display_name.as_deref(), Some("eve_t"));
        assert_eq!(
            updated.avatar_url.as_deref(),
            Some("https://example.com/avatar.png")
        );
        assert_eq!(updated.bio.as_deref(), Some("Software engineer"));
        assert_eq!(updated.language, "es");
        assert_eq!(updated.country, "ES");
        assert_eq!(updated.timezone.as_deref(), Some("Europe/Madrid"));
        assert_eq!(updated.phone.as_deref(), Some("+34123456789"));
        assert_eq!(updated.website.as_deref(), Some("https://eve.dev"));
    }

    #[rstest]
    #[tokio::test]
    async fn test_update_profile_not_found_returns_none(#[future] db: Surreal<Any>) {
        let db = db.await;
        let repo = ProfileRepo::new(db.clone());
        let result = repo
            .update_profile(
                "nonexistent",
                UpdateProfileDto {
                    first_name: Some("Ghost".to_string()),
                    ..empty_update()
                },
            )
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
