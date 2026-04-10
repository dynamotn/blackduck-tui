use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const APP_NAME: &str = "blackduck-tui";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub server: ServerConfig,
    pub tui: TuiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    /// Black Duck server URL, e.g. `<https://blackduck.example.com>`
    pub url: String,
    /// API token for authentication
    pub api_token: Option<String>,
    /// Accept invalid/self-signed TLS certificates
    #[serde(default)]
    pub accept_invalid_certs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Number of items per page when fetching from API
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page_size() -> u32 {
    100
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            page_size: default_page_size(),
        }
    }
}

impl Config {
    /// Load config from file and environment variables.
    /// Environment variables override file config:
    ///   `BLACKDUCK_URL`   -> `server.url`
    ///   `BLACKDUCK_TOKEN` -> `server.api_token`
    ///   `BLACKDUCK_ACCEPT_INVALID_CERTS` -> `server.accept_invalid_certs`
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        let mut builder = config::Config::builder();

        // Load from file if it exists
        if let Some(path) = &config_path {
            if path.exists() {
                builder = builder.add_source(config::File::from(path.as_path()).required(false));
            }
        }

        // Override with environment variables
        builder = builder.add_source(
            config::Environment::with_prefix("BLACKDUCK")
                .separator("_")
                .try_parsing(true),
        );

        let raw = builder.build().context("Failed to build config")?;

        let mut cfg: Config = raw.try_deserialize().unwrap_or_default();

        // Also support flat env vars for convenience
        if let Ok(url) = std::env::var("BLACKDUCK_URL") {
            if !url.is_empty() {
                cfg.server.url = url;
            }
        }
        if let Ok(token) = std::env::var("BLACKDUCK_TOKEN") {
            if !token.is_empty() {
                cfg.server.api_token = Some(token);
            }
        }
        if let Ok(val) = std::env::var("BLACKDUCK_ACCEPT_INVALID_CERTS") {
            cfg.server.accept_invalid_certs =
                matches!(val.to_lowercase().as_str(), "1" | "true" | "yes");
        }

        Ok(cfg)
    }

    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join(APP_NAME).join("config.toml"))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path().context("Cannot determine config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content).context("Failed to write config file")?;
        Ok(())
    }
}
