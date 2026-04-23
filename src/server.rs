use crate::api::create_router;
use crate::config::AppConfig;
use crate::state::AppState;
use crate::utils::banner::log_startup;
use axum_server::tls_rustls::RustlsConfig;
use rcgen::CertifiedKey;
use std::net::SocketAddr;

pub async fn start(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let base_url = config.base_url();
    let addr = format!("{}:{}", config.host, config.get_port()).parse::<SocketAddr>()?;

    log_startup(&base_url);

    let db = crate::db::connect_to_db(&config)
        .await
        .expect("Failed to initialize SurrealDB connections and migrations");

    let app = create_router(AppState::new(config.clone(), db));

    run_axum_server(app, &config, addr).await
}

async fn run_axum_server(
    app: axum::Router,
    config: &AppConfig,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = axum_server::Handle::new();
    let shutdown_handle = handle.clone();

    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::info!("Received shutdown signal, initiating graceful shutdown...");
            shutdown_handle.graceful_shutdown(Some(std::time::Duration::from_secs(10)));
        }
    });

    let builder = axum_server::bind(addr).handle(handle);

    if config.enable_tls {
        tracing::info!("Starting HTTPS server on {}", addr);
        let tls_config = get_tls_config(config).await?;
        builder
            .acceptor(axum_server::tls_rustls::RustlsAcceptor::new(tls_config))
            .serve(app.into_make_service())
            .await?;
    } else {
        tracing::info!("Starting HTTP server on {}", addr);
        builder.serve(app.into_make_service()).await?;
    }

    tracing::warn!("Server has been shutdown, it will not accept connections anymore.");
    Ok(())
}

async fn get_tls_config(config: &AppConfig) -> Result<RustlsConfig, Box<dyn std::error::Error>> {
    let mut subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];

    if !config.tls_alt_name.is_empty() {
        subject_alt_names.push(config.tls_alt_name.clone());
    }

    let CertifiedKey { cert, key_pair } = rcgen::generate_simple_self_signed(subject_alt_names)?;

    Ok(RustlsConfig::from_pem(
        cert.pem().into_bytes(),
        key_pair.serialize_pem().into_bytes(),
    )
    .await?)
}
