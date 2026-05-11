//! Lifecycle metrics for GraphQL subscriptions.
//!
//! HTTP-level instrumentation can't see WebSocket subscriptions — each
//! `/api/graphql/ws` upgrade is a single long-lived request. This
//! extension fills the gap with three OTel instruments keyed on the
//! cardinality-safe operation label from [`observability`]:
//!
//! - `graphql_subscriptions_active` — live gauge of concurrent streams.
//! - `graphql_subscription_messages_total` — counter incremented per yield.
//! - `graphql_subscription_duration_seconds` — histogram recorded on drop.
//!
//! Operation name is captured in `prepare_request` (the only hook that
//! sees the `Request`) and stashed in per-request data so `subscribe` can
//! recover it.
//!
//! [`observability`]: crate::api::graphql::observability

use async_graphql::extensions::{
    Extension, ExtensionContext, ExtensionFactory, NextPrepareRequest, NextSubscribe,
};
use async_graphql::{Request, Response, ServerResult};
use async_trait::async_trait;
use futures_util::Stream;
use futures_util::stream::BoxStream;
use opentelemetry::metrics::{Counter, Histogram, UpDownCounter};
use opentelemetry::{KeyValue, global};
use std::pin::Pin;
use std::sync::{Arc, LazyLock};
use std::task::{Context, Poll};
use std::time::Instant;

use crate::api::graphql::observability::resolve_operation_label;

pub struct GraphQLSubscriptionMetrics;

impl ExtensionFactory for GraphQLSubscriptionMetrics {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(GraphQLSubscriptionMetricsExtension)
    }
}

struct GraphQLSubscriptionMetricsExtension;

static SUBS_ACTIVE: LazyLock<UpDownCounter<i64>> = LazyLock::new(|| {
    global::meter("merk")
        .i64_up_down_counter("graphql_subscriptions_active")
        .build()
});

static SUB_MESSAGES_TOTAL: LazyLock<Counter<u64>> = LazyLock::new(|| {
    global::meter("merk")
        .u64_counter("graphql_subscription_messages_total")
        .build()
});

static SUB_DURATION: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    global::meter("merk")
        .f64_histogram("graphql_subscription_duration_seconds")
        .build()
});

#[derive(Clone)]
struct SubscriptionOperation(&'static str);

#[async_trait]
impl Extension for GraphQLSubscriptionMetricsExtension {
    async fn prepare_request(
        &self,
        ctx: &ExtensionContext<'_>,
        request: Request,
        next: NextPrepareRequest<'_>,
    ) -> ServerResult<Request> {
        let label = resolve_operation_label(request.operation_name.as_deref());
        let request = request.data(SubscriptionOperation(label));
        next.run(ctx, request).await
    }

    fn subscribe<'s>(
        &self,
        ctx: &ExtensionContext<'_>,
        stream: BoxStream<'s, Response>,
        next: NextSubscribe<'_>,
    ) -> BoxStream<'s, Response> {
        let operation = ctx
            .data_opt::<SubscriptionOperation>()
            .map(|s| s.0)
            .unwrap_or("anonymous");

        SUBS_ACTIVE.add(1, &[KeyValue::new("operation", operation)]);

        Box::pin(InstrumentedSubscriptionStream {
            inner: next.run(ctx, stream),
            operation,
            start: Instant::now(),
        })
    }
}

struct InstrumentedSubscriptionStream<'s> {
    inner: BoxStream<'s, Response>,
    operation: &'static str,
    start: Instant,
}

impl<'s> Stream for InstrumentedSubscriptionStream<'s> {
    type Item = Response;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Pin<Box<dyn Stream>> is Unpin, so destructuring through get_mut is safe.
        let this = self.get_mut();
        let poll = this.inner.as_mut().poll_next(cx);
        if matches!(poll, Poll::Ready(Some(_))) {
            SUB_MESSAGES_TOTAL.add(1, &[KeyValue::new("operation", this.operation)]);
        }
        poll
    }
}

impl Drop for InstrumentedSubscriptionStream<'_> {
    fn drop(&mut self) {
        let attrs = [KeyValue::new("operation", self.operation)];
        SUBS_ACTIVE.add(-1, &attrs);
        SUB_DURATION.record(self.start.elapsed().as_secs_f64(), &attrs);
    }
}
