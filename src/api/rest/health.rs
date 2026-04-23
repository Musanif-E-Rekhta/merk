use crate::error::{Error, ErrorResponse};
use crate::state::AppState;
use aide::axum::ApiRouter;
use aide::axum::routing::get_with;
use aide::transform::TransformOperation;
use axum::Json;
use axum::extract::State;
use chrono::Utc;
use schemars::JsonSchema;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize, JsonSchema)]
pub struct ComponentStatus {
    status: String,
    latency_ms: Option<u64>,
    error: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct HealthComponents {
    db: ComponentStatus,
}

#[derive(Serialize, JsonSchema)]
pub struct HealthResponse {
    status: String,
    version: String,
    timestamp: String,
    components: HealthComponents,
}

pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_status = check_db(&state).await;
    let overall = if db_status.status == "ok" { "ok" } else { "degraded" };

    Json(HealthResponse {
        status: overall.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now().to_rfc3339(),
        components: HealthComponents { db: db_status },
    })
}

async fn check_db(state: &AppState) -> ComponentStatus {
    let start = Instant::now();
    match state.db.query("INFO FOR DB;").await {
        Ok(_) => ComponentStatus {
            status: "ok".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(e) => ComponentStatus {
            status: "degraded".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: Some(e.to_string()),
        },
    }
}

pub fn health_check_doc(op: TransformOperation) -> TransformOperation {
    op.description("Health check endpoint — returns service status, version, and component health")
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

pub fn routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route("/health", get_with(health_check, health_check_doc))
        .api_route("/error", get_with(error_example, error_example_doc))
        .with_state(state)
}
