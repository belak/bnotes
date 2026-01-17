//! CLI module
//!
//! This module contains all CLI-specific functionality including:
//! - Configuration (CLIConfig)
//! - Command implementations
//! - Utility functions

pub mod commands;
pub mod config;
pub mod utils;

pub use commands::PeriodicAction;
