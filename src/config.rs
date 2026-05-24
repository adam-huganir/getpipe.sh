use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Config {
  pub(crate) server: ServerConfig,
  pub(crate) cache: CacheConfig,
  pub(crate) github: GithubConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ServerConfig {
  pub(crate) host: String,
  pub(crate) port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CacheConfig {
  pub(crate) github_releases: GithubReleaseCacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GithubReleaseCacheConfig {
  pub(crate) max_capacity: u64,
  pub(crate) ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GithubConfig {
  pub(crate) api_timeout_seconds: u64,
}

impl Config {
  pub(crate) fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
    let content = fs::read_to_string(path.as_ref())
      .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
    let config: Config = serde_yaml::from_str(&content)
      .with_context(|| format!("Failed to parse config file: {:?}", path.as_ref()))?;
    Ok(config)
  }

  pub(crate) fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
    Self::load(path).unwrap_or_else(|err| {
      log::warn!("Failed to load config, using defaults: {}", err);
      Self::default()
    })
  }
}

impl Default for Config {
  fn default() -> Self {
    Config {
      server: ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
      },
      cache: CacheConfig {
        github_releases: GithubReleaseCacheConfig {
          max_capacity: 100,
          ttl_seconds: 600,
        },
      },
      github: GithubConfig {
        api_timeout_seconds: 10,
      },
    }
  }
}

pub(crate) static CONFIG: LazyLock<Config> =
  LazyLock::new(|| Config::load_or_default("config.yaml"));
