use async_graphql::{Context, Object, Result, SimpleObject};

use crate::api::middleware::Claims;
use crate::db::record_id_key_to_string;
use crate::db::user_repo::{CreateUserDto, UserRepo, UserResponse};
use crate::services::auth::{generate_jwt, verify_password};
use crate::state::AppState;

#[derive(Default)]
pub struct UserQuery;

#[Object]
impl UserQuery {
    async fn me(&self, ctx: &Context<'_>) -> Result<UserResponseGql> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let user = UserRepo::get_user_by_id(&state.db, &claims.sub)
            .await?
            .ok_or("User not found")?;

        Ok(UserResponse::from(user).into())
    }
}

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

        let dto = CreateUserDto {
            username,
            email,
            raw_password: password,
        };

        let user = UserRepo::create_user(&state.db, dto).await?;

        let id_str = user
            .id
            .as_ref()
            .ok_or("User ID missing from database")
            .map(|r| record_id_key_to_string(&r.key))?;
        let token = generate_jwt(&id_str, user.is_active, &state.config)
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(AuthPayload {
            token,
            user: UserResponse::from(user).into(),
        })
    }

    async fn login_user(
        &self,
        ctx: &Context<'_>,
        email: String,
        password: String,
    ) -> Result<AuthPayload> {
        let state = ctx.data::<AppState>()?;

        let user = UserRepo::get_user_by_email(&state.db, &email)
            .await?
            .ok_or("Invalid credentials")?;

        if !verify_password(&password, &user.password_hash) {
            return Err("Invalid credentials".into());
        }
        if !user.is_active {
            return Err("Banned User".into());
        }

        let id_str = user
            .id
            .as_ref()
            .ok_or("User ID missing from database")
            .map(|r| record_id_key_to_string(&r.key))?;
        let _ = UserRepo::update_last_login(&state.db, &id_str).await;

        let token = generate_jwt(&id_str, user.is_active, &state.config)
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(AuthPayload {
            token,
            user: UserResponse::from(user).into(),
        })
    }

    async fn logout_user(&self, ctx: &Context<'_>) -> Result<bool> {
        let _claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        Ok(true)
    }

    async fn reset_password(
        &self,
        ctx: &Context<'_>,
        old_password: String,
        new_password: String,
    ) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        let user = UserRepo::get_user_by_id(&state.db, &claims.sub)
            .await?
            .ok_or("Invalid credentials")?;

        if !verify_password(&old_password, &user.password_hash) {
            return Err("Invalid credentials".into());
        }

        UserRepo::reset_password(&state.db, &claims.sub, &new_password).await?;
        Ok(true)
    }

    async fn deactivate_user(&self, ctx: &Context<'_>, user_id: String) -> Result<bool> {
        let claims = ctx.data_opt::<Claims>().ok_or("Unauthorized")?;
        let state = ctx.data::<AppState>()?;

        if claims.sub != user_id {
            return Err("Forbidden".into());
        }

        UserRepo::deactivate_user(&state.db, &user_id).await?;
        Ok(true)
    }
}
