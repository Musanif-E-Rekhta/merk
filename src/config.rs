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

    /// Outbound mail transport. `log` (default) writes the message to the
    /// tracing log; `noop` discards; `smtp` delivers via the `MAIL_SMTP_*`
    /// fields below.
    #[serde(default = "default_mail_transport")]
    pub mail_transport: String,

    /// Template used to build the password-reset URL emailed to the user.
    /// `{token}` is replaced with the freshly-issued reset token.
    #[serde(default = "default_reset_url_template")]
    pub mail_reset_url_template: String,

    /// Template used to build the email-verification URL emailed to the user.
    /// `{token}` is replaced with the freshly-issued verification token.
    #[serde(default = "default_verify_url_template")]
    pub mail_verify_url_template: String,

    /// SMTP relay hostname (only consulted when `MAIL_TRANSPORT=smtp`).
    #[serde(default)]
    pub mail_smtp_host: String,

    /// SMTP relay port. Defaults to `587` (STARTTLS submission).
    #[serde(default = "default_smtp_port")]
    pub mail_smtp_port: u16,

    /// SMTP authentication username. Empty disables auth (anonymous relay).
    #[serde(default)]
    pub mail_smtp_user: String,

    /// SMTP authentication password.
    #[serde(default)]
    pub mail_smtp_pass: String,

    /// `From:` mailbox the relay puts on outbound messages, e.g.
    /// `"Musanif <noreply@musanif.app>"`.
    #[serde(default = "default_smtp_from")]
    pub mail_smtp_from: String,

    /// Comma-separated list of allowed CORS origins. Empty (default) opens
    /// the gate to any origin — appropriate for local dev where the Dioxus
    /// build server runs on an arbitrary port. Set to a fixed list in
    /// production: `CORS_ORIGINS=https://musanif.app,https://admin.musanif.app`.
    #[serde(default)]
    pub cors_origins: String,
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
fn default_mail_transport() -> String {
    "log".to_string()
}
fn default_reset_url_template() -> String {
    "http://localhost:3000/reset?token={token}".to_string()
}
fn default_verify_url_template() -> String {
    "http://localhost:3000/verify?token={token}".to_string()
}
fn default_smtp_port() -> u16 {
    587
}
fn default_smtp_from() -> String {
    "Musanif <noreply@musanif.app>".to_string()
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

    /// Parse the comma-separated `cors_origins` field into a list. Empty
    /// strings (including the default) signal "any origin allowed".
    pub fn parsed_cors_origins(&self) -> Vec<String> {
        self.cors_origins
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()
    }
}
