use crate::template;
use crate::util::CommandContext;
use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(config_path: Option<PathBuf>, title: Option<String>, template_name: Option<String>) -> Result<()> {
    let ctx = CommandContext::load(config_path)?;

    // Get title
    let title = if let Some(t) = title {
        t
    } else {
        print!("Enter note title: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            anyhow::bail!("Title cannot be empty");
        }

        input.to_string()
    };

    // Generate filename from title (lowercase, replace spaces/special chars with hyphens)
    let filename = title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();

    // Remove consecutive hyphens
    let filename = filename
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    let note_path = ctx.config.notes_dir.join(format!("{}.md", filename));

    // Check if file already exists
    if note_path.exists() {
        anyhow::bail!("Note already exists: {}", note_path.display());
    }

    // Generate content
    let content = if let Some(template) = template_name {
        let template_dir = ctx.config.template_dir_path();
        let template_path = template_dir.join(format!("{}.md", template));

        if !template_path.exists() {
            let available = template::list_templates(&template_dir)?;
            if available.is_empty() {
                anyhow::bail!("Template '{}' not found. No templates available.", template);
            } else {
                anyhow::bail!(
                    "Template '{}' not found. Available templates: {}",
                    template,
                    available.join(", ")
                );
            }
        }

        template::render(&template_path, &title)?
    } else {
        // Default note with frontmatter
        let now = Utc::now();
        let datetime = now.to_rfc3339();

        format!(
            r#"---
tags: []
created: {}
updated: {}
---

# {}

"#,
            datetime, datetime, title
        )
    };

    // Ensure notes directory exists
    fs::create_dir_all(&ctx.config.notes_dir)
        .with_context(|| format!("Failed to create notes directory: {}", ctx.config.notes_dir.display()))?;

    // Write note
    fs::write(&note_path, content)
        .with_context(|| format!("Failed to write note: {}", note_path.display()))?;

    println!("Created note: {}", note_path.display());

    Ok(())
}
