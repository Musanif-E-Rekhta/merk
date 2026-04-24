use merk::config::AppConfig;
use merk::utils::tracing::init as init_tracing;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::from_filename(".env.local").ok();
    init_tracing();

    let config = AppConfig::from_env()?;
    merk::server::start(config).await
}
