//! `merk` binary entry point.
//!
//! Walks up from the cwd looking for `.env.local` (so a single
//! workspace-root file is shared with `merk-seed`), initialises tracing
//! (with OTLP export when reachable; `RUST_LOG` overrides the default
//! filter), parses [`AppConfig`](merk::config::AppConfig) from the
//! environment via `envy`, then hands off to [`merk::server::start`].
//! Tracing is flushed on the way out so spans emitted during shutdown
//! still reach the collector.

use merk::config::AppConfig;

fn load_dotenv() {
    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        let f = d.join(".env.local");
        if f.exists() {
            dotenvy::from_path(&f).ok();
            return;
        }
        dir = d.parent().map(std::path::Path::to_path_buf);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_dotenv();
    merk_observability::init_observability("merk", "merk=debug,tower_http=debug,axum=info,info");

    let config = AppConfig::from_env()?;
    let result = merk::server::start(config).await;
    merk_observability::shutdown_observability();
    result
}
