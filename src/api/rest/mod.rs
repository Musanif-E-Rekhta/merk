use crate::error::Error;
use crate::state::AppState;
use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use axum::{extract::State, Json};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Serialize, JsonSchema)]
pub struct HealthResponse {
    status: String,
}

pub async fn health_check(State(_state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

pub async fn error_example() -> Result<Json<()>, Error> {
    Err(Error::bad_request("example_error", "This is an example error response!"))
}

pub fn api_routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route(
            "/",
            get_with(health_check, |op| op.description("Health check endpoint")),
        )
        .api_route(
            "/error",
            get_with(error_example, |op| op.description("Returns an example structured error")),
        )
}
