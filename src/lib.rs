//! BNotes - A note-taking and task management library
//!
//! This library provides the core business logic for managing notes, tasks,
//! and periodic notes. It separates business logic from CLI concerns like I/O,
//! formatting, and editor integration.
//!
//! # Example
//!
//! ```no_run
//! use bnotes::{BNotes, RealStorage};
//! use std::path::PathBuf;
//!
//! let notes_dir = PathBuf::from("~/notes");
//! let storage = Box::new(RealStorage::new(notes_dir));
//! let bnotes = BNotes::with_defaults(storage);
//!
//! // Search for notes
//! let results = bnotes.search("rust").unwrap();
//! for search_match in results {
//!     println!("{}", search_match.note.title);
//! }
//! ```

pub mod config;
pub mod note;
pub mod periodic;
pub mod repository;
pub mod storage;
mod templates;

use std::collections::HashSet;
use std::path::PathBuf;

/// Result type alias using anyhow::Error
pub type Result<T> = std::result::Result<T, anyhow::Error>;

/// Main library API for BNotes
///
/// This struct provides the primary interface for interacting with notes.
/// It manages configuration and delegates operations to the repository layer.
pub struct BNotes {
    config: config::LibraryConfig,
    repo: repository::Repository,
}

impl BNotes {
    /// Create a new BNotes instance with the given configuration and storage
    pub fn new(config: config::LibraryConfig, storage: Box<dyn storage::Storage>) -> Self {
        let repo = repository::Repository::new(storage);
        Self { config, repo }
    }

    /// Create BNotes by loading configuration from storage
    pub fn from_storage(storage: Box<dyn storage::Storage>) -> Result<Self> {
        let config = config::LibraryConfig::load(&*storage)?;
        Ok(Self::new(config, storage))
    }

    /// Create BNotes with default configuration
    pub fn with_defaults(storage: Box<dyn storage::Storage>) -> Self {
        let config = config::LibraryConfig::load_or_default(&*storage);
        Self::new(config, storage)
    }

    /// Search notes by query (case-insensitive substring matching)
    pub fn search(&self, query: &str) -> Result<Vec<repository::SearchMatch>> {
        self.repo.search(query)
    }

    /// List all notes, optionally filtered by tags
    pub fn list_notes(&self, tags: &[String]) -> Result<Vec<note::Note>> {
        if tags.is_empty() {
            self.repo.discover_notes()
        } else {
            self.repo.filter_by_tags(tags)
        }
    }

    /// Find a note by title (case-insensitive)
    pub fn find_note_by_title(&self, title: &str) -> Result<Vec<note::Note>> {
        self.repo.find_by_title(title)
    }

    /// Get inbound and outbound links for a note
    ///
    /// Returns (outbound_links, inbound_links) where each is a set of note titles
    pub fn get_note_links(&self, title: &str) -> Result<(HashSet<String>, HashSet<String>)> {
        let all_notes = self.repo.discover_notes()?;
        let graph = repository::LinkGraph::build(&all_notes);

        let outbound = graph
            .outbound
            .get(title)
            .cloned()
            .unwrap_or_default();

        let inbound = graph
            .inbound
            .get(title)
            .cloned()
            .unwrap_or_default();

        Ok((outbound, inbound))
    }

    /// Get the full link graph for all notes
    pub fn get_link_graph(&self) -> Result<repository::LinkGraph> {
        let all_notes = self.repo.discover_notes()?;
        Ok(repository::LinkGraph::build(&all_notes))
    }

    /// Create a new note with the given title and optional template
    ///
    /// Returns the relative path to the created note
    pub fn create_note(&self, title: &str, template_name: Option<&str>) -> Result<std::path::PathBuf> {
        let template_dir = self.config.template_dir_path();
        self.repo.create_note(title, template_dir, template_name)
    }

    /// List all tasks, optionally filtered by tags and status
    ///
    /// Status can be Some("open"), Some("done"), or None for all tasks
    pub fn list_tasks(&self, tags: &[String], status: Option<&str>) -> Result<Vec<note::Task>> {
        // Get notes, optionally filtered by tags
        let notes = if tags.is_empty() {
            self.repo.discover_notes()?
        } else {
            self.repo.filter_by_tags(tags)?
        };

        // Extract tasks from all notes
        let mut tasks = note::extract_tasks_from_notes(&notes);

        // Filter by status if specified
        if let Some(status_filter) = status {
            let filter_open = status_filter.eq_ignore_ascii_case("open");
            let filter_done = status_filter.eq_ignore_ascii_case("done");

            if !filter_open && !filter_done {
                anyhow::bail!("Invalid status filter: {}. Use 'open' or 'done'.", status_filter);
            }

            tasks.retain(|task| {
                if filter_open {
                    !task.completed
                } else {
                    task.completed
                }
            });
        }

        // Sort based on configuration
        match self.config.task.sort_order {
            config::TaskSortOrder::PriorityId => {
                // Sort by priority (ascending, None last), then by ID
                tasks.sort_by(|a, b| {
                    match (&a.priority, &b.priority) {
                        (Some(p1), Some(p2)) => {
                            // Compare priorities as strings (A < B < C, etc.)
                            let cmp = p1.cmp(p2);
                            if cmp != std::cmp::Ordering::Equal {
                                cmp
                            } else {
                                a.id().cmp(&b.id())
                            }
                        }
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.id().cmp(&b.id()),
                    }
                });
            }
            config::TaskSortOrder::Id => {
                // Sort by ID only
                tasks.sort_by(|a, b| a.id().cmp(&b.id()));
            }
        }

        Ok(tasks)
    }

    /// Get a specific task by its ID (format: "filename#index")
    ///
    /// Returns (task, note) tuple
    pub fn get_task(&self, task_id: &str) -> Result<(note::Task, note::Note)> {
        // Parse task ID (format: "filename#index")
        let parts: Vec<&str> = task_id.split('#').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid task ID format. Expected 'filename#index'");
        }

        let filename = parts[0];
        let index: usize = parts[1]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid task index: {}", parts[1]))?;

        // Find the note
        let notes = self.repo.discover_notes()?;
        let note = notes
            .iter()
            .find(|n| {
                n.path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s == filename)
                    .unwrap_or(false)
            })
            .ok_or_else(|| anyhow::anyhow!("Note not found: {}", filename))?;

        // Extract tasks from the note
        let tasks = note::Task::extract_from_note(note);

        // Find the specific task
        let task = tasks
            .into_iter()
            .find(|t| t.index == index)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

        Ok((task, note.clone()))
    }

    /// Open or create a periodic note for a given period
    ///
    /// Returns the relative path to the periodic note
    pub fn open_periodic<P: periodic::PeriodType>(
        &self,
        period: P,
        template_name: Option<&str>,
    ) -> Result<PathBuf> {
        let note_path = PathBuf::from(period.filename());

        // If note already exists, just return the path
        if self.repo.storage.exists(&note_path) {
            return Ok(note_path);
        }

        // Create the note
        let template_dir = self.config.template_dir_path();

        // Determine which template to use
        let template = if let Some(name) = template_name {
            name.to_string()
        } else {
            // Get configured template based on period type
            match P::template_name() {
                "daily" => self.config.periodic.daily_template.clone(),
                "weekly" => self.config.periodic.weekly_template.clone(),
                "quarterly" => self.config.periodic.quarterly_template.clone(),
                _ => format!("{}.md", P::template_name()),
            }
        };

        let template_path = template_dir.join(&template);

        // Generate content
        let template_content = if self.repo.storage.exists(&template_path) {
            self.repo.storage.read_to_string(&template_path)?
        } else {
            // Fall back to embedded template
            let template_name = P::template_name();
            templates::get_embedded_template(template_name)
                .unwrap_or("# {{title}}\n\n")
                .to_string()
        };

        let content = note::render_template(&template_content, &period.identifier());

        // Write note
        self.repo.storage.write(&note_path, &content)?;

        Ok(note_path)
    }

    /// List all periodic notes of a given type
    ///
    /// Returns a list of periods that have notes
    pub fn list_periodic<P: periodic::PeriodType>(&self) -> Result<Vec<P>> {
        let mut periods: Vec<P> = Vec::new();

        // Scan notes directory for matching files
        let entries = self.repo.storage.read_dir(std::path::Path::new(""))?;

        for path in entries {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // Try to parse as this period type
                if let Ok(period) = P::from_date_str(stem) {
                    periods.push(period);
                }
            }
        }

        // Sort by identifier (chronological)
        periods.sort_by_key(|a| a.identifier());

        Ok(periods)
    }

    /// Navigate to previous or next period and open/create the note
    ///
    /// Direction: "prev" or "next"
    /// Returns the relative path to the periodic note
    pub fn navigate_periodic<P: periodic::PeriodType>(
        &self,
        direction: &str,
        template_name: Option<&str>,
    ) -> Result<PathBuf> {
        let current = P::current();
        let period = match direction {
            "prev" => current.prev(),
            "next" => current.next(),
            _ => anyhow::bail!("Invalid direction: {}. Use 'prev' or 'next'.", direction),
        };

        self.open_periodic(period, template_name)
    }

    /// Run health checks on the note collection
    ///
    /// Returns a report of potential issues including broken links, missing metadata,
    /// duplicate titles, and orphaned notes
    pub fn check_health(&self) -> Result<repository::HealthReport> {
        let notes = self.repo.discover_notes()?;
        Ok(repository::check_health(&notes))
    }
}

// Re-export main types for convenience
pub use config::{LibraryConfig, PeriodicConfig};
pub use note::{Frontmatter, Note, Task};
pub use periodic::{Daily, PeriodType, Quarterly, Weekly};
pub use repository::{HealthReport, LinkGraph, MatchLocation, SearchMatch};
pub use storage::{MemoryStorage, RealStorage, Storage};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_bnotes_with_defaults() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(Path::new("test.md"), "# Test Note\n\nContent")
            .unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let notes = bnotes.list_notes(&[]).unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Test Note");
    }

    #[test]
    fn test_bnotes_search() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(
                Path::new("work.md"),
                "# Work Note\n\nDiscuss the project timeline",
            )
            .unwrap();
        storage
            .write(
                Path::new("personal.md"),
                "# Personal Note\n\nBuy groceries",
            )
            .unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let results = bnotes.search("project").unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].note.title, "Work Note");
        assert!(!results[0].locations.is_empty());
    }

    #[test]
    fn test_bnotes_list_with_tags() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(
                Path::new("work.md"),
                r#"---
tags: [work, important]
---

# Work Note"#,
            )
            .unwrap();
        storage
            .write(
                Path::new("personal.md"),
                r#"---
tags: [personal]
---

# Personal Note"#,
            )
            .unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let results = bnotes.list_notes(&["work".to_string()]).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Work Note");
    }

    #[test]
    fn test_bnotes_find_by_title() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(Path::new("test.md"), "# Test Note\n\nContent")
            .unwrap();
        storage
            .write(Path::new("other.md"), "# Other Note\n\nContent")
            .unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let results = bnotes.find_note_by_title("test note").unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Test Note");
    }

    #[test]
    fn test_bnotes_from_storage_with_config() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(
                Path::new(".bnotes/config.toml"),
                r#"
template_dir = "my-templates"

[periodic]
daily_template = "custom-daily.md"
"#,
            )
            .unwrap();
        storage
            .write(Path::new("note.md"), "# Note\n\nContent")
            .unwrap();

        let bnotes = BNotes::from_storage(storage).unwrap();
        let notes = bnotes.list_notes(&[]).unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(bnotes.config.template_dir, Path::new("my-templates"));
    }

    #[test]
    fn test_bnotes_create_note_without_template() {
        let storage = Box::new(MemoryStorage::new());
        let bnotes = BNotes::with_defaults(storage);

        let path = bnotes.create_note("My Test Note", None).unwrap();

        assert_eq!(path, Path::new("my-test-note.md"));

        // Verify the note was created and can be read
        let notes = bnotes.list_notes(&[]).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "My Test Note");
        assert!(notes[0].content.contains("# My Test Note"));
    }

    #[test]
    fn test_bnotes_create_note_with_template() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(
                Path::new(".btools/templates/daily.md"),
                r#"---
tags: [daily]
created: {{datetime}}
---

# {{title}}

## Tasks
- [ ]

## Notes
"#,
            )
            .unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let path = bnotes.create_note("2024-01-15", Some("daily")).unwrap();

        assert_eq!(path, Path::new("2024-01-15.md"));

        // Verify the note was created with template content
        let notes = bnotes.list_notes(&[]).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "2024-01-15");
        assert!(notes[0].content.contains("## Tasks"));
        assert!(notes[0].content.contains("## Notes"));
        assert_eq!(notes[0].tags, vec!["daily"]);
    }

    #[test]
    fn test_bnotes_create_note_duplicate_error() {
        let storage = Box::new(MemoryStorage::new());
        let bnotes = BNotes::with_defaults(storage);

        bnotes.create_note("Test", None).unwrap();
        let result = bnotes.create_note("Test", None);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_bnotes_create_note_template_not_found() {
        let storage = Box::new(MemoryStorage::new());
        let bnotes = BNotes::with_defaults(storage);

        let result = bnotes.create_note("Test", Some("nonexistent"));

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_bnotes_create_note_with_default_template() {
        let storage = Box::new(MemoryStorage::new());
        storage
            .write(
                Path::new(".btools/templates/default.md"),
                r#"---
tags: []
created: {{datetime}}
---

# {{title}}
"#,
            )
            .unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let path = bnotes.create_note("Test Note", None).unwrap();

        assert_eq!(path, Path::new("test-note.md"));

        // Verify the note was created with default template
        let notes = bnotes.list_notes(&[]).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Test Note");
        assert!(notes[0].content.contains("# Test Note"));
    }

    #[test]
    fn test_bnotes_create_note_with_embedded_default_template() {
        // No templates in storage - should use embedded default
        let storage = Box::new(MemoryStorage::new());
        let bnotes = BNotes::with_defaults(storage);

        let path = bnotes.create_note("Test Note", None).unwrap();

        assert_eq!(path, Path::new("test-note.md"));

        // Verify the note was created with embedded default template
        let notes = bnotes.list_notes(&[]).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Test Note");
        assert!(notes[0].content.contains("# Test Note"));
    }

    #[test]
    fn test_bnotes_create_note_with_embedded_template() {
        // No templates in storage - should use embedded template
        let storage = Box::new(MemoryStorage::new());
        let bnotes = BNotes::with_defaults(storage);

        let path = bnotes.create_note("Test Daily", Some("daily")).unwrap();

        assert_eq!(path, Path::new("test-daily.md"));

        // Verify the note was created with embedded daily template
        let notes = bnotes.list_notes(&[]).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Test Daily");
        assert!(notes[0].content.contains("## Tasks"));
        assert_eq!(notes[0].tags, vec!["daily"]);
    }

    #[test]
    fn test_bnotes_list_tasks_sorted_by_priority() {
        let storage = Box::new(MemoryStorage::new());

        // Create notes with various priority tasks
        storage
            .write(
                Path::new("note1.md"),
                r#"---
title: Note 1
---

# Note 1

- [ ] (B) Medium priority task
- [ ] (A) High priority task
- [ ] Task without priority
"#,
            )
            .unwrap();

        storage
            .write(
                Path::new("note2.md"),
                r#"---
title: Note 2
---

# Note 2

- [ ] (C) Low priority task
- [ ] (A) Another high priority
"#,
            )
            .unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let tasks = bnotes.list_tasks(&[], None).unwrap();

        // Should be sorted by priority (A, B, C) then by ID
        assert_eq!(tasks.len(), 5);

        // Both A priority tasks should come first
        assert_eq!(tasks[0].priority, Some("A".to_string()));
        assert_eq!(tasks[1].priority, Some("A".to_string()));

        // B priority next
        assert_eq!(tasks[2].priority, Some("B".to_string()));

        // C priority next
        assert_eq!(tasks[3].priority, Some("C".to_string()));

        // No priority last
        assert_eq!(tasks[4].priority, None);
        assert_eq!(tasks[4].text, "Task without priority");
    }

    #[test]
    fn test_bnotes_list_tasks_sorted_by_id() {
        let storage = Box::new(MemoryStorage::new());

        // Create notes with various priority tasks
        storage
            .write(
                Path::new("a-note.md"),
                r#"---
title: A Note
---

# A Note

- [ ] (C) Low priority task from a-note
"#,
            )
            .unwrap();

        storage
            .write(
                Path::new("b-note.md"),
                r#"---
title: B Note
---

# B Note

- [ ] (A) High priority task from b-note
"#,
            )
            .unwrap();

        // Set config to sort by ID only
        storage
            .write(
                Path::new(".bnotes/config.toml"),
                r#"
[task]
sort_order = "id"
"#,
            )
            .unwrap();

        let config = config::LibraryConfig::load(&*storage).unwrap();
        let bnotes = BNotes::new(config, storage);
        let tasks = bnotes.list_tasks(&[], None).unwrap();

        // Should be sorted by ID (filename#index), ignoring priority
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id(), "a-note#1");
        assert_eq!(tasks[0].priority, Some("C".to_string()));
        assert_eq!(tasks[1].id(), "b-note#1");
        assert_eq!(tasks[1].priority, Some("A".to_string()));
    }
}
