use crate::error::{Error, ErrorResponse};
use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
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

pub fn health_check_doc(op: TransformOperation) -> TransformOperation {
    op.description("Health check endpoint")
        .tag("Utility")
        .response::<200, Json<HealthResponse>>()
}

pub async fn error_example() -> Result<Json<()>, Error> {
    Err(Error::bad_request(
        "example_error",
        "This is an example error response!",
    ))
}

pub fn error_example_doc(op: TransformOperation) -> TransformOperation {
    op.description("Returns an example structured error")
        .tag("Utility")
        .response::<200, Json<()>>()
        .response_with::<400, Json<ErrorResponse>, _>(|res| {
            res.description("Bad request error structure example")
        })
}

pub fn api_routes() -> ApiRouter<()> {
    ApiRouter::new()
        .api_route(
            "/health",
            get_with(health_check, health_check_doc),
        )
        .api_route(
            "/error",
            get_with(error_example, error_example_doc),
        )
}
