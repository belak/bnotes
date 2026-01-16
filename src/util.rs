use crate::config::Config;
use crate::repository::Repository;
use anyhow::Result;
use std::path::PathBuf;

/// Context for command execution, containing config and repository
pub struct CommandContext {
    pub config: Config,
    pub repo: Repository,
}

impl CommandContext {
    /// Load configuration and create repository context
    pub fn load(config_path: Option<PathBuf>) -> Result<Self> {
        let config = Config::resolve_and_load(config_path.as_deref())?;
        let repo = Repository::new(&config.notes_dir);
        Ok(Self { config, repo })
    }
}

/// Return singular or plural form based on count
pub fn pluralize<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}
