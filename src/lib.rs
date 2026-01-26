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

use anyhow::Context;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Result type alias using anyhow::Error
pub type Result<T> = std::result::Result<T, anyhow::Error>;

/// Capture the current state of a note file for change detection
/// Returns modification time that can be compared to detect changes
pub fn capture_note_state(path: &Path) -> Result<SystemTime> {
    let metadata = std::fs::metadata(path)?;
    metadata.modified().context("Failed to get modification time")
}

/// Task sort order - comma-separated list of fields
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSortOrder {
    fields: Vec<SortField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortField {
    Urgency,
    Priority,
    Id,
}

impl TaskSortOrder {
    /// Parse sort order from comma-separated string
    pub fn parse(s: &str) -> Result<Self> {
        let fields: Result<Vec<_>> = s
            .split(',')
            .map(|f| match f.trim() {
                "urgency" => Ok(SortField::Urgency),
                "priority" => Ok(SortField::Priority),
                "id" => Ok(SortField::Id),
                unknown => anyhow::bail!("Unknown sort field: {}. Valid fields: urgency, priority, id", unknown),
            })
            .collect();

        Ok(TaskSortOrder { fields: fields? })
    }
}

impl Default for TaskSortOrder {
    fn default() -> Self {
        Self {
            fields: vec![SortField::Urgency, SortField::Priority, SortField::Id]
        }
    }
}

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

    /// Compare urgency levels: !!! < !! < ! < None
    fn compare_urgency(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
        match (a, b) {
            (Some(a_urg), Some(b_urg)) => {
                let a_val = match a_urg.as_str() {
                    "!!!" => 1,
                    "!!" => 2,
                    "!" => 3,
                    _ => 4,
                };
                let b_val = match b_urg.as_str() {
                    "!!!" => 1,
                    "!!" => 2,
                    "!" => 3,
                    _ => 4,
                };
                a_val.cmp(&b_val)
            }
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    }

    /// Compare priority levels: A < B < C < ... < None
    fn compare_priority(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
        match (a, b) {
            (Some(a_pri), Some(b_pri)) => a_pri.cmp(b_pri),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
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
    /// Status can be Some("open"), Some("completed"), Some("migrated"), Some("all"), or None for all tasks
    pub fn list_tasks(&self, tags: &[String], status: Option<&str>, sort_order: TaskSortOrder) -> Result<Vec<note::Task>> {
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
            if status_filter.eq_ignore_ascii_case("all") {
                // No filtering
            } else if status_filter.eq_ignore_ascii_case("open") {
                tasks.retain(|task| task.status == note::TaskStatus::Uncompleted);
            } else if status_filter.eq_ignore_ascii_case("completed") || status_filter.eq_ignore_ascii_case("done") {
                tasks.retain(|task| task.status == note::TaskStatus::Completed);
            } else if status_filter.eq_ignore_ascii_case("migrated") {
                tasks.retain(|task| task.status == note::TaskStatus::Migrated);
            } else {
                anyhow::bail!("Invalid status filter: {}. Use 'open', 'completed', 'migrated', or 'all'.", status_filter);
            }
        }

        // Sort based on provided sort order
        tasks.sort_by(|a, b| {
            for field in &sort_order.fields {
                let cmp = match field {
                    SortField::Urgency => Self::compare_urgency(&a.urgency, &b.urgency),
                    SortField::Priority => Self::compare_priority(&a.priority, &b.priority),
                    SortField::Id => {
                        // Sort by note title first, then by index
                        a.note_title.cmp(&b.note_title)
                            .then_with(|| a.index.cmp(&b.index))
                    }
                };
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            std::cmp::Ordering::Equal
        });

        Ok(tasks)
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

    /// Find the most recent weekly note before the given period
    fn find_previous_weekly_note(&self, period: periodic::Weekly) -> Option<PathBuf> {
        let mut current = period.prev();

        // Search backwards for up to 52 weeks (one year)
        for _ in 0..52 {
            let filename = current.filename();
            let note_path = Path::new(&filename);

            if self.repo.storage.exists(note_path) {
                return Some(note_path.to_path_buf());
            }

            current = current.prev();
        }

        None
    }

    /// Mark all uncompleted tasks as migrated in a note
    fn mark_tasks_migrated(&self, note_path: &Path) -> Result<()> {
        let content = self.repo.storage.read_to_string(note_path)?;
        let updated_content = content.replace("- [ ]", "- [>]");
        self.repo.storage.write(note_path, &updated_content)?;
        Ok(())
    }

    /// Build the migrated tasks section from a list of tasks
    /// Returns just the task list without heading (heading should be in template)
    fn build_migrated_section(tasks: &[note::Task]) -> String {
        tasks
            .iter()
            .map(|task| task.to_markdown_line())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Create a weekly note with optional task migration from the previous week
    ///
    /// If the weekly note is for the current week and doesn't exist yet, prompts
    /// to migrate uncompleted tasks from the most recent previous weekly note.
    ///
    /// Returns (note_path, migrated_count) where migrated_count is the number of tasks migrated
    pub fn create_weekly_with_migration(
        &self,
        period: periodic::Weekly,
        template_name: Option<&str>,
        should_prompt: bool,
    ) -> Result<(PathBuf, usize)> {
        use std::io::{self, Write};

        let filename = period.filename();
        let note_path = PathBuf::from(filename);

        // If note already exists, just return it
        if self.repo.storage.exists(&note_path) {
            return Ok((note_path, 0));
        }

        // Check if this is the current week or if we're in non-interactive mode (testing)
        let is_current_week = period == periodic::Weekly::current();
        let should_migrate_check = is_current_week || !should_prompt;

        // Find previous weekly note and extract uncompleted tasks
        let (previous_note, uncompleted_tasks) = if should_migrate_check {
            if let Some(prev_path) = self.find_previous_weekly_note(period) {
                // Read and parse the previous note
                let content = self.repo.storage.read_to_string(&prev_path)?;
                let prev_note = note::Note::parse(&prev_path, &content)?;
                let all_tasks = note::Task::extract_from_note(&prev_note);
                let uncompleted: Vec<_> = all_tasks
                    .into_iter()
                    .filter(|t| t.status == note::TaskStatus::Uncompleted)
                    .collect();

                (Some(prev_path), uncompleted)
            } else {
                (None, Vec::new())
            }
        } else {
            (None, Vec::new())
        };

        // Prompt for migration if needed
        let should_migrate = if should_prompt && !uncompleted_tasks.is_empty() {
            if let Some(ref prev_path) = previous_note {
                let prev_identifier = prev_path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("previous week");

                print!("Found {} uncompleted tasks from {}. Migrate to {}? [Y/n] ",
                    uncompleted_tasks.len(), prev_identifier, period.identifier());
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim().to_lowercase();

                input.is_empty() || input == "y" || input == "yes"
            } else {
                false
            }
        } else {
            !uncompleted_tasks.is_empty()
        };

        // Create note content
        let title = period.identifier();
        let template_path = if let Some(name) = template_name {
            name.to_string()
        } else {
            self.config.periodic.weekly_template.clone()
        };

        let template_dir = self.config.template_dir_path();
        let full_template_path = template_dir.join(&template_path);

        // Build migrated tasks section if migrating
        let (content, migrated_count) = if should_migrate {
            let migrated_section = Self::build_migrated_section(&uncompleted_tasks);
            let count = uncompleted_tasks.len();

            let content = if self.repo.storage.exists(&full_template_path) {
                let template_content = self.repo.storage.read_to_string(&full_template_path)?;
                note::render_template_with_tasks(&template_content, &title, Some(&migrated_section))
            } else {
                // Use embedded default template
                let embedded = templates::get_embedded_template(periodic::Weekly::template_name())
                    .unwrap_or("# {{title}}\n\n")
                    .to_string();
                note::render_template_with_tasks(&embedded, &title, Some(&migrated_section))
            };

            (content, count)
        } else {
            let content = if self.repo.storage.exists(&full_template_path) {
                let template_content = self.repo.storage.read_to_string(&full_template_path)?;
                note::render_template(&template_content, &title)
            } else {
                // Use embedded default template
                let embedded = templates::get_embedded_template(periodic::Weekly::template_name())
                    .unwrap_or("# {{title}}\n\n")
                    .to_string();
                note::render_template(&embedded, &title)
            };

            (content, 0)
        };

        // Mark tasks as migrated in the previous note if migration happened
        if migrated_count > 0 {
            if let Some(prev_path) = previous_note {
                self.mark_tasks_migrated(&prev_path)?;
            }
        }

        // Write the new note
        self.repo.storage.write(&note_path, &content)?;

        Ok((note_path, migrated_count))
    }

    /// Run health checks on the note collection
    ///
    /// Returns a report of potential issues including broken links, missing metadata,
    /// duplicate titles, and orphaned notes
    pub fn check_health(&self) -> Result<repository::HealthReport> {
        let notes = self.repo.discover_notes()?;
        Ok(repository::check_health(&notes))
    }

    /// Parse frontmatter from note content
    /// Returns (frontmatter, body_content) where body is everything after frontmatter
    fn parse_frontmatter(&self, content: &str) -> Result<(Option<note::Frontmatter>, String)> {
        use pulldown_cmark::{Event, MetadataBlockKind, Options, Parser, Tag, TagEnd};

        let mut options = Options::empty();
        options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

        let parser = Parser::new_ext(content, options);
        let mut in_metadata = false;
        let mut yaml_content = String::new();
        let mut found_metadata = false;
        let mut body_start = 0;

        // Find the end of frontmatter to determine where body starts
        let mut current_pos = 0;
        for event in parser {
            match event {
                Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                    in_metadata = true;
                    found_metadata = true;
                }
                Event::End(TagEnd::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                    in_metadata = false;
                    // Body starts after frontmatter block
                    // Find the position after the closing ---
                    if let Some(pos) = content[current_pos..].find("---\n") {
                        body_start = current_pos + pos + 4; // Skip past "---\n"
                    }
                }
                Event::Text(text) if in_metadata => {
                    yaml_content.push_str(&text);
                }
                _ => {}
            }
            current_pos = content.len();
        }

        let frontmatter = if found_metadata && !yaml_content.is_empty() {
            match serde_yaml::from_str::<note::Frontmatter>(&yaml_content) {
                Ok(fm) => Some(fm),
                Err(e) => {
                    anyhow::bail!("Failed to parse frontmatter: {}", e);
                }
            }
        } else {
            None
        };

        let body = if body_start > 0 && body_start < content.len() {
            &content[body_start..]
        } else {
            content
        };

        Ok((frontmatter, body.to_string()))
    }

    /// Update the 'updated' timestamp in a note's frontmatter
    pub fn update_note_timestamp(&self, note_path: &Path) -> Result<()> {
        use chrono::Utc;

        // Read the note file
        let content = self.repo.storage().read_to_string(note_path)?;

        // Parse to extract frontmatter and body
        let (frontmatter_opt, body) = self.parse_frontmatter(&content)?;

        // Skip if no frontmatter
        let mut frontmatter = match frontmatter_opt {
            Some(fm) => fm,
            None => return Ok(()), // No frontmatter, nothing to update
        };

        // Update the 'updated' field with current UTC timestamp
        frontmatter.updated = Some(Utc::now());

        // Serialize frontmatter back to YAML
        let yaml = serde_yaml::to_string(&frontmatter)?;

        // Reconstruct file: frontmatter + body
        let new_content = format!("---\n{}---\n{}", yaml, body);

        // Write back to file
        self.repo.storage().write(note_path, &new_content)?;

        Ok(())
    }

    /// Get the library configuration
    pub fn config(&self) -> &config::LibraryConfig {
        &self.config
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
                Path::new(".bnotes/templates/daily.md"),
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
                Path::new(".bnotes/templates/default.md"),
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
        let tasks = bnotes.list_tasks(&[], None, TaskSortOrder::parse("priority,id").unwrap()).unwrap();

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

        let bnotes = BNotes::with_defaults(storage);
        let tasks = bnotes.list_tasks(&[], None, TaskSortOrder::parse("id").unwrap()).unwrap();

        // Should be sorted by ID (filename#index), ignoring priority
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].note_title, "A Note");
        assert_eq!(tasks[0].index, 1);
        assert_eq!(tasks[0].priority, Some("C".to_string()));
        assert_eq!(tasks[1].note_title, "B Note");
        assert_eq!(tasks[1].index, 1);
        assert_eq!(tasks[1].priority, Some("A".to_string()));
    }

    #[test]
    fn test_task_sort_order_parse() {
        let order = TaskSortOrder::parse("urgency,priority,id").unwrap();
        assert_eq!(order.fields.len(), 3);

        let order = TaskSortOrder::parse("priority,id").unwrap();
        assert_eq!(order.fields.len(), 2);

        let order = TaskSortOrder::parse("id").unwrap();
        assert_eq!(order.fields.len(), 1);

        let result = TaskSortOrder::parse("invalid,priority");
        assert!(result.is_err());
    }

    #[test]
    fn test_task_sort_order_default() {
        let order = TaskSortOrder::default();
        assert_eq!(order.fields.len(), 3);
    }

    #[test]
    fn test_task_sorting_by_urgency_priority_id() {
        let storage = Box::new(MemoryStorage::new());

        storage.write(Path::new("tasks.md"), r#"# Tasks

- [ ] !!! (A) Critical and important
- [ ] !! (A) Soon and important
- [ ] !!! (C) Critical but low priority
- [ ] (A) Important but not urgent
- [ ] ! Eventually do this
- [ ] Plain task
"#).unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let sort_order = TaskSortOrder::parse("urgency,priority,id").unwrap();
        let tasks = bnotes.list_tasks(&[], None, sort_order).unwrap();

        assert_eq!(tasks.len(), 6);

        // First two should both have !!!, sorted by priority (A < C)
        assert_eq!(tasks[0].urgency, Some("!!!".to_string()));
        assert_eq!(tasks[0].priority, Some("A".to_string()));

        assert_eq!(tasks[1].urgency, Some("!!!".to_string()));
        assert_eq!(tasks[1].priority, Some("C".to_string()));

        // Next should have !!
        assert_eq!(tasks[2].urgency, Some("!!".to_string()));

        // Then !
        assert_eq!(tasks[3].urgency, Some("!".to_string()));

        // Then tasks without urgency, sorted by priority
        assert_eq!(tasks[4].urgency, None);
        assert_eq!(tasks[4].priority, Some("A".to_string()));

        // Finally no urgency, no priority
        assert_eq!(tasks[5].urgency, None);
        assert_eq!(tasks[5].priority, None);
    }

    #[test]
    fn test_task_sorting_by_priority_id() {
        let storage = Box::new(MemoryStorage::new());

        storage.write(Path::new("tasks.md"), r#"# Tasks

- [ ] !!! (C) Critical C
- [ ] (A) Important A
- [ ] (B) Important B
"#).unwrap();

        let bnotes = BNotes::with_defaults(storage);
        let sort_order = TaskSortOrder::parse("priority,id").unwrap();
        let tasks = bnotes.list_tasks(&[], None, sort_order).unwrap();

        // Should sort by priority only, ignoring urgency
        assert_eq!(tasks[0].priority, Some("A".to_string()));
        assert_eq!(tasks[1].priority, Some("B".to_string()));
        assert_eq!(tasks[2].priority, Some("C".to_string()));
    }

    #[test]
    fn test_weekly_migration_full_flow() {
        use periodic::Weekly;

        let storage = Box::new(MemoryStorage::new());

        // Create a previous weekly note with uncompleted tasks
        storage.write(Path::new("2026-W03.md"), r#"# 2026-W03

## Tasks
- [x] Completed task
- [ ] Uncompleted task 1
- [ ] !!! (A) Urgent and important task @backend
- [ ] Regular task @frontend
- [>] Already migrated task
"#).unwrap();

        let bnotes = BNotes::with_defaults(storage);

        // Create week 4 with migration (without prompting)
        let week4 = Weekly::from_date_str("2026-W04").unwrap();
        let (note_path, migrated_count) = bnotes.create_weekly_with_migration(week4, None, false).unwrap();

        assert_eq!(note_path, PathBuf::from("2026-W04.md"));
        assert_eq!(migrated_count, 3); // Only uncompleted tasks

        // Check the new note has migrated tasks
        let content = bnotes.repo.storage.read_to_string(&note_path).unwrap();
        assert!(content.contains("- [ ] Uncompleted task 1"));
        assert!(content.contains("- [ ] !!! (A) Urgent and important task @backend"));
        assert!(content.contains("- [ ] Regular task @frontend"));
        assert!(!content.contains("Completed task")); // Completed tasks not migrated
        assert!(!content.contains("Already migrated task")); // Already migrated tasks not re-migrated

        // Check the old note has tasks marked as migrated
        let old_content = bnotes.repo.storage.read_to_string(Path::new("2026-W03.md")).unwrap();
        assert!(old_content.contains("- [x] Completed task")); // Completed task unchanged
        assert!(old_content.contains("- [>] Uncompleted task 1")); // Now migrated
        assert!(old_content.contains("- [>] !!! (A) Urgent and important task @backend"));
        assert!(old_content.contains("- [>] Regular task @frontend"));
        assert!(old_content.contains("- [>] Already migrated task")); // Was already migrated, still marked
    }

    #[test]
    fn test_weekly_migration_no_previous_note() {
        use periodic::{PeriodType, Weekly};

        let storage = Box::new(MemoryStorage::new());
        let bnotes = BNotes::with_defaults(storage);

        // Create week 4 without any previous weekly notes
        let week4 = Weekly::from_date_str("2026-W04").unwrap();
        let (note_path, migrated_count) = bnotes.create_weekly_with_migration(week4, None, false).unwrap();

        assert_eq!(note_path, PathBuf::from("2026-W04.md"));
        assert_eq!(migrated_count, 0); // No tasks to migrate

        // Check the new note exists and contains the template content
        let content = bnotes.repo.storage.read_to_string(&note_path).unwrap();
        assert!(content.contains("# 2026-W04"));
        assert!(content.contains("## Goals"));
    }

    #[test]
    fn test_weekly_migration_with_gap() {
        use periodic::Weekly;

        let storage = Box::new(MemoryStorage::new());

        // Create week 2 with tasks (skip week 3)
        storage.write(Path::new("2026-W02.md"), r#"# 2026-W02

## Tasks
- [ ] Old task from week 2
"#).unwrap();

        let bnotes = BNotes::with_defaults(storage);

        // Create week 4, should find week 2 as the previous note
        let week4 = Weekly::from_date_str("2026-W04").unwrap();
        let (note_path, migrated_count) = bnotes.create_weekly_with_migration(week4, None, false).unwrap();

        assert_eq!(note_path, PathBuf::from("2026-W04.md"));
        assert_eq!(migrated_count, 1);

        // Check the new note has tasks from week 2
        let content = bnotes.repo.storage.read_to_string(&note_path).unwrap();
        assert!(content.contains("- [ ] Old task from week 2"));

        // Check week 2 has tasks marked as migrated
        let week2_content = bnotes.repo.storage.read_to_string(Path::new("2026-W02.md")).unwrap();
        assert!(week2_content.contains("- [>] Old task from week 2"));
    }

    #[test]
    fn test_weekly_no_migration_for_past_weeks() {
        use chrono::NaiveDate;
        use periodic::Weekly;

        let storage = Box::new(MemoryStorage::new());

        // Create a note in the past
        storage.write(Path::new("2025-W01.md"), r#"# 2025-W01

- [ ] Old task
"#).unwrap();

        let bnotes = BNotes::with_defaults(storage);

        // Create a past weekly note (not current week) with should_prompt=true
        // This means migration should not happen for non-current weeks
        let past_week = Weekly::from_date(NaiveDate::from_ymd_opt(2025, 1, 13).unwrap());
        let (_note_path, migrated_count) = bnotes.create_weekly_with_migration(past_week, None, true).unwrap();

        assert_eq!(migrated_count, 0); // No migration for past weeks when prompting
    }
}
