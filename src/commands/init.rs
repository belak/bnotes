use crate::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn run(notes_dir: Option<PathBuf>) -> Result<()> {
    let config_path = Config::default_config_path()?;

    // Check if config already exists
    if config_path.exists() {
        print!("Config file already exists at {}. Overwrite? [y/N] ", config_path.display());
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Get notes directory
    let notes_dir = if let Some(dir) = notes_dir {
        dir
    } else {
        print!("Enter notes directory path (default: ~/notes): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            expand_home("~/notes")?
        } else {
            expand_home(input)?
        }
    };

    // Create config directory
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    // Create config
    let config = Config {
        notes_dir: notes_dir.clone(),
        ..Default::default()
    };

    let config_content = toml::to_string_pretty(&config)
        .context("Failed to serialize config")?;

    fs::write(&config_path, config_content)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    println!("✓ Config created at: {}", config_path.display());

    // Create notes directory
    fs::create_dir_all(&notes_dir)
        .with_context(|| format!("Failed to create notes directory: {}", notes_dir.display()))?;

    println!("✓ Notes directory created at: {}", notes_dir.display());

    // Create templates directory
    let templates_dir = notes_dir.join(".templates");
    fs::create_dir_all(&templates_dir)
        .with_context(|| format!("Failed to create templates directory: {}", templates_dir.display()))?;

    // Create example template
    let example_template = templates_dir.join("daily.md");
    let template_content = r#"---
tags: [daily]
created: {{datetime}}
updated: {{datetime}}
---

# {{title}}

## Tasks
- [ ]

## Notes
"#;

    fs::write(&example_template, template_content)
        .with_context(|| format!("Failed to create example template: {}", example_template.display()))?;

    println!("✓ Template directory created at: {}", templates_dir.display());
    println!("✓ Example template created: daily.md");
    println!("\nbnotes is ready! Try:");
    println!("  bnotes new \"My First Note\"");

    Ok(())
}

fn expand_home(path: &str) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME")
            .context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(rest))
    } else if path == "~" {
        let home = std::env::var("HOME")
            .context("HOME environment variable not set")?;
        Ok(PathBuf::from(home))
    } else {
        Ok(PathBuf::from(path))
    }
}
