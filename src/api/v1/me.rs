use crate::api::middleware::Claims;
use crate::db::book_repo::AuthorResponse;
use crate::db::profile_repo::{ProfileResponse, UpdateProfileDto};
use crate::db::user_repo::{ReadingSessionResponse, UserResponse, UserStats};
use crate::error::Error;
use crate::state::AppState;
use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, put_with},
};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/me",
            get_with(get_me, |op| op.description("Get current user with profile")),
        )
        .api_route(
            "/me/profile",
            put_with(update_profile, |op| op.description("Update profile")),
        )
        .api_route(
            "/me/password",
            put_with(change_password, |op| op.description("Change password")),
        )
        .api_route(
            "/me",
            delete_with(delete_me, |op| op.description("Deactivate account")),
        )
        .api_route(
            "/me/stats",
            get_with(get_stats, |op| op.description("Get reading statistics")),
        )
        .api_route(
            "/me/reading-sessions",
            get_with(get_reading_sessions, |op| {
                op.description("Get reading session history")
            }),
        )
        .api_route(
            "/me/following",
            get_with(get_following, |op| op.description("Get followed authors")),
        )
        .with_state(state)
}

#[derive(Serialize, JsonSchema)]
pub struct MeResponse {
    #[serde(flatten)]
    pub user: UserResponse,
    pub profile: Option<ProfileResponse>,
}

async fn get_me(claims: Claims, State(state): State<AppState>) -> Result<Json<MeResponse>, Error> {
    let (user, profile) = tokio::try_join!(
        state.services.user.get_user_by_id(&claims.sub),
        state
            .services
            .profile_repo
            .get_profile_by_user_id(&claims.sub),
    )?;

    Ok(Json(MeResponse {
        user,
        profile: profile.map(Into::into),
    }))
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct UpdateProfileRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    #[validate(length(min = 2, max = 10))]
    pub language: Option<String>,
    #[validate(length(min = 2, max = 10))]
    pub country: Option<String>,
    pub timezone: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
}

async fn update_profile(
    claims: Claims,
    State(state): State<AppState>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, Error> {
    payload.validate()?;

    let profile = state
        .services
        .profile_repo
        .update_profile(
            &claims.sub,
            UpdateProfileDto {
                first_name: payload.first_name,
                last_name: payload.last_name,
                display_name: payload.display_name,
                avatar_url: payload.avatar_url,
                bio: payload.bio,
                language: payload.language,
                country: payload.country,
                timezone: payload.timezone,
                phone: payload.phone,
                website: payload.website,
            },
        )
        .await?
        .ok_or_else(|| Error::not_found("Profile not found"))?;

    Ok(Json(profile.into()))
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1))]
    pub old_password: String,
    #[validate(length(min = 8))]
    pub new_password: String,
}

async fn change_password(
    claims: Claims,
    State(state): State<AppState>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<StatusCode, Error> {
    payload.validate()?;

    state
        .services
        .user
        .change_password(&claims.sub, &payload.old_password, &payload.new_password)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_me(claims: Claims, State(state): State<AppState>) -> Result<StatusCode, Error> {
    state.services.user.deactivate_user(&claims.sub).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_stats(
    claims: Claims,
    State(state): State<AppState>,
) -> Result<Json<UserStats>, Error> {
    let stats = state.services.user.get_user_stats(&claims.sub).await?;
    Ok(Json(stats))
}

#[derive(Deserialize, JsonSchema)]
pub struct PaginationQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

async fn get_reading_sessions(
    claims: Claims,
    State(state): State<AppState>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<Vec<ReadingSessionResponse>>, Error> {
    let sessions = state
        .services
        .user
        .get_reading_sessions(&claims.sub, q.limit.unwrap_or(20), q.offset.unwrap_or(0))
        .await?;
    Ok(Json(sessions))
}

async fn get_following(
    claims: Claims,
    State(state): State<AppState>,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<Vec<AuthorResponse>>, Error> {
    let authors = state
        .services
        .user
        .get_following_authors(&claims.sub, q.limit.unwrap_or(20), q.offset.unwrap_or(0))
        .await?;
    Ok(Json(authors))
}
