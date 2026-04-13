use crate::api::create_router;
use crate::config::AppConfig;
use crate::state::AppState;
use axum_server::tls_rustls::RustlsConfig;
use metrics_exporter_prometheus::PrometheusBuilder;
use rcgen::CertifiedKey;
use std::net::SocketAddr;

pub async fn start(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let recorder_handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    let port = config.get_port();
    let scheme = if config.enable_tls { "https" } else { "http" };
    let server_url = format!("{}://{}:{}", scheme, config.host, port);
    let addr = format!("{}:{}", config.host, port).parse::<SocketAddr>()?;

    let banner = r#"
   __  __           _    
  |  \/  |         | |   
  | \  / | ___ _ __| | __
  | |\/| |/ _ \ '__| |/ /
  | |  | |  __/ |  |   < 
  |_|  |_|\___|_|  |_|\_\
"#;
    let author = "Usairim Isani";

    tracing::info!("\n{}\nProject: merk\nAuthor: {}\nAPI URL: {}", banner, author, server_url);

    let state = AppState::new(config.clone());
    
    // Mount the primary application routes
    let mut app = create_router(state);
    
    // Add metrics onto the router
    app = app.route("/metrics", axum::routing::get(move || async move { recorder_handle.render() }));

    run_axum_server(app, &config, addr).await
}

async fn handle_shutdown(handle: axum_server::Handle) {
    if let Ok(_) = tokio::signal::ctrl_c().await {
        tracing::info!("Received shutdown signal, initiating graceful shutdown...");
        handle.graceful_shutdown(Some(std::time::Duration::from_secs(10)));
    }
}

async fn run_axum_server(
    app: axum::Router,
    config: &AppConfig,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = axum_server::Handle::new();

    tokio::spawn(handle_shutdown(handle.clone()));

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

    let tls_config = RustlsConfig::from_pem(
        cert.pem().into_bytes(),
        key_pair.serialize_pem().into_bytes(),
    )
    .await?;

    Ok(tls_config)
}
