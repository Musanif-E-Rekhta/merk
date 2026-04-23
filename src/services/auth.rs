use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::error::Error;

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string()
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(_) => return false,
    };
    let argon2 = Argon2::default();
    matches!(
        argon2.verify_password(password.as_bytes(), &parsed_hash),
        Ok(())
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub state: String,
}

pub fn generate_jwt(user_id: &str, is_active: bool, config: &AppConfig) -> Result<String, Error> {
    if !is_active {
        return Err(Error::forbidden("banned_user", "User suspended"));
    }

    let now = chrono::Utc::now();

    let expiration = now
        .checked_add_signed(chrono::Duration::days(1))
        .ok_or_else(|| Error::internal("jwt", "Token expiration calculation failed"))?
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
        state: "active".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|_| Error::internal("jwt", "Token creation failed"))
}
