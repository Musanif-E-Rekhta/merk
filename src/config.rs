use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,

    pub port: Option<u16>,

    #[serde(default)]
    pub enable_tls: bool,

    #[serde(default)]
    pub tls_alt_name: String,

    #[serde(default = "default_db_url")]
    pub surrealdb_url: String,

    #[serde(default = "default_db_user")]
    pub surrealdb_user: String,

    #[serde(default = "default_db_pass")]
    pub surrealdb_pass: String,

    #[serde(default = "default_db_ns")]
    pub surrealdb_ns: String,

    #[serde(default = "default_db_db")]
    pub surrealdb_db: String,

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

    pub fn get_port(&self) -> u16 {
        self.port
            .unwrap_or(if self.enable_tls { 8443 } else { 9678 })
    }
}
