use crate::api::middleware::Claims;
use crate::db::user_repo::UserResponse;
use crate::error::Error;
use crate::state::AppState;
use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with, put_with},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Build the `/api/v1/auth` router with all authentication endpoints.
pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/register",
            post_with(register, |op| op.description("Register new user")),
        )
        .api_route(
            "/login",
            post_with(login, |op| op.description("Login user")),
        )
        .api_route(
            "/logout",
            post_with(logout, |op| op.description("Logout user")),
        )
        .api_route(
            "/forgot-password",
            post_with(forgot_password, |op| {
                op.description("Request a password reset email")
            }),
        )
        .api_route(
            "/reset-password",
            post_with(reset_password_with_token, |op| {
                op.description("Reset password using emailed token")
            }),
        )
        .api_route(
            "/{id}/deactivate",
            put_with(deactivate, |op| op.description("Deactivate user")),
        )
        .api_route(
            "/me",
            get_with(me, |op| op.description("Get current user profile")),
        )
        .with_state(state)
}

/// Request body for `POST /register`.
#[derive(Deserialize, JsonSchema, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
}

/// Response body returned on successful register or login.
#[derive(Serialize, JsonSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, Error> {
    payload.validate()?;

    let (token, user) = state
        .services
        .user
        .register(payload.username, payload.email, payload.password)
        .await?;

    Ok(Json(AuthResponse { token, user }))
}

/// Request body for `POST /login`.
#[derive(Deserialize, JsonSchema, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, Error> {
    payload.validate()?;

    let (token, user) = state
        .services
        .user
        .login(payload.email, payload.password)
        .await?;

    Ok(Json(AuthResponse { token, user }))
}

async fn logout(_claims: Claims, State(_state): State<AppState>) -> Result<StatusCode, Error> {
    Ok(StatusCode::NO_CONTENT)
}

/// Request body for `POST /auth/forgot-password`.
#[derive(Deserialize, JsonSchema, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    pub email: String,
}

async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<StatusCode, Error> {
    payload.validate()?;

    state.services.user.forgot_password(payload.email).await?;

    Ok(StatusCode::OK)
}

/// Request body for `POST /auth/reset-password`.
#[derive(Deserialize, JsonSchema, Validate)]
pub struct ResetPasswordRequest {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(length(min = 8))]
    pub new_password: String,
}

async fn reset_password_with_token(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<StatusCode, Error> {
    payload.validate()?;

    state
        .services
        .user
        .reset_password(payload.token, payload.new_password)
        .await?;

    Ok(StatusCode::OK)
}

async fn deactivate(
    claims: Claims,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, Error> {
    if claims.sub != id {
        return Err(Error::forbidden(
            "forbidden",
            "Not authorized to perform this action",
        ));
    }

    state.services.user.deactivate_user(&id).await?;

    Ok(StatusCode::OK)
}

async fn me(claims: Claims, State(state): State<AppState>) -> Result<Json<UserResponse>, Error> {
    let user = state.services.user.get_user_by_id(&claims.sub).await?;

    Ok(Json(user))
}
