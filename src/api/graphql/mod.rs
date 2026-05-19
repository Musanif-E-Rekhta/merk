pub mod admin;
pub mod admin_crawl;
pub mod books;
pub mod chapters;
pub mod collections;
pub mod comments;
pub mod highlights;
pub mod logging;
pub mod metrics;
pub mod observability;
pub mod resolver_metrics;
pub mod reviews;
pub mod subscription_metrics;
pub mod subscriptions;
pub mod translations;
pub mod users;

use aide::axum::ApiRouter;
use async_graphql::extensions::{Logger, OpenTelemetry, Tracing};
use async_graphql::http::GraphiQLSource;
use async_graphql::{MergedObject, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::Extension;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use opentelemetry::global;

use crate::api::graphql::admin::{AdminMutation, AdminQuery};
use crate::api::graphql::admin_crawl::{AdminCrawlMutation, AdminCrawlQuery};
use crate::api::graphql::books::{BookMutation, BookQuery};
use crate::api::graphql::chapters::{ChapterMutation, ChapterQuery};
use crate::api::graphql::collections::{CollectionMutation, CollectionQuery};
use crate::api::graphql::comments::{CommentMutation, CommentQuery};
use crate::api::graphql::highlights::{HighlightMutation, HighlightQuery};
use crate::api::graphql::logging::GraphQLLogging;
use crate::api::graphql::metrics::GraphQLMetrics;
use crate::api::graphql::resolver_metrics::GraphQLResolverMetrics;
use crate::api::graphql::reviews::{ReviewMutation, ReviewQuery};
use crate::api::graphql::subscription_metrics::GraphQLSubscriptionMetrics;
use crate::api::graphql::subscriptions::SubscriptionRoot;
use crate::api::graphql::translations::{TranslationMutation, TranslationQuery};
use crate::api::graphql::users::{UserMutation, UserQuery};
use crate::state::AppState;
use merk_auth::Claims;

#[derive(MergedObject, Default)]
pub struct QueryRoot(
    UserQuery,
    BookQuery,
    ChapterQuery,
    ReviewQuery,
    HighlightQuery,
    CommentQuery,
    TranslationQuery,
    CollectionQuery,
    AdminQuery,
    AdminCrawlQuery,
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    UserMutation,
    BookMutation,
    ChapterMutation,
    ReviewMutation,
    HighlightMutation,
    CommentMutation,
    TranslationMutation,
    CollectionMutation,
    AdminMutation,
    AdminCrawlMutation,
);

pub type AppSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the GraphQL schema in SDL form. Used by `merk dump-schema` to
/// produce `musanif-contracts/schema.graphql`.
pub fn schema_sdl() -> String {
    Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        SubscriptionRoot,
    )
    .finish()
    .sdl()
}

pub fn router(state: AppState) -> ApiRouter {
    let schema = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        SubscriptionRoot,
    )
    .data(state.clone())
    .extension(Logger)
    .extension(Tracing)
    .extension(OpenTelemetry::new(global::tracer("merk-graphql")))
    .extension(GraphQLLogging)
    .extension(GraphQLMetrics)
    .extension(GraphQLSubscriptionMetrics)
    .extension(GraphQLResolverMetrics)
    .finish();

    ApiRouter::new()
        .route(
            "/api/graphql",
            axum::routing::get(graphiql).post(graphql_handler),
        )
        // graphql-transport-ws over WebSocket. Same path; the upgrade is
        // negotiated via the `Sec-WebSocket-Protocol` header.
        .route_service("/api/graphql/ws", GraphQLSubscription::new(schema.clone()))
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
    merk_auth::decode_jwt(token, secret).ok()
}

async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/api/graphql").finish())
}
