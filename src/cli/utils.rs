//! Utility functions for CLI operations

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Return singular or plural form based on count
pub fn pluralize<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

/// Expand tilde in path to home directory
pub fn expand_home(path: &str) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(rest))
    } else if path == "~" {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home))
    } else {
        Ok(PathBuf::from(path))
    }
}
