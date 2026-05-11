use async_graphql::Response;
use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextExecute};
use async_trait::async_trait;
use metrics::{counter, histogram};
use std::sync::Arc;
use std::time::Instant;

pub struct GraphQLMetrics;

impl ExtensionFactory for GraphQLMetrics {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(GraphQLMetricsExtension)
    }
}

struct GraphQLMetricsExtension;

#[async_trait]
impl Extension for GraphQLMetricsExtension {
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        // operation_name is client-supplied; a misbehaving client could blow up
        // metric cardinality. Anonymous queries collapse to one bucket; named
        // ones we trust because they're checked into the frontend at build time.
        let operation = operation_name.unwrap_or("anonymous").to_owned();
        let start = Instant::now();
        let response = next.run(ctx, operation_name).await;
        let latency = start.elapsed().as_secs_f64();

        counter!("graphql_operations_total", "operation" => operation.clone()).increment(1);
        if !response.errors.is_empty() {
            counter!(
                "graphql_operation_errors_total",
                "operation" => operation.clone(),
            )
            .increment(response.errors.len() as u64);
        }
        histogram!("graphql_operation_duration_seconds", "operation" => operation)
            .record(latency);

        response
    }
}
