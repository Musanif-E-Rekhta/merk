use crate::api::middleware::Claims;
use crate::db::record_id_key_to_string;
use crate::db::user_repo::{CreateUserDto, UserRepo, UserResponse};
use crate::error::Error;
use crate::services::auth::{generate_jwt, verify_password};
use crate::state::AppState;
use aide::axum::{
    routing::{get_with, post_with, put_with},
    ApiRouter,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

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
            "/reset-password",
            post_with(reset_password, |op| op.description("Reset user password")),
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

#[derive(Deserialize, JsonSchema, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
}

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

    let dto = CreateUserDto {
        username: payload.username,
        email: payload.email,
        raw_password: payload.password,
    };

    let user = UserRepo::create_user(&state.db, dto).await?;

    let id_str = user
        .id
        .as_ref()
        .ok_or_else(|| Error::internal("auth", "User ID missing after creation"))
        .map(|r| record_id_key_to_string(&r.key))?;
    let token = generate_jwt(&id_str, user.is_active, &state.config)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

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

    let user = UserRepo::get_user_by_email(&state.db, &payload.email)
        .await
        .map_err(|_| Error::wrong_credentials())?
        .ok_or_else(|| Error::wrong_credentials())?;

    if !verify_password(&payload.password, &user.password_hash) {
        return Err(Error::wrong_credentials());
    }

    if !user.is_active {
        return Err(Error::forbidden("banned_user", "Account is suspended"));
    }

    let id_str = user
        .id
        .as_ref()
        .ok_or_else(|| Error::internal("auth", "User ID missing"))
        .map(|r| record_id_key_to_string(&r.key))?;

    let _ = UserRepo::update_last_login(&state.db, &id_str).await;

    let token = generate_jwt(&id_str, user.is_active, &state.config)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

async fn logout(_claims: Claims, State(_state): State<AppState>) -> Result<StatusCode, Error> {
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, JsonSchema, Validate)]
pub struct ResetPasswordRequest {
    #[validate(length(min = 1))]
    pub old_password: String,
    #[validate(length(min = 8))]
    pub new_password: String,
}

async fn reset_password(
    claims: Claims,
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<StatusCode, Error> {
    payload.validate()?;

    let user = UserRepo::get_user_by_id(&state.db, &claims.sub)
        .await
        .map_err(|_| Error::wrong_credentials())?
        .ok_or_else(|| Error::wrong_credentials())?;

    if !verify_password(&payload.old_password, &user.password_hash) {
        return Err(Error::wrong_credentials());
    }

    UserRepo::reset_password(&state.db, &claims.sub, &payload.new_password).await?;

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

    UserRepo::deactivate_user(&state.db, &id).await?;

    Ok(StatusCode::OK)
}

async fn me(claims: Claims, State(state): State<AppState>) -> Result<Json<UserResponse>, Error> {
    let user = UserRepo::get_user_by_id(&state.db, &claims.sub)
        .await
        .map_err(|_| Error::invalid_token())?
        .ok_or_else(|| Error::invalid_token())?;

    Ok(Json(user.into()))
}
