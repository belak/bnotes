//! Library configuration
//!
//! Configuration that lives within the notes directory itself,
//! making each notes directory self-contained and portable.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::storage::Storage;

/// Library configuration loaded from the notes directory
///
/// This configuration is stored within the notes directory (at .bnotes/config.toml
/// or config.toml) and contains settings specific to that notes collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    #[serde(default = "default_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default)]
    pub periodic: PeriodicConfig,
}

/// Configuration for periodic notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicConfig {
    #[serde(default = "default_daily_template")]
    pub daily_template: String,
    #[serde(default = "default_weekly_template")]
    pub weekly_template: String,
    #[serde(default = "default_quarterly_template")]
    pub quarterly_template: String,
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

fn default_template_dir() -> PathBuf {
    PathBuf::from(".templates")
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

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            template_dir: default_template_dir(),
            periodic: PeriodicConfig::default(),
        }
    }
}

impl LibraryConfig {
    /// Load library config from storage
    ///
    /// Looks for .bnotes/config.toml or config.toml in the notes directory
    pub fn load(storage: &dyn Storage) -> Result<Self> {
        // Try .bnotes/config.toml first (preferred location)
        if storage.exists(Path::new(".bnotes/config.toml")) {
            let content = storage.read_to_string(Path::new(".bnotes/config.toml"))?;
            return toml::from_str(&content).context("Failed to parse .bnotes/config.toml");
        }

        // Fall back to config.toml in root
        if storage.exists(Path::new("config.toml")) {
            let content = storage.read_to_string(Path::new("config.toml"))?;
            return toml::from_str(&content).context("Failed to parse config.toml");
        }

        anyhow::bail!("No library config found. Expected .bnotes/config.toml or config.toml")
    }

    /// Load config or return defaults if not found
    pub fn load_or_default(storage: &dyn Storage) -> Self {
        Self::load(storage).unwrap_or_default()
    }

    /// Get the template directory path (relative to notes directory)
    pub fn template_dir_path(&self) -> &Path {
        &self.template_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;

    #[test]
    fn test_load_config_from_bnotes_dir() {
        let storage = MemoryStorage::new();
        storage
            .write(
                Path::new(".bnotes/config.toml"),
                r#"
template_dir = "my-templates"

[periodic]
daily_template = "custom-daily.md"
"#,
            )
            .unwrap();

        let config = LibraryConfig::load(&storage).unwrap();
        assert_eq!(config.template_dir, PathBuf::from("my-templates"));
        assert_eq!(config.periodic.daily_template, "custom-daily.md");
    }

    #[test]
    fn test_load_config_from_root() {
        let storage = MemoryStorage::new();
        storage
            .write(
                Path::new("config.toml"),
                r#"
template_dir = ".my-templates"

[periodic]
weekly_template = "custom-weekly.md"
"#,
            )
            .unwrap();

        let config = LibraryConfig::load(&storage).unwrap();
        assert_eq!(config.template_dir, PathBuf::from(".my-templates"));
        assert_eq!(config.periodic.weekly_template, "custom-weekly.md");
    }

    #[test]
    fn test_load_or_default_with_no_config() {
        let storage = MemoryStorage::new();
        let config = LibraryConfig::load_or_default(&storage);

        assert_eq!(config.template_dir, PathBuf::from(".templates"));
        assert_eq!(config.periodic.daily_template, "daily.md");
        assert_eq!(config.periodic.weekly_template, "weekly.md");
        assert_eq!(config.periodic.quarterly_template, "quarterly.md");
    }

    #[test]
    fn test_prefers_bnotes_dir_over_root() {
        let storage = MemoryStorage::new();
        storage
            .write(
                Path::new("config.toml"),
                r#"template_dir = "root-templates""#,
            )
            .unwrap();
        storage
            .write(
                Path::new(".bnotes/config.toml"),
                r#"template_dir = "bnotes-templates""#,
            )
            .unwrap();

        let config = LibraryConfig::load(&storage).unwrap();
        assert_eq!(config.template_dir, PathBuf::from("bnotes-templates"));
    }
}
