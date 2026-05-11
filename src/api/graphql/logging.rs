use async_graphql::Response;
use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextExecute};
use async_trait::async_trait;
use std::sync::Arc;

use merk_auth::Claims;

use crate::api::graphql::observability::{classify_error, resolve_operation_label};

pub struct GraphQLLogging;

impl ExtensionFactory for GraphQLLogging {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(GraphQLLoggingExtension)
    }
}

struct GraphQLLoggingExtension;

#[async_trait]
impl Extension for GraphQLLoggingExtension {
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let operation = resolve_operation_label(operation_name);
        let authenticated = ctx.data_opt::<Claims>().is_some();

        tracing::info!(operation, authenticated, "graphql request");

        let response = next.run(ctx, operation_name).await;

        if !response.errors.is_empty() {
            let errors: Vec<(&'static str, &str)> = response
                .errors
                .iter()
                .map(|e| (classify_error(e), e.message.as_str()))
                .collect();
            tracing::warn!(operation, ?errors, "graphql request failed");
        }

        response
    }
}
