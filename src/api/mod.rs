pub mod graphql;
pub mod middleware;
pub mod openapi;
pub mod v1;

use crate::state::AppState;
use aide::axum::ApiRouter;
use axum::Extension;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> axum::Router {
    let metrics = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    let mut api = openapi::setup();

    ApiRouter::new()
        .nest_api_service("/api/v1", v1::router(state.clone()))
        .nest_api_service("/docs", openapi::router())
        .merge(graphql::router())
        .merge(openapi::metrics_router(metrics))
        .finish_api_with(&mut api, |api| api.default_response::<String>())
        .layer(Extension(Arc::new(api)))
        .layer(TraceLayer::new_for_http())
}
