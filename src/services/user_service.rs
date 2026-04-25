use crate::db::book_repo::AuthorResponse;
use crate::db::record_id_key_to_string;
use crate::db::user_repo::{
    CreateUserDto, ReadingSessionResponse, UserRepo, UserResponse, UserStats,
};
use crate::error::Error;
use crate::services::auth::{generate_jwt, generate_reset_token, verify_password};
use chrono::Utc;
use std::sync::Arc;

pub struct UserService {
    repo: UserRepo,
    config: Arc<crate::config::AppConfig>,
}

impl UserService {
    pub fn new(repo: UserRepo, config: Arc<crate::config::AppConfig>) -> Self {
        Self { repo, config }
    }

    pub async fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<(String, UserResponse), Error> {
        let dto = CreateUserDto {
            username,
            email,
            raw_password: password,
        };

        let user = self.repo.create_user(dto).await?;

        let id_str = user
            .id
            .as_ref()
            .ok_or_else(|| Error::internal("auth", "User ID missing after creation"))
            .map(|r| record_id_key_to_string(&r.key))?;

        let token = generate_jwt(&id_str, user.is_active, &self.config)?;

        Ok((token, user.into()))
    }

    pub async fn login(
        &self,
        email: String,
        password: String,
    ) -> Result<(String, UserResponse), Error> {
        let user = self
            .repo
            .get_user_by_email(&email)
            .await
            .map_err(|_| Error::wrong_credentials())?
            .ok_or_else(Error::wrong_credentials)?;

        if !verify_password(&password, &user.password_hash) {
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

        let _ = self.repo.update_last_login(&id_str).await;

        let token = generate_jwt(&id_str, user.is_active, &self.config)?;

        Ok((token, user.into()))
    }

    pub async fn forgot_password(&self, email: String) -> Result<(), Error> {
        let expires_at = Utc::now() + chrono::Duration::hours(1);
        let token = generate_reset_token();

        // Best-effort: always return success to avoid leaking whether the email exists.
        let _ = self.repo.set_reset_token(&email, &token, expires_at).await;

        // TODO: send email with token
        Ok(())
    }

    pub async fn reset_password(&self, token: String, new_password: String) -> Result<(), Error> {
        let user = self
            .repo
            .get_user_by_reset_token(&token)
            .await?
            .ok_or_else(|| Error::bad_request("invalid_token", "Invalid or expired reset token"))?;

        let id_str = user
            .id
            .as_ref()
            .map(|r| record_id_key_to_string(&r.key))
            .ok_or_else(|| Error::internal("auth", "User ID missing"))?;

        self.repo
            .reset_password_and_clear_token(&id_str, &new_password)
            .await
    }

    pub async fn change_password(
        &self,
        user_id: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), Error> {
        let user = self
            .repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(Error::wrong_credentials)?;

        if !verify_password(old_password, &user.password_hash) {
            return Err(Error::wrong_credentials());
        }

        self.repo.reset_password(user_id, new_password).await
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<UserResponse, Error> {
        let user = self
            .repo
            .get_user_by_id(id)
            .await?
            .ok_or_else(|| Error::not_found("User not found"))?;
        Ok(user.into())
    }

    pub async fn deactivate_user(&self, id: &str) -> Result<(), Error> {
        self.repo.deactivate_user(id).await
    }

    pub async fn get_user_stats(&self, user_id: &str) -> Result<UserStats, Error> {
        self.repo.get_user_stats(user_id).await
    }

    pub async fn get_reading_sessions(
        &self,
        user_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ReadingSessionResponse>, Error> {
        self.repo.get_reading_sessions(user_id, limit, offset).await
    }

    pub async fn get_following_authors(
        &self,
        user_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuthorResponse>, Error> {
        self.repo
            .get_following_authors(user_id, limit, offset)
            .await
    }
}
