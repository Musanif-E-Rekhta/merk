use crate::error::Error;
use aide::axum::ApiRouter;
use axum::routing::get;
use axum::Json;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Serialize, JsonSchema)]
pub struct HealthResponse {
    status: String,
}

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

pub async fn error_example() -> Result<Json<()>, Error> {
    Err(Error::bad_request(
        "example_error",
        "This is an example error response!",
    ))
}

pub fn api_routes() -> ApiRouter<()> {
    ApiRouter::new()
        .route("/health", get(health_check))
        .route("/error", get(error_example))
}
