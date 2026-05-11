use async_graphql::Response;
use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextExecute};
use async_trait::async_trait;
use opentelemetry::metrics::{Counter, Histogram};
use opentelemetry::{KeyValue, global};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use crate::api::graphql::observability::{
    classify_error, is_unknown_operation, resolve_operation_label,
};

pub struct GraphQLMetrics;

impl ExtensionFactory for GraphQLMetrics {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(GraphQLMetricsExtension)
    }
}

struct GraphQLMetricsExtension;

static OPS_TOTAL: LazyLock<Counter<u64>> = LazyLock::new(|| {
    global::meter("merk")
        .u64_counter("graphql_operations_total")
        .build()
});

static OP_ERRORS_TOTAL: LazyLock<Counter<u64>> = LazyLock::new(|| {
    global::meter("merk")
        .u64_counter("graphql_operation_errors_total")
        .build()
});

static OP_DURATION: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    global::meter("merk")
        .f64_histogram("graphql_operation_duration_seconds")
        .build()
});

static OPS_REJECTED_TOTAL: LazyLock<Counter<u64>> = LazyLock::new(|| {
    global::meter("merk")
        .u64_counter("graphql_operations_rejected_total")
        .build()
});

#[async_trait]
impl Extension for GraphQLMetricsExtension {
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        // Cardinality control: only labels from the build-time allowlist
        // reach metrics. Unknown names bump a side counter so we can see
        // probing without polluting the main series.
        if is_unknown_operation(operation_name) {
            OPS_REJECTED_TOTAL.add(1, &[KeyValue::new("reason", "unknown_name")]);
        }
        let operation = resolve_operation_label(operation_name);

        let start = Instant::now();
        let response = next.run(ctx, operation_name).await;
        let latency = start.elapsed().as_secs_f64();

        let attrs = [KeyValue::new("operation", operation)];
        OPS_TOTAL.add(1, &attrs);
        OP_DURATION.record(latency, &attrs);

        if !response.errors.is_empty() {
            // Collapse same-class errors so one bad query doesn't write N
            // points to the counter for the same (operation, class) tuple.
            let mut counts: HashMap<&'static str, u64> = HashMap::new();
            for err in &response.errors {
                *counts.entry(classify_error(err)).or_insert(0) += 1;
            }
            for (class, count) in counts {
                OP_ERRORS_TOTAL.add(
                    count,
                    &[
                        KeyValue::new("operation", operation),
                        KeyValue::new("error_class", class),
                    ],
                );
            }
        }

        response
    }
}
