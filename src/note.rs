//! Core note domain types
//!
//! This module contains the fundamental types for representing notes, tasks,
//! and frontmatter. These types are used throughout the library for parsing
//! and working with markdown notes.

use anyhow::Result;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pulldown_cmark::{Event, MetadataBlockKind, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::{Path, PathBuf};

// ============================================================================
// Frontmatter
// ============================================================================

/// Custom deserializer for tags that accepts either array or comma-separated string
fn deserialize_tags<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TagsFormat {
        Array(Vec<String>),
        String(String),
    }

    match TagsFormat::deserialize(deserializer)? {
        TagsFormat::Array(tags) => Ok(tags),
        TagsFormat::String(s) => Ok(s
            .split(',')
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect()),
    }
}

/// Custom deserializer for datetime that accepts both RFC3339 and YYYY-MM-DD formats
fn deserialize_datetime<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) => {
            // Try parsing as RFC3339 first
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                return Ok(Some(dt.with_timezone(&Utc)));
            }

            // Try parsing as YYYY-MM-DD
            if let Ok(date) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                // Convert to DateTime at midnight UTC
                if let Some(dt) = date.and_hms_opt(0, 0, 0) {
                    return Ok(Some(Utc.from_utc_datetime(&dt)));
                }
            }

            Err(Error::custom(format!(
                "expected datetime in RFC3339 or YYYY-MM-DD format, got: {}",
                s
            )))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_tags")]
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_datetime")]
    pub created: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_datetime")]
    pub updated: Option<DateTime<Utc>>,
}

// ============================================================================
// Note
// ============================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
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
        let (frontmatter, body) = Self::extract_frontmatter(path, content)?;

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
    fn extract_frontmatter(path: &Path, content: &str) -> Result<(Option<Frontmatter>, String)> {
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
                    eprintln!("Warning: Failed to parse frontmatter in {}: {}", path.display(), e);
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

// ============================================================================
// Task
// ============================================================================

#[derive(Debug, Clone)]
pub struct Task {
    pub note_path: PathBuf,
    pub note_title: String,
    pub index: usize, // 1-based index within the note
    pub completed: bool,
    pub text: String,
    pub priority: Option<String>,
    pub urgency: Option<String>,  // !!!, !!, !
}

impl Task {
    /// Parse priority from task text in format (Priority) Task text
    /// Returns (priority, remaining_text) or (None, original_text)
    fn parse_priority(text: &str) -> (Option<String>, String) {
        let trimmed = text.trim();
        if trimmed.starts_with('(') {
            if let Some(end_paren) = trimmed.find(')') {
                let priority = trimmed[1..end_paren].trim().to_string();
                let remaining = trimmed[end_paren + 1..].trim().to_string();
                if !priority.is_empty() {
                    return (Some(priority), remaining);
                }
            }
        }
        (None, text.to_string())
    }

    /// Extract all tasks from a note
    pub fn extract_from_note(note: &Note) -> Vec<Task> {
        let mut tasks = Vec::new();
        let mut task_index = 0;

        // Parse the markdown to find task list items
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TASKLISTS);
        let parser = Parser::new_ext(&note.content, options);
        let mut in_task_item = false;
        let mut task_text = String::new();
        let mut is_checked = false;

        for event in parser {
            match event {
                Event::Start(Tag::Item) => {
                    in_task_item = false;
                    task_text.clear();
                }
                Event::TaskListMarker(checked) => {
                    in_task_item = true;
                    is_checked = checked;
                }
                Event::Text(text) if in_task_item => {
                    task_text.push_str(&text);
                }
                Event::End(TagEnd::Item) if in_task_item => {
                    task_index += 1;

                    let (priority, text) = Self::parse_priority(&task_text);

                    tasks.push(Task {
                        note_path: note.path.clone(),
                        note_title: note.title.clone(),
                        index: task_index,
                        completed: is_checked,
                        text,
                        priority,
                        urgency: None,
                    });

                    in_task_item = false;
                }
                _ => {}
            }
        }

        tasks
    }

    /// Get the task ID in format "filename#index"
    pub fn id(&self) -> String {
        let filename = self
            .note_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        format!("{}#{}", filename, self.index)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract all tasks from multiple notes
pub(crate) fn extract_tasks_from_notes(notes: &[Note]) -> Vec<Task> {
    let mut all_tasks = Vec::new();

    for note in notes {
        let tasks = Task::extract_from_note(note);
        all_tasks.extend(tasks);
    }

    all_tasks
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tasks() {
        let content = r#"---
tags: [test]
---

# My Note

Some text.

## Tasks
- [ ] First task
- [x] Completed task
- [ ] Another task

More text.
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        let tasks = Task::extract_from_note(&note);

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].text, "First task");
        assert!(!tasks[0].completed);
        assert_eq!(tasks[0].priority, None);
        assert_eq!(tasks[1].text, "Completed task");
        assert!(tasks[1].completed);
        assert_eq!(tasks[1].priority, None);
        assert_eq!(tasks[2].text, "Another task");
        assert!(!tasks[2].completed);
        assert_eq!(tasks[2].priority, None);
    }

    #[test]
    fn test_extract_tasks_with_priorities() {
        let content = r#"---
tags: [test]
---

# My Note

## Tasks
- [ ] (A) High priority task
- [ ] (B) Medium priority task
- [ ] Regular task without priority
- [x] (A) Completed high priority
- [ ] (C) Low priority task

"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        let tasks = Task::extract_from_note(&note);

        assert_eq!(tasks.len(), 5);

        // Check priority parsing
        assert_eq!(tasks[0].priority, Some("A".to_string()));
        assert_eq!(tasks[0].text, "High priority task");

        assert_eq!(tasks[1].priority, Some("B".to_string()));
        assert_eq!(tasks[1].text, "Medium priority task");

        assert_eq!(tasks[2].priority, None);
        assert_eq!(tasks[2].text, "Regular task without priority");

        assert_eq!(tasks[3].priority, Some("A".to_string()));
        assert_eq!(tasks[3].text, "Completed high priority");
        assert!(tasks[3].completed);

        assert_eq!(tasks[4].priority, Some("C".to_string()));
        assert_eq!(tasks[4].text, "Low priority task");
    }

    #[test]
    fn test_task_id() {
        let task = Task {
            note_path: PathBuf::from("test-note.md"),
            note_title: "Test Note".to_string(),
            index: 3,
            completed: false,
            text: "Do something".to_string(),
            priority: None,
            urgency: None,
        };

        assert_eq!(task.id(), "test-note#3");
    }

    #[test]
    fn test_tags_array_format() {
        let content = r#"---
title: Test Note
tags: [rust, testing, example]
---

# Test Note
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert_eq!(note.tags, vec!["rust", "testing", "example"]);
    }

    #[test]
    fn test_tags_comma_separated_format() {
        let content = r#"---
title: Test Note
tags: "rust, testing, example"
---

# Test Note
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert_eq!(note.tags, vec!["rust", "testing", "example"]);
    }

    #[test]
    fn test_tags_comma_separated_with_extra_whitespace() {
        let content = r#"---
title: Test Note
tags: "rust,  testing  ,   example"
---

# Test Note
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert_eq!(note.tags, vec!["rust", "testing", "example"]);
    }

    #[test]
    fn test_tags_empty_string() {
        let content = r#"---
title: Test Note
tags: ""
---

# Test Note
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert_eq!(note.tags, Vec::<String>::new());
    }

    #[test]
    fn test_tags_missing_field() {
        let content = r#"---
title: Test Note
---

# Test Note
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert_eq!(note.tags, Vec::<String>::new());
    }

    #[test]
    fn test_datetime_yyyy_mm_dd_format() {
        let content = r#"---
title: Test Note
created: 2024-01-15
updated: 2024-02-20
---

# Test Note
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert!(note.created.is_some());
        assert!(note.updated.is_some());

        let created = note.created.unwrap();
        assert_eq!(created.format("%Y-%m-%d").to_string(), "2024-01-15");

        let updated = note.updated.unwrap();
        assert_eq!(updated.format("%Y-%m-%d").to_string(), "2024-02-20");
    }

    #[test]
    fn test_datetime_rfc3339_format() {
        let content = r#"---
title: Test Note
created: 2024-01-15T10:30:00Z
updated: 2024-02-20T15:45:30Z
---

# Test Note
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert!(note.created.is_some());
        assert!(note.updated.is_some());

        let created = note.created.unwrap();
        assert_eq!(created.format("%Y-%m-%d").to_string(), "2024-01-15");

        let updated = note.updated.unwrap();
        assert_eq!(updated.format("%Y-%m-%d").to_string(), "2024-02-20");
    }

    #[test]
    fn test_datetime_mixed_formats() {
        let content = r#"---
title: Test Note
created: 2024-01-15
updated: 2024-02-20T15:45:30Z
---

# Test Note
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert!(note.created.is_some());
        assert!(note.updated.is_some());

        let created = note.created.unwrap();
        assert_eq!(created.format("%Y-%m-%d").to_string(), "2024-01-15");

        let updated = note.updated.unwrap();
        assert_eq!(updated.format("%Y-%m-%d").to_string(), "2024-02-20");
    }

    #[test]
    fn test_datetime_missing_fields() {
        let content = r#"---
title: Test Note
---

# Test Note
"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        assert!(note.created.is_none());
        assert!(note.updated.is_none());
    }
}
