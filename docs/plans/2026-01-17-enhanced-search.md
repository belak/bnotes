# Enhanced Search Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add heading context breadcrumbs and match highlighting to search results, showing where matches occur in document structure.

**Architecture:** Extend search to parse markdown and track heading hierarchy, return structured match data with breadcrumbs, render with color highlighting in CLI.

**Tech Stack:** Rust, pulldown-cmark (markdown parser), termcolor (already integrated)

---

## Task 1: Add SearchMatch Data Structures

**Files:**
- Modify: `src/repository.rs` (add types after imports)

**Step 1: Add SearchMatch types**

Add these types after the imports in `src/repository.rs`:

```rust
/// Represents a search match with all occurrences in a note
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub note: Note,
    pub locations: Vec<MatchLocation>,
}

/// Where a match was found in a note
#[derive(Debug, Clone)]
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
        /// Positions of matches within snippet (start, length)
        match_positions: Vec<(usize, usize)>,
    },
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: SUCCESS (new types compile)

**Step 3: Commit**

```bash
git add src/repository.rs
git commit -m "feat: add SearchMatch data structures"
```

---

## Task 2: Add Snippet Extraction Helper

**Files:**
- Modify: `src/repository.rs` (add function before impl block)

**Step 1: Write test for snippet extraction**

Add test at bottom of `src/repository.rs`:

```rust
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
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test search_tests`
Expected: FAIL with "function `extract_snippet` not found"

**Step 3: Implement extract_snippet**

Add before the `impl Repository` block in `src/repository.rs`:

```rust
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test search_tests`
Expected: PASS (all 3 tests)

**Step 5: Commit**

```bash
git add src/repository.rs
git commit -m "feat: add snippet extraction with word boundaries"
```

---

## Task 3: Add Heading Breadcrumb Tracking

**Files:**
- Modify: `src/repository.rs`

**Step 1: Write test for breadcrumb building**

Add to `search_tests` module in `src/repository.rs`:

```rust
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
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_build_heading_breadcrumbs`
Expected: FAIL with "function `build_heading_breadcrumbs` not found"

**Step 3: Implement build_heading_breadcrumbs**

Add before `extract_snippet` in `src/repository.rs`:

```rust
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::HashMap;

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
                    let level_num = match current_heading_level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };

                    // Pop headings at same or deeper level
                    heading_stack.retain(|(lvl, _)| {
                        let stack_level_num = match lvl {
                            HeadingLevel::H1 => 1,
                            HeadingLevel::H2 => 2,
                            HeadingLevel::H3 => 3,
                            HeadingLevel::H4 => 4,
                            HeadingLevel::H5 => 5,
                            HeadingLevel::H6 => 6,
                        };
                        stack_level_num < level_num
                    });

                    // Format heading with markers
                    let markers = "#".repeat(level_num);
                    let formatted = format!("{} {}", markers, current_heading_text.trim());

                    // Build breadcrumb path
                    let mut path: Vec<String> = heading_stack.iter()
                        .map(|(lvl, txt)| {
                            let n = match lvl {
                                HeadingLevel::H1 => 1,
                                HeadingLevel::H2 => 2,
                                HeadingLevel::H3 => 3,
                                HeadingLevel::H4 => 4,
                                HeadingLevel::H5 => 5,
                                HeadingLevel::H6 => 6,
                            };
                            format!("{} {}", "#".repeat(n), txt)
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_build_heading_breadcrumbs`
Expected: PASS

**Step 5: Commit**

```bash
git add src/repository.rs
git commit -m "feat: add heading breadcrumb tracking"
```

---

## Task 4: Parse Content for Matches with Position Tracking

**Files:**
- Modify: `src/repository.rs`

**Step 1: Write test for finding content matches**

Add to `search_tests` module:

```rust
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_find_content_matches`
Expected: FAIL with "function `find_content_matches` not found"

**Step 3: Add ContentMatch helper struct**

Add before `build_heading_breadcrumbs`:

```rust
/// Temporary struct for building content matches
#[derive(Debug, Clone)]
struct ContentMatch {
    breadcrumb: Vec<String>,
    snippet: String,
    match_positions: Vec<(usize, usize)>,
}
```

**Step 4: Implement find_content_matches**

Add before `build_heading_breadcrumbs`:

```rust
/// Find all content matches with position and heading context
fn find_content_matches(content: &str, query: &str) -> Vec<ContentMatch> {
    let query_lower = query.to_lowercase();
    let content_lower = content.to_lowercase();
    let mut matches = Vec::new();

    // Build heading breadcrumbs
    let breadcrumb_map = build_heading_breadcrumbs(content);

    // Track current position and active heading
    let mut char_pos = 0;
    let mut current_breadcrumb = vec!["Document Start".to_string()];
    let mut in_heading = false;
    let mut current_heading_text = String::new();
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
                    let level_num = match current_heading_level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    let markers = "#".repeat(level_num);
                    let formatted = format!("{} {}", markers, current_heading_text.trim());

                    // Update current breadcrumb from map
                    if let Some(breadcrumb) = breadcrumb_map.get(&formatted) {
                        current_breadcrumb = breadcrumb.clone();
                    }
                }
                in_heading = false;
            }
            Event::Text(text) => {
                if in_heading {
                    current_heading_text.push_str(&text);
                } else {
                    // Search for matches in this text chunk
                    let text_lower = text.to_lowercase();
                    let mut search_pos = 0;

                    while let Some(relative_pos) = text_lower[search_pos..].find(&query_lower) {
                        let absolute_pos = char_pos + search_pos + relative_pos;

                        // Extract snippet
                        let snippet = extract_snippet(&content_lower, absolute_pos, query.len(), 60);

                        // Find all query occurrences in snippet for highlighting
                        let snippet_lower = snippet.to_lowercase();
                        let mut match_positions = Vec::new();
                        let mut snippet_search_pos = 0;

                        while let Some(pos) = snippet_lower[snippet_search_pos..].find(&query_lower) {
                            match_positions.push((snippet_search_pos + pos, query.len()));
                            snippet_search_pos += pos + query.len();
                        }

                        // Use original content for snippet (preserve case)
                        let original_snippet = extract_snippet(content, absolute_pos, query.len(), 60);

                        matches.push(ContentMatch {
                            breadcrumb: current_breadcrumb.clone(),
                            snippet: original_snippet,
                            match_positions,
                        });

                        search_pos += relative_pos + query.len();
                    }

                    char_pos += text.len();
                }
            }
            _ => {}
        }
    }

    matches
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test test_find_content_matches`
Expected: PASS

**Step 6: Commit**

```bash
git add src/repository.rs
git commit -m "feat: add content match finding with breadcrumbs"
```

---

## Task 5: Update Repository search() Method

**Files:**
- Modify: `src/repository.rs` (replace existing search method)

**Step 1: Update search method signature and implementation**

Find the existing `pub fn search(&self, query: &str) -> Result<Vec<Note>>` method and replace it:

```rust
pub fn search(&self, query: &str) -> Result<Vec<SearchMatch>> {
    let all_notes = self.discover_notes()?;
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for note in all_notes {
        let mut locations = Vec::new();

        // Check title
        if let Some(pos) = note.title.to_lowercase().find(&query_lower) {
            locations.push(MatchLocation::Title { position: pos });
        }

        // Check tags
        for tag in &note.tags {
            if tag.to_lowercase().contains(&query_lower) {
                locations.push(MatchLocation::Tag {
                    tag: tag.clone(),
                });
            }
        }

        // Check content
        let content_matches = find_content_matches(&note.content, query);
        for m in content_matches {
            locations.push(MatchLocation::Content {
                breadcrumb: m.breadcrumb,
                snippet: m.snippet,
                match_positions: m.match_positions,
            });
        }

        // If any matches found, add to results
        if !locations.is_empty() {
            results.push(SearchMatch {
                note,
                locations,
            });
        }
    }

    Ok(results)
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: FAIL - CLI code needs updating

This is expected. We'll fix the CLI in next task.

**Step 3: Commit**

```bash
git add src/repository.rs
git commit -m "feat: update search() to return SearchMatch"
```

---

## Task 6: Update BNotes search() Wrapper

**Files:**
- Modify: `src/lib.rs`

**Step 1: Update BNotes search method**

Find the `pub fn search(&self, query: &str)` method and update return type:

```rust
pub fn search(&self, query: &str) -> Result<Vec<repository::SearchMatch>> {
    self.repository.search(query)
}
```

**Step 2: Export SearchMatch types**

At top of `src/lib.rs`, update the public exports:

```rust
pub use repository::{HealthReport, LinkGraph, Repository, SearchMatch, MatchLocation};
```

**Step 3: Verify it compiles (still expecting CLI errors)**

Run: `cargo check`
Expected: FAIL in CLI code (expected)

**Step 4: Commit**

```bash
git add src/lib.rs
git commit -m "feat: export SearchMatch types from library"
```

---

## Task 7: Add --limit Flag to CLI

**Files:**
- Modify: `src/main.rs`

**Step 1: Add limit field to Search command**

Find the `Commands::Search` variant and update it:

```rust
/// Full-text search across all notes
Search {
    /// Search query
    query: String,

    /// Number of matches to show per note
    #[arg(long, default_value = "3")]
    limit: usize,
},
```

**Step 2: Update search command call**

Find where `Commands::Search` is matched and update:

```rust
Commands::Search { query, limit } => {
    cli::commands::search(&notes_dir, &query, cli_args.color, limit)?;
}
```

**Step 3: Update function signature in commands.rs**

In `src/cli/commands.rs`, find `pub fn search(` and update:

```rust
pub fn search(notes_dir: &Path, query: &str, color: ColorChoice, limit: usize) -> Result<()> {
```

**Step 4: Verify it compiles (still expecting errors)**

Run: `cargo check`
Expected: FAIL - search function body needs updating

**Step 5: Commit**

```bash
git add src/main.rs src/cli/commands.rs
git commit -m "feat: add --limit flag to search command"
```

---

## Task 8: Implement Search Output with Breadcrumbs

**Files:**
- Modify: `src/cli/commands.rs`

**Step 1: Add use statements**

Add to imports at top of `src/cli/commands.rs`:

```rust
use bnotes::{BNotes, RealStorage, SearchMatch, MatchLocation};
```

**Step 2: Replace search function body**

Replace the entire `search` function body in `src/cli/commands.rs`:

```rust
pub fn search(notes_dir: &Path, query: &str, color: ColorChoice, limit: usize) -> Result<()> {
    validate_notes_dir(notes_dir)?;
    let storage = Box::new(RealStorage::new(notes_dir.to_path_buf()));
    let bnotes = BNotes::with_defaults(storage);

    let matches = bnotes.search(query)?;

    let mut stdout = colors::create_stdout(color);

    if matches.is_empty() {
        writeln!(stdout, "No notes found matching: {}", query)?;
        return Ok(());
    }

    let query_lower = query.to_lowercase();

    for search_match in &matches {
        let note = &search_match.note;

        // Count content matches for limiting
        let content_matches: Vec<&MatchLocation> = search_match.locations.iter()
            .filter(|loc| matches!(loc, MatchLocation::Content { .. }))
            .collect();

        let total_content_matches = content_matches.len();
        let shown_content_matches = content_matches.len().min(limit);

        // Display note title (with highlighting if matched)
        let title_match = search_match.locations.iter()
            .find(|loc| matches!(loc, MatchLocation::Title { .. }));

        if title_match.is_some() {
            // Highlight matched words in title
            highlight_and_write(&mut stdout, &note.title, &query_lower)?;
            writeln!(stdout)?;
        } else {
            stdout.set_color(&colors::highlight())?;
            writeln!(stdout, "{}", note.title)?;
            stdout.reset()?;
        }

        // Display tag matches
        let tag_matches: Vec<&String> = search_match.locations.iter()
            .filter_map(|loc| {
                if let MatchLocation::Tag { tag } = loc {
                    Some(tag)
                } else {
                    None
                }
            })
            .collect();

        if !tag_matches.is_empty() && !note.tags.is_empty() {
            write!(stdout, "  [")?;
            for (i, tag) in note.tags.iter().enumerate() {
                if i > 0 {
                    write!(stdout, ", ")?;
                }
                if tag_matches.contains(&tag) {
                    // Highlight matched tag
                    highlight_and_write(&mut stdout, tag, &query_lower)?;
                } else {
                    write!(stdout, "{}", tag)?;
                }
            }
            writeln!(stdout, "]")?;
        }

        // Display content matches
        for (i, location) in search_match.locations.iter().enumerate() {
            if i >= shown_content_matches {
                break;
            }

            if let MatchLocation::Content { breadcrumb, snippet, .. } = location {
                // Display breadcrumb
                stdout.set_color(&colors::dim())?;
                write!(stdout, "  [")?;
                for (j, heading) in breadcrumb.iter().enumerate() {
                    if j > 0 {
                        write!(stdout, " > ")?;
                    }
                    write!(stdout, "{}", heading)?;
                }
                writeln!(stdout, "]")?;

                // Display snippet with highlighted matches
                write!(stdout, "  ")?;
                highlight_snippet(&mut stdout, snippet, &query_lower)?;
                writeln!(stdout)?;
                stdout.reset()?;
                writeln!(stdout)?;
            }
        }

        // Show truncation message if needed
        if total_content_matches > limit {
            stdout.set_color(&colors::dim())?;
            writeln!(
                stdout,
                "  ({} matches shown, {} more in this note)",
                shown_content_matches,
                total_content_matches - shown_content_matches
            )?;
            stdout.reset()?;
            writeln!(stdout)?;
        }
    }

    writeln!(
        stdout,
        "Found {} {}",
        matches.len(),
        pluralize(matches.len(), "note", "notes")
    )?;

    Ok(())
}
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: FAIL - missing `highlight_and_write` and `highlight_snippet` functions

This is expected. We'll add those next.

**Step 4: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat: implement search output with breadcrumbs"
```

---

## Task 9: Implement Match Highlighting Helpers

**Files:**
- Modify: `src/cli/commands.rs`

**Step 1: Add highlight_and_write helper**

Add before the `search` function in `src/cli/commands.rs`:

```rust
/// Write text with query matches highlighted in bold
fn highlight_and_write(
    stdout: &mut termcolor::StandardStream,
    text: &str,
    query_lower: &str,
) -> Result<()> {
    let text_lower = text.to_lowercase();
    let mut last_pos = 0;

    // Find all matches and highlight them
    while let Some(pos) = text_lower[last_pos..].find(query_lower) {
        let absolute_pos = last_pos + pos;

        // Write text before match in current color
        write!(stdout, "{}", &text[last_pos..absolute_pos])?;

        // Write match in bold
        let mut bold_spec = termcolor::ColorSpec::new();
        bold_spec.set_bold(true);
        stdout.set_color(&bold_spec)?;
        write!(stdout, "{}", &text[absolute_pos..absolute_pos + query_lower.len()])?;
        stdout.reset()?;

        last_pos = absolute_pos + query_lower.len();
    }

    // Write remaining text
    write!(stdout, "{}", &text[last_pos..])?;

    Ok(())
}
```

**Step 2: Add highlight_snippet helper**

Add after `highlight_and_write`:

```rust
/// Write snippet with matches highlighted
fn highlight_snippet(
    stdout: &mut termcolor::StandardStream,
    snippet: &str,
    query_lower: &str,
) -> Result<()> {
    // Set dim for snippet text
    stdout.set_color(&colors::dim())?;

    let snippet_lower = snippet.to_lowercase();
    let mut last_pos = 0;

    // Find all matches and highlight them
    while let Some(pos) = snippet_lower[last_pos..].find(query_lower) {
        let absolute_pos = last_pos + pos;

        // Write text before match in dim
        write!(stdout, "{}", &snippet[last_pos..absolute_pos])?;

        // Write match in bold (not dim)
        stdout.reset()?;
        let mut bold_spec = termcolor::ColorSpec::new();
        bold_spec.set_bold(true);
        stdout.set_color(&bold_spec)?;
        write!(stdout, "{}", &snippet[absolute_pos..absolute_pos + query_lower.len()])?;

        // Back to dim for rest of snippet
        stdout.set_color(&colors::dim())?;

        last_pos = absolute_pos + query_lower.len();
    }

    // Write remaining text in dim
    write!(stdout, "{}", &snippet[last_pos..])?;

    Ok(())
}
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: SUCCESS

**Step 4: Run tests**

Run: `cargo test`
Expected: PASS (all tests)

**Step 5: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat: add match highlighting helpers"
```

---

## Task 10: Add Integration Test

**Files:**
- Create: `tests/integration_test.rs` (if doesn't exist)
- Or modify existing integration test file

**Step 1: Add search integration test**

Create or append to `tests/integration_test.rs`:

```rust
use bnotes::{BNotes, MemoryStorage, MatchLocation};

#[test]
fn test_search_with_breadcrumbs() {
    let mut storage = MemoryStorage::new();

    // Add note with headings and matches
    let content = r#"---
title: Test Note
tags: [test, project]
created: 2024-01-01T00:00:00Z
---

# Main Section

This is a project about testing.

## Subsection

Another project mention here.

### Deep Section

Yet another project reference.
"#;

    storage.write("test.md", content.as_bytes()).unwrap();

    let bnotes = BNotes::with_defaults(Box::new(storage));
    let results = bnotes.search("project").unwrap();

    // Should find the note
    assert_eq!(results.len(), 1);

    let search_match = &results[0];
    assert_eq!(search_match.note.title, "Test Note");

    // Should have tag match and content matches
    let tag_matches: Vec<_> = search_match.locations.iter()
        .filter(|loc| matches!(loc, MatchLocation::Tag { .. }))
        .collect();
    assert_eq!(tag_matches.len(), 1);

    let content_matches: Vec<_> = search_match.locations.iter()
        .filter(|loc| matches!(loc, MatchLocation::Content { .. }))
        .collect();
    assert!(content_matches.len() >= 3);

    // Verify breadcrumbs are present
    for location in &search_match.locations {
        if let MatchLocation::Content { breadcrumb, .. } = location {
            assert!(!breadcrumb.is_empty());
            // At least one heading in breadcrumb
            assert!(breadcrumb.iter().any(|h| h.starts_with('#')));
        }
    }
}

#[test]
fn test_search_match_in_title() {
    let mut storage = MemoryStorage::new();

    let content = r#"---
title: Project Overview
tags: [test]
created: 2024-01-01T00:00:00Z
---

Some content without the query word.
"#;

    storage.write("test.md", content.as_bytes()).unwrap();

    let bnotes = BNotes::with_defaults(Box::new(storage));
    let results = bnotes.search("project").unwrap();

    assert_eq!(results.len(), 1);

    // Should have title match
    let title_matches: Vec<_> = results[0].locations.iter()
        .filter(|loc| matches!(loc, MatchLocation::Title { .. }))
        .collect();
    assert_eq!(title_matches.len(), 1);
}
```

**Step 2: Run integration tests**

Run: `cargo test --test integration_test`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/integration_test.rs
git commit -m "test: add integration tests for enhanced search"
```

---

## Task 11: Manual Testing

**Files:**
- None (command-line testing)

**Step 1: Create test notes**

```bash
# In test notes directory
mkdir -p /tmp/test-bnotes-search
cd /tmp/test-bnotes-search

cat > planning.md <<'EOF'
---
title: Planning Document
tags: [planning, project]
created: 2024-01-01T00:00:00Z
---

# Overview

This project aims to improve the search functionality.

## Goals

The main project goals are:
- Add breadcrumb context
- Highlight matches
- Improve usability

### Timeline

Project timeline spans Q1 2024.

## Resources

Review the project budget and team allocation.
EOF

cat > meeting.md <<'EOF'
---
title: Meeting Notes
tags: [meeting]
created: 2024-01-02T00:00:00Z
---

# Team Meeting

Discussed the project kickoff.

## Action Items

- Review project charter
- Assign project leads
EOF
```

**Step 2: Test basic search with color**

Run: `cargo run -- search --notes-dir /tmp/test-bnotes-search project`

Expected output similar to:
```
Planning Document [planning, **project**]
  [# Overview]
  ... This **project** aims to improve the search ...

  [# Goals]
  ... The main **project** goals are ...

  [# Goals > ### Timeline]
  ... **Project** timeline spans Q1 2024 ...

  (3 matches shown, 1 more in this note)

Meeting Notes
  [# Team Meeting]
  ... Discussed the **project** kickoff ...

  [# Team Meeting > ## Action Items]
  ... Review **project** charter ...

  (2 matches shown)

Found 2 notes
```

**Step 3: Test with limit flag**

Run: `cargo run -- search --notes-dir /tmp/test-bnotes-search project --limit 1`

Expected: Shows only first match per note with truncation message

**Step 4: Test with --color=never**

Run: `cargo run -- search --notes-dir /tmp/test-bnotes-search project --color=never`

Expected: No ANSI color codes, but brackets around breadcrumbs still visible

**Step 5: Test piped output**

Run: `cargo run -- search --notes-dir /tmp/test-bnotes-search project | cat`

Expected: No color codes (auto-detection works)

**Step 6: Document test results**

If all tests pass, document in commit message.

**Step 7: Commit**

```bash
git commit --allow-empty -m "test: verify manual testing of enhanced search"
```

---

## Task 12: Update Documentation

**Files:**
- Modify: `README.md` (if search is documented there)

**Step 1: Update README search section**

Find the search command documentation in `README.md` and update to mention new features:

```markdown
### Search

Search across all notes with heading context and match highlighting:

\`\`\`bash
# Basic search
bnotes search "query"

# Limit matches per note (default: 3)
bnotes search "query" --limit 5

# Disable colors
bnotes search "query" --color=never
\`\`\`

Search results show:
- Matched words highlighted in titles, tags, and content
- Heading breadcrumb trails showing document structure context
- Smart snippets with word boundary detection
```

**Step 2: Verify documentation accuracy**

Run: `cargo run -- search --help`

Verify help text matches documentation.

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: update search documentation"
```

---

## Task 13: Final Verification

**Files:**
- None (verification only)

**Step 1: Run full test suite**

Run: `cargo test`
Expected: ALL PASS (40+ tests)

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Build release**

Run: `cargo build --release`
Expected: SUCCESS

**Step 4: Final manual test**

Run the release binary with test data:
```bash
target/release/bnotes search --notes-dir /tmp/test-bnotes-search project
```

Expected: Clean output with colors, breadcrumbs, and highlighting

**Step 5: Commit**

```bash
git commit --allow-empty -m "chore: verify enhanced search implementation complete"
```

---

## Completion Checklist

- [ ] All tasks completed
- [ ] All tests passing (unit + integration)
- [ ] Clippy clean (no warnings)
- [ ] Manual testing verified
- [ ] Documentation updated
- [ ] Ready for code review

## Next Steps

After plan execution:
1. Use `superpowers:finishing-a-development-branch` to complete
2. Choose merge/PR option
3. Clean up worktree
