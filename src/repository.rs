//! Repository for managing notes
//!
//! The Repository provides high-level operations for discovering and querying notes,
//! using the Storage abstraction for file access. This module also includes link
//! analysis (LinkGraph) and health checking (HealthReport) functionality.

use crate::note::{render_template, Note};
use crate::storage::Storage;
use anyhow::{Context, Result};
use chrono::Utc;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Represents a search match with all occurrences in a note
///
/// Note: Contains full note content; acceptable for typical result set sizes
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchMatch {
    pub note: Note,
    pub locations: Vec<MatchLocation>,
}

/// Where a match was found in a note
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MatchLocation {
    /// Match in note title
    Title {
        /// Position of match in title
        position: usize,
    },
    /// Match in a tag
    Tag {
        /// The tag that matched
        tag: String,
    },
    /// Match in note content
    Content {
        /// Heading breadcrumb trail (e.g., ["# Main", "## Section"])
        breadcrumb: Vec<String>,
        /// Snippet of content around match
        snippet: String,
        /// Positions of matches within snippet (snippet-relative byte offset, length)
        match_positions: Vec<(usize, usize)>,
    },
}

// ============================================================================
// Repository
// ============================================================================

/// Temporary struct for building content matches
#[derive(Debug, Clone)]
struct ContentMatch {
    breadcrumb: Vec<String>,
    snippet: String,
    match_positions: Vec<(usize, usize)>,
}

/// Find all content matches with position and heading context
fn find_content_matches(content: &str, query: &str) -> Vec<ContentMatch> {
    // Guard against empty query to prevent infinite loop
    if query.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let content_lower = content.to_lowercase();
    let mut matches = Vec::new();

    // Build heading position map: for each position, what's the active breadcrumb?
    let heading_positions = build_heading_positions(content);

    // Find all matches in content
    let mut search_pos = 0;
    while let Some(relative_pos) = content_lower[search_pos..].find(&query_lower) {
        let absolute_pos = search_pos + relative_pos;

        // Determine breadcrumb for this position
        let breadcrumb = get_breadcrumb_at_position(&heading_positions, absolute_pos);

        // Extract snippet with original case
        let snippet = extract_snippet(content, absolute_pos, query.len(), 60);

        // Find match positions in snippet for highlighting
        let snippet_lower = snippet.to_lowercase();
        let mut match_positions = Vec::new();
        let mut snippet_pos = 0;

        while let Some(pos) = snippet_lower[snippet_pos..].find(&query_lower) {
            match_positions.push((snippet_pos + pos, query.len()));
            snippet_pos += pos + query.len();
        }

        matches.push(ContentMatch {
            breadcrumb,
            snippet,
            match_positions,
        });

        search_pos = absolute_pos + query.len();
    }

    matches
}

/// Build list of (position, breadcrumb) pairs by parsing headings
fn build_heading_positions(content: &str) -> Vec<(usize, Vec<String>)> {
    let mut positions = Vec::new();
    let mut in_heading = false;
    let mut heading_text = String::new();
    let mut heading_level = HeadingLevel::H1;

    let breadcrumb_map = build_heading_breadcrumbs(content);
    let parser = Parser::new(content);

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = level;
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if in_heading && !heading_text.is_empty() {
                    let markers = "#".repeat(heading_level_to_num(&heading_level) as usize);
                    let formatted = format!("{} {}", markers, heading_text.trim());

                    if let Some(breadcrumb) = breadcrumb_map.get(&formatted) {
                        // Record that from this position onward, we're under this breadcrumb
                        positions.push((range.end, breadcrumb.clone()));
                    }
                }
                in_heading = false;
            }
            Event::Text(text) => {
                if in_heading {
                    heading_text.push_str(&text);
                }
            }
            _ => {}
        }
    }

    positions
}

/// Get the active breadcrumb at a given position
fn get_breadcrumb_at_position(positions: &[(usize, Vec<String>)], pos: usize) -> Vec<String> {
    // Find the last heading position before or at pos
    for (heading_pos, breadcrumb) in positions.iter().rev() {
        if *heading_pos <= pos {
            return breadcrumb.clone();
        }
    }

    // Default if before any heading
    vec!["Document Start".to_string()]
}

/// Convert HeadingLevel to numeric level (1-6)
fn heading_level_to_num(level: &HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Build map of heading text to breadcrumb path
fn build_heading_breadcrumbs(content: &str) -> HashMap<String, Vec<String>> {
    let mut breadcrumbs = HashMap::new();
    let mut heading_stack: Vec<(HeadingLevel, String)> = Vec::new();
    let mut current_heading_text = String::new();
    let mut in_heading = false;
    let mut current_heading_level = HeadingLevel::H1;

    let parser = Parser::new(content);

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                current_heading_level = level;
                current_heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if in_heading && !current_heading_text.is_empty() {
                    let level_num = heading_level_to_num(&current_heading_level);

                    // Pop headings at same or deeper level
                    heading_stack.retain(|(lvl, _)| {
                        let stack_level_num = heading_level_to_num(lvl);
                        stack_level_num < level_num
                    });

                    // Format heading with markers
                    let markers = "#".repeat(level_num as usize);
                    let formatted = format!("{} {}", markers, current_heading_text.trim());

                    // Build breadcrumb path
                    let mut path: Vec<String> = heading_stack.iter()
                        .map(|(lvl, txt)| {
                            let n = heading_level_to_num(lvl);
                            format!("{} {}", "#".repeat(n as usize), txt)
                        })
                        .collect();
                    path.push(formatted.clone());

                    breadcrumbs.insert(formatted.clone(), path.clone());
                    heading_stack.push((current_heading_level, current_heading_text.trim().to_string()));
                }
                in_heading = false;
            }
            Event::Text(text) => {
                if in_heading {
                    current_heading_text.push_str(&text);
                }
            }
            _ => {}
        }
    }

    breadcrumbs
}

/// Extract snippet around a match position with smart word boundaries
fn extract_snippet(content: &str, match_pos: usize, query_len: usize, context_chars: usize) -> String {
    let start = match_pos.saturating_sub(context_chars);
    let end = (match_pos + query_len + context_chars).min(content.len());

    let mut snippet = &content[start..end];

    // Trim to word boundaries (don't cut mid-word)
    if start > 0 {
        if let Some(space_pos) = snippet.find(char::is_whitespace) {
            snippet = &snippet[space_pos..].trim_start();
        }
    }
    if end < content.len() {
        if let Some(space_pos) = snippet.rfind(char::is_whitespace) {
            snippet = &snippet[..space_pos].trim_end();
        }
    }

    // Add ellipsis indicators
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < content.len() { "..." } else { "" };

    format!("{}{}{}", prefix, snippet, suffix)
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

// ============================================================================
// LinkGraph
// ============================================================================

#[derive(Debug, Clone)]
pub struct LinkGraph {
    /// Map from note title to set of titles it links to (outbound)
    pub outbound: HashMap<String, HashSet<String>>,
    /// Map from note title to set of titles that link to it (inbound)
    pub inbound: HashMap<String, HashSet<String>>,
}

impl Default for LinkGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkGraph {
    pub fn new() -> Self {
        Self {
            outbound: HashMap::new(),
            inbound: HashMap::new(),
        }
    }

    /// Build a link graph from a collection of notes
    pub fn build(notes: &[Note]) -> Self {
        let mut graph = Self::new();

        // Create title -> note mapping for resolving links
        let title_map: HashMap<String, &Note> = notes
            .iter()
            .map(|n| (n.title.to_lowercase(), n))
            .collect();

        for note in notes {
            let links = extract_wiki_links(&note.content);
            let note_title = note.title.clone();

            // Initialize outbound set for this note
            graph
                .outbound
                .entry(note_title.clone())
                .or_default();

            for link_text in links {
                let link_lower = link_text.to_lowercase();

                // Try to resolve the link
                if title_map.contains_key(&link_lower) {
                    // Add to outbound links
                    graph
                        .outbound
                        .entry(note_title.clone())
                        .or_default()
                        .insert(link_text.clone());

                    // Add to inbound links for the target
                    graph
                        .inbound
                        .entry(link_text)
                        .or_default()
                        .insert(note_title.clone());
                }
            }
        }

        graph
    }

    /// Get notes that have no incoming or outgoing links
    pub fn orphaned_notes(&self, all_note_titles: &[String]) -> Vec<String> {
        all_note_titles
            .iter()
            .filter(|title| {
                let has_outbound = self
                    .outbound
                    .get(*title)
                    .map(|set| !set.is_empty())
                    .unwrap_or(false);

                let has_inbound = self
                    .inbound
                    .get(*title)
                    .map(|set| !set.is_empty())
                    .unwrap_or(false);

                !has_outbound && !has_inbound
            })
            .cloned()
            .collect()
    }

    /// Find broken links (links to non-existent notes)
    pub fn broken_links(&self, notes: &[Note]) -> HashMap<String, Vec<String>> {
        let title_set: HashSet<String> = notes
            .iter()
            .map(|n| n.title.to_lowercase())
            .collect();

        let mut broken = HashMap::new();

        for note in notes {
            let links = extract_wiki_links(&note.content);
            let broken_in_note: Vec<String> = links
                .into_iter()
                .filter(|link| !title_set.contains(&link.to_lowercase()))
                .collect();

            if !broken_in_note.is_empty() {
                broken.insert(note.title.clone(), broken_in_note);
            }
        }

        broken
    }
}

/// Extract wiki-style links from markdown content
///
/// Parses markdown using pulldown-cmark and extracts [[wiki link]] patterns
/// from text events. Wiki links are not standard markdown, so they appear
/// as plain text in the event stream.
pub(crate) fn extract_wiki_links(content: &str) -> Vec<String> {
    let parser = Parser::new(content);
    let mut links = Vec::new();
    let mut accumulated_text = String::new();

    for event in parser {
        match event {
            Event::Text(text) => {
                // Accumulate text to handle wiki links that might be split across events
                accumulated_text.push_str(text.as_ref());
            }
            Event::Code(text) => {
                // Also check code spans for wiki links
                accumulated_text.push_str(text.as_ref());
            }
            // When we hit a non-text event, process accumulated text and reset
            _ => {
                if !accumulated_text.is_empty() {
                    extract_wiki_links_from_text(&accumulated_text, &mut links);
                    accumulated_text.clear();
                }
            }
        }
    }

    // Process any remaining accumulated text
    if !accumulated_text.is_empty() {
        extract_wiki_links_from_text(&accumulated_text, &mut links);
    }

    links
}

/// Helper function to extract wiki links from a text string
fn extract_wiki_links_from_text(text: &str, links: &mut Vec<String>) {
    let mut start = 0;

    while let Some(begin) = text[start..].find("[[") {
        let begin = start + begin;
        if let Some(end) = text[begin + 2..].find("]]") {
            let end = begin + 2 + end;
            let link_text = &text[begin + 2..end];
            links.push(link_text.to_string());
            start = end + 2;
        } else {
            break;
        }
    }
}

// ============================================================================
// HealthReport
// ============================================================================

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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{MemoryStorage, Storage};

    #[test]
    fn test_extract_wiki_links() {
        let content = r#"
# My Note

See also [[Other Note]] and [[Another Note]].

More content with [[Third Link]].
"#;

        let links = extract_wiki_links(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains(&"Other Note".to_string()));
        assert!(links.contains(&"Another Note".to_string()));
        assert!(links.contains(&"Third Link".to_string()));
    }

    #[test]
    fn test_link_graph() {
        let note1 = Note::parse(
            Path::new("note1.md"),
            r#"
# Note One

Links to [[Note Two]].
"#,
        )
        .unwrap();

        let note2 = Note::parse(
            Path::new("note2.md"),
            r#"
# Note Two

Links to [[Note One]] and [[Note Three]].
"#,
        )
        .unwrap();

        let note3 = Note::parse(Path::new("note3.md"), "# Note Three\n\nNo links.").unwrap();

        let notes = vec![note1, note2, note3];
        let graph = LinkGraph::build(&notes);

        // Note One links to Note Two
        assert!(graph
            .outbound
            .get("Note One")
            .unwrap()
            .contains("Note Two"));

        // Note Two is linked from Note One
        assert!(graph.inbound.get("Note Two").unwrap().contains("Note One"));

        // Note Two links to Note One and Note Three
        assert_eq!(graph.outbound.get("Note Two").unwrap().len(), 2);

        // Note Three has no outbound links
        assert!(graph
            .outbound
            .get("Note Three")
            .unwrap_or(&HashSet::new())
            .is_empty());
    }

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

#[cfg(test)]
mod search_tests {
    use super::*;

    #[test]
    fn test_extract_snippet() {
        let content = "This is some longer text with the word project in the middle and more text after";
        let match_pos = 42; // position of "project"
        let query_len = 7;

        let snippet = extract_snippet(content, match_pos, query_len, 20);

        // Should have trimmed to word boundaries with ellipsis
        assert!(snippet.starts_with("..."));
        assert!(snippet.ends_with("..."));
        assert!(snippet.contains("project"));
        assert!(!snippet.contains("This is")); // Too far before
    }

    #[test]
    fn test_extract_snippet_at_start() {
        let content = "project is at the beginning of text";
        let snippet = extract_snippet(content, 0, 7, 20);

        // No leading ellipsis when at start
        assert!(!snippet.starts_with("..."));
        assert!(snippet.starts_with("project"));
    }

    #[test]
    fn test_extract_snippet_at_end() {
        let content = "text with word at the end project";
        let match_pos = 28;
        let snippet = extract_snippet(content, match_pos, 7, 20);

        // No trailing ellipsis when at end
        assert!(!snippet.ends_with("..."));
        assert!(snippet.ends_with("project"));
    }

    #[test]
    fn test_build_heading_breadcrumbs() {
        let markdown = r#"# Main Heading
Some text here.

## Section One
Content in section one.

### Subsection
Content in subsection.

## Section Two
More content."#;

        let breadcrumbs = build_heading_breadcrumbs(markdown);

        // Should have entries for each heading
        assert!(breadcrumbs.contains_key(&"# Main Heading".to_string()));
        assert!(breadcrumbs.contains_key(&"## Section One".to_string()));
        assert!(breadcrumbs.contains_key(&"### Subsection".to_string()));

        // Subsection should have full path
        let subsection_path = &breadcrumbs["### Subsection"];
        assert_eq!(subsection_path.len(), 3);
        assert_eq!(subsection_path[0], "# Main Heading");
        assert_eq!(subsection_path[1], "## Section One");
        assert_eq!(subsection_path[2], "### Subsection");

        // Section Two should have path from Main
        let section_two_path = &breadcrumbs["## Section Two"];
        assert_eq!(section_two_path.len(), 2);
        assert_eq!(section_two_path[0], "# Main Heading");
        assert_eq!(section_two_path[1], "## Section Two");
    }

    #[test]
    fn test_build_heading_breadcrumbs_empty() {
        let markdown = "Just text, no headings.";
        let breadcrumbs = build_heading_breadcrumbs(markdown);
        assert!(breadcrumbs.is_empty());
    }

    #[test]
    fn test_build_heading_breadcrumbs_single() {
        let markdown = "# Only Heading\nSome text.";
        let breadcrumbs = build_heading_breadcrumbs(markdown);
        assert_eq!(breadcrumbs.len(), 1);
        let path = &breadcrumbs["# Only Heading"];
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], "# Only Heading");
    }

    #[test]
    fn test_build_heading_breadcrumbs_skipped_levels() {
        let markdown = "# Main\n### Deep\nText.";
        let breadcrumbs = build_heading_breadcrumbs(markdown);
        // H3 should still show just under H1 in path
        let path = &breadcrumbs["### Deep"];
        assert_eq!(path, &vec!["# Main", "### Deep"]);
    }

    #[test]
    fn test_find_content_matches() {
        let content = r#"# Main
Some text with project word here.

## Section
Another project mention.

More project references."#;

        let matches = find_content_matches(content, "project");

        // Should find all 3 matches
        assert_eq!(matches.len(), 3);

        // First match should be under "# Main"
        assert_eq!(matches[0].breadcrumb, vec!["# Main"]);
        assert!(matches[0].snippet.contains("project"));

        // Second match should be under "# Main > ## Section"
        assert_eq!(matches[1].breadcrumb, vec!["# Main", "## Section"]);
    }

    #[test]
    fn test_find_content_matches_empty_query() {
        let content = r#"# Main
Some text with content here.

## Section
More content."#;

        let matches = find_content_matches(content, "");

        // Empty query should return no matches (not infinite loop)
        assert_eq!(matches.len(), 0);
    }
}
