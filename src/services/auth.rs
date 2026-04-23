use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::error::Error;

/// Hash a plaintext password with Argon2id using a random salt.
pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string()
}

/// Verify a plaintext password against an Argon2id hash. Returns `false` on any error.
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

/// JWT claims embedded in every issued token.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User record ID (SurrealDB key).
    pub sub: String,
    /// Unix timestamp of token expiry (24 h from issuance).
    pub exp: usize,
    /// Account state at issuance time — always `"active"` for valid tokens.
    pub state: String,
}

/// Sign and return an HS256 JWT for `user_id`, valid for 24 hours.
///
/// Returns `Err(Forbidden)` when `is_active` is `false` so suspended users cannot receive tokens.
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
