//! Repository for managing notes
//!
//! The Repository provides high-level operations for discovering and querying notes,
//! using the Storage abstraction for file access.

use crate::storage::Storage;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use pulldown_cmark::{Event, MetadataBlockKind, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub path: PathBuf,
    pub title: String,
    pub tags: Vec<String>,
    pub created: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
    pub content: String,
}

impl Note {
    /// Parse a note from content
    pub fn parse(path: &Path, content: &str) -> Result<Self> {
        let (frontmatter, body) = Self::extract_frontmatter(content)?;

        // Determine title: frontmatter > first H1 > filename
        let title = frontmatter
            .as_ref()
            .and_then(|fm| fm.title.clone())
            .or_else(|| Self::extract_first_h1(&body))
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

        let tags = frontmatter
            .as_ref()
            .map(|fm| fm.tags.clone())
            .unwrap_or_default();

        let created = frontmatter.as_ref().and_then(|fm| fm.created);
        let updated = frontmatter.as_ref().and_then(|fm| fm.updated);

        Ok(Self {
            path: path.to_path_buf(),
            title,
            tags,
            created,
            updated,
            content: content.to_string(),
        })
    }

    /// Extract frontmatter and body from content using pulldown-cmark's built-in parsing
    fn extract_frontmatter(content: &str) -> Result<(Option<Frontmatter>, String)> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

        let parser = Parser::new_ext(content, options);
        let mut in_metadata = false;
        let mut yaml_content = String::new();
        let mut found_metadata = false;

        for event in parser {
            match event {
                Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                    in_metadata = true;
                    found_metadata = true;
                }
                Event::End(TagEnd::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                    in_metadata = false;
                }
                Event::Text(text) if in_metadata => {
                    yaml_content.push_str(&text);
                }
                _ => {}
            }
        }

        let frontmatter = if found_metadata && !yaml_content.is_empty() {
            match serde_yaml::from_str::<Frontmatter>(&yaml_content) {
                Ok(fm) => Some(fm),
                Err(e) => {
                    // Log warning but continue
                    eprintln!("Warning: Failed to parse frontmatter: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Extract body by removing the frontmatter block from the original content
        let body = if found_metadata {
            // Find the end of the frontmatter block in the original content
            if let Some(end_pos) = content.find("\n---\n").or_else(|| content.find("\n---")) {
                content[end_pos + 4..].trim_start().to_string()
            } else {
                content.to_string()
            }
        } else {
            content.to_string()
        };

        Ok((frontmatter, body))
    }

    /// Extract the first H1 heading from markdown
    fn extract_first_h1(content: &str) -> Option<String> {
        let parser = Parser::new(content);
        let mut in_h1 = false;
        let mut h1_text = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading {
                    level: pulldown_cmark::HeadingLevel::H1,
                    ..
                }) => {
                    in_h1 = true;
                }
                Event::End(TagEnd::Heading(pulldown_cmark::HeadingLevel::H1)) => {
                    if !h1_text.is_empty() {
                        return Some(h1_text);
                    }
                    in_h1 = false;
                }
                Event::Text(text) if in_h1 => {
                    h1_text.push_str(&text);
                }
                _ => {}
            }
        }

        None
    }
}

pub struct Repository {
    pub(crate) storage: Box<dyn Storage>,
}

impl Repository {
    pub fn new(storage: Box<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Discover all notes in the repository
    pub fn discover_notes(&self) -> Result<Vec<Note>> {
        let mut notes = Vec::new();
        self.discover_notes_recursive(Path::new(""), &mut notes)?;
        Ok(notes)
    }

    /// Recursively discover notes starting from the given path
    fn discover_notes_recursive(&self, path: &Path, notes: &mut Vec<Note>) -> Result<()> {
        // Skip if any component of the path starts with '.'
        for component in path.components() {
            if let Some(name_str) = component.as_os_str().to_str()
                && name_str.starts_with('.') {
                    return Ok(());
                }
        }

        // If it's a directory, recurse into it
        if self.storage.is_dir(path) {
            let entries = self.storage.read_dir(path)?;
            for entry in entries {
                self.discover_notes_recursive(&entry, notes)?;
            }
        } else if self.storage.exists(path) {
            // Only process .md files
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                // Read content and parse note
                match self.storage.read_to_string(path) {
                    Ok(content) => match Note::parse(path, &content) {
                        Ok(note) => notes.push(note),
                        Err(e) => {
                            eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Warning: Failed to read {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Find a note by title (case-insensitive)
    pub fn find_by_title(&self, title: &str) -> Result<Vec<Note>> {
        let all_notes = self.discover_notes()?;
        let title_lower = title.to_lowercase();

        let matches: Vec<Note> = all_notes
            .into_iter()
            .filter(|note| note.title.to_lowercase() == title_lower)
            .collect();

        Ok(matches)
    }

    /// Search notes by query (case-insensitive substring matching)
    pub fn search(&self, query: &str) -> Result<Vec<Note>> {
        let all_notes = self.discover_notes()?;
        let query_lower = query.to_lowercase();

        let matches: Vec<Note> = all_notes
            .into_iter()
            .filter(|note| {
                note.content.to_lowercase().contains(&query_lower)
                    || note.title.to_lowercase().contains(&query_lower)
                    || note
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(matches)
    }

    /// Filter notes by tags
    pub fn filter_by_tags(&self, tags: &[String]) -> Result<Vec<Note>> {
        let all_notes = self.discover_notes()?;

        let matches: Vec<Note> = all_notes
            .into_iter()
            .filter(|note| {
                tags.iter()
                    .all(|tag| note.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            })
            .collect();

        Ok(matches)
    }

    /// Create a new note with the given title and optional template
    ///
    /// Returns the relative path to the created note
    pub fn create_note(&self, title: &str, template_dir: &Path, template_name: Option<&str>) -> Result<PathBuf> {
        // Generate filename from title (lowercase, replace spaces/special chars with hyphens)
        let filename = title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();

        // Remove consecutive hyphens
        let filename = filename
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        let note_path = PathBuf::from(format!("{}.md", filename));

        // Check if file already exists
        if self.storage.exists(&note_path) {
            anyhow::bail!("Note already exists: {}", note_path.display());
        }

        // Generate content
        let content = if let Some(template) = template_name {
            let template_path = template_dir.join(format!("{}.md", template));

            if !self.storage.exists(&template_path) {
                anyhow::bail!("Template '{}' not found", template);
            }

            let template_content = self.storage.read_to_string(&template_path)
                .with_context(|| format!("Failed to read template: {}", template_path.display()))?;

            render_template(&template_content, title)
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

        // Write note
        self.storage.write(&note_path, &content)
            .with_context(|| format!("Failed to write note: {}", note_path.display()))?;

        Ok(note_path)
    }
}

/// Render a template by replacing placeholders
pub(crate) fn render_template(template_content: &str, title: &str) -> String {
    let now = Utc::now();
    let date = now.format("%Y-%m-%d").to_string();
    let datetime = now.to_rfc3339();

    template_content
        .replace("{{title}}", title)
        .replace("{{date}}", &date)
        .replace("{{datetime}}", &datetime)
}
