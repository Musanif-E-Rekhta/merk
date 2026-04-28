use std::borrow::Cow;

use aide::operation::OperationOutput;
use anyhow;
use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use serde::Serialize;
use thiserror::Error;
use tracing::{error, warn};
use validator::{ValidationError, ValidationErrors};

/// Application error taxonomy.
///
/// Variants map 1-to-1 to HTTP status codes via [`Error::status_code`]. `Internal` and `Upstream`
/// are logged at `error` level and expose only an opaque message to clients; all other variants
/// are logged at `warn` level and forward their `message` field to the response body.
#[derive(Debug, Error)]
pub enum Error {
    /// 400 — malformed input (validation failures, bad JSON, etc.).
    #[error("{message}")]
    BadRequest { code: Cow<'static, str>, message: String },

    /// 401 — missing or invalid credentials / token.
    #[error("{message}")]
    Unauthorized { code: Cow<'static, str>, message: String },

    /// 403 — authenticated but not permitted.
    #[error("{message}")]
    Forbidden { code: Cow<'static, str>, message: String },

    /// 404 — requested resource does not exist.
    #[error("{message}")]
    NotFound { code: Cow<'static, str>, message: String },

    /// 409 — resource state conflict (e.g. duplicate username/email).
    #[error("{message}")]
    Conflict { code: Cow<'static, str>, message: String },

    /// 502 — a dependency (e.g. SurrealDB) returned an unexpected error.
    #[error("{origin}: {message}")]
    Upstream {
        origin: Cow<'static, str>,
        message: String,
        #[source]
        source: Option<anyhow::Error>,
    },

    /// 500 — an unrecoverable internal error (logged server-side, never leaked to clients).
    #[error("{origin}: {message}")]
    Internal {
        origin: Cow<'static, str>,
        message: String,
        #[source]
        source: Option<anyhow::Error>,
    },
}

impl Error {
    pub fn bad_request(code: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self::BadRequest {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            code: Cow::Borrowed("unauthorized"),
            message: message.into(),
        }
    }

    pub fn wrong_credentials() -> Self {
        Self::Unauthorized {
            code: Cow::Borrowed("wrong_credentials"),
            message: "Invalid email or password".into(),
        }
    }

    pub fn invalid_token() -> Self {
        Self::Unauthorized {
            code: Cow::Borrowed("invalid_token"),
            message: "Invalid or expired token".into(),
        }
    }

    pub fn forbidden(code: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self::Forbidden {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            code: Cow::Borrowed("not_found"),
            message: message.into(),
        }
    }

    pub fn conflict(code: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self::Conflict {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn upstream(origin: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self::Upstream {
            origin: origin.into(),
            message: message.into(),
            source: None,
        }
    }

    pub fn internal(origin: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self::Internal {
            origin: origin.into(),
            message: message.into(),
            source: None,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Error::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Error::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Error::Forbidden { .. } => StatusCode::FORBIDDEN,
            Error::NotFound { .. } => StatusCode::NOT_FOUND,
            Error::Conflict { .. } => StatusCode::CONFLICT,
            Error::Upstream { .. } => StatusCode::BAD_GATEWAY,
            Error::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn client_code(&self) -> &str {
        match self {
            Error::BadRequest { code, .. }
            | Error::Unauthorized { code, .. }
            | Error::Forbidden { code, .. }
            | Error::NotFound { code, .. }
            | Error::Conflict { code, .. } => code,
            Error::Upstream { .. } => "bad_gateway",
            Error::Internal { .. } => "internal_error",
        }
    }

    fn message(&self) -> &str {
        match self {
            Error::BadRequest { message, .. }
            | Error::Unauthorized { message, .. }
            | Error::Forbidden { message, .. }
            | Error::NotFound { message, .. }
            | Error::Conflict { message, .. }
            | Error::Upstream { message, .. }
            | Error::Internal { message, .. } => message,
        }
    }
}

crate::from_as_error! {
    surrealdb::Error         => internal("database") + src,
    std::io::Error           => internal("io") + src,
    std::str::Utf8Error      => internal("utf8") + src,
    envy::Error              => internal("envy") + src,
    rcgen::Error             => internal("rcgen") + src,
    JsonRejection            => bad_request("invalid_json"),
    serde_json::error::Error => bad_request("invalid_json_syntax"),
    ValidationError          => bad_request("validation_error"),
    ValidationErrors         => bad_request("validation_error"),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match &self {
            Error::Internal { source, .. } | Error::Upstream { source, .. } => match source {
                Some(src) => error!(error = ?self, source_chain = %format!("{src:#}")),
                None => error!(?self),
            },
            _ => warn!(?self),
        }

        let status = self.status_code();
        let code = self.client_code().to_owned();
        let message = self.message().to_owned();

        (status, Json(ErrorResponse::new(message, Some(code)))).into_response()
    }
}

/// JSON body returned for all error responses: `{ "error": "...", "code": "..." }`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ErrorResponse {
    error: String,
    code: Option<String>,
}

impl ErrorResponse {
    pub fn new(error: String, code: Option<String>) -> Self {
        Self { error, code }
    }
}

impl OperationOutput for Error {
    type Inner = Error;
}
