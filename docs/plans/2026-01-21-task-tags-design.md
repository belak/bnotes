# Task Tags Design

**Date**: 2026-01-21
**Status**: Approved

## Overview

Add ability to tag individual tasks with context/category labels using `@tag` syntax, separate from note-level tags. Support hierarchical tags and filtering.

## Tag Syntax

**Format:**
- Syntax: `@` followed by any non-whitespace characters
- Position: Only at the end of task text (after urgency, priority, and main task description)
- Multiple tags: Space-separated
- Hierarchical: Use `/` for hierarchy (e.g., `@work/planning`, `@project/admin`)
- Case-insensitive: `@Work` = `@work` = `@WORK`

**Task structure:**
```markdown
[urgency] [priority] task description @tag1 @tag2 @tag3
```

**Examples:**
```markdown
- [ ] !!! (P0) Review PR for authentication @work @urgent
- [ ] Buy groceries @home @errands
- [ ] !! Prepare Q2 roadmap @work/planning @q2
- [ ] Call dentist @personal @health
- [ ] Fix bug in API @work/backend @high-priority
```

## Data Model

**Changes to `Task` struct in `src/note.rs`:**

```rust
pub struct Task {
    pub note_path: PathBuf,
    pub note_title: String,
    pub index: usize,
    pub completed: bool,
    pub text: String,           // Task text with tags removed
    pub priority: Option<String>,
    pub urgency: Option<String>,
    pub tags: Vec<String>,      // NEW: Extracted tags (normalized to lowercase, without @ prefix)
}
```

**Tag storage:**
- Tags stored as `Vec<String>` without the `@` prefix
- Normalized to lowercase for case-insensitive matching
- Deduplicated (if `@work @work` appears, store only `["work"]`)
- Empty vec if no tags
- Examples:
  - `@Work @URGENT` → `vec!["work", "urgent"]`
  - `@work/planning @q2` → `vec!["work/planning", "q2"]`

## Parsing

### Tag Extraction

**New function in `src/note.rs`:**

```rust
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
            break;  // Stop at first non-tag
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
```

### Updated Task Parsing

**Update `Task::parse_urgency_and_priority()` → `Task::parse_task_metadata()`:**

```rust
/// Parse all metadata from task text
/// Returns (urgency, priority, tags, remaining_text)
fn parse_task_metadata(text: &str) -> (Option<String>, Option<String>, Vec<String>, String) {
    // Parse urgency first
    let (urgency, rest) = parse_urgency(text);

    // Parse priority next
    let (priority, rest) = parse_priority(rest);

    // Parse tags from end
    let (tags, text) = parse_tags(rest);

    (urgency, priority, tags, text)
}
```

**Update `Task::extract_from_note()`:**

Use `parse_task_metadata()` to populate all fields including tags.

## CLI Interface

**Add `--tag` flag to task commands:**

```bash
bnotes tasks [OPTIONS]
bnotes task list [OPTIONS]

Options:
  --note <PATTERN>        Filter by note name (supports * wildcard)
  --tag <TAG>             Filter by tag (can be specified multiple times)
  --status <STATUS>       Filter by status: open, done, all
  --sort-order <ORDER>    Sort order [default: urgency,priority,id]
```

**Tag filtering behavior:**
- Multiple `--tag` flags = AND logic (task must have all specified tags)
- Case-insensitive matching: `--tag Work` matches `@work`, `@WORK`, `@Work`
- Hierarchical prefix matching: `--tag work` matches `@work` AND `@work/planning`
- Tags specified without `@` prefix: `--tag work` (not `--tag @work`)
- Deduplicate filter tags: `--tag work --tag work` treated as single filter

**Hierarchical matching logic:**
- Filter tag is a prefix match on stored tags
- `--tag work` matches: `@work`, `@work/urgent`, `@work/planning/q2`
- `--tag work/urgent` matches: `@work/urgent`, `@work/urgent/high`
- `--tag work/urgent` does NOT match: `@work`, `@work/planning`

**Example usage:**
```bash
# Tasks tagged with work (including subtags)
bnotes tasks --tag work              # matches @work, @work/urgent, @work/planning

# Tasks with specific work context
bnotes tasks --tag work/planning     # matches @work/planning only

# Tasks with both work and urgent tags (AND logic)
bnotes tasks --tag work --tag urgent

# Combine with note and status filters
bnotes tasks --note '2026-W*' --tag work --status open

# Case-insensitive
bnotes tasks --tag Work              # same as --tag work
```

## Implementation

### 1. Update CLI Argument Parsing (`src/main.rs`)

**Add `--tag` to both commands:**

```rust
Tasks {
    /// Filter by note name (supports * wildcard)
    #[arg(long)]
    note: Option<String>,

    /// Filter by tag (can be specified multiple times)
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Filter by status (open, done, all)
    #[arg(long, default_value = "open")]
    status: String,

    /// Sort order: comma-separated fields (urgency, priority, id)
    #[arg(long, default_value = "urgency,priority,id")]
    sort_order: String,
},

TaskCommands::List {
    /// Filter by note name (supports * wildcard)
    #[arg(long)]
    note: Option<String>,

    /// Filter by tag (can be specified multiple times)
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Filter by status (open, done, all)
    #[arg(long)]
    status: Option<String>,

    /// Sort order: comma-separated fields (urgency, priority, id)
    #[arg(long, default_value = "urgency,priority,id")]
    sort_order: String,
},
```

**Update command handlers to pass tags:**

```rust
Commands::Tasks { note, tags, status, sort_order } => {
    let sort_order = bnotes::TaskSortOrder::parse(&sort_order)?;
    cli::commands::task_list(&notes_dir, &tags, Some(status), note.as_deref(), sort_order, cli_args.color)?;
}

TaskCommands::List { note, tags, status, sort_order } => {
    let sort_order = bnotes::TaskSortOrder::parse(&sort_order)?;
    cli::commands::task_list(&notes_dir, &tags, status, note.as_deref(), sort_order, cli_args.color)?;
}
```

### 2. Update Task List Function (`src/cli/commands.rs`)

**Update `task_list()` signature:**

Already accepts `tags: &[String]` parameter (currently unused for note filtering).

**Add tag filtering logic after note filtering:**

```rust
// Filter by tags if provided (AND logic with hierarchical matching)
if !tags.is_empty() {
    // Normalize and deduplicate filter tags
    let mut filter_tags: Vec<String> = tags.iter()
        .map(|t| t.to_lowercase())
        .collect();
    filter_tags.sort();
    filter_tags.dedup();

    tasks.retain(|task| {
        filter_tags.iter().all(|filter_tag| {
            task.tags.iter().any(|task_tag| {
                // Hierarchical: task_tag equals or starts with filter_tag/
                task_tag == filter_tag || task_tag.starts_with(&format!("{}/", filter_tag))
            })
        })
    });
}
```

**Update display to show tags:**

After task text, before "(from note)", show tags with `@` prefix:

```rust
// Task text
write!(stdout, "{} ", task.text)?;

// Tags (if any)
if !task.tags.is_empty() {
    stdout.set_color(&colors::highlight())?; // Cyan, same as task ID
    for tag in &task.tags {
        write!(stdout, "@{} ", tag)?;
    }
    stdout.reset()?;
}

// "from [note]" in dim
stdout.set_color(&colors::dim())?;
writeln!(stdout, "(from {})", task.note_title)?;
stdout.reset()?;
```

### 3. Update Note Parsing (`src/note.rs`)

**Add `tags` field to `Task` struct**

**Implement `parse_tags()` function** (as shown in Parsing section above)

**Update `Task::extract_from_note()`:**

Replace call to `parse_urgency_and_priority()` with `parse_task_metadata()` and populate tags field.

### 4. Update Library API (`src/lib.rs`)

No changes needed - `list_tasks()` already accepts tags parameter (currently used for note-level tag filtering). The CLI will now use it for task-level tag filtering instead.

## Display Format

**Task list with tags:**

```
2026-W04#1  [ ]  !!!  (P0)  Review PR for authentication @work @urgent (from 2026-W04)
2026-W04#2  [ ]  !!!  (P0)  Gov Migration Cohorts @work (from 2026-W04)
2026-W04#3  [ ]   !!  (P0)  IL5 Distributor work @work/backend (from 2026-W04)
2026-W04#4  [ ]   !!  (P1)  Finish targeted distribution planning @work/planning (from 2026-W04)
2026-W04#5  [ ]    !  (P3)  Wheel of Misfortune planning @personal (from 2026-W04)
```

**Color scheme:**
- Task ID: Cyan (highlight)
- Checkbox: Green if completed, default otherwise
- Urgency/Priority: Default
- Task text: Default
- **Tags: Cyan (highlight) - same as task ID**
- Source note: Dim

## Testing

### Manual Verification

```bash
# Create test notes with tagged tasks
echo '- [ ] Task one @work' >> test-note.md
echo '- [ ] Task two @work @urgent' >> test-note.md
echo '- [ ] Task three @work/planning' >> test-note.md
echo '- [ ] Task four @personal' >> test-note.md
echo '- [ ] Task five @work @work @urgent' >> test-note.md  # Duplicate tags

# Build and run
cargo build
cargo test

# Test filtering
bnotes tasks --tag work              # Should show tasks 1, 2, 3, 5
bnotes tasks --tag work/planning     # Should show task 3 only
bnotes tasks --tag work --tag urgent # Should show task 2 and 5
bnotes tasks --tag personal          # Should show task 4 only

# Test case-insensitive
bnotes tasks --tag Work              # Should match @work
bnotes tasks --tag WORK              # Should match @work

# Test deduplication
bnotes tasks --tag work --tag work   # Same as --tag work (deduplicated)
bnotes tasks --tag Work --tag work   # Same as --tag work (deduplicated)

# Test display - task 5 should show @work @urgent (not @work @work @urgent)
bnotes tasks

# Test combined filters
bnotes tasks --note '2026-W*' --tag work --status open
```

### Edge Cases

- Tasks with no tags → display normally, no tags shown
- Tags with special characters: `@high-priority`, `@work/urgent/now`
- Multiple tags in different order: `@work @urgent` vs `@urgent @work` (both valid)
- Tag at end after punctuation: `Review PR. @work` (should parse correctly)
- Duplicate tags in task markdown: `@work @work` → stored as `["work"]`
- Duplicate filter tags specified by user: `--tag work --tag work` → single filter
- Empty tag: `@` alone → ignore or handle gracefully
- Tag with only `/`: `@/` → handle gracefully

### Unit Tests

Add tests in `src/note.rs`:

```rust
#[test]
fn test_parse_tags() {
    // Basic tags
    let (tags, text) = parse_tags("Task text @work @urgent");
    assert_eq!(tags, vec!["work", "urgent"]);
    assert_eq!(text, "Task text");

    // Hierarchical tags
    let (tags, text) = parse_tags("Task @work/planning");
    assert_eq!(tags, vec!["work/planning"]);

    // Case-insensitive
    let (tags, text) = parse_tags("Task @Work @URGENT");
    assert_eq!(tags, vec!["work", "urgent"]);

    // Duplicate tags
    let (tags, text) = parse_tags("Task @work @work @urgent");
    assert_eq!(tags, vec!["work", "urgent"]);

    // No tags
    let (tags, text) = parse_tags("Task text");
    assert_eq!(tags, Vec::<String>::new());
    assert_eq!(text, "Task text");

    // Tags in middle are not parsed (only at end)
    let (tags, text) = parse_tags("Task @work more text");
    assert_eq!(tags, Vec::<String>::new());
    assert_eq!(text, "Task @work more text");
}

#[test]
fn test_task_with_tags() {
    // Test full task parsing with urgency, priority, and tags
    let content = "- [ ] !!! (P0) Review PR @work @urgent";
    // ... parse and verify Task struct has correct fields
}
```

## Design Rationale

**Why @ syntax:**
- Widely recognized from GTD tools (Things, OmniFocus)
- Intuitive for categorization
- Low conflict risk (email addresses rare in task text)

**Why tags at end:**
- Consistent structure: urgency/priority at start, context at end
- Common pattern in productivity tools
- Easy to parse and strip for display

**Why case-insensitive:**
- More user-friendly (no need to remember exact casing)
- Matches note-level tag behavior
- Consistent with other filtering (note names)

**Why hierarchical prefix matching:**
- Predictable behavior (parent matches children)
- Semantic clarity (@work/planning is work-related)
- Precise control while allowing broad queries

**Why AND logic for multiple tags:**
- More useful for narrowing down tasks
- Example: `--tag work --tag urgent` finds urgent work tasks
- Can always run separate queries for OR logic

**Why deduplicate tags:**
- Cleaner display
- Prevents confusion
- No semantic difference between `@work @work` and `@work`

**Why store tags separately:**
- Faster filtering (no regex on every operation)
- Easier to display/format tags distinctly
- Consistent with urgency/priority approach

## Future Enhancements

- OR logic for tags: `--tag work --or --tag personal`
- Tag exclusion: `--not-tag urgent`
- List all used tags: `bnotes task tags`
- Tag statistics: show task counts per tag
- Tag renaming/management commands
- Auto-complete for tags in CLI
- Color-coded tags (different colors for different tag types)
