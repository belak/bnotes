//! Health check module for analyzing notes
//!
//! This module provides functionality to check for common issues in a note collection
//! such as broken links, missing metadata, and duplicate titles.

use crate::link::LinkGraph;
use crate::repository::Note;
use std::collections::HashMap;

/// Results of a health check operation
#[derive(Debug, Clone)]
pub struct HealthReport {
    /// Broken wiki links: note title -> list of broken link targets
    pub broken_links: HashMap<String, Vec<String>>,
    /// Notes without any tags
    pub notes_without_tags: Vec<String>,
    /// Notes missing frontmatter (no tags, no dates)
    pub notes_without_frontmatter: Vec<String>,
    /// Duplicate titles: lowercase title -> list of file paths
    pub duplicate_titles: HashMap<String, Vec<String>>,
    /// Orphaned notes (no links and no tags)
    pub orphaned_notes: Vec<String>,
}

impl HealthReport {
    /// Check if the report has any issues
    pub fn has_issues(&self) -> bool {
        !self.broken_links.is_empty()
            || !self.notes_without_tags.is_empty()
            || !self.notes_without_frontmatter.is_empty()
            || !self.duplicate_titles.is_empty()
            || !self.orphaned_notes.is_empty()
    }

    /// Count total number of issues
    pub fn issue_count(&self) -> usize {
        self.broken_links.len()
            + self.notes_without_tags.len()
            + self.notes_without_frontmatter.len()
            + self.duplicate_titles.len()
            + self.orphaned_notes.len()
    }
}

/// Run health checks on a collection of notes
pub(crate) fn check_health(notes: &[Note]) -> HealthReport {
    let graph = LinkGraph::build(notes);

    // Check for broken wiki links
    let broken_links = graph.broken_links(notes);

    // Check for notes without tags
    let notes_without_tags: Vec<String> = notes
        .iter()
        .filter(|n| n.tags.is_empty())
        .map(|n| n.title.clone())
        .collect();

    // Check for notes missing frontmatter
    let notes_without_frontmatter: Vec<String> = notes
        .iter()
        .filter(|n| n.tags.is_empty() && n.created.is_none() && n.updated.is_none())
        .map(|n| n.title.clone())
        .collect();

    // Check for multiple notes with the same title
    let mut title_counts: HashMap<String, Vec<String>> = HashMap::new();
    for note in notes {
        title_counts
            .entry(note.title.to_lowercase())
            .or_default()
            .push(note.path.display().to_string());
    }

    let duplicate_titles: HashMap<String, Vec<String>> = title_counts
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .collect();

    // Check for orphaned notes (no links and no tags)
    let all_titles: Vec<String> = notes.iter().map(|n| n.title.clone()).collect();
    let orphaned = graph.orphaned_notes(&all_titles);

    let orphaned_notes: Vec<String> = orphaned
        .into_iter()
        .filter(|title| {
            notes
                .iter()
                .find(|n| &n.title == title)
                .map(|n| n.tags.is_empty())
                .unwrap_or(true)
        })
        .collect();

    HealthReport {
        broken_links,
        notes_without_tags,
        notes_without_frontmatter,
        duplicate_titles,
        orphaned_notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{MemoryStorage, Storage};
    use std::path::Path;

    #[test]
    fn test_health_check_no_issues() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(
                Path::new("note1.md"),
                r#"---
tags: [test]
---

# Note 1

Content with [[Note 2]] link"#,
            )
            .unwrap();
        storage
            .write(
                Path::new("note2.md"),
                r#"---
tags: [test]
---

# Note 2

Content"#,
            )
            .unwrap();

        let notes = vec![
            Note::parse(Path::new("note1.md"), &storage.read_to_string(Path::new("note1.md")).unwrap()).unwrap(),
            Note::parse(Path::new("note2.md"), &storage.read_to_string(Path::new("note2.md")).unwrap()).unwrap(),
        ];

        let report = check_health(&notes);
        assert!(!report.has_issues());
        assert_eq!(report.issue_count(), 0);
    }

    #[test]
    fn test_health_check_broken_links() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(
                Path::new("note1.md"),
                r#"---
tags: [test]
---

# Note 1

Content with [[Missing Note]] link"#,
            )
            .unwrap();

        let notes = vec![
            Note::parse(Path::new("note1.md"), &storage.read_to_string(Path::new("note1.md")).unwrap()).unwrap(),
        ];

        let report = check_health(&notes);
        assert!(report.has_issues());
        assert_eq!(report.broken_links.len(), 1);
        assert!(report.broken_links.contains_key("Note 1"));
    }

    #[test]
    fn test_health_check_missing_frontmatter() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(Path::new("note1.md"), "# Note 1\n\nContent without frontmatter")
            .unwrap();

        let notes = vec![
            Note::parse(Path::new("note1.md"), &storage.read_to_string(Path::new("note1.md")).unwrap()).unwrap(),
        ];

        let report = check_health(&notes);
        assert!(report.has_issues());
        assert_eq!(report.notes_without_frontmatter.len(), 1);
        assert_eq!(report.notes_without_tags.len(), 1);
    }

    #[test]
    fn test_health_check_duplicate_titles() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(
                Path::new("note1.md"),
                r#"---
tags: [test]
---

# Same Title"#,
            )
            .unwrap();
        storage
            .write(
                Path::new("subfolder/note2.md"),
                r#"---
tags: [test]
---

# Same Title"#,
            )
            .unwrap();

        let notes = vec![
            Note::parse(Path::new("note1.md"), &storage.read_to_string(Path::new("note1.md")).unwrap()).unwrap(),
            Note::parse(Path::new("subfolder/note2.md"), &storage.read_to_string(Path::new("subfolder/note2.md")).unwrap()).unwrap(),
        ];

        let report = check_health(&notes);
        assert!(report.has_issues());
        assert_eq!(report.duplicate_titles.len(), 1);
    }
}
