use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};

use crate::api::middleware::Claims;
use crate::db::book_repo::AuthorResponse;
use crate::db::profile_repo::{ProfileResponse, UpdateProfileDto};
use crate::db::user_repo::{
    ReadingSessionResponse, RecordReadingSessionDto, UserPlan, UserResponse, UserStats,
};
use crate::services::user_service::LoginOutcome;
use crate::state::AppState;

#[derive(Default)]
pub struct UserQuery;

#[Object]
impl UserQuery {
    async fn me(&self, ctx: &Context<'_>) -> Result<MeResponseGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let (ur, profile) = tokio::try_join!(
            state.services.user.get_user_by_id(&claims.sub),
            state
                .services
                .profile_repo
                .get_profile_by_user_id(&claims.sub),
        )?;

        let plan = UserPlan::from_tier(&ur.plan_tier);

        Ok(MeResponseGql {
            id: ur.id,
            username: ur.username,
            email: ur.email,
            is_active: ur.is_active,
            is_verified: ur.is_verified,
            profile: profile.map(|p| ProfileResponse::from(p).into()),
            plan: PlanGql {
                tier: plan.tier,
                name: plan.name,
                features: plan.features,
            },
        })
    }

    async fn my_stats(&self, ctx: &Context<'_>) -> Result<UserStatsGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let stats = state.services.user.get_user_stats(&claims.sub).await?;
        Ok(stats.into())
    }

    async fn my_reading_sessions(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<ReadingSessionGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let sessions = state
            .services
            .user
            .get_reading_sessions(
                &claims.sub,
                limit.unwrap_or(20) as u32,
                offset.unwrap_or(0) as u32,
            )
            .await?;
        Ok(sessions.into_iter().map(Into::into).collect())
    }

    async fn my_following(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<AuthorGql>> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let authors = state
            .services
            .user
            .get_following_authors(
                &claims.sub,
                limit.unwrap_or(20) as u32,
                offset.unwrap_or(0) as u32,
            )
            .await?;
        Ok(authors.into_iter().map(Into::into).collect())
    }

    async fn my2fa_status(&self, ctx: &Context<'_>) -> Result<TwoFactorStatusGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let s = state.services.user.my_2fa_status(&claims.sub).await?;
        Ok(TwoFactorStatusGql {
            enabled: s.enabled,
            last_used_at: s.last_used_at,
        })
    }
}

// ── Output types ─────────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct AuthPayload {
    /// Empty string when `requires_2fa` is true — the client should not
    /// store this as a session token. The real access token comes back
    /// from `login2faComplete`.
    pub token: String,
    /// Long-lived opaque token used to mint a fresh access token via
    /// `refreshToken`. `None` during the 2FA challenge step (no session
    /// has been established yet).
    pub refresh_token: Option<String>,
    pub user: UserResponseGql,
    /// `true` when the account has 2FA enabled and the caller still needs
    /// to complete the TOTP step. `false` for registration and for
    /// non-2FA logins.
    #[graphql(default)]
    pub requires_2fa: bool,
    /// Short-lived (5 min) opaque token to pass back into
    /// `login2faComplete`. `None` outside the 2FA flow.
    pub challenge: Option<String>,
}

#[derive(SimpleObject)]
pub struct Setup2faPayload {
    /// Plaintext base32 secret. Returned exactly once at setup.
    pub secret: String,
    /// `otpauth://totp/...` URL for QR-code rendering.
    pub otpauth_url: String,
    /// Plaintext recovery codes. Server only stores Argon2id hashes.
    pub recovery_codes: Vec<String>,
}

#[derive(SimpleObject)]
pub struct TwoFactorStatusGql {
    pub enabled: bool,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(SimpleObject)]
pub struct UserResponseGql {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    pub is_verified: bool,
    pub plan_tier: String,
}

impl From<UserResponse> for UserResponseGql {
    fn from(r: UserResponse) -> Self {
        UserResponseGql {
            id: r.id,
            username: r.username,
            email: r.email,
            is_active: r.is_active,
            is_verified: r.is_verified,
            plan_tier: r.plan_tier,
        }
    }
}

#[derive(SimpleObject)]
pub struct PlanGql {
    pub tier: String,
    pub name: String,
    pub features: Vec<String>,
}

#[derive(SimpleObject)]
pub struct ProfileResponseGql {
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

impl From<ProfileResponse> for ProfileResponseGql {
    fn from(p: ProfileResponse) -> Self {
        ProfileResponseGql {
            id: p.id,
            user_id: p.user_id,
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

#[derive(SimpleObject)]
pub struct MeResponseGql {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    pub is_verified: bool,
    pub profile: Option<ProfileResponseGql>,
    pub plan: PlanGql,
}

#[derive(SimpleObject)]
pub struct UserStatsGql {
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

impl From<UserStats> for UserStatsGql {
    fn from(s: UserStats) -> Self {
        UserStatsGql {
            books_reading: s.books_reading,
            books_completed: s.books_completed,
            books_read_later: s.books_read_later,
            books_dropped: s.books_dropped,
            highlights_count: s.highlights_count,
            reviews_count: s.reviews_count,
            reading_sessions_count: s.reading_sessions_count,
            hours_read: s.hours_read,
            day_streak: s.day_streak,
        }
    }
}

#[derive(SimpleObject)]
pub struct ReadingSessionGql {
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

impl From<ReadingSessionResponse> for ReadingSessionGql {
    fn from(r: ReadingSessionResponse) -> Self {
        ReadingSessionGql {
            id: r.id,
            book_id: r.book_id,
            chapter_id: r.chapter_id,
            started_at: r.started_at,
            ended_at: r.ended_at,
            duration_mins: r.duration_mins,
            page_start: r.page_start,
            page_end: r.page_end,
            device: r.device,
        }
    }
}

#[derive(SimpleObject)]
pub struct AuthorGql {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
}

impl From<AuthorResponse> for AuthorGql {
    fn from(a: AuthorResponse) -> Self {
        AuthorGql {
            id: a.id,
            name: a.name,
            slug: a.slug,
            bio: a.bio,
            avatar_url: a.avatar_url,
            website: a.website,
        }
    }
}

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct UpdateProfileInput {
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

#[derive(InputObject)]
pub struct RecordReadingSessionInput {
    pub book_slug: String,
    pub chapter_slug: Option<String>,
    /// RFC3339 timestamp; required because the client knows when the session
    /// actually started (server can't reconstruct from a heartbeat).
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_mins: Option<i64>,
    pub page_start: Option<i64>,
    pub page_end: Option<i64>,
    pub device: Option<String>,
}

// ── Mutations ─────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct UserMutation;

#[Object]
impl UserMutation {
    async fn register_user(
        &self,
        ctx: &Context<'_>,
        username: String,
        email: String,
        password: String,
    ) -> Result<AuthPayload> {
        let state = ctx.data::<AppState>()?;

        let (tokens, user) = state
            .services
            .user
            .register(username, email, password)
            .await?;

        Ok(AuthPayload {
            token: tokens.access,
            refresh_token: Some(tokens.refresh),
            user: UserResponseGql::from(user),
            requires_2fa: false,
            challenge: None,
        })
    }

    async fn login_user(
        &self,
        ctx: &Context<'_>,
        email: String,
        password: String,
    ) -> Result<AuthPayload> {
        let state = ctx.data::<AppState>()?;

        match state.services.user.login(email, password).await? {
            LoginOutcome::Authenticated { tokens, user } => Ok(AuthPayload {
                token: tokens.access,
                refresh_token: Some(tokens.refresh),
                user: UserResponseGql::from(user),
                requires_2fa: false,
                challenge: None,
            }),
            LoginOutcome::Requires2fa { challenge, user } => Ok(AuthPayload {
                token: String::new(),
                refresh_token: None,
                user: UserResponseGql::from(user),
                requires_2fa: true,
                challenge: Some(challenge),
            }),
        }
    }

    /// Second leg of 2FA login. Pass back the `challenge` from `login_user`
    /// plus the user's current 6-digit TOTP code (or one of their recovery
    /// codes). Returns the real access token.
    async fn login2fa_complete(
        &self,
        ctx: &Context<'_>,
        challenge: String,
        code: String,
    ) -> Result<AuthPayload> {
        let state = ctx.data::<AppState>()?;
        let (tokens, user) = state
            .services
            .user
            .complete_2fa_login(challenge, code)
            .await?;
        Ok(AuthPayload {
            token: tokens.access,
            refresh_token: Some(tokens.refresh),
            user: UserResponseGql::from(user),
            requires_2fa: false,
            challenge: None,
        })
    }

    /// Exchange a refresh token for a fresh access JWT and a rotated
    /// refresh token. The submitted refresh token is revoked on success.
    async fn refresh_token(
        &self,
        ctx: &Context<'_>,
        refresh_token: String,
    ) -> Result<AuthPayload> {
        let state = ctx.data::<AppState>()?;
        let (tokens, user) = state.services.user.refresh_session(refresh_token).await?;
        Ok(AuthPayload {
            token: tokens.access,
            refresh_token: Some(tokens.refresh),
            user: UserResponseGql::from(user),
            requires_2fa: false,
            challenge: None,
        })
    }

    async fn logout_user(&self, ctx: &Context<'_>, refresh_token: String) -> Result<bool> {
        let _claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state.services.user.logout(refresh_token).await?;
        Ok(true)
    }

    async fn update_profile(
        &self,
        ctx: &Context<'_>,
        input: UpdateProfileInput,
    ) -> Result<ProfileResponseGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let profile = state
            .services
            .profile_repo
            .update_profile(
                &claims.sub,
                UpdateProfileDto {
                    first_name: input.first_name,
                    last_name: input.last_name,
                    display_name: input.display_name,
                    avatar_url: input.avatar_url,
                    bio: input.bio,
                    language: input.language,
                    country: input.country,
                    timezone: input.timezone,
                    phone: input.phone,
                    website: input.website,
                },
            )
            .await?
            .ok_or("Profile not found")?;

        Ok(ProfileResponse::from(profile).into())
    }

    async fn change_password(
        &self,
        ctx: &Context<'_>,
        old_password: String,
        new_password: String,
    ) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .user
            .change_password(&claims.sub, &old_password, &new_password)
            .await?;
        Ok(true)
    }

    async fn forgot_password(&self, ctx: &Context<'_>, email: String) -> Result<bool> {
        let state = ctx.data::<AppState>()?;
        state.services.user.forgot_password(email).await?;
        Ok(true)
    }

    async fn reset_password_with_token(
        &self,
        ctx: &Context<'_>,
        token: String,
        new_password: String,
    ) -> Result<bool> {
        let state = ctx.data::<AppState>()?;
        state
            .services
            .user
            .reset_password(token, new_password)
            .await?;
        Ok(true)
    }

    /// Issue a fresh email-verification token and mail the verification URL
    /// to the authenticated user. Always succeeds (returns `true`) even if
    /// the user is already verified, to keep the resolver idempotent.
    async fn request_email_verification(&self, ctx: &Context<'_>) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .user
            .request_email_verification(&claims.sub)
            .await?;
        Ok(true)
    }

    /// Consume an email-verification token. Returns `true` on success;
    /// `false` if the token is unknown or expired.
    async fn verify_email(&self, ctx: &Context<'_>, token: String) -> Result<bool> {
        let state = ctx.data::<AppState>()?;
        Ok(state.services.user.verify_email(token).await?)
    }

    async fn delete_me(&self, ctx: &Context<'_>) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state.services.user.deactivate_user(&claims.sub).await?;
        Ok(true)
    }

    async fn record_reading_session(
        &self,
        ctx: &Context<'_>,
        input: RecordReadingSessionInput,
    ) -> Result<ReadingSessionGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let dto = RecordReadingSessionDto {
            book_slug: input.book_slug,
            chapter_slug: input.chapter_slug,
            started_at: input.started_at,
            ended_at: input.ended_at,
            duration_mins: input.duration_mins,
            page_start: input.page_start,
            page_end: input.page_end,
            device: input.device,
        };
        let session = state
            .services
            .user
            .record_reading_session(&claims.sub, dto)
            .await?;
        Ok(session.into())
    }

    /// Begin 2FA setup. Persists the encrypted secret in `pending` state
    /// (login still bypasses 2FA until `verify2fa` runs).
    async fn setup2fa(&self, ctx: &Context<'_>) -> Result<Setup2faPayload> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        let r = state.services.user.setup_2fa(&claims.sub).await?;
        Ok(Setup2faPayload {
            secret: r.secret,
            otpauth_url: r.otpauth_url,
            recovery_codes: r.recovery_codes,
        })
    }

    /// Confirm setup with the first valid TOTP code. Pass the recovery
    /// codes the client received from `setup2fa` back so they can be
    /// persisted as Argon2id hashes — that's the only moment the server
    /// sees them in plaintext.
    async fn verify2fa(
        &self,
        ctx: &Context<'_>,
        code: String,
        recovery_codes: Vec<String>,
    ) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state
            .services
            .user
            .verify_2fa(&claims.sub, &code, &recovery_codes)
            .await?;
        Ok(true)
    }

    /// Disable 2FA. Requires a current TOTP code or a recovery code so a
    /// stolen session token can't silently downgrade the account.
    async fn disable2fa(&self, ctx: &Context<'_>, code: String) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state.services.user.disable_2fa(&claims.sub, &code).await?;
        Ok(true)
    }
}
