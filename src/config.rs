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
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

impl AppConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env::<Self>()
    }

    pub fn get_port(&self) -> u16 {
        self.port
            .unwrap_or(if self.enable_tls { 8443 } else { 3000 })
    }
}
