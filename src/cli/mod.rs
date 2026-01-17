//! CLI module
//!
//! This module contains all CLI-specific functionality including:
//! - Command implementations
//! - Git operations
//! - Utility functions

pub mod commands;
pub mod git;
pub mod utils;

pub use commands::PeriodicAction;
