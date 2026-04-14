pub mod graphql;
pub mod openapi;
pub mod rest;

use crate::state::AppState;
use aide::axum::ApiRouter;
use axum::Extension;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> axum::Router {
    let mut api = openapi::setup_aide();
    let schema = graphql::build_schema();

    ApiRouter::new()
        .nest_api_service("/api/v1", rest::api_routes())
        .nest_api_service("/docs", openapi::docs_routes())
        .route(
            "/graphql",
            axum::routing::get(graphql::graphiql).post(graphql::graphql_handler),
        )
        .finish_api_with(&mut api, |api| api.default_response::<String>())
        .layer(Extension(Arc::new(api)))
        .layer(Extension(schema))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
