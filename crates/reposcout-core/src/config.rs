use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
///
/// This gets loaded from config file, env vars, and CLI args.
/// Priority: CLI > Env > File > Defaults (like a sensible person would do)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub platforms: PlatformConfig,
    pub cache: CacheConfig,
    pub ui: UiConfig,
}

impl Config {
    /// Load config from default location or create one if it doesn't exist
    pub fn load() -> crate::Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&contents)
                .map_err(|e| crate::Error::ConfigError(format!("Failed to parse config: {}", e)))?;
            Ok(config)
        } else {
            // No config file? Use defaults
            Ok(Self::default())
        }
    }

    /// Save config to disk
    pub fn save(&self) -> crate::Result<()> {
        let config_path = Self::config_path()?;

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| crate::Error::ConfigError(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(&config_path, contents)?;
        Ok(())
    }

    /// Get the config file path
    /// Uses XDG on Linux/macOS, AppData on Windows
    fn config_path() -> crate::Result<PathBuf> {
        let config_dir = if cfg!(target_os = "windows") {
            dirs::config_dir()
                .ok_or_else(|| crate::Error::ConfigError("Could not find config directory".into()))?
                .join("reposcout")
        } else {
            // XDG config dir on Unix-like systems
            dirs::config_dir()
                .ok_or_else(|| crate::Error::ConfigError("Could not find config directory".into()))?
                .join("reposcout")
        };

        Ok(config_dir.join("config.toml"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub github: Option<GitHubConfig>,
    pub gitlab: Option<GitLabConfig>,
    pub bitbucket: Option<BitbucketConfig>,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            github: Some(GitHubConfig::default()),
            gitlab: None,
            bitbucket: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// GitHub personal access token
    /// Get one at https://github.com/settings/tokens
    pub token: Option<String>,

    /// API URL (for GitHub Enterprise)
    #[serde(default = "default_github_url")]
    pub api_url: String,
}

fn default_github_url() -> String {
    "https://api.github.com".to_string()
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            token: None,
            api_url: default_github_url(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabConfig {
    pub token: Option<String>,

    /// GitLab instance URL (default: gitlab.com)
    #[serde(default = "default_gitlab_url")]
    pub url: String,
}

fn default_gitlab_url() -> String {
    "https://gitlab.com".to_string()
}

impl Default for GitLabConfig {
    fn default() -> Self {
        Self {
            token: None,
            url: default_gitlab_url(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BitbucketConfig {
    pub username: Option<String>,
    pub app_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache TTL in hours
    #[serde(default = "default_cache_ttl")]
    pub ttl_hours: u64,

    /// Max cache size in MB
    #[serde(default = "default_cache_size")]
    pub max_size_mb: u64,

    /// Enable offline mode (use cache even if stale)
    #[serde(default)]
    pub offline_mode: bool,
}

fn default_cache_ttl() -> u64 {
    24 // 24 hours is reasonable for repo metadata
}

fn default_cache_size() -> u64 {
    500 // 500MB should be plenty
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_hours: default_cache_ttl(),
            max_size_mb: default_cache_size(),
            offline_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// UI theme name (Default Dark, Light, Nord, Dracula, Gruvbox Dark)
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Enable mouse support in TUI
    #[serde(default = "default_mouse")]
    pub mouse_enabled: bool,

    /// Enable portfolio/watchlist feature
    #[serde(default = "default_portfolio_enabled")]
    pub portfolio_enabled: bool,
}

fn default_theme() -> String {
    "Default Dark".to_string() // because who uses light theme in a terminal?
}

fn default_mouse() -> bool {
    true
}

fn default_portfolio_enabled() -> bool {
    true // Enable portfolio feature by default
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            mouse_enabled: default_mouse(),
            portfolio_enabled: default_portfolio_enabled(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.cache.ttl_hours, 24);
        assert_eq!(config.cache.max_size_mb, 500);
        assert_eq!(config.ui.theme, "dark");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("ttl_hours"));
        assert!(toml.contains("theme"));
    }
}
