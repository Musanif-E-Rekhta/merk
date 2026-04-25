use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::api::middleware::Claims;
use crate::db::book_repo::AuthorResponse;
use crate::db::profile_repo::{ProfileResponse, UpdateProfileDto};
use crate::db::user_repo::{ReadingSessionResponse, UserResponse, UserStats};
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

        Ok(MeResponseGql {
            id: ur.id,
            username: ur.username,
            email: ur.email,
            is_active: ur.is_active,
            is_verified: ur.is_verified,
            profile: profile.map(|p| ProfileResponse::from(p).into()),
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
}

// ── Output types ─────────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct AuthPayload {
    pub token: String,
    pub user: UserResponseGql,
}

#[derive(SimpleObject)]
pub struct UserResponseGql {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    pub is_verified: bool,
}

impl From<UserResponse> for UserResponseGql {
    fn from(r: UserResponse) -> Self {
        UserResponseGql {
            id: r.id,
            username: r.username,
            email: r.email,
            is_active: r.is_active,
            is_verified: r.is_verified,
        }
    }
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

        let (token, user) = state
            .services
            .user
            .register(username, email, password)
            .await?;

        Ok(AuthPayload {
            token,
            user: UserResponseGql::from(user),
        })
    }

    async fn login_user(
        &self,
        ctx: &Context<'_>,
        email: String,
        password: String,
    ) -> Result<AuthPayload> {
        let state = ctx.data::<AppState>()?;

        let (token, user) = state.services.user.login(email, password).await?;

        Ok(AuthPayload {
            token,
            user: UserResponseGql::from(user),
        })
    }

    async fn logout_user(&self, ctx: &Context<'_>) -> Result<bool> {
        let _claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
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

    async fn delete_me(&self, ctx: &Context<'_>) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;
        state.services.user.deactivate_user(&claims.sub).await?;
        Ok(true)
    }
}
