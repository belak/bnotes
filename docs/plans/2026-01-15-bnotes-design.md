# bnotes Design Document

**Date:** 2026-01-15
**Status:** Approved

## Overview

bnotes is a personal note-taking CLI tool that unifies three capabilities:
- Managing markdown notes with flexible organization
- Tracking tasks using GFM checkbox syntax within notes
- Building a knowledge base through wiki-style links

## Design Principles

- **Everything is a markdown file** - No databases, no special formats
- **Parse on demand** - Keep it simple, optimize only if needed
- **YAGNI** - Ship focused features, add complexity only when necessary
- **User controls organization** - Support any directory structure

## Data Model

### File Structure

Notes are markdown files stored in a user-configured directory with any folder structure. All notes are discovered recursively, skipping hidden files/directories (anything starting with `.`).

### Note Anatomy

```markdown
---
tags: [work, project-x]
created: 2026-01-15T10:30:00Z
updated: 2026-01-15T14:20:00Z
---

# My Project Notes

Some thoughts about the project.

## Tasks
- [ ] Review the proposal
- [x] Set up repository
- [ ] Write initial design

## Related Ideas
See also [[Architecture Decisions]] and [[Meeting Notes 2026-01-10]].
```

### Frontmatter (all optional)

- `title`: Display title override (rarely needed)
- `tags`: List of tags for filtering
- `created`: ISO 8601 datetime
- `updated`: ISO 8601 datetime
- Custom fields allowed and ignored

### Title Resolution

Title is determined by (in priority order):
1. Frontmatter `title` field (if present)
2. First H1 heading in the document (if present)
3. Filename without .md extension (fallback)

This avoids duplication - most notes just use the H1 as title.

### Tasks

Standard GFM checkboxes anywhere in the note:
- `- [ ]` = open task
- `- [x]` = completed task
- Tasks inherit context from the note they're in
- Task IDs: `<filename-without-extension>#<index>` (e.g., `project-notes#2`)
- Index is 1-based, counts tasks within that file only

### Wiki Links

`[[Title]]` creates links between notes:
- Case-insensitive title matching
- One match: resolve it
- Multiple matches: warn about ambiguity
- No matches: note broken link

## CLI Structure

### Top-level Commands

- `bnotes search <query>` - Full-text search across all notes
- `bnotes new [title] [--template NAME]` - Create a new note
- `bnotes edit <title>` - Open a note in $EDITOR
- `bnotes tasks` - Alias for `task list --status open`
- `bnotes init` - Create initial config file
- `bnotes doctor` - Lint the note collection

### Note Subcommands

- `bnotes note list [--tag TAG]...` - List all notes, optionally filtered by tags
- `bnotes note show <title>` - Display a note
- `bnotes note links <title>` - Show both outbound and inbound links for a note
- `bnotes note graph` - Show link graph of all notes

### Task Subcommands

- `bnotes task list [--tag TAG]... [--status open|done]` - List tasks across all notes
- `bnotes task show <task-id>` - Show task with context (surrounding lines, heading)

## Configuration

### Config File

**Location (priority order):**
1. `--config` CLI argument
2. `$BNOTES_CONFIG` environment variable
3. `~/.config/bnotes/config.toml` (default)

**Structure:**
```toml
# Path to your notes directory
notes_dir = "~/notes"

# Default editor (falls back to $EDITOR env var)
editor = "nvim"

# Template directory (relative to notes_dir)
template_dir = ".templates"
```

### Templates

Templates are markdown files in the template directory (hidden by default at `.templates/`).

**Template rendering:**
- Simple string replacement (no template engine needed for v1)
- Available placeholders:
  - `{{title}}` - The note title
  - `{{date}}` - Current date (ISO format)
  - `{{datetime}}` - Current datetime (ISO format)

**Example template (`.templates/daily.md`):**
```markdown
---
tags: [daily]
created: {{datetime}}
updated: {{datetime}}
---

# {{title}}

## What I did today

## Tasks
- [ ]

## Notes
```

## Core Behaviors

### Note Discovery

Recursively walks `notes_dir` for `*.md` files, skipping anything starting with `.` (hidden files/directories). This automatically excludes `.git`, `.templates`, and other hidden content.

### Title Matching

For commands like `show`, `edit`, `links`:
1. Parse all notes to extract titles
2. Case-insensitive matching
3. One match: use it
4. Multiple matches: list them and ask user to be more specific
5. No matches: report "Note not found"

### Search Behavior

Case-insensitive substring matching across:
- Note content (body)
- Frontmatter title
- Tags

Returns matching notes with snippets showing match locations.

### Doctor Checks

- Broken wiki links (links to non-existent notes)
- Notes without any tags
- Notes missing frontmatter entirely
- Multiple notes with the same title (causes ambiguity)
- Orphaned notes (no incoming/outgoing links, no tags)

## Error Handling

### File Operations

- `new`: If filename exists, error and don't overwrite
- `edit`/`show`: If title matches multiple notes, list them and exit
- Template not found: Error with helpful message listing available templates

### Invalid Frontmatter

If a note has malformed YAML frontmatter:
- Log warning but don't fail
- Skip that note's metadata
- Fall back to title extraction from H1 or filename
- Continue processing other notes

### Missing Config

If no config file found:
- Error: "No config found. Run `bnotes init` to create one."

### Init Command

- Creates `~/.config/bnotes/config.toml` with defaults
- Prompts for `notes_dir` location (or takes `--notes-dir` flag)
- Creates notes directory if it doesn't exist
- Creates `.templates` directory with example template
- If config exists: ask to overwrite or exit

### Missing Notes Directory

Most commands error gracefully: "Notes directory not found: ~/notes"
Only `init` creates directories.

### Empty Notes Directory

Commands like `list`, `search`, `tasks` return empty results (not an error).

## Implementation Architecture

### Project Structure

```
src/
  main.rs           # CLI entry point, command routing
  config.rs         # Config file parsing, resolution
  note.rs           # Note struct, parsing, title extraction
  repository.rs     # Note discovery, file walking
  search.rs         # Search implementation
  task.rs           # Task extraction from notes
  link.rs           # Wiki link parsing, graph building
  doctor.rs         # Linting checks
  template.rs       # Template rendering
```

### Dependencies

- `clap` - CLI argument parsing with subcommands
- `serde` / `toml` - Config file parsing
- `serde_yaml` - YAML frontmatter parsing
- `pulldown-cmark` - Markdown parsing (with `ENABLE_WIKILINKS` option)
- `walkdir` - Recursive directory traversal
- `chrono` - DateTime parsing and handling
- `anyhow` - Error handling

### Core Data Structures

```rust
struct Note {
    path: PathBuf,
    title: String,
    tags: Vec<String>,
    created: Option<DateTime<Utc>>,
    updated: Option<DateTime<Utc>>,
    content: String,
}

struct Task {
    note_path: PathBuf,
    index: usize,
    completed: bool,
    text: String,
    context: String,  // Surrounding lines
}

struct Config {
    notes_dir: PathBuf,
    editor: String,
    template_dir: PathBuf,
}
```

### Parse-on-Demand Approach

Each command:
1. Loads config
2. Walks notes directory
3. Parses only what it needs (e.g., `task list` skips notes without checkboxes)
4. Outputs results

No caching or indexing for v1 - keep it simple.

### Testing Strategy

- Unit tests for parsing (frontmatter, tasks, links)
- Integration tests with temp directories containing sample notes
- No mocks needed - simple file operations

## Future Considerations

Features explicitly deferred for later:
- Publishing notes to a server
- Interactive TUI mode
- Task completion commands (`task complete`, `task toggle`)
- Tag listing command
- Note deletion command
- Performance optimization with caching/indexing
