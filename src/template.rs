use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::Path;

pub fn render(template_path: &Path, title: &str) -> Result<String> {
    let template_content = fs::read_to_string(template_path)
        .with_context(|| format!("Failed to read template: {}", template_path.display()))?;

    let now = Utc::now();
    let date = now.format("%Y-%m-%d").to_string();
    let datetime = now.to_rfc3339();

    let rendered = template_content
        .replace("{{title}}", title)
        .replace("{{date}}", &date)
        .replace("{{datetime}}", &datetime);

    Ok(rendered)
}

pub fn list_templates(template_dir: &Path) -> Result<Vec<String>> {
    if !template_dir.exists() {
        return Ok(Vec::new());
    }

    let mut templates = Vec::new();

    for entry in fs::read_dir(template_dir)
        .with_context(|| format!("Failed to read template directory: {}", template_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("md")
        {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                templates.push(stem.to_string());
            }
        }
    }

    templates.sort();
    Ok(templates)
}
