use aide::operation::OperationOutput;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use schemars::JsonSchema;
use serde::Serialize;
use thiserror::Error;
use tracing::{error, warn};
use validator::{ValidationError, ValidationErrors};

#[derive(Debug, Error)]
pub enum Error {
    #[error("{message}")]
    BadRequest { code: &'static str, message: String },

    #[error("{message}")]
    Unauthorized { code: &'static str, message: String },

    #[error("{message}")]
    Forbidden { code: &'static str, message: String },

    #[error("{message}")]
    NotFound { code: &'static str, message: String },

    #[error("{message}")]
    Conflict { code: &'static str, message: String },

    #[error("{origin}: {message}")]
    Upstream {
        origin: &'static str,
        message: String,
    },

    #[error("{origin}: {message}")]
    Internal {
        origin: &'static str,
        message: String,
    },
}

impl Error {
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::BadRequest {
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            code: "unauthorized",
            message: message.into(),
        }
    }

    pub fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self::Forbidden {
            code,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            code: "not_found",
            message: message.into(),
        }
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::Conflict {
            code,
            message: message.into(),
        }
    }

    pub fn upstream(origin: &'static str, message: impl Into<String>) -> Self {
        Self::Upstream {
            origin,
            message: message.into(),
        }
    }

    pub fn internal(origin: &'static str, message: impl Into<String>) -> Self {
        Self::Internal {
            origin,
            message: message.into(),
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

crate::from_as_internal! {
    surrealdb::Error => "database",
    std::io::Error   => "io",
    envy::Error      => "envy",
    rcgen::Error     => "rcgen",
}

crate::from_as_bad_request! {
    JsonRejection            => "invalid_json",
    serde_json::error::Error => "invalid_json_syntax",
    ValidationError          => "validation_error",
    ValidationErrors         => "validation_error",
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match &self {
            Error::Internal { .. } | Error::Upstream { .. } => error!(?self),
            _ => warn!(?self),
        }
        let status = self.status_code();
        let code = self.client_code().to_owned();
        let message = self.message().to_owned();
        (status, Json(ErrorResponse::new(message, Some(code)))).into_response()
    }
}

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
