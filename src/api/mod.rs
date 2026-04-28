pub mod graphql;
pub mod metrics;
pub mod middleware;
pub mod openapi;
pub mod v1;

use crate::state::AppState;
use aide::axum::ApiRouter;
use axum::Extension;
use axum::middleware::from_fn;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> axum::Router {
    let mut api = openapi::setup();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    ApiRouter::new()
        .nest_api_service("/api/v1", v1::router(state.clone()))
        .nest_api_service("/docs", openapi::router())
        .merge(graphql::router(state.clone()))
        .merge(metrics::router())
        .finish_api_with(&mut api, |api| api.default_response::<String>())
        .layer(from_fn(middleware::metrics::track_metrics))
        .layer(Extension(Arc::new(api)))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
