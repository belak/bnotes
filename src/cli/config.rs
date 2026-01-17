//! CLI configuration

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// CLI configuration
///
/// Minimal configuration for the CLI that only specifies where to find the notes directory.
/// Other configuration (templates, periodic settings) lives in LibraryConfig within the notes directory.
#[derive(Debug, Serialize, Deserialize)]
pub struct CLIConfig {
    pub notes_dir: PathBuf,
}

impl CLIConfig {
    /// Load config from the specified path
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: CLIConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Resolve and load config file from CLI arg, env var, or default location
    pub fn resolve_and_load(config_path: Option<&Path>) -> Result<Self> {
        let path = if let Some(p) = config_path {
            // CLI argument takes precedence
            p.to_path_buf()
        } else if let Ok(env_path) = std::env::var("BNOTES_CONFIG") {
            // Environment variable
            PathBuf::from(env_path)
        } else {
            // Default location
            Self::default_config_path()?
        };

        if !path.exists() {
            anyhow::bail!(
                "No config found at: {}\nRun `bnotes init` to create one.",
                path.display()
            );
        }

        Self::load(&path)
    }

    /// Get the default config file path
    pub fn default_config_path() -> Result<PathBuf> {
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config)
        } else {
            let home =
                std::env::var("HOME").context("HOME environment variable not set")?;
            PathBuf::from(home).join(".config")
        };

        Ok(config_dir.join("bnotes").join("config.toml"))
    }
}

impl Default for CLIConfig {
    fn default() -> Self {
        Self {
            notes_dir: PathBuf::from("~/notes"),
        }
    }
}
