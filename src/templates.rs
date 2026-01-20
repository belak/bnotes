//! Embedded default templates
//!
//! These templates are embedded in the binary at compile time and serve as
//! fallbacks when templates don't exist in the user's .templates/ directory.

/// Default template for regular notes
pub const DEFAULT: &str = include_str!("../templates/default.md");

/// Template for daily notes
pub const DAILY: &str = include_str!("../templates/daily.md");

/// Template for weekly notes
pub const WEEKLY: &str = include_str!("../templates/weekly.md");

/// Template for quarterly notes
pub const QUARTERLY: &str = include_str!("../templates/quarterly.md");

/// Get embedded template by name
pub fn get_embedded_template(name: &str) -> Option<&'static str> {
    match name {
        "default" | "default.md" => Some(DEFAULT),
        "daily" | "daily.md" => Some(DAILY),
        "weekly" | "weekly.md" => Some(WEEKLY),
        "quarterly" | "quarterly.md" => Some(QUARTERLY),
        _ => None,
    }
}
