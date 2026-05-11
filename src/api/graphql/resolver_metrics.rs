//! Per-field resolver latency for a hand-curated allowlist.
//!
//! Field labels are server-controlled (defined in our `#[Object]` impls)
//! so cardinality is bounded by the size of [`SLOW_RESOLVERS`]. Fields
//! not in the list short-circuit before any timing — the only cost on
//! the hot path is the linear scan, which stays cheap below ~16 entries.
//!
//! Pick fields suspected of N+1 or slow downstream work. The merk schema
//! is mostly flat (relationship traversal lives on `QueryRoot`), so the
//! best candidates today are nav fields on chapter pages and the few
//! list endpoints that fan out to multiple repos.

use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextResolve};
use async_graphql::{ServerResult, Value};
use async_graphql::extensions::ResolveInfo;
use async_trait::async_trait;
use opentelemetry::metrics::Histogram;
use opentelemetry::{KeyValue, global};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

pub struct GraphQLResolverMetrics;

impl ExtensionFactory for GraphQLResolverMetrics {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(GraphQLResolverMetricsExtension)
    }
}

struct GraphQLResolverMetricsExtension;

/// `(parent_type, field_name)` pairs to time. Both halves are server-defined,
/// so cardinality is exactly `len(SLOW_RESOLVERS)`. Keep small.
const SLOW_RESOLVERS: &[(&str, &str)] = &[
    // Nav resolvers on chapter pages — risk per-chapter DB hops.
    ("ChapterGql", "prevChapter"),
    ("ChapterGql", "nextChapter"),
    // List endpoints that fan out across repos.
    ("QueryRoot", "books"),
    ("QueryRoot", "chapters"),
    ("QueryRoot", "booksByAuthor"),
    ("QueryRoot", "booksByCategory"),
    ("QueryRoot", "booksByTag"),
    ("QueryRoot", "featured"),
];

fn is_slow(parent_type: &str, field_name: &str) -> bool {
    SLOW_RESOLVERS
        .iter()
        .any(|(p, f)| *p == parent_type && *f == field_name)
}

static RESOLVER_DURATION: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    global::meter("merk")
        .f64_histogram("graphql_resolver_duration_seconds")
        .build()
});

#[async_trait]
impl Extension for GraphQLResolverMetricsExtension {
    async fn resolve(
        &self,
        ctx: &ExtensionContext<'_>,
        info: ResolveInfo<'_>,
        next: NextResolve<'_>,
    ) -> ServerResult<Option<Value>> {
        if !is_slow(info.parent_type, info.name) {
            return next.run(ctx, info).await;
        }

        // Copy the labels before moving `info` into `next.run`.
        let parent_label = info.parent_type.to_owned();
        let field_label = info.name.to_owned();

        let start = Instant::now();
        let result = next.run(ctx, info).await;
        let latency = start.elapsed().as_secs_f64();

        RESOLVER_DURATION.record(
            latency,
            &[
                KeyValue::new("parent_type", parent_label),
                KeyValue::new("field", field_label),
            ],
        );

        result
    }
}
