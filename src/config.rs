use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub notes_dir: PathBuf,
    #[serde(default = "default_editor")]
    pub editor: String,
    #[serde(default = "default_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default)]
    pub periodic: PeriodicConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeriodicConfig {
    #[serde(default = "default_daily_template")]
    pub daily_template: String,
    #[serde(default = "default_weekly_template")]
    pub weekly_template: String,
    #[serde(default = "default_quarterly_template")]
    pub quarterly_template: String,
}

fn default_daily_template() -> String {
    "daily.md".to_string()
}

fn default_weekly_template() -> String {
    "weekly.md".to_string()
}

fn default_quarterly_template() -> String {
    "quarterly.md".to_string()
}

impl Default for PeriodicConfig {
    fn default() -> Self {
        Self {
            daily_template: default_daily_template(),
            weekly_template: default_weekly_template(),
            quarterly_template: default_quarterly_template(),
        }
    }
}

fn default_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string())
}

fn default_template_dir() -> PathBuf {
    PathBuf::from(".templates")
}

impl Config {
    /// Load config from the specified path
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
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
            let home = std::env::var("HOME")
                .context("HOME environment variable not set")?;
            PathBuf::from(home).join(".config")
        };

        Ok(config_dir.join("bnotes").join("config.toml"))
    }

    /// Get the absolute path to the template directory
    pub fn template_dir_path(&self) -> PathBuf {
        if self.template_dir.is_absolute() {
            self.template_dir.clone()
        } else {
            self.notes_dir.join(&self.template_dir)
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notes_dir: PathBuf::from("~/notes"),
            editor: default_editor(),
            template_dir: default_template_dir(),
            periodic: PeriodicConfig::default(),
        }
    }
}
