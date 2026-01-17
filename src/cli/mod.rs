//! CLI module
//!
//! This module contains all CLI-specific functionality including:
//! - Configuration (CLIConfig)
//! - Command implementations
//! - Git operations
//! - Utility functions

pub mod commands;
pub mod config;
pub mod git;
pub mod utils;

pub use commands::PeriodicAction;
