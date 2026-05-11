use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::error::Error;
use crate::state::AppState;

pub use merk_auth::Claims;

/// Axum extractor that validates the `Authorization: Bearer <token>` header
/// and returns the decoded [`Claims`]. Rejects with `401` when the header
/// is missing or the token is invalid, and `403` when the embedded `state`
/// field is not `"active"`.
impl FromRequestParts<AppState> for Claims {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| Error::unauthorized("Invalid token"))?;

        let claims = merk_auth::decode_jwt(token, &state.config.jwt_secret)
            .map_err(|_| Error::unauthorized("Invalid token"))?;

        if claims.state != "active" {
            return Err(Error::forbidden("banned_user", "User suspended"));
        }
        Ok(claims)
    }
}
