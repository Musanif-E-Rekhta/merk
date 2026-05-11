//! `merk` binary entry point.
//!
//! Loads `.env.local` from the cwd if present, initialises tracing
//! (with OTLP export when reachable; `RUST_LOG` overrides the default
//! filter), parses [`AppConfig`](merk::config::AppConfig) from the
//! environment via `envy`, then hands off to [`merk::server::start`].
//! Tracing is flushed on the way out so spans emitted during shutdown
//! still reach the collector.

use merk::config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::from_filename(".env.local").ok();
    merk_observability::init_tracing("merk", "merk=debug,tower_http=debug,axum=info,info");

    let config = AppConfig::from_env()?;
    let result = merk::server::start(config).await;
    merk_observability::shutdown_tracing();
    result
}
