use async_graphql::Response;
use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextExecute};
use async_trait::async_trait;
use std::sync::Arc;

use crate::services::auth::Claims;

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
        let operation = operation_name.unwrap_or("anonymous");
        let authenticated = ctx.data_opt::<Claims>().is_some();

        tracing::info!(operation, authenticated, "graphql request");

        let response = next.run(ctx, operation_name).await;

        if !response.errors.is_empty() {
            let errors: Vec<_> = response.errors.iter().map(|e| e.message.as_str()).collect();
            tracing::warn!(operation, ?errors, "graphql request failed");
        }

        response
    }
}
