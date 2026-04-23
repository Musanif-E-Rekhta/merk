use serde::Deserialize;

/// Application configuration loaded from environment variables via [`envy`].
///
/// All fields have defaults suitable for local development. In release builds,
/// using the default `jwt_secret` causes a startup panic — set `JWT_SECRET` explicitly.
#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    /// Network interface to bind to (e.g. `0.0.0.0` or `127.0.0.1`).
    #[serde(default = "default_host")]
    pub host: String,

    /// Listen port. When `None`, defaults to `8443` (TLS) or `9678` (plain HTTP).
    pub port: Option<u16>,

    /// When `true`, axum-server uses an auto-generated rcgen self-signed certificate.
    #[serde(default)]
    pub enable_tls: bool,

    /// Extra Subject Alternative Name added to the auto-generated TLS certificate.
    #[serde(default)]
    pub tls_alt_name: String,

    /// SurrealDB connection URL (e.g. `ws://127.0.0.1:8000` or `http://...`).
    #[serde(default = "default_db_url")]
    pub surrealdb_url: String,

    /// SurrealDB root username used for WebSocket sign-in.
    #[serde(default = "default_db_user")]
    pub surrealdb_user: String,

    /// SurrealDB root password.
    #[serde(default = "default_db_pass")]
    pub surrealdb_pass: String,

    /// SurrealDB namespace to select after connecting.
    #[serde(default = "default_db_ns")]
    pub surrealdb_ns: String,

    /// SurrealDB database to select within the namespace.
    #[serde(default = "default_db_db")]
    pub surrealdb_db: String,

    /// HS256 JWT signing secret. Must be ≥ 32 characters. The default value is
    /// rejected in release builds.
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
}

fn default_db_url() -> String {
    "ws://127.0.0.1:8000".to_string()
}
fn default_db_user() -> String {
    "root".to_string()
}
fn default_db_pass() -> String {
    "root".to_string()
}
fn default_db_ns() -> String {
    "merk".to_string()
}
fn default_db_db() -> String {
    "merk".to_string()
}
fn default_jwt_secret() -> String {
    "super-secret-local-dev-key-change-me".to_string()
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

impl AppConfig {
    /// Parse configuration from the process environment.
    ///
    /// Panics in release builds if `JWT_SECRET` is the development default or shorter than 32 chars.
    pub fn from_env() -> Result<Self, envy::Error> {
        let config = envy::from_env::<Self>()?;
        #[cfg(not(debug_assertions))]
        if config.jwt_secret == "super-secret-local-dev-key-change-me" {
            panic!(
                "JWT_SECRET must be explicitly set in production — the default development secret is not safe"
            );
        }
        if config.jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters long");
        }
        Ok(config)
    }

    /// Resolve the effective listen port, falling back to TLS/HTTP defaults when unset.
    pub fn get_port(&self) -> u16 {
        self.port
            .unwrap_or(if self.enable_tls { 8443 } else { 9678 })
    }

    /// Build the base URL from the configured host, port, and TLS state.
    pub fn base_url(&self) -> String {
        let scheme = if self.enable_tls { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.host, self.get_port())
    }
}
