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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialise all tests that touch process-wide env vars so they do not
    /// race each other (Rust test threads share the same process).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // ------------------------------------------------------------------
    // Default values
    // ------------------------------------------------------------------

    #[test]
    fn server_config_default_url_is_empty() {
        let cfg = ServerConfig::default();
        assert!(cfg.url.is_empty());
    }

    #[test]
    fn server_config_default_token_is_none() {
        let cfg = ServerConfig::default();
        assert!(cfg.api_token.is_none());
    }

    #[test]
    fn server_config_default_accept_invalid_certs_is_false() {
        let cfg = ServerConfig::default();
        assert!(!cfg.accept_invalid_certs);
    }

    #[test]
    fn tui_config_default_page_size_is_100() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.page_size, 100);
    }

    #[test]
    fn config_default_is_composed_of_defaults() {
        let cfg = Config::default();
        assert!(cfg.server.url.is_empty());
        assert_eq!(cfg.tui.page_size, 100);
    }

    // ------------------------------------------------------------------
    // Serialization round-trip
    // ------------------------------------------------------------------

    #[test]
    fn config_serializes_and_deserializes() {
        let original = Config {
            server: ServerConfig {
                url: "https://bd.example.com".to_string(),
                api_token: Some("tok123".to_string()),
                accept_invalid_certs: true,
            },
            tui: TuiConfig { page_size: 50 },
        };

        let toml_str = toml::to_string_pretty(&original).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.server.url, original.server.url);
        assert_eq!(parsed.server.api_token, original.server.api_token);
        assert_eq!(
            parsed.server.accept_invalid_certs,
            original.server.accept_invalid_certs
        );
        assert_eq!(parsed.tui.page_size, original.tui.page_size);
    }

    #[test]
    fn config_deserializes_with_missing_page_size_uses_default() {
        let toml_str = "[server]\nurl = \"https://bd.example.com\"\n[tui]\n";
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.tui.page_size, 100);
    }

    // ------------------------------------------------------------------
    // Environment-variable overrides (BLACKDUCK_URL / BLACKDUCK_TOKEN)
    // Tests that mutate process-wide env vars are serialised via ENV_LOCK.
    // ------------------------------------------------------------------

    #[test]
    fn env_var_blackduck_url_overrides_empty() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("BLACKDUCK_URL", "https://env.example.com");
        let cfg = Config::load().unwrap();
        std::env::remove_var("BLACKDUCK_URL");
        assert_eq!(cfg.server.url, "https://env.example.com");
    }

    #[test]
    fn env_var_blackduck_token_overrides_empty() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("BLACKDUCK_TOKEN", "env_token_xyz");
        let cfg = Config::load().unwrap();
        std::env::remove_var("BLACKDUCK_TOKEN");
        assert_eq!(cfg.server.api_token.as_deref(), Some("env_token_xyz"));
    }

    #[test]
    fn env_var_accept_invalid_certs_true_values() {
        let _guard = ENV_LOCK.lock().unwrap();
        for val in ["1", "true", "yes"] {
            std::env::set_var("BLACKDUCK_ACCEPT_INVALID_CERTS", val);
            let cfg = Config::load().unwrap();
            std::env::remove_var("BLACKDUCK_ACCEPT_INVALID_CERTS");
            assert!(
                cfg.server.accept_invalid_certs,
                "expected true for value '{val}'"
            );
        }
    }

    #[test]
    fn env_var_accept_invalid_certs_false_values() {
        let _guard = ENV_LOCK.lock().unwrap();
        for val in ["0", "false", "no"] {
            std::env::set_var("BLACKDUCK_ACCEPT_INVALID_CERTS", val);
            let cfg = Config::load().unwrap();
            std::env::remove_var("BLACKDUCK_ACCEPT_INVALID_CERTS");
            assert!(
                !cfg.server.accept_invalid_certs,
                "expected false for value '{val}'"
            );
        }
    }
}
