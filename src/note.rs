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
    /// Parse a note from a file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read note: {}", path.display()))?;

        Self::parse(path, &content)
    }

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
            if let Some(end_pos) = content.find("\n---\n").or_else(|| content.find("\n---").map(|pos| {
                if content.len() > pos + 4 {
                    pos
                } else {
                    pos
                }
            })) {
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
                Event::Start(Tag::Heading { level: pulldown_cmark::HeadingLevel::H1, .. }) => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_frontmatter() {
        let content = r#"---
title: "Test Note"
tags: [work, project]
---

# Different Heading

Some content
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert_eq!(note.title, "Test Note");
        assert_eq!(note.tags, vec!["work", "project"]);
    }

    #[test]
    fn test_parse_without_frontmatter() {
        let content = r#"# My Note Title

Some content
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert_eq!(note.title, "My Note Title");
        assert!(note.tags.is_empty());
    }

    #[test]
    fn test_parse_filename_fallback() {
        let content = "Just some content without heading";

        let note = Note::parse(Path::new("my-note.md"), content).unwrap();
        assert_eq!(note.title, "my-note");
    }
}
