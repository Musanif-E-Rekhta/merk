pub mod logging;
pub mod users;

use aide::axum::ApiRouter;
use async_graphql::extensions::{Logger, OpenTelemetry, Tracing};
use async_graphql::http::GraphiQLSource;
use async_graphql::{EmptySubscription, MergedObject, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::Extension;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use jsonwebtoken::{DecodingKey, Validation, decode};
use opentelemetry::global;

use crate::api::graphql::logging::GraphQLLogging;
use crate::api::graphql::users::{UserMutation, UserQuery};
use crate::services::auth::Claims;
use crate::state::AppState;

#[derive(MergedObject, Default)]
pub struct QueryRoot(UserQuery);

#[derive(MergedObject, Default)]
pub struct MutationRoot(UserMutation);

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn router(state: AppState) -> ApiRouter {
    let schema = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription,
    )
    .data(state.clone())
    .extension(Logger)
    .extension(Tracing)
    .extension(OpenTelemetry::new(global::tracer("merk-graphql")))
    .extension(GraphQLLogging)
    .finish();

    ApiRouter::new()
        .route(
            "/graphql",
            axum::routing::get(graphiql).post(graphql_handler),
        )
        .layer(Extension(schema))
        .layer(Extension(state))
}

async fn graphql_handler(
    Extension(schema): Extension<AppSchema>,
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut request = req.into_inner();

    if let Some(claims) = extract_claims(&headers, &state.config.jwt_secret) {
        request = request.data(claims);
    }

    schema.execute(request).await.into()
}

fn extract_claims(headers: &HeaderMap, secret: &str) -> Option<Claims> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?
        .strip_prefix("Bearer ")?;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .ok()
    .map(|data| data.claims)
}

async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/graphql").finish())
}
