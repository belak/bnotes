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

/// Helper function to check if a serde_yaml::Value is empty (null or empty mapping)
fn is_empty_value(value: &serde_yaml::Value) -> bool {
    matches!(value, serde_yaml::Value::Null) ||
    (matches!(value, serde_yaml::Value::Mapping(m) if m.is_empty()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_tags", skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_datetime", skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_datetime", skip_serializing_if = "Option::is_none")]
    pub updated: Option<DateTime<Utc>>,
    /// Preserve any unknown fields
    #[serde(flatten, skip_serializing_if = "is_empty_value")]
    pub extra: serde_yaml::Value,
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

/// Task status - the checkbox marker in markdown
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Uncompleted,   // - [ ]
    Completed,     // - [x] or [X]
    Migrated,      // - [>]
}

impl TaskStatus {
    /// Parse status from checkbox character
    fn from_checkbox_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(TaskStatus::Uncompleted),
            'x' | 'X' => Some(TaskStatus::Completed),
            '>' => Some(TaskStatus::Migrated),
            _ => None,
        }
    }

    /// Check if this status represents an incomplete task
    pub fn is_incomplete(&self) -> bool {
        matches!(self, TaskStatus::Uncompleted)
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub note_path: PathBuf,
    pub note_title: String,
    pub index: usize, // 1-based index within the note
    pub status: TaskStatus,
    pub text: String,
    pub priority: Option<String>,
    pub urgency: Option<String>,  // !!!, !!, !
    pub tags: Vec<String>,  // Tags extracted from task text (lowercase, without @ prefix)
}

impl Task {
    /// Parse tags from the end of text
    /// Returns (tags, remaining_text)
    /// Tags are returned in lowercase without @ prefix, deduplicated
    fn parse_tags(text: &str) -> (Vec<String>, String) {
        let trimmed = text.trim();
        let words: Vec<&str> = trimmed.split_whitespace().collect();

        // Find where tags start (scan backwards for @-prefixed words)
        let mut tag_start_idx = words.len();
        for (i, word) in words.iter().enumerate().rev() {
            if word.starts_with('@') {
                tag_start_idx = i;
            } else {
                break; // Stop at first non-tag
            }
        }

        // Extract tags (remove @ prefix, convert to lowercase, deduplicate)
        let mut tags: Vec<String> = words[tag_start_idx..]
            .iter()
            .filter_map(|w| w.strip_prefix('@'))
            .map(|t| t.to_lowercase())
            .collect();

        // Deduplicate while preserving order
        let mut seen = std::collections::HashSet::new();
        tags.retain(|tag| seen.insert(tag.clone()));

        // Remaining text (everything before tags)
        let text = words[..tag_start_idx].join(" ");

        (tags, text)
    }

    /// Parse urgency and priority from task text
    /// Format: [urgency] [(priority)] task text
    /// Urgency: !!!, !!, ! (must have space after)
    /// Priority: (A), (B), etc.
    /// Returns (urgency, priority, remaining_text)
    fn parse_urgency_and_priority(text: &str) -> (Option<String>, Option<String>, String) {
        let trimmed = text.trim();

        // Parse urgency first (requires space after)
        let (urgency, rest) = if let Some(rest) = trimmed.strip_prefix("!!! ") {
            (Some("!!!".to_string()), rest)
        } else if let Some(rest) = trimmed.strip_prefix("!! ") {
            (Some("!!".to_string()), rest)
        } else if let Some(rest) = trimmed.strip_prefix("! ") {
            (Some("!".to_string()), rest)
        } else {
            (None, trimmed)
        };

        // Then parse priority
        let (priority, task_text) = if rest.starts_with('(') {
            if let Some(end_paren) = rest.find(')') {
                let priority_str = &rest[1..end_paren];
                if priority_str.trim().is_empty() {
                    (None, rest[end_paren + 1..].trim().to_string())
                } else {
                    let remaining = rest[end_paren + 1..].trim();
                    (Some(priority_str.to_string()), remaining.to_string())
                }
            } else {
                (None, rest.to_string())
            }
        } else {
            (None, rest.to_string())
        };

        (urgency, priority, task_text)
    }

    /// Extract all tasks from a note
    pub fn extract_from_note(note: &Note) -> Vec<Task> {
        let mut tasks = Vec::new();
        let mut task_index = 0;

        // Parse the markdown to find list items (don't use ENABLE_TASKLISTS so we get raw text)
        let options = Options::empty();
        let parser = Parser::new_ext(&note.content, options);
        let mut in_list_item = false;
        let mut item_text = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Item) => {
                    in_list_item = true;
                    item_text.clear();
                }
                Event::Text(text) if in_list_item => {
                    item_text.push_str(&text);
                }
                Event::End(TagEnd::Item) if in_list_item => {
                    // Check if this list item is a task (starts with [X])
                    let trimmed = item_text.trim();
                    if let Some(rest) = trimmed.strip_prefix('[') {
                        if let Some(close_bracket) = rest.find(']') {
                            if close_bracket == 1 {
                                // We have a checkbox: [X]
                                let checkbox_char = rest.chars().next().unwrap();
                                if let Some(status) = TaskStatus::from_checkbox_char(checkbox_char) {
                                    task_index += 1;
                                    let task_text = rest[close_bracket + 1..].trim();

                                    let (urgency, priority, rest) = Self::parse_urgency_and_priority(task_text);
                                    let (tags, text) = Self::parse_tags(&rest);

                                    tasks.push(Task {
                                        note_path: note.path.clone(),
                                        note_title: note.title.clone(),
                                        index: task_index,
                                        status,
                                        text,
                                        priority,
                                        urgency,
                                        tags,
                                    });
                                }
                            }
                        }
                    }

                    in_list_item = false;
                }
                _ => {}
            }
        }

        tasks
    }

    /// Reconstruct a markdown task line from this Task
    pub fn to_markdown_line(&self) -> String {
        let mut line = String::from("- [ ] ");

        // Add urgency
        if let Some(urgency) = &self.urgency {
            line.push_str(urgency);
            line.push(' ');
        }

        // Add priority
        if let Some(priority) = &self.priority {
            line.push('(');
            line.push_str(priority);
            line.push_str(") ");
        }

        // Add task text
        line.push_str(&self.text);

        // Add tags
        if !self.tags.is_empty() {
            line.push(' ');
            for tag in &self.tags {
                line.push('@');
                line.push_str(tag);
                line.push(' ');
            }
            // Remove trailing space
            line.pop();
        }

        line
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
    render_template_with_tasks(template_content, title, None)
}

/// Render a template with optional migrated tasks section
pub(crate) fn render_template_with_tasks(
    template_content: &str,
    title: &str,
    migrated_tasks: Option<&str>,
) -> String {
    let now = Utc::now();
    let date = now.format("%Y-%m-%d").to_string();
    let datetime = now.to_rfc3339();

    let migrated_section = migrated_tasks.unwrap_or("");

    template_content
        .replace("{{title}}", title)
        .replace("{{date}}", &date)
        .replace("{{datetime}}", &datetime)
        .replace("{{migrated_tasks}}", migrated_section)
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
        assert_eq!(tasks[0].status, TaskStatus::Uncompleted);
        assert_eq!(tasks[0].priority, None);
        assert_eq!(tasks[1].text, "Completed task");
        assert_eq!(tasks[1].status, TaskStatus::Completed);
        assert_eq!(tasks[1].priority, None);
        assert_eq!(tasks[2].text, "Another task");
        assert_eq!(tasks[2].status, TaskStatus::Uncompleted);
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
        assert_eq!(tasks[3].status, TaskStatus::Completed);

        assert_eq!(tasks[4].priority, Some("C".to_string()));
        assert_eq!(tasks[4].text, "Low priority task");
    }

    #[test]
    fn test_extract_migrated_tasks() {
        let content = r#"---
tags: [test]
---

# My Note

## Tasks
- [ ] Uncompleted task
- [x] Completed task
- [>] Migrated task
- [>] !!! (A) Migrated with priority and urgency @backend

"#;

        let note = Note::parse(Path::new("test.md"), content).unwrap();
        let tasks = Task::extract_from_note(&note);

        assert_eq!(tasks.len(), 4);

        assert_eq!(tasks[0].status, TaskStatus::Uncompleted);
        assert_eq!(tasks[0].text, "Uncompleted task");

        assert_eq!(tasks[1].status, TaskStatus::Completed);
        assert_eq!(tasks[1].text, "Completed task");

        assert_eq!(tasks[2].status, TaskStatus::Migrated);
        assert_eq!(tasks[2].text, "Migrated task");

        assert_eq!(tasks[3].status, TaskStatus::Migrated);
        assert_eq!(tasks[3].text, "Migrated with priority and urgency");
        assert_eq!(tasks[3].priority, Some("A".to_string()));
        assert_eq!(tasks[3].urgency, Some("!!!".to_string()));
        assert_eq!(tasks[3].tags, vec!["backend"]);
    }

    #[test]
    fn test_reconstruct_task_line() {
        let task = Task {
            note_path: PathBuf::from("test.md"),
            note_title: "Test".to_string(),
            index: 1,
            status: TaskStatus::Uncompleted,
            text: "Simple task".to_string(),
            priority: None,
            urgency: None,
            tags: vec![],
        };
        assert_eq!(task.to_markdown_line(), "- [ ] Simple task");

        let task_with_priority = Task {
            note_path: PathBuf::from("test.md"),
            note_title: "Test".to_string(),
            index: 1,
            status: TaskStatus::Uncompleted,
            text: "High priority task".to_string(),
            priority: Some("A".to_string()),
            urgency: None,
            tags: vec![],
        };
        assert_eq!(task_with_priority.to_markdown_line(), "- [ ] (A) High priority task");

        let task_with_all = Task {
            note_path: PathBuf::from("test.md"),
            note_title: "Test".to_string(),
            index: 1,
            status: TaskStatus::Uncompleted,
            text: "Complete task".to_string(),
            priority: Some("B".to_string()),
            urgency: Some("!!!".to_string()),
            tags: vec!["backend".to_string(), "urgent".to_string()],
        };
        assert_eq!(task_with_all.to_markdown_line(), "- [ ] !!! (B) Complete task @backend @urgent");
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

    #[test]
    fn test_parse_urgency_only() {
        let content = "- [ ] !!! Fix critical bug";
        let note_path = PathBuf::from("test.md");
        let note = Note::parse(&note_path, &format!("# Test\n\n{}", content)).unwrap();
        let tasks = Task::extract_from_note(&note);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].urgency, Some("!!!".to_string()));
        assert_eq!(tasks[0].priority, None);
        assert_eq!(tasks[0].text, "Fix critical bug");
    }

    #[test]
    fn test_parse_urgency_and_priority() {
        let content = "- [ ] !! (B) Moderate task";
        let note_path = PathBuf::from("test.md");
        let note = Note::parse(&note_path, &format!("# Test\n\n{}", content)).unwrap();
        let tasks = Task::extract_from_note(&note);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].urgency, Some("!!".to_string()));
        assert_eq!(tasks[0].priority, Some("B".to_string()));
        assert_eq!(tasks[0].text, "Moderate task");
    }

    #[test]
    fn test_parse_no_space_after_urgency() {
        let content = "- [ ] !!!(A) Task";
        let note_path = PathBuf::from("test.md");
        let note = Note::parse(&note_path, &format!("# Test\n\n{}", content)).unwrap();
        let tasks = Task::extract_from_note(&note);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].urgency, None);
        assert_eq!(tasks[0].priority, None);
        assert_eq!(tasks[0].text, "!!!(A) Task");
    }

    #[test]
    fn test_parse_exclamation_in_text() {
        let content = "- [ ] Do this now!";
        let note_path = PathBuf::from("test.md");
        let note = Note::parse(&note_path, &format!("# Test\n\n{}", content)).unwrap();
        let tasks = Task::extract_from_note(&note);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].urgency, None);
        assert_eq!(tasks[0].text, "Do this now!");
    }

    #[test]
    fn test_parse_empty_priority() {
        let content = "- [ ] !!! () Task with empty priority";
        let note_path = PathBuf::from("test.md");
        let note = Note::parse(&note_path, &format!("# Test\n\n{}", content)).unwrap();
        let tasks = Task::extract_from_note(&note);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].urgency, Some("!!!".to_string()));
        assert_eq!(tasks[0].priority, None);
        assert_eq!(tasks[0].text, "Task with empty priority");
    }
}
