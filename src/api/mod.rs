pub mod graphql;
pub mod middleware;
pub mod openapi;
#[cfg(feature = "embed-frontend")]
pub mod spa;
pub mod v1;

use crate::state::AppState;
use aide::axum::ApiRouter;
use axum::Extension;
use axum::http::HeaderValue;
use axum::middleware::from_fn;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> axum::Router {
    let mut api = openapi::setup();

    // CORS_ORIGINS defaults to empty, which opens the gate to any origin
    // — appropriate for local dev where the Dioxus build server picks an
    // arbitrary port. In production set it to a comma-separated list:
    // `CORS_ORIGINS=https://musanif.app,https://admin.musanif.app`.
    let allowed = state.config.parsed_cors_origins();
    let cors_builder = CorsLayer::new().allow_methods(Any).allow_headers(Any);
    let cors = if allowed.is_empty() {
        cors_builder.allow_origin(Any)
    } else {
        let origins: Vec<HeaderValue> = allowed
            .iter()
            .filter_map(|o| HeaderValue::from_str(o).ok())
            .collect();
        cors_builder.allow_origin(AllowOrigin::list(origins))
    };

    #[cfg_attr(not(feature = "embed-frontend"), allow(unused_mut))]
    let mut router = ApiRouter::new()
        .nest_api_service("/api/v1", v1::router(state.clone()))
        .nest_api_service("/docs", openapi::router())
        .merge(graphql::router(state.clone()))
        .merge(merk_observability::metrics_router());

    #[cfg(feature = "embed-frontend")]
    {
        router = router.merge(spa::routes());
    }

    router
        .finish_api_with(&mut api, |api| api.default_response::<String>())
        .layer(from_fn(merk_observability::track_metrics))
        .layer(Extension(Arc::new(api)))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
