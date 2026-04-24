use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::{Tracer, TracerProvider};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "merk=debug".into());
    let fmt = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .with_target(true);

    match build_tracer() {
        Some(tracer) => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt)
                .with(tracing_opentelemetry::layer().with_tracer(tracer))
                .init();
        }
        None => {
            tracing_subscriber::registry().with(filter).with(fmt).init();
        }
    }
}

/// Flush pending spans before process exit.
pub fn shutdown() {
    global::shutdown_tracer_provider();
}

fn build_tracer() -> Option<Tracer> {
    let exporter = SpanExporter::builder().with_tonic().build().ok()?;

    let provider = TracerProvider::builder()
        .with_resource(Resource::new([opentelemetry::KeyValue::new(
            SERVICE_NAME,
            "merk",
        )]))
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    let tracer = provider.tracer("merk");
    global::set_tracer_provider(provider);

    Some(tracer)
}
